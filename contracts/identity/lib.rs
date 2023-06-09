#![cfg_attr(not(feature = "std"), no_std, no_main)]

use ink::prelude::{string::String, vec::Vec};

#[cfg(feature = "std")]
use ink::storage::traits::StorageLayout;

#[cfg(test)]
mod tests;

macro_rules! ensure {
	( $x:expr, $y:expr $(,)? ) => {{
		if !$x {
			return Err($y)
		}
	}};
}

/// Encrypted addresses should never exceed this size limit.
const ADDRESS_SIZE_LIMIT: usize = 128;

/// Limit the name length of a network
const NETWORK_NAME_LIMIT: usize = 128;

/// Each identity will be associated with a unique identifier called `IdentityNo`.
pub type IdentityNo = u32;

/// We want to keep the address type very generic since we want to support any
/// address format. We won't actually keep the addresses in the contract itself.
/// Before storing them, we'll encrypt them to ensure privacy.
pub type Address = Vec<u8>;

/// Used to represent any blockchain in the Polkadot, Kusama or Rococo network.
pub type NetworkId = u32;

#[derive(scale::Encode, scale::Decode, Debug, Default, PartialEq)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo, StorageLayout))]
pub struct IdentityInfo {
	/// Each address is associated with a specific blockchain.
	pub(crate) addresses: Vec<(NetworkId, Address)>,
}

#[derive(scale::Encode, scale::Decode, Debug, PartialEq)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum Error {
	NotAllowed,
	IdentityDoesntExist,
	AddressAlreadyAdded,
	InvalidNetwork,
	AddressSizeExceeded,
	NetworkNameTooLong,
	AlreadyIdentityOwner,
}

impl IdentityInfo {
	/// Adds an address for the given network
	pub fn add_address(&mut self, network: NetworkId, address: Address) -> Result<(), Error> {
		ensure!(address.len() <= ADDRESS_SIZE_LIMIT, Error::AddressSizeExceeded);

		ensure!(
			!self.addresses.clone().into_iter().any(|address| address.0 == network),
			Error::AddressAlreadyAdded
		);
		self.addresses.push((network, address));

		Ok(())
	}

	/// Updates the address of the given network
	pub fn update_address(
		&mut self,
		network: NetworkId,
		new_address: Address,
	) -> Result<(), Error> {
		ensure!(new_address.len() <= ADDRESS_SIZE_LIMIT, Error::AddressSizeExceeded);

		if let Some(position) =
			self.addresses.clone().into_iter().position(|address| address.0 == network)
		{
			self.addresses[position] = (network, new_address);
			Ok(())
		} else {
			Err(Error::InvalidNetwork)
		}
	}

	/// Remove an address record by network
	pub fn remove_address(&mut self, network: NetworkId) -> Result<(), Error> {
		let old_count = self.addresses.len();
		self.addresses.retain(|(net, _)| *net != network);

		let new_count = self.addresses.len();

		if old_count == new_count {
			Err(Error::InvalidNetwork)
		} else {
			Ok(())
		}
	}
}

#[ink::contract]
mod identity {
	use super::*;
	use ink::storage::Mapping;

	/// Storage
	#[ink(storage)]
	pub struct Identity {
		pub(crate) number_to_identity: Mapping<IdentityNo, IdentityInfo>,
		pub(crate) owner_of: Mapping<IdentityNo, AccountId>,
		pub(crate) identity_of: Mapping<AccountId, IdentityNo>,
		pub(crate) recovery_account_of: Mapping<IdentityNo, AccountId>,
		pub(crate) latest_identity_no: IdentityNo,
		pub(crate) network_name: Mapping<NetworkId, String>,
		pub(crate) network_id_counter: NetworkId,
		pub(crate) admin: AccountId,
	}

	/// Events
	#[ink(event)]
	pub struct IdentityCreated {
		#[ink(topic)]
		pub(crate) owner: AccountId,
		pub(crate) identity_no: IdentityNo,
	}

	#[ink(event)]
	pub struct AddressAdded {
		#[ink(topic)]
		pub(crate) identity_no: IdentityNo,
		pub(crate) network: NetworkId,
		pub(crate) address: Address,
	}

	#[ink(event)]
	pub struct AddressUpdated {
		#[ink(topic)]
		pub(crate) identity_no: IdentityNo,
		pub(crate) network: NetworkId,
		pub(crate) updated_address: Address,
	}

	#[ink(event)]
	pub struct AddressRemoved {
		#[ink(topic)]
		pub(crate) identity_no: IdentityNo,
		pub(crate) network: NetworkId,
	}

