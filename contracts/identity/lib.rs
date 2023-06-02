#![cfg_attr(not(feature = "std"), no_std, no_main)]

use ink::{
	prelude::{string::String, vec::Vec},
	storage::traits::StorageLayout,
};

/// Each identity will be associated with a unique identifier called `IdentityNo`.
pub type IdentityNo = u64;

/// We want to keep the address type very generic since we want to support any
/// address format. We won't actually keep the addresses in the contract itself.
/// Before storing them, we'll encrypt them to ensure privacy.
pub type Address = Vec<u8>;

/// Used to represent any blockchain in the Polkadot, Kusama or Rococo network.
pub type Network = String;

#[derive(scale::Encode, scale::Decode, Debug)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo, StorageLayout))]
pub struct IdentityInfo {
	/// Each address is associated with a specific blockchain.
	addresses: Vec<(Network, Address)>,
}

impl IdentityInfo {
	/// Adds an address for the given network
	pub fn add_address(network: Network, address: Address) {
		// TODO:
	}

	/// Updates the address of the given network
	pub fn update_address(network: Network, new_address: Address) {
		// TODO:
	}

	/// Remove address of the given network
	pub fn remove_address(network: Network) {
		// TODO:
	}
}

#[ink::contract]
mod identity {
	use super::*;
	use ink::storage::Mapping;

	#[ink(storage)]
	#[derive(Default)]
	pub struct Identity {
		number_to_identity: Mapping<IdentityNo, IdentityInfo>,
		owner_of: Mapping<IdentityNo, AccountId>,
		identity_of: Mapping<AccountId, IdentityNo>,
		total_identity: u64,
	}

	impl Identity {
		#[ink(constructor)]
		pub fn new() -> Self {
			Default::default()
		}

		// TODO: remove this
		#[ink(message)]
		pub fn foo(&self) -> u32 {
			42
		}

		#[ink(message)]
		/// Create an identity and returns the `IdentityNo`
		/// A user can only create one identity
		pub fn create_identity(&self) -> IdentityNo {
			// TODO: Check if the caller already owns an identity

			// Generate a new IdentityNo
			let new_id = self.total_identity;

			// TODO: initialize IdentityInfo
			// TODO: Associate the newly created IdentityInfo with required mappings

			// Increase the number of identities
			self.total_identity = self.total_identity + 1;

			new_id
		}

		#[ink(message)]
		/// Adds an address for a given network
		pub fn add_address(&self, network: Network, address: Address) {
			// TODO:
		}

		#[ink(message)]
		/// Updates the address of the given network
		pub fn update_address(&self, network: Network, address: Address) {
			// TODO:
		}

		#[ink(message)]
		/// Removes the address by network
		pub fn remove_address(&self, network: Network) {
			// TODO:
		}

		#[ink(message)]
		/// Removes an identity
		pub fn remove_identity(&self, identity_no: IdentityNo) {
			// TODO:
		}
	}

	#[cfg(test)]
	mod tests {
		use super::*;

		/// We test if the constructor does its job.
		#[ink::test]
		fn constructor_works() {
			let identity = Identity::new();
			assert_eq!(identity.foo(), 42);
		}
	}

	#[cfg(all(test, feature = "e2e-tests"))]
	mod e2e_tests {}
}
