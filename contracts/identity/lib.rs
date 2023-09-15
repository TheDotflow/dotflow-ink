//! The source code of the identity contract.

#![cfg_attr(not(feature = "std"), no_std, no_main)]

use ink::prelude::vec::Vec;
#[cfg(test)]
mod tests;

mod types;

use common::ensure;

pub use self::identity::{Identity, IdentityRef};

/// Encrypted addresses should never exceed this size limit.
const ADDRESS_SIZE_LIMIT: usize = 128;

/// All the possible errors that may occur when interacting with the identity
/// contract.
#[derive(scale::Encode, scale::Decode, Debug, PartialEq)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum Error {
	NotAllowed,
	IdentityDoesntExist,
	AddressAlreadyAdded,
	InvalidChain,
	AddressSizeExceeded,
	ChainNameTooLong,
	AlreadyIdentityOwner,
}

#[ink::contract]
mod identity {
	use super::*;
	use crate::types::*;
	use common::types::{ChainInfo, *};
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

		/// The chain information associated with a specific `ChainId`.
		///
		/// NOTE: This mapping is only modifiable by the admin.
		pub(crate) chain_info_of: Mapping<ChainId, ChainInfo>,

		/// Keeps track of the `ChainId`s for all the chains that are inside the
		/// `chain_info_of` mapping.
		///
		/// NOTE: This mapping is only modifiable by the admin.
		pub(crate) chain_ids: Vec<ChainId>,

