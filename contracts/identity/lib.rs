#![cfg_attr(not(feature = "std"), no_std, no_main)]

use ink::prelude::{string::String, vec::Vec};

#[cfg(feature = "std")]
use ink::storage::traits::StorageLayout;

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
	addresses: Vec<(NetworkId, Address)>,
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
		number_to_identity: Mapping<IdentityNo, IdentityInfo>,
		owner_of: Mapping<IdentityNo, AccountId>,
		identity_of: Mapping<AccountId, IdentityNo>,
		recovery_account_of: Mapping<IdentityNo, AccountId>,
		identity_count: u32,
		network_name: Mapping<NetworkId, String>,
		network_id_counter: NetworkId,
		admin: AccountId,
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
		network: NetworkId,
		address: Address,
	}

	#[ink(event)]
	pub struct AddressUpdated {
		#[ink(topic)]
		identity_no: IdentityNo,
		network: NetworkId,
		updated_address: Address,
	}

	#[ink(event)]
	pub struct AddressRemoved {
		#[ink(topic)]
		identity_no: IdentityNo,
		network: NetworkId,
	}

	#[ink(event)]
	pub struct IdentityRemoved {
		#[ink(topic)]
		identity_no: IdentityNo,
	}

	#[ink(event)]
	pub struct NetworkAdded {
		#[ink(topic)]
		network_id: NetworkId,
		name: String,
	}

	#[ink(event)]
	pub struct NetworkUpdated {
		#[ink(topic)]
		network_id: NetworkId,
		name: String,
	}

	#[ink(event)]
	pub struct NetworkRemoved {
		#[ink(topic)]
		network_id: NetworkId,
	}

	#[ink(event)]
	pub struct RecoveryAccountSet {
		#[ink(topic)]
		identity_no: IdentityNo,
		recovery_account: AccountId,
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
				identity_count: 0,
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

		/// Returns the number of identities that exist.
		#[ink(message)]
		pub fn identity_count(&self) -> IdentityNo {
			self.identity_count
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
			let accounts = get_default_accounts();

			assert_eq!(identity.identity_count, 0);
			assert_eq!(identity.network_id_counter, 0);
			assert_eq!(identity.admin, accounts.alice);
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
			let DefaultAccounts::<DefaultEnvironment> { alice, bob, .. } = get_default_accounts();

			let mut identity = Identity::new();

			assert!(identity.create_identity().is_ok());

			assert_eq!(identity.owner_of.get(0), Some(alice));
			assert_eq!(
				identity.number_to_identity.get(0).unwrap(),
				IdentityInfo { addresses: Default::default() }
			);

			assert!(identity.add_network("Polkadot".to_string()).is_ok());
			assert!(identity.add_network("Moonbeam".to_string()).is_ok());

			let polkadot: NetworkId = 0;
			let moonbeam: NetworkId = 1;

			// In reality this address would be encrypted before storing in the contract.
			let encoded_address = alice.encode();

			assert!(identity.add_address(polkadot, encoded_address.clone()).is_ok());
			assert_eq!(
				identity.number_to_identity.get(0).unwrap(),
				IdentityInfo { addresses: vec![(polkadot, encoded_address.clone())] }
			);

			assert_eq!(recorded_events().count(), 4);
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
				identity.add_address(polkadot, encoded_address.clone()),
				Err(Error::AddressAlreadyAdded)
			);

			// Bob is not allowed to add an address to alice's identity.
			set_caller::<DefaultEnvironment>(bob);
			assert_eq!(
				identity.add_address(moonbeam, encoded_address.clone()),
				Err(Error::NotAllowed)
			);
		}

		#[ink::test]
		fn update_address_works() {
			let DefaultAccounts::<DefaultEnvironment> { alice, bob, charlie, .. } =
				get_default_accounts();

			let mut identity = Identity::new();

			assert!(identity.create_identity().is_ok());
			assert!(identity.add_network("Polkadot".to_string()).is_ok());
			assert!(identity.add_network("Moonbeam".to_string()).is_ok());

			assert_eq!(identity.owner_of.get(0), Some(alice));
			assert_eq!(
				identity.number_to_identity.get(0).unwrap(),
				IdentityInfo { addresses: Default::default() }
			);

			let polkadot: NetworkId = 0;
			let moonbeam: NetworkId = 1;

			let polkadot_address = alice.encode();

			assert!(identity.add_address(polkadot, polkadot_address.clone()).is_ok());
			assert_eq!(
				identity.number_to_identity.get(0).unwrap(),
				IdentityInfo { addresses: vec![(polkadot, polkadot_address.clone())] }
			);

			// Alice lost the key phrase of her old address so now she wants to use her other
			// address.
			let new_polkadot_address = bob.encode();

			assert!(identity.update_address(polkadot, new_polkadot_address.clone()).is_ok());
			assert_eq!(
				identity.number_to_identity.get(0).unwrap(),
				IdentityInfo { addresses: vec![(polkadot, new_polkadot_address.clone())] }
			);

			assert_eq!(recorded_events().count(), 5);
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
				identity.update_address(moonbeam, alice.encode()),
				Err(Error::InvalidNetwork)
			);

			// Charlie is not allowed to update to alice's identity.
			set_caller::<DefaultEnvironment>(charlie);
			assert_eq!(identity.update_address(polkadot, charlie.encode()), Err(Error::NotAllowed));
		}

		#[ink::test]
		fn remove_address_works() {
			let DefaultAccounts::<DefaultEnvironment> { alice, bob, .. } = get_default_accounts();

			let mut identity = Identity::new();

			assert!(identity.create_identity().is_ok());
			assert!(identity.add_network("Polkadot".to_string()).is_ok());

			assert_eq!(identity.owner_of.get(0), Some(alice));
			assert_eq!(
				identity.number_to_identity.get(0).unwrap(),
				IdentityInfo { addresses: Default::default() }
			);

			let polkadot: NetworkId = 0;
			// In reality this address would be encrypted before storing in the contract.
			let encoded_address = alice.encode();

			assert!(identity.add_address(polkadot, encoded_address.clone()).is_ok());
			assert_eq!(
				identity.number_to_identity.get(0).unwrap(),
				IdentityInfo { addresses: vec![(polkadot, encoded_address.clone())] }
			);

			// Bob is not allowed to remove an address from alice's identity.
			set_caller::<DefaultEnvironment>(bob);
			assert_eq!(identity.remove_address(polkadot), Err(Error::NotAllowed));

			set_caller::<DefaultEnvironment>(alice);
			assert!(identity.remove_address(polkadot).is_ok());

			assert_eq!(recorded_events().count(), 4);
			let last_event = recorded_events().last().unwrap();
			let decoded_event = <Event as scale::Decode>::decode(&mut &last_event.data[..])
				.expect("Failed to decode event");

			let Event::AddressRemoved(AddressRemoved { identity_no, network }) =
				decoded_event else { panic!("AddressRemoved event should be emitted") };

			assert_eq!(identity_no, 0);
			assert_eq!(network, polkadot);

			assert_eq!(
				identity.number_to_identity.get(0).unwrap(),
				IdentityInfo { addresses: vec![] }
			);

			// Cannot remove an address from a network that is not part of the
			// identity.
			assert_eq!(identity.remove_address(polkadot), Err(Error::InvalidNetwork));
		}

		#[ink::test]
		fn remove_identity_works() {
			let DefaultAccounts::<DefaultEnvironment> { alice, bob, .. } = get_default_accounts();

			let mut identity = Identity::new();

			assert!(identity.create_identity().is_ok());

			assert!(identity.add_network("Polkadot".to_string()).is_ok());

			assert_eq!(identity.owner_of.get(0), Some(alice));
			assert_eq!(
				identity.number_to_identity.get(0).unwrap(),
				IdentityInfo { addresses: Default::default() }
			);

			// In reality this address would be encrypted before storing in the contract.
			let encoded_address = alice.encode();
			let polkadot: NetworkId = 0;

			assert!(identity.add_address(polkadot, encoded_address.clone()).is_ok());
			assert_eq!(
				identity.number_to_identity.get(0).unwrap(),
				IdentityInfo { addresses: vec![(polkadot, encoded_address.clone())] }
			);

			// Bob is not allowed to remove alice's identity.
			set_caller::<DefaultEnvironment>(bob);
			assert_eq!(identity.remove_identity(), Err(Error::NotAllowed));

			set_caller::<DefaultEnvironment>(alice);
			assert!(identity.remove_identity().is_ok());

			assert_eq!(recorded_events().count(), 4);
			let last_event = recorded_events().last().unwrap();
			let decoded_event = <Event as scale::Decode>::decode(&mut &last_event.data[..])
				.expect("Failed to decode event");

			let Event::IdentityRemoved(IdentityRemoved { identity_no }) =
				decoded_event else { panic!("IdentityRemoved event should be emitted") };

			assert_eq!(identity_no, 0);

			// Make sure all of the state got removed.
			assert_eq!(identity.owner_of.get(0), None);
			assert_eq!(identity.identity_of.get(alice), None);
			assert_eq!(identity.number_to_identity.get(0), None);
		}

		#[ink::test]
		fn address_size_limit_works() {
			let mut identity = Identity::new();

			assert!(identity.create_identity().is_ok());
			assert!(identity.add_network("Polkadot".to_string()).is_ok());

			let polkadot = 0;

			let mut polkadot_address: Vec<u8> = vec![];
			(0..150).for_each(|n| polkadot_address.push(n));

			assert_eq!(
				identity.add_address(polkadot, polkadot_address.clone()),
				Err(Error::AddressSizeExceeded)
			);
		}

		#[ink::test]
		fn add_network_works() {
			let DefaultAccounts::<DefaultEnvironment> { alice, bob, .. } = get_default_accounts();

			let mut identity = Identity::new();
			assert_eq!(identity.admin, alice);

			let polkadot = "Polkadot".to_string();
			let kusama = "Kusama".to_string();

			// Adding a network successful
			assert!(identity.add_network(polkadot.clone()).is_ok());

			// Check emitted events
			assert_eq!(recorded_events().count(), 1);
			let last_event = recorded_events().last().unwrap();
			let decoded_event = <Event as scale::Decode>::decode(&mut &last_event.data[..])
				.expect("Failed to decode event");

			let Event::NetworkAdded(NetworkAdded { network_id, name }) = decoded_event else { panic!("NetworkAdded event should be emitted") };

			assert_eq!(network_id, 0);
			assert_eq!(name, polkadot);

			// Check storage items updated
			assert_eq!(identity.network_name.get(network_id), Some(name.clone()));
			assert_eq!(identity.network_id_counter, 1);

			// Only the contract creator can add a new network
			set_caller::<DefaultEnvironment>(bob);
			assert_eq!(identity.add_network(kusama), Err(Error::NotAllowed));

			set_caller::<DefaultEnvironment>(alice);

			// Name of the network should not be too long
			let long_network_name: String = String::from_utf8(vec!['a' as u8; 150]).unwrap();
			assert_eq!(identity.add_network(long_network_name), Err(Error::NetworkNameTooLong));
		}

		#[ink::test]
		fn remove_network_works() {
			let DefaultAccounts::<DefaultEnvironment> { alice, bob, .. } = get_default_accounts();
			let polkadot = "Polkadot".to_string();

			let mut identity = Identity::new();
			assert_eq!(identity.admin, alice);

			let Ok(network_id) = identity.add_network(polkadot.clone()) else {
				panic!("Failed to add network")
			};

			// Remove network: network doesn't exist
			assert_eq!(identity.remove_network(network_id + 1), Err(Error::InvalidNetwork));

			// Only the contract owner can remove a network
			set_caller::<DefaultEnvironment>(bob);
			assert_eq!(identity.remove_network(network_id), Err(Error::NotAllowed));

			// Remove network successful
			set_caller::<DefaultEnvironment>(alice);
			assert!(identity.remove_network(network_id).is_ok());

			assert!(identity.network_name.get(0).is_none());

			// Check emitted events
			let last_event = recorded_events().last().unwrap();
			let decoded_event = <Event as scale::Decode>::decode(&mut &last_event.data[..])
				.expect("Failed to decode event");

			let Event::NetworkRemoved(NetworkRemoved { network_id: removed_network_id }) = decoded_event else { panic!("NetworkRemoved event should be emitted") };

			assert_eq!(removed_network_id, network_id);
		}

		#[ink::test]
		fn update_network_works() {
			let DefaultAccounts::<DefaultEnvironment> { alice, bob, .. } = get_default_accounts();
			let polkadot = "Polkadot".to_string();
			let kusama = "Kusama".to_string();
			let moonbeam = "Moonbeam".to_string();

			let mut identity = Identity::new();
			assert_eq!(identity.admin, alice);

			let Ok(polkadot_id) = identity.add_network(polkadot.clone()) else {
				panic!("Failed to add network")
			};

			assert!(identity.add_network(kusama.clone()).is_ok());

			// Only the contract owner can update a network
			set_caller::<DefaultEnvironment>(bob);
			assert_eq!(
				identity.update_network(polkadot_id, moonbeam.clone()),
				Err(Error::NotAllowed)
			);

			set_caller::<DefaultEnvironment>(alice);

			// Network name should not be too long
			let long_network_name: String = String::from_utf8(vec!['a' as u8; 150]).unwrap();
			assert_eq!(
				identity.update_network(polkadot_id, long_network_name),
				Err(Error::NetworkNameTooLong)
			);

			// Must be an existing network
			assert_eq!(identity.update_network(3, moonbeam.clone()), Err(Error::InvalidNetwork));

			// Update network success
			assert!(identity.update_network(polkadot_id, moonbeam.clone()).is_ok());

			// Check the emitted events
			assert_eq!(recorded_events().count(), 3);
			let last_event = recorded_events().last().unwrap();
			let decoded_event = <Event as scale::Decode>::decode(&mut &last_event.data[..])
				.expect("Failed to decode event");

			let Event::NetworkUpdated(NetworkUpdated { network_id: network_updated, name: new_name }) = decoded_event else { panic!("NetworkUpdated event should be emitted") };

			assert_eq!(network_updated, polkadot_id);
			assert_eq!(new_name, moonbeam);
		}

		#[ink::test]
		fn set_recovery_account_works() {
			let DefaultAccounts::<DefaultEnvironment> { alice, bob, .. } = get_default_accounts();

			let mut identity = Identity::new();

			assert!(identity.create_identity().is_ok());

			// Only alice is able to set the recovery account for her identity.
			set_caller::<DefaultEnvironment>(bob);
			assert_eq!(identity.set_recovery_account(bob), Err(Error::NotAllowed));

			set_caller::<DefaultEnvironment>(alice);
			assert!(identity.set_recovery_account(bob).is_ok());

			assert_eq!(recorded_events().count(), 2);
			let last_event = recorded_events().last().unwrap();
			let decoded_event = <Event as scale::Decode>::decode(&mut &last_event.data[..])
				.expect("Failed to decode event");

			let Event::RecoveryAccountSet(RecoveryAccountSet { identity_no, recovery_account }) =
				decoded_event else { panic!("RecoveryAccountSet event should be emitted") };

			assert_eq!(identity_no, 0);
			assert_eq!(recovery_account, bob);

			assert_eq!(identity.recovery_account_of.get(identity_no), Some(bob));
		}

		#[ink::test]
		fn transfer_ownership_works() {
			let accounts = get_default_accounts();
			let alice = accounts.alice;
			let bob = accounts.bob;
			let polkadot = "Polkadot".to_string();

			let mut identity = Identity::new();

			let Ok(polkadot_id) = identity.add_network(polkadot.clone()) else {
				panic!("Failed to add network")
			};

			assert!(identity.create_identity().is_ok());

			assert_eq!(identity.owner_of.get(0), Some(alice));
			assert_eq!(
				identity.number_to_identity.get(0).unwrap(),
				IdentityInfo { addresses: Default::default() }
			);

			// In reality this address would be encrypted before storing in the contract.
			let encoded_address = alice.encode();

			assert!(identity.add_address(polkadot_id.clone(), encoded_address.clone()).is_ok());
			assert_eq!(
				identity.number_to_identity.get(0).unwrap(),
				IdentityInfo { addresses: vec![(polkadot_id.clone(), encoded_address.clone())] }
			);

			// Bob is not allowed to transfer the ownership.
			set_caller::<DefaultEnvironment>(bob);
			assert_eq!(identity.transfer_ownership(bob), Err(Error::NotAllowed));

			set_caller::<DefaultEnvironment>(alice);
			assert!(identity.transfer_ownership(bob).is_ok());

			assert_eq!(identity.owner_of.get(0), Some(bob));
			assert_eq!(
				identity.number_to_identity.get(0).unwrap(),
				IdentityInfo { addresses: vec![(polkadot_id.clone(), encoded_address.clone())] }
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