	#[ink(event)]
	pub struct IdentityRemoved {
		#[ink(topic)]
		pub(crate) identity_no: IdentityNo,
	}

	#[ink(event)]
	pub struct NetworkAdded {
		#[ink(topic)]
		pub(crate) network_id: NetworkId,
		pub(crate) name: String,
	}

	#[ink(event)]
	pub struct NetworkUpdated {
		#[ink(topic)]
		pub(crate) network_id: NetworkId,
		pub(crate) name: String,
	}

	#[ink(event)]
	pub struct NetworkRemoved {
		#[ink(topic)]
		pub(crate) network_id: NetworkId,
	}

	#[ink(event)]
	pub struct RecoveryAccountSet {
		#[ink(topic)]
		pub(crate) identity_no: IdentityNo,
		pub(crate) recovery_account: AccountId,
	}

	impl Default for Identity {
		fn default() -> Self {
			Self::new()
		}
	}

	impl Identity {
		#[ink(constructor)]
		pub fn new() -> Self {
			let caller = Self::env().caller();
			Self {
				number_to_identity: Default::default(),
				owner_of: Default::default(),
				identity_of: Default::default(),
				latest_identity_no: 0,
				network_name: Default::default(),
				network_id_counter: 0,
				recovery_account_of: Default::default(),
				admin: caller,
			}
		}

		/// Returns the `IdentityInfo` of an identity that is associated with
		/// the provided `IdentityNo`.
		#[ink(message)]
		pub fn identity(&self, identity_no: IdentityNo) -> Option<IdentityInfo> {
			self.number_to_identity.get(identity_no)
		}

		/// Returns the owner of an identity.
		#[ink(message)]
		pub fn owner_of(&self, identity_no: IdentityNo) -> Option<AccountId> {
			self.owner_of.get(identity_no)
		}

		/// Returns the owner of an identity.
		#[ink(message)]
		pub fn identity_of(&self, owner: AccountId) -> Option<IdentityNo> {
			self.identity_of.get(owner)
		}

		/// Creates an identity and returns the `IdentityNo`.
		///
		/// A user can only create one identity.
		#[ink(message)]
		pub fn create_identity(&mut self) -> Result<IdentityNo, Error> {
			let caller = self.env().caller();

			ensure!(self.identity_of.get(caller).is_none(), Error::AlreadyIdentityOwner);

			let identity_no = self.latest_identity_no;

			let new_identity: IdentityInfo = Default::default();

			self.number_to_identity.insert(identity_no, &new_identity);
			self.identity_of.insert(caller, &identity_no);
			self.owner_of.insert(identity_no, &caller);

			self.latest_identity_no = self.latest_identity_no.saturating_add(1);

			self.env().emit_event(IdentityCreated { owner: caller, identity_no });

			Ok(identity_no)
		}

		/// Adds an address for a given network
		pub fn add_address(&mut self, network: NetworkId, address: Address) -> Result<(), Error> {
			let caller = self.env().caller();
			ensure!(self.identity_of.get(caller).is_some(), Error::NotAllowed);

			let identity_no = self.identity_of.get(caller).unwrap();
			let mut identity_info = self.get_identity_info_of_caller(caller)?;

			identity_info.add_address(network, address.clone())?;
			self.number_to_identity.insert(identity_no, &identity_info);

			self.env().emit_event(AddressAdded { identity_no, network, address });

			Ok(())
		}

		/// Updates the address of the given network
		pub fn update_address(
			&mut self,
			network: NetworkId,
			address: Address,
		) -> Result<(), Error> {
			let caller = self.env().caller();
			ensure!(self.identity_of.get(caller).is_some(), Error::NotAllowed);

			let identity_no = self.identity_of.get(caller).unwrap();
			let mut identity_info = self.get_identity_info_of_caller(caller)?;

			identity_info.update_address(network, address.clone())?;
			self.number_to_identity.insert(identity_no, &identity_info);

			self.env().emit_event(AddressUpdated {
				identity_no,
				network,
				updated_address: address,
			});

			Ok(())
		}

		/// Removes the address by network
		pub fn remove_address(&mut self, network: NetworkId) -> Result<(), Error> {
			let caller = self.env().caller();
			ensure!(self.identity_of.get(caller).is_some(), Error::NotAllowed);

			let identity_no = self.identity_of.get(caller).unwrap();
			let mut identity_info = self.get_identity_info_of_caller(caller)?;

			identity_info.remove_address(network)?;
			self.number_to_identity.insert(identity_no, &identity_info);

			self.env().emit_event(AddressRemoved { identity_no, network });

			Ok(())
		}