		/// The admin account has the ability to update the list of supported
		/// chains that can be used in Dotflow.
		///
		/// In the future it could be a good idea to have this controlled by
		/// governance.
		pub(crate) admin: AccountId,
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
		/// The chain on which a new address has been added.
		pub(crate) chain: ChainId,
		/// The newly added address.
		pub(crate) address: EncryptedAddress,
	}

	#[ink(event)]
	pub struct AddressUpdated {
		/// The `IdentityNo` of the identity that got updated.
		#[ink(topic)]
		pub(crate) identity_no: IdentityNo,
		/// The chain on which the address has been updated.
		pub(crate) chain: ChainId,
		/// The updated address value.
		pub(crate) updated_address: EncryptedAddress,
	}

	#[ink(event)]
	pub struct AddressRemoved {
		/// The `IdentityNo` of the identity that got updated.
		#[ink(topic)]
		pub(crate) identity_no: IdentityNo,
		/// The chain on which the address has been removed.
		pub(crate) chain: ChainId,
	}

	#[ink(event)]
	pub struct IdentityRemoved {
		/// The `IdentityNo` of the identity that got removed.
		#[ink(topic)]
		pub(crate) identity_no: IdentityNo,
	}

	#[ink(event)]
	pub struct ChainAdded {
		/// The `ChainId` that is associated with the newly added chain.
		#[ink(topic)]
		pub(crate) chain_id: ChainId,
		/// The network to which the chain belongs.
		pub(crate) network: Network,
		/// The address type used on the chain.
		pub(crate) account_type: AccountType,
	}

	#[ink(event)]
	pub struct ChainUpdated {
		/// The `ChainId` that is associated with the updated chain.
		#[ink(topic)]
		pub(crate) chain_id: ChainId,
		/// The address type used on the updated chain.
		pub(crate) account_type: AccountType,
	}

	#[ink(event)]
	pub struct ChainRemoved {
		/// The `ChainId` that is associated with the chain that got
		/// removed.
		#[ink(topic)]
		pub(crate) chain_id: ChainId,
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
				chain_info_of: Default::default(),
				chain_ids: Default::default(),
				recovery_account_of: Default::default(),
				admin: caller,
			}
		}

		#[ink(constructor)]
		pub fn init_with_chains(chains: Vec<ChainInfo>, chain_ids: Vec<ChainId>) -> Self {
			let mut chain_info_of = Mapping::default();

			assert!(
				chains.len() == chain_ids.len(),
				"Each chain needs to have a corresponding chain id specified."
			);

			// Iterate over all the chains provided and make sure that no
			// fields are exceeding the length limits.
			chain_ids
				.clone()
				.into_iter()
				.zip(chains.into_iter())
				.for_each(|(chain_id, chain)| {
					let chain_id = chain_id as ChainId;
					chain_info_of.insert(chain_id, &chain);
				});

			let caller = Self::env().caller();
			Self {
				number_to_identity: Default::default(),
				owner_of: Default::default(),
				identity_of: Default::default(),
				latest_identity_no: 0,
				chain_info_of,
				chain_ids,
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

		/// Returns the chain name that is associated with the specified `ChainId`.
		#[ink(message)]
		pub fn chain_info_of(&self, chain_id: ChainId) -> Option<ChainInfo> {
			self.chain_info_of.get(chain_id)
		}

		/// Returns the destination address of a transaction that needs to be
		/// routed to the specified identity on the specified chain.
		#[ink(message)]
		pub fn transaction_destination(
			&self,
			receiver: IdentityNo,
			chain: ChainId,
		) -> Result<EncryptedAddress, Error> {
			let receiver_identity = self
				.number_to_identity
				.get(receiver)
				.map_or(Err(Error::IdentityDoesntExist), Ok)?;

			match receiver_identity.addresses.into_iter().find(|(id, _)| *id == chain) {
				Some((_, address)) => Ok(address),
				None => Err(Error::InvalidChain),
			}
		}

		/// A list of all the available chains each associated with a `ChainId`.
		#[ink(message)]
		pub fn available_chains(&self) -> Vec<(ChainId, ChainInfo)> {
			self.chain_ids
				.clone()
				.into_iter()
				.map(|id| (id, self.chain_info_of(id)))
				.filter_map(|(id, maybe_chain)| maybe_chain.map(|info| (id, info)))
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

		/// Adds an address for a given chain
		#[ink(message)]
		pub fn add_address(
			&mut self,
			chain: ChainId,
			address: EncryptedAddress,
		) -> Result<(), Error> {
			let caller = self.env().caller();

			let identity_no = self.identity_of.get(caller).map_or(Err(Error::NotAllowed), Ok)?;

			let mut identity_info = self.get_identity_info_of_caller(caller)?;

			identity_info.add_address(chain, address.clone())?;
			self.number_to_identity.insert(identity_no, &identity_info);

			self.env().emit_event(AddressAdded { identity_no, chain, address });

			Ok(())
		}

		/// Updates the address of the given chain
		#[ink(message)]
		pub fn update_address(
			&mut self,
			chain: ChainId,
			address: EncryptedAddress,
		) -> Result<(), Error> {
			let caller = self.env().caller();

			let identity_no = self.identity_of.get(caller).map_or(Err(Error::NotAllowed), Ok)?;

			let mut identity_info = self.get_identity_info_of_caller(caller)?;

			identity_info.update_address(chain, address.clone())?;
			self.number_to_identity.insert(identity_no, &identity_info);

			self.env()
				.emit_event(AddressUpdated { identity_no, chain, updated_address: address });

			Ok(())
		}

		/// Removes the address by chain
		#[ink(message)]
		pub fn remove_address(&mut self, chain: ChainId) -> Result<(), Error> {
			let caller = self.env().caller();

			let identity_no = self.identity_of.get(caller).map_or(Err(Error::NotAllowed), Ok)?;

			let mut identity_info = self.get_identity_info_of_caller(caller)?;

			identity_info.remove_address(chain)?;
			self.number_to_identity.insert(identity_no, &identity_info);

			self.env().emit_event(AddressRemoved { identity_no, chain });

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
		pub fn add_chain(&mut self, chain_id: ChainId, info: ChainInfo) -> Result<(), Error> {
			let caller = self.env().caller();

			// Only the contract owner can add a chain
			ensure!(caller == self.admin, Error::NotAllowed);

			self.chain_info_of.insert(chain_id, &info);
			self.chain_ids.push(chain_id);

			let ChainInfo { account_type, network } = info;

			self.env().emit_event(ChainAdded { chain_id, account_type, network });

			Ok(())
		}

		#[ink(message)]
		pub fn update_chain(
			&mut self,
			chain_id: ChainId,
			new_address_type: Option<AccountType>,
		) -> Result<(), Error> {
			let caller = self.env().caller();

			// Only the contract owner can update a chain
			ensure!(caller == self.admin, Error::NotAllowed);

			// Ensure that the given chain id exists
			let mut info = self.chain_info_of.get(chain_id).map_or(Err(Error::InvalidChain), Ok)?;

			if let Some(account_type) = new_address_type {
				info.account_type = account_type;
			}

			// Update storage items
			self.chain_info_of.insert(chain_id, &info);

			self.env()
				.emit_event(ChainUpdated { chain_id, account_type: info.account_type });

			Ok(())
		}

		#[ink(message)]
		pub fn remove_chain(&mut self, chain_id: ChainId) -> Result<(), Error> {
			let caller = self.env().caller();

			// Only the contract owner can update a chain
			ensure!(caller == self.admin, Error::NotAllowed);

			// Ensure that the given `chain_id` exists
			let chain = self.chain_info_of.get(chain_id);
			ensure!(chain.is_some(), Error::InvalidChain);

			self.chain_info_of.remove(chain_id);
			self.chain_ids.retain(|&c_id| c_id != chain_id);

			self.env().emit_event(ChainRemoved { chain_id });

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
			let Some(identity_owner) = self.owner_of(identity_no) else {
				return Err(Error::NotAllowed)
			};

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
