//! The source code of the identity contract.

#![cfg_attr(not(feature = "std"), no_std, no_main)]

use ink::prelude::{string::String, vec::Vec};
#[cfg(test)]
mod tests;

mod types;

use common::ensure;

pub use self::identity::{Identity, IdentityRef};

/// Encrypted addresses should never exceed this size limit.
const ADDRESS_SIZE_LIMIT: usize = 128;

/// Limit the name length of a network.
const NETWORK_NAME_LIMIT: usize = 16;

/// Limit the rpc url length of a network.
const NETWORK_RPC_URL_LIMIT: usize = 64;

/// All the possible errors that may occur when interacting with the identity
/// contract.
#[derive(scale::Encode, scale::Decode, Debug, PartialEq)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum Error {
	NotAllowed,
	IdentityDoesntExist,
	AddressAlreadyAdded,
	InvalidNetwork,
	AddressSizeExceeded,
	NetworkNameTooLong,
	NetworkRpcUrlTooLong,
	AlreadyIdentityOwner,
}

#[ink::contract]
mod identity {
	use super::*;
	use crate::types::*;
	use common::types::{NetworkInfo, Ss58Prefix, *};
	use ink::storage::Mapping;

	/// Storage
	#[ink(storage)]
	pub struct Identity {
		/// Each identity is associated with its own unique `IdentityNo`.
		pub(crate) number_to_identity: Mapping<IdentityNo, IdentityInfo>,

		/// Each identity must have an owner.
		pub(crate) owner_of: Mapping<IdentityNo, AccountId>,

		/// Mapping an `AccountId` to an `IdentityNo`. An account can only have
		/// one identity.
		pub(crate) identity_of: Mapping<AccountId, IdentityNo>,

		/// The recovery account of a specific identity. This account has the
		/// power to transfer the ownership of an identity to another account.
		///
		/// WARNING: It is recommended to have a recovery account specified
		/// since otherwise if you lose access to the account that owns the
		/// identity you won't be able to make any changes to your identity.
		pub(crate) recovery_account_of: Mapping<IdentityNo, AccountId>,

		/// `IdentityNo`s are incremented every time a new identity is created
		/// so this storage value keeps track of that.
		pub(crate) latest_identity_no: IdentityNo,

		/// The network information associated with a specific `NetworkId`.
		pub(crate) network_info_of: Mapping<NetworkId, NetworkInfo>,

		/// The admin account has the ability to update the list of supported
		/// networks that can be used in Dotflow.
		///
		/// In the future it could be a good idea to have this controlled by
		/// governance.
		pub(crate) admin: AccountId,

		/// Keeps track of the lastest `NetworkId`. Gets incremented whenever a
		/// new network is added.
		pub(crate) network_id_count: NetworkId,
	}

	/// Events
	#[ink(event)]
	pub struct IdentityCreated {
		/// Owner of the created identity.
		#[ink(topic)]
		pub(crate) owner: AccountId,
		/// The `IdentityNo` associated with the created identity.
		pub(crate) identity_no: IdentityNo,
	}

	#[ink(event)]
	pub struct AddressAdded {
		/// The `IdentityNo` of the identity that got updated.
		#[ink(topic)]
		pub(crate) identity_no: IdentityNo,
		/// The network on which a new address has been added.
		pub(crate) network: NetworkId,
		/// The newly added address.
		pub(crate) address: NetworkAddress,
	}

	#[ink(event)]
	pub struct AddressUpdated {
		/// The `IdentityNo` of the identity that got updated.
		#[ink(topic)]
		pub(crate) identity_no: IdentityNo,
		/// The network on which the address has been updated.
		pub(crate) network: NetworkId,
		/// The updated address value.
		pub(crate) updated_address: NetworkAddress,
	}

	#[ink(event)]
	pub struct AddressRemoved {
		/// The `IdentityNo` of the identity that got updated.
		#[ink(topic)]
		pub(crate) identity_no: IdentityNo,
		/// The network on which the address has been removed.
		pub(crate) network: NetworkId,
	}

	#[ink(event)]
	pub struct IdentityRemoved {
		/// The `IdentityNo` of the identity that got removed.
		#[ink(topic)]
		pub(crate) identity_no: IdentityNo,
	}