		/// Removes an identity
		#[ink(message)]
		pub fn remove_identity(&mut self) -> Result<(), Error> {
			let caller = self.env().caller();
			ensure!(self.identity_of.get(caller).is_some(), Error::NotAllowed);

			let identity_no = self.identity_of.get(caller).unwrap();

			self.identity_of.remove(caller);
			self.owner_of.remove(identity_no);
			self.number_to_identity.remove(identity_no);

			self.env().emit_event(IdentityRemoved { identity_no });

			Ok(())
		}

		#[ink(message)]
		pub fn add_network(&mut self, name: String) -> Result<NetworkId, Error> {
			let caller = self.env().caller();

			// Only the contract owner can add a network
			ensure!(caller == self.admin, Error::NotAllowed);

			// Ensure that the name of the network doesn't exceed length limit
			ensure!(name.len() <= NETWORK_NAME_LIMIT, Error::NetworkNameTooLong);

			let network_id = self.network_id_counter;

			self.network_name.insert(network_id, &name);

			self.network_id_counter = self.network_id_counter.saturating_add(1);

			self.env().emit_event(NetworkAdded { network_id, name });

			Ok(network_id)
		}

		#[ink(message)]
		pub fn update_network(
			&mut self,
			network_id: NetworkId,
			new_name: String,
		) -> Result<(), Error> {
			let caller = self.env().caller();

			// Only the contract owner can update a network
			ensure!(caller == self.admin, Error::NotAllowed);

			// Ensure that the name of the network doesn't exceed length limit
			ensure!(new_name.len() <= NETWORK_NAME_LIMIT, Error::NetworkNameTooLong);

			// Ensure that the given network id exists
			let old_name = self.network_name.get(network_id);
			ensure!(old_name.is_some(), Error::InvalidNetwork);

			// Update storage items
			self.network_name.insert(network_id, &new_name);

			self.env().emit_event(NetworkUpdated { network_id, name: new_name });

			Ok(())
		}

		#[ink(message)]
		pub fn remove_network(&mut self, network_id: NetworkId) -> Result<(), Error> {
			let caller = self.env().caller();

			// Only the contract owner can update a network
			ensure!(caller == self.admin, Error::NotAllowed);

			// Ensure that the given `network_id` exists
			let name = self.network_name.get(network_id);
			ensure!(name.is_some(), Error::InvalidNetwork);

			self.network_name.remove(network_id);

			self.env().emit_event(NetworkRemoved { network_id });

			Ok(())
		}

		/// Sets the recovery account that will be able to change the ownership
		/// of the identity.
		///
		/// Only callable by the identity owner.
		#[ink(message)]
		pub fn set_recovery_account(&mut self, recovery_account: AccountId) -> Result<(), Error> {
			let caller = self.env().caller();
			ensure!(self.identity_of.get(caller).is_some(), Error::NotAllowed);

			let identity_no = self.identity_of.get(caller).unwrap();

			self.recovery_account_of.insert(identity_no, &recovery_account);
			self.env().emit_event(RecoveryAccountSet { identity_no, recovery_account });

			Ok(())
		}

		/// Transfers the ownership of an identity to another account.
		///
		/// Only callable by the identity owner or any account that the identity
		/// owner added as a proxy.
		#[ink(message)]
		pub fn transfer_ownership(
			&mut self,
			identity_no: IdentityNo,
			new_owner: AccountId,
		) -> Result<(), Error> {
			let caller = self.env().caller();

			let is_recovery_account = self.recovery_account_of.get(identity_no) == Some(caller);
			let Some(identity_owner) = self.owner_of(identity_no) else { return Err(Error::NotAllowed) };

			ensure!(identity_owner == caller || is_recovery_account, Error::NotAllowed);
			// The new owner cannot already have an identity since we allow only
			// one identity per account.
			ensure!(self.identity_of(new_owner).is_none(), Error::AlreadyIdentityOwner);

			self.identity_of.remove(identity_owner);
			self.identity_of.insert(new_owner, &identity_no);

			self.owner_of.insert(identity_no, &new_owner);

			Ok(())
		}

		pub fn get_identity_info_of_caller(
			&self,
			caller: AccountId,
		) -> Result<IdentityInfo, Error> {
			let identity_no = self.identity_of.get(caller).unwrap();
			let identity_info = self.number_to_identity.get(identity_no);

			// This is a defensive check. The identity info should always exist
			// when the identity no associated to it is stored in the
			// `identity_of` mapping.
			ensure!(identity_info.is_some(), Error::IdentityDoesntExist);

			let identity_info = identity_info.unwrap();

			Ok(identity_info)
		}
	}

	#[cfg(all(test, feature = "e2e-tests"))]
	mod e2e_tests {}
}
