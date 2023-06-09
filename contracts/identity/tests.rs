/// Ink! integration tests convering the identity contract functionality.
use crate::{identity::*, types::*, *};

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
	let DefaultAccounts::<DefaultEnvironment> { alice, .. } = get_default_accounts();

	assert_eq!(identity.latest_identity_no, 0);
	assert_eq!(identity.network_id_counter, 0);
	assert_eq!(identity.admin, alice);
}

#[ink::test]
fn create_identity_works() {
	let DefaultAccounts::<DefaultEnvironment> { alice, .. } = get_default_accounts();

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
	assert_eq!(identity.latest_identity_no, 1);
}

#[ink::test]
fn create_identity_already_exist() {
	let mut identity = Identity::new();

	assert!(identity.create_identity().is_ok());

	// A user can create one identity only
	assert_eq!(identity.create_identity(), Err(Error::AlreadyIdentityOwner));
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
	assert_eq!(identity.add_address(moonbeam, encoded_address.clone()), Err(Error::NotAllowed));
}

#[ink::test]
fn update_address_works() {
	let DefaultAccounts::<DefaultEnvironment> { alice, bob, charlie, .. } = get_default_accounts();

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
	assert_eq!(identity.update_address(moonbeam, alice.encode()), Err(Error::InvalidNetwork));

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

	assert_eq!(identity.number_to_identity.get(0).unwrap(), IdentityInfo { addresses: vec![] });

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
	assert_eq!(identity.update_network(polkadot_id, moonbeam.clone()), Err(Error::NotAllowed));

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
	let DefaultAccounts::<DefaultEnvironment> { alice, bob, .. } = get_default_accounts();
	let identity_no = 0;
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

	// Bob is not allowed to transfer the ownership. Only alice or the
	// recovery can transfer the ownerhsip.
	set_caller::<DefaultEnvironment>(bob);
	assert_eq!(identity.transfer_ownership(identity_no, bob), Err(Error::NotAllowed));

	set_caller::<DefaultEnvironment>(alice);
	assert!(identity.transfer_ownership(identity_no, bob).is_ok());

	// Bob is now the identity owner.
	assert_eq!(identity.owner_of.get(0), Some(bob));
	assert_eq!(
		identity.number_to_identity.get(0).unwrap(),
		IdentityInfo { addresses: vec![(polkadot_id.clone(), encoded_address.clone())] }
	);
	assert_eq!(identity.identity_of.get(alice), None);
	assert_eq!(identity.identity_of.get(bob), Some(0));

	// He will add alice as a recovery account.
	set_caller::<DefaultEnvironment>(bob);
	assert!(identity.set_recovery_account(alice).is_ok());

	// Alice will transfer the ownership back to her account.
	set_caller::<DefaultEnvironment>(alice);
	assert!(identity.transfer_ownership(identity_no, alice).is_ok());

	assert_eq!(identity.owner_of.get(0), Some(alice));
	assert_eq!(
		identity.number_to_identity.get(0).unwrap(),
		IdentityInfo { addresses: vec![(polkadot_id.clone(), encoded_address.clone())] }
	);
	assert_eq!(identity.identity_of.get(alice), Some(0));
	assert_eq!(identity.identity_of.get(bob), None);
}

#[ink::test]
fn transfer_ownership_fails_when_new_owner_has_an_identity() {
	let DefaultAccounts::<DefaultEnvironment> { alice, bob, .. } = get_default_accounts();
	let identity_no = 0;

	let mut identity = Identity::new();

	assert!(identity.create_identity().is_ok());

	set_caller::<DefaultEnvironment>(bob);
	assert!(identity.create_identity().is_ok());

	set_caller::<DefaultEnvironment>(alice);

	assert_eq!(identity.transfer_ownership(identity_no, bob), Err(Error::AlreadyIdentityOwner));
}

#[ink::test]
fn init_with_networks_works() {
	let polkadot = "Polkadot".to_string();
	let kusama = "Kusama".to_string();
	let moonbeam = "Moonbeam".to_string();
	let astar = "Astar".to_string();
	let networks: Vec<String> =
		vec![polkadot.clone(), kusama.clone(), moonbeam.clone(), astar.clone()];
	let identity = Identity::init_with_networks(networks);
	assert_eq!(identity.network_id_counter, 4);
	assert_eq!(identity.network_name_of(0), Some(polkadot));
	assert_eq!(identity.network_name_of(1), Some(kusama));
	assert_eq!(identity.network_name_of(2), Some(moonbeam));
	assert_eq!(identity.network_name_of(3), Some(astar));
}

#[ink::test]
#[should_panic]
fn init_with_networks_fail() {
	let very_long_name = String::from_utf8(vec!['a' as u8; 150]).unwrap();
	Identity::init_with_networks(vec![very_long_name]);
}

fn get_default_accounts() -> DefaultAccounts<DefaultEnvironment> {
	default_accounts::<DefaultEnvironment>()
}