	#[ink(event)]
	pub struct NetworkAdded {
		/// The `NetworkId` that is associated with the newly added network.
		#[ink(topic)]
		pub(crate) network_id: NetworkId,
		/// The name of the network name that got added.
		pub(crate) name: String,
		/// The `Ss58Prefix`  of the network that got added.
		pub(crate) ss58_prefix: Ss58Prefix,
		/// The rpc url of the network that got added.
		pub(crate) rpc_url: String,
	}

	#[ink(event)]
	pub struct NetworkUpdated {
		/// The `NetworkId` that is associated with the updated network.
		#[ink(topic)]
		pub(crate) network_id: NetworkId,
		/// The name of the updated network.
		pub(crate) name: String,
		/// The `Ss58Prefix` of the updated network.
		pub(crate) ss58_prefix: Ss58Prefix,
		/// The rpc url of the updated network.
		pub(crate) rpc_url: String,
	}

	#[ink(event)]
	pub struct NetworkRemoved {
		/// The `NetworkId` that is associated with the network that got
		/// removed.
		#[ink(topic)]
		pub(crate) network_id: NetworkId,
	}

	#[ink(event)]
	pub struct RecoveryAccountSet {
		/// The `IdentityNo` of the identity that set a recovery account.
		#[ink(topic)]
		pub(crate) identity_no: IdentityNo,
		/// The newly set recovery account.
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
				network_info_of: Default::default(),
				recovery_account_of: Default::default(),
				admin: caller,
				network_id_count: 0,
			}
		}

		#[ink(constructor)]
		pub fn init_with_networks(networks: Vec<NetworkInfo>) -> Self {
			let mut network_info_of = Mapping::default();

			// Iterate over all the networks provided and make sure that no
			// fields are exceeding the length limits.
			networks.clone().into_iter().enumerate().for_each(|(network_id, network)| {
				assert!(network.name.len() <= NETWORK_NAME_LIMIT, "Network name is too long");
				assert!(
					network.rpc_url.len() <= NETWORK_RPC_URL_LIMIT,
					"Network rpc url is too long"
				);
				let network_id = network_id as NetworkId;
				network_info_of.insert(network_id, &network);
			});

			let caller = Self::env().caller();
			Self {
				number_to_identity: Default::default(),
				owner_of: Default::default(),
				identity_of: Default::default(),
				latest_identity_no: 0,
				network_info_of,
				network_id_count: networks.len() as NetworkId,
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

		/// Returns the network name that is associated with the specified `NetworkId`.
		#[ink(message)]
		pub fn network_info_of(&self, network_id: NetworkId) -> Option<NetworkInfo> {
			self.network_info_of.get(network_id)
		}

		/// Returns the destination address of a transaction that needs to be
		/// routed to the specified identity on the specified network.
		#[ink(message)]
		pub fn transaction_destination(
			&self,
			receiver: IdentityNo,
			network: NetworkId,
		) -> Result<NetworkAddress, Error> {
			let receiver_identity = self
				.number_to_identity
				.get(receiver)
				.map_or(Err(Error::IdentityDoesntExist), Ok)?;

			match receiver_identity.addresses.into_iter().find(|(id, _)| *id == network) {
				Some((_, address)) => Ok(address),
				None => Err(Error::InvalidNetwork),
			}
		}

		/// A list of all the available networks each associated with a `NetworkId`.
		#[ink(message)]
		pub fn available_networks(&self) -> Vec<(NetworkId, NetworkInfo)> {
			(0..self.network_id_count)
				.map(|id| (id, self.network_info_of(id)))
				.filter_map(|(id, maybe_network)| maybe_network.map(|info| (id, info)))
				.collect()
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
		#[ink(message)]
		pub fn add_address(
			&mut self,
			network: NetworkId,
			address: NetworkAddress,
		) -> Result<(), Error> {
			let caller = self.env().caller();

			let identity_no = self.identity_of.get(caller).map_or(Err(Error::NotAllowed), Ok)?;

			let mut identity_info = self.get_identity_info_of_caller(caller)?;

			identity_info.add_address(network, address.clone())?;
			self.number_to_identity.insert(identity_no, &identity_info);

			self.env().emit_event(AddressAdded { identity_no, network, address });

			Ok(())
		}

		/// Updates the address of the given network
		#[ink(message)]
		pub fn update_address(
			&mut self,
			network: NetworkId,
			address: NetworkAddress,
		) -> Result<(), Error> {
			let caller = self.env().caller();

			let identity_no = self.identity_of.get(caller).map_or(Err(Error::NotAllowed), Ok)?;

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
		#[ink(message)]
		pub fn remove_address(&mut self, network: NetworkId) -> Result<(), Error> {
			let caller = self.env().caller();

			let identity_no = self.identity_of.get(caller).map_or(Err(Error::NotAllowed), Ok)?;

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

			let identity_no = self.identity_of.get(caller).map_or(Err(Error::NotAllowed), Ok)?;

			self.identity_of.remove(caller);
			self.owner_of.remove(identity_no);
			self.number_to_identity.remove(identity_no);

			self.env().emit_event(IdentityRemoved { identity_no });

			Ok(())
		}

		#[ink(message)]
		pub fn add_network(&mut self, info: NetworkInfo) -> Result<NetworkId, Error> {
			let caller = self.env().caller();

			// Only the contract owner can add a network
			ensure!(caller == self.admin, Error::NotAllowed);

			// Ensure that no fields are exceeding the length limits.
			ensure!(info.name.len() <= NETWORK_NAME_LIMIT, Error::NetworkNameTooLong);
			ensure!(info.rpc_url.len() <= NETWORK_RPC_URL_LIMIT, Error::NetworkRpcUrlTooLong);

			let network_id = self.network_id_count;
			self.network_info_of.insert(network_id, &info);

			self.network_id_count = self.network_id_count.saturating_add(1);

			let NetworkInfo { name, ss58_prefix, rpc_url } = info;

			self.env().emit_event(NetworkAdded { network_id, name, ss58_prefix, rpc_url });

			Ok(network_id)
		}

		#[ink(message)]
		pub fn update_network(
			&mut self,
			network_id: NetworkId,
			new_prefix: Option<Ss58Prefix>,
			new_name: Option<String>,
			new_rpc_url: Option<String>,
		) -> Result<(), Error> {
			let caller = self.env().caller();

			// Only the contract owner can update a network
			ensure!(caller == self.admin, Error::NotAllowed);

			// Ensure that the given network id exists
			let mut info =
				self.network_info_of.get(network_id).map_or(Err(Error::InvalidNetwork), Ok)?;

			// Ensure that the name of the network doesn't exceed length limit.
			if let Some(name) = new_name {
				ensure!(name.len() <= NETWORK_NAME_LIMIT, Error::NetworkNameTooLong);
				info.name = name;
			}

			// Ensure that the rpc url of the network doesn't exceed length limit.
			if let Some(rpc_url) = new_rpc_url {
				ensure!(rpc_url.len() <= NETWORK_RPC_URL_LIMIT, Error::NetworkRpcUrlTooLong);
				info.rpc_url = rpc_url;
			}

			if let Some(prefix) = new_prefix {
				info.ss58_prefix = prefix;
			}

			// Update storage items
			self.network_info_of.insert(network_id, &info);

			self.env().emit_event(NetworkUpdated {
				network_id,
				name: info.name,
				ss58_prefix: info.ss58_prefix,
				rpc_url: info.rpc_url,
			});

			Ok(())
		}

		#[ink(message)]
		pub fn remove_network(&mut self, network_id: NetworkId) -> Result<(), Error> {
			let caller = self.env().caller();

			// Only the contract owner can update a network
			ensure!(caller == self.admin, Error::NotAllowed);

			// Ensure that the given `network_id` exists
			let network = self.network_info_of.get(network_id);
			ensure!(network.is_some(), Error::InvalidNetwork);

			self.network_info_of.remove(network_id);

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

			let identity_no = self.identity_of.get(caller).map_or(Err(Error::NotAllowed), Ok)?;

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
			let identity_no =
				self.identity_of.get(caller).map_or(Err(Error::IdentityDoesntExist), Ok)?;

			let identity_info = self.number_to_identity.get(identity_no);

			// This is a defensive check. The identity info should always exist
			// when the identity no associated to it is stored in the
			// `identity_of` mapping.
			ensure!(identity_info.is_some(), Error::IdentityDoesntExist);

			let identity_info = identity_info.expect(
				"The identity info must exist if an `IdentityNo` is associated with it; qed",
			);

			Ok(identity_info)
		}
	}

	#[cfg(all(test, feature = "e2e-tests"))]
	mod e2e_tests {}
}
