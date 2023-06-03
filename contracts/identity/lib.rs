#![cfg_attr(not(feature = "std"), no_std, no_main)]

use ink::{
	prelude::{string::String, vec::Vec},
	storage::traits::StorageLayout,
};

macro_rules! ensure {
	( $x:expr, $y:expr $(,)? ) => {{
		if !$x {
			return Err($y)
		}
	}};
}

/// Each identity will be associated with a unique identifier called `IdentityNo`.
pub type IdentityNo = u64;

/// We want to keep the address type very generic since we want to support any
/// address format. We won't actually keep the addresses in the contract itself.
/// Before storing them, we'll encrypt them to ensure privacy.
pub type Address = Vec<u8>;

/// Used to represent any blockchain in the Polkadot, Kusama or Rococo network.
pub type Network = String;

#[derive(scale::Encode, scale::Decode, Debug, Default, PartialEq)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo, StorageLayout))]
pub struct IdentityInfo {
	/// Each address is associated with a specific blockchain.
	addresses: Vec<(Network, Address)>,
}

#[derive(scale::Encode, scale::Decode, Debug, PartialEq)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum Error {
	NotAllowed,
	IdentityDoesntExist,
	AddressAlreadyAdded,
}

impl IdentityInfo {
	/// Adds an address for the given network
	pub fn add_address(&mut self, network: Network, address: Address) -> Result<(), Error> {
		ensure!(
			self.addresses.clone().into_iter().find(|address| address.0 == network) == None,
			Error::AddressAlreadyAdded
		);
		self.addresses.push((network, address));

		Ok(())
	}

	/// Updates the address of the given network
	pub fn update_address(&mut self, network: Network, new_address: Address) {
		// TODO:
	}

	/// Remove address of the given network
	pub fn remove_address(&mut self, network: Network) {
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
		identity_count: u64,
	}

	// TODO: Add events
	#[ink(event)]
	pub struct IdentityCreated {
		#[ink(topic)]
		owner: AccountId,
		identity_no: IdentityNo,
	}

	impl Identity {
		#[ink(constructor)]
		pub fn new() -> Self {
			Default::default()
		}

		#[ink(message)]
		/// Creates an identity and returns the `IdentityNo` A user can only
		/// create one identity.
		pub fn create_identity(&mut self) -> Result<IdentityNo, Error> {
			let caller = self.env().caller();

			ensure!(self.identity_of.get(caller).is_none(), Error::NotAllowed);

			let identity_no = self.identity_count;

			let new_identity: IdentityInfo = Default::default();

			self.number_to_identity.insert(identity_no, &new_identity);
			self.identity_of.insert(caller, &identity_no);
			self.owner_of.insert(identity_no, &caller);

			self.identity_count = self.identity_count.saturating_add(1);

			self.env().emit_event(IdentityCreated { owner: caller, identity_no });

			Ok(identity_no)
		}

		#[ink(message)]
		/// Adds an address for a given network
		pub fn add_address(&mut self, network: Network, address: Address) -> Result<(), Error> {
			let caller = self.env().caller();
			ensure!(self.identity_of.get(caller).is_some(), Error::NotAllowed);

			let identity_no = self.identity_of.get(caller).unwrap();
			let Some(mut identity_info) = self.number_to_identity.get(identity_no) else { return Err(Error::IdentityDoesntExist) };

			identity_info.add_address(network, address)?;

			self.number_to_identity.insert(identity_no, &identity_info);

			Ok(())
		}

		#[ink(message)]
		/// Updates the address of the given network
		pub fn update_address(&mut self, network: Network, address: Address) {
			// TODO:
		}

		#[ink(message)]
		/// Removes the address by network
		pub fn remove_address(&mut self, network: Network) {
			// TODO:
		}

		#[ink(message)]
		/// Removes an identity
		pub fn remove_identity(&mut self, identity_no: IdentityNo) {
			// TODO:
		}
	}

	#[cfg(test)]
	mod tests {
		use super::*;
		use ink::env::{
			test::{default_accounts, recorded_events, DefaultAccounts},
			DefaultEnvironment,
		};

		type Event = <Identity as ::ink::reflect::ContractEventBase>::Type;

		/// We test if the constructor does its job.
		#[ink::test]
		fn constructor_works() {
			let identity = Identity::new();

			assert_eq!(identity.identity_count, 0);
		}

		#[ink::test]
		fn create_identity_works() {
			let accounts = get_default_accounts();
			let alice = accounts.alice;

			let mut identity = Identity::new();

			assert!(identity.create_identity().is_ok());

			// Test the emitted event
			assert_eq!(recorded_events().count(), 1);
			let last_event = recorded_events().last().unwrap();
			let decoded_event = <Event as scale::Decode>::decode(&mut &last_event.data[..])
				.expect("Failed to decode event");

			let Event::IdentityCreated(IdentityCreated { owner, identity_no }) = decoded_event;

			assert_eq!(owner, alice);
			assert_eq!(identity_no, 0);

			// Make sure all the storage values got properly updated.
			assert_eq!(identity.identity_of.get(alice), Some(0));
			assert_eq!(identity.owner_of.get(0), Some(alice));
			assert_eq!(
				identity.number_to_identity.get(0).unwrap(),
				IdentityInfo { addresses: Default::default() }
			);
			assert_eq!(identity.identity_count, 1);
		}

		#[ink::test]
		fn create_identity_already_exist() {
			let mut identity = Identity::new();

			assert!(identity.create_identity().is_ok());

			// A user can create one identity only
			assert_eq!(identity.create_identity(), Err(Error::NotAllowed));
		}

		#[ink::test]
		fn add_address_works() {
			// TODO:
		}

		fn get_default_accounts() -> DefaultAccounts<DefaultEnvironment> {
			default_accounts::<DefaultEnvironment>()
		}
	}

	#[cfg(all(test, feature = "e2e-tests"))]
	mod e2e_tests {}
}
