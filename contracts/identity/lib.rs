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

/// Encrypted addresses should never exceed this size limit.
const ADDRESS_SIZE_LIMIT: usize = 128;

/// Each identity will be associated with a unique identifier called `IdentityNo`.
pub type IdentityNo = u32;

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
	InvalidNetwork,
	AddressSizeExceeded,
}

impl IdentityInfo {
	/// Adds an address for the given network
	pub fn add_address(&mut self, network: Network, address: Address) -> Result<(), Error> {
		ensure!(address.len() <= ADDRESS_SIZE_LIMIT, Error::AddressSizeExceeded);

		ensure!(
			self.addresses.clone().into_iter().find(|address| address.0 == network) == None,
			Error::AddressAlreadyAdded
		);
		self.addresses.push((network, address));

		Ok(())
	}

	/// Updates the address of the given network
	pub fn update_address(&mut self, network: Network, new_address: Address) -> Result<(), Error> {
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
	pub fn remove_address(&mut self, network: Network) -> Result<(), Error> {
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
	#[derive(Default)]
	pub struct Identity {
		number_to_identity: Mapping<IdentityNo, IdentityInfo>,
		owner_of: Mapping<IdentityNo, AccountId>,
		identity_of: Mapping<AccountId, IdentityNo>,
		identity_count: u32,
	}

	/// Events
	#[ink(event)]
	pub struct IdentityCreated {
		#[ink(topic)]
		owner: AccountId,
		identity_no: IdentityNo,
	}

	#[ink(event)]
	pub struct AddressAdded {
		#[ink(topic)]
		identity_no: IdentityNo,
		network: Network,
		address: Address,
	}

	#[ink(event)]
	pub struct AddressUpdated {
		#[ink(topic)]
		identity_no: IdentityNo,
		network: Network,
		updated_address: Address,
	}

	#[ink(event)]
	pub struct AddressRemoved {
		#[ink(topic)]
		identity_no: IdentityNo,
		network: Network,
	}

	#[ink(event)]
	pub struct IdentityRemoved {
		#[ink(topic)]
		identity_no: IdentityNo,
	}

	impl Identity {
		#[ink(constructor)]
		pub fn new() -> Self {
			Default::default()
		}

		/// Creates an identity and returns the `IdentityNo`.
		///
		/// A user can only create one identity.
		#[ink(message)]
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

		/// Adds an address for a given network
		#[ink(message)]
		pub fn add_address(&mut self, network: Network, address: Address) -> Result<(), Error> {
			let caller = self.env().caller();
			ensure!(self.identity_of.get(caller).is_some(), Error::NotAllowed);

			let identity_no = self.identity_of.get(caller).unwrap();
			let mut identity_info = self.get_identity_info_of_caller(caller)?;

			identity_info.add_address(network.clone(), address.clone())?;
			self.number_to_identity.insert(identity_no, &identity_info);

			self.env().emit_event(AddressAdded { identity_no, network, address });

			Ok(())
		}

		/// Updates the address of the given network
		#[ink(message)]
		pub fn update_address(&mut self, network: Network, address: Address) -> Result<(), Error> {
			let caller = self.env().caller();
			ensure!(self.identity_of.get(caller).is_some(), Error::NotAllowed);

			let identity_no = self.identity_of.get(caller).unwrap();
			let mut identity_info = self.get_identity_info_of_caller(caller)?;

			identity_info.update_address(network.clone(), address.clone())?;
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
		pub fn remove_address(&mut self, network: Network) -> Result<(), Error> {
			let caller = self.env().caller();
			ensure!(self.identity_of.get(caller).is_some(), Error::NotAllowed);

			let identity_no = self.identity_of.get(caller).unwrap();
			let mut identity_info = self.get_identity_info_of_caller(caller)?;

			identity_info.remove_address(network.clone())?;
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

		/// Transfers the ownership of an identity to another account.
		///
		/// Only callable by the identity owner or any account that the identity
		/// owner added as a proxy.
		#[ink(message)]
		pub fn transfer_ownership(&mut self, new_owner: AccountId) -> Result<(), Error> {
			let caller = self.env().caller();
			ensure!(self.identity_of.get(caller).is_some(), Error::NotAllowed);

			let identity_no = self.identity_of.get(caller).unwrap();

			self.identity_of.remove(caller);
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

	#[cfg(test)]
	mod tests {
		use super::*;
		use ink::env::{
			test::{default_accounts, recorded_events, set_caller, DefaultAccounts},
			DefaultEnvironment,
		};
		use scale::Encode;

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

			let Event::IdentityCreated(IdentityCreated { owner, identity_no }) =
				decoded_event else { panic!("IdentityCreated event should be emitted") };

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
		fn add_address_to_identity_works() {
			let accounts = get_default_accounts();
			let alice = accounts.alice;
			let bob = accounts.bob;
			let polkadot = "Polkadot".to_string();
			let moonbeam = "Moonbeam".to_string();

			let mut identity = Identity::new();

			assert!(identity.create_identity().is_ok());

			assert_eq!(identity.owner_of.get(0), Some(alice));
			assert_eq!(
				identity.number_to_identity.get(0).unwrap(),
				IdentityInfo { addresses: Default::default() }
			);

			// In reality this address would be encrypted before storing in the contract.
			let encoded_address = alice.encode();

			assert!(identity.add_address(polkadot.clone(), encoded_address.clone()).is_ok());
			assert_eq!(
				identity.number_to_identity.get(0).unwrap(),
				IdentityInfo { addresses: vec![(polkadot.clone(), encoded_address.clone())] }
			);

			assert_eq!(recorded_events().count(), 2);
			let last_event = recorded_events().last().unwrap();
			let decoded_event = <Event as scale::Decode>::decode(&mut &last_event.data[..])
				.expect("Failed to decode event");

			let Event::AddressAdded(AddressAdded { identity_no, network, address }) =
				decoded_event else { panic!("AddressAdded event should be emitted") };

			assert_eq!(identity_no, 0);
			assert_eq!(network, polkadot);
			assert_eq!(address, encoded_address);

			// Cannot add an address for the same network twice.
			assert_eq!(
				identity.add_address(polkadot.clone(), encoded_address.clone()),
				Err(Error::AddressAlreadyAdded)
			);

			// Bob is not allowed to add an address to alice's identity.
			set_caller::<DefaultEnvironment>(bob);
			assert_eq!(
				identity.add_address(moonbeam.clone(), encoded_address.clone()),
				Err(Error::NotAllowed)
			);
		}

		#[ink::test]
		fn update_address_works() {
			let accounts = get_default_accounts();
			let alice = accounts.alice;
			let charlie = accounts.charlie;
			let polkadot = "Polkadot".to_string();
			let moonbeam = "Moonbeam".to_string();

			let mut identity = Identity::new();

			assert!(identity.create_identity().is_ok());

			assert_eq!(identity.owner_of.get(0), Some(alice));
			assert_eq!(
				identity.number_to_identity.get(0).unwrap(),
				IdentityInfo { addresses: Default::default() }
			);

			let polkadot_address = alice.encode();

			assert!(identity.add_address(polkadot.clone(), polkadot_address.clone()).is_ok());
			assert_eq!(
				identity.number_to_identity.get(0).unwrap(),
				IdentityInfo { addresses: vec![(polkadot.clone(), polkadot_address.clone())] }
			);

			// Alice lost the key phrase of her old address so now she wants to use her other
			// address.
			let new_polkadot_address = accounts.bob.encode();

			assert!(identity
				.update_address(polkadot.clone(), new_polkadot_address.clone())
				.is_ok());
			assert_eq!(
				identity.number_to_identity.get(0).unwrap(),
				IdentityInfo { addresses: vec![(polkadot.clone(), new_polkadot_address.clone())] }
			);

			assert_eq!(recorded_events().count(), 3);
			let last_event = recorded_events().last().unwrap();
			let decoded_event = <Event as scale::Decode>::decode(&mut &last_event.data[..])
				.expect("Failed to decode event");

			let Event::AddressUpdated(AddressUpdated { identity_no, network, updated_address }) =
				decoded_event else { panic!("AddressUpdated event should be emitted") };

			assert_eq!(identity_no, 0);
			assert_eq!(network, polkadot);
			assert_eq!(updated_address, new_polkadot_address);

			// Won't work since the identity doesn't have an address on the
			// Moonbeam parachain.
			assert_eq!(
				identity.update_address(moonbeam.clone(), alice.encode()),
				Err(Error::InvalidNetwork)
			);

			// Charlie is not allowed to update to alice's identity.
			set_caller::<DefaultEnvironment>(charlie);
			assert_eq!(
				identity.update_address(polkadot.clone(), charlie.encode()),
				Err(Error::NotAllowed)
			);
		}

		#[ink::test]
		fn remove_address_works() {
			let accounts = get_default_accounts();
			let alice = accounts.alice;
			let bob = accounts.bob;
			let polkadot = "Polkadot".to_string();

			let mut identity = Identity::new();

			assert!(identity.create_identity().is_ok());

			assert_eq!(identity.owner_of.get(0), Some(alice));
			assert_eq!(
				identity.number_to_identity.get(0).unwrap(),
				IdentityInfo { addresses: Default::default() }
			);

			// In reality this address would be encrypted before storing in the contract.
			let encoded_address = alice.encode();

			assert!(identity.add_address(polkadot.clone(), encoded_address.clone()).is_ok());
			assert_eq!(
				identity.number_to_identity.get(0).unwrap(),
				IdentityInfo { addresses: vec![(polkadot.clone(), encoded_address.clone())] }
			);

			// Bob is not allowed to remove an address from alice's identity.
			set_caller::<DefaultEnvironment>(bob);
			assert_eq!(identity.remove_address(polkadot.clone()), Err(Error::NotAllowed));

			set_caller::<DefaultEnvironment>(alice);

			assert!(identity.remove_address(polkadot.clone()).is_ok());
			assert_eq!(
				identity.number_to_identity.get(0).unwrap(),
				IdentityInfo { addresses: vec![] }
			);

			// Cannot remove an address from a network that is not part of the
			// identity.
			assert_eq!(identity.remove_address(polkadot.clone()), Err(Error::InvalidNetwork));
		}

		#[ink::test]
		fn remove_identity_works() {
			let accounts = get_default_accounts();
			let alice = accounts.alice;
			let bob = accounts.bob;
			let polkadot = "Polkadot".to_string();

			let mut identity = Identity::new();

			assert!(identity.create_identity().is_ok());

			assert_eq!(identity.owner_of.get(0), Some(alice));
			assert_eq!(
				identity.number_to_identity.get(0).unwrap(),
				IdentityInfo { addresses: Default::default() }
			);

			// In reality this address would be encrypted before storing in the contract.
			let encoded_address = alice.encode();

			assert!(identity.add_address(polkadot.clone(), encoded_address.clone()).is_ok());
			assert_eq!(
				identity.number_to_identity.get(0).unwrap(),
				IdentityInfo { addresses: vec![(polkadot.clone(), encoded_address.clone())] }
			);

			// Bob is not allowed to remove alice's identity.
			set_caller::<DefaultEnvironment>(bob);
			assert_eq!(identity.remove_identity(), Err(Error::NotAllowed));

			set_caller::<DefaultEnvironment>(alice);
			assert!(identity.remove_identity().is_ok());

			// Make sure all of the state got removed.
			assert_eq!(identity.owner_of.get(0), None);
			assert_eq!(identity.identity_of.get(alice), None);
			assert_eq!(identity.number_to_identity.get(0), None);
		}

		#[ink::test]
		fn address_size_limit_works() {
			let polkadot = "Polkadot".to_string();

			let mut identity = Identity::new();
			assert!(identity.create_identity().is_ok());

			let mut polkadot_address: Vec<u8> = vec![];
			(0..150).for_each(|n| polkadot_address.push(n));

			assert_eq!(
				identity.add_address(polkadot.clone(), polkadot_address.clone()),
				Err(Error::AddressSizeExceeded)
			);
		}

		#[ink::test]
		fn transfer_ownership_works() {
			let accounts = get_default_accounts();
			let alice = accounts.alice;
			let bob = accounts.bob;
			let polkadot = "Polkadot".to_string();

			let mut identity = Identity::new();

			assert!(identity.create_identity().is_ok());

			assert_eq!(identity.owner_of.get(0), Some(alice));
			assert_eq!(
				identity.number_to_identity.get(0).unwrap(),
				IdentityInfo { addresses: Default::default() }
			);

			// In reality this address would be encrypted before storing in the contract.
			let encoded_address = alice.encode();

			assert!(identity.add_address(polkadot.clone(), encoded_address.clone()).is_ok());
			assert_eq!(
				identity.number_to_identity.get(0).unwrap(),
				IdentityInfo { addresses: vec![(polkadot.clone(), encoded_address.clone())] }
			);

			// Bob is not allowed to transfer the ownership.
			set_caller::<DefaultEnvironment>(bob);
			assert_eq!(identity.transfer_ownership(bob), Err(Error::NotAllowed));

			set_caller::<DefaultEnvironment>(alice);
			assert!(identity.transfer_ownership(bob).is_ok());

			assert_eq!(identity.owner_of.get(0), Some(bob));
			assert_eq!(
				identity.number_to_identity.get(0).unwrap(),
				IdentityInfo { addresses: vec![(polkadot.clone(), encoded_address.clone())] }
			);
			assert_eq!(identity.identity_of.get(alice), None);
			assert_eq!(identity.identity_of.get(bob), Some(0));
		}

		fn get_default_accounts() -> DefaultAccounts<DefaultEnvironment> {
			default_accounts::<DefaultEnvironment>()
		}
	}

	#[cfg(all(test, feature = "e2e-tests"))]
	mod e2e_tests {}
}
