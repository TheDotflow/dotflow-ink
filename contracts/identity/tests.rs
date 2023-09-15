//! Ink! integration tests convering the identity contract functionality.
use crate::{identity::*, types::*, *};
use common::types::{AccountType::*, Network::*, *};

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
	assert_eq!(identity.admin, alice);
	assert_eq!(identity.chain_ids, vec![]);
	assert_eq!(identity.available_chains(), Vec::default());
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

	let Event::IdentityCreated(IdentityCreated { owner, identity_no }) = decoded_event else {
		panic!("IdentityCreated event should be emitted")
	};

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

	let polkadot: ChainId = (0, Polkadot);
	let moonbeam: ChainId = (2004, Polkadot);

	assert!(identity
		.add_chain(polkadot.clone(), ChainInfo { account_type: AccountId32 })
		.is_ok());
	assert!(identity
		.add_chain(moonbeam.clone(), ChainInfo { account_type: AccountId32 })
		.is_ok());

	// In reality this address would be encrypted before storing in the contract.
	let encoded_address = alice.encode();

	assert!(identity.add_address(polkadot.clone(), encoded_address.clone()).is_ok());
	assert_eq!(
		identity.number_to_identity.get(0).unwrap(),
		IdentityInfo { addresses: vec![(polkadot.clone(), encoded_address.clone())] }
	);

	assert_eq!(recorded_events().count(), 4);
	let last_event = recorded_events().last().unwrap();
	let decoded_event = <Event as scale::Decode>::decode(&mut &last_event.data[..])
		.expect("Failed to decode event");

	let Event::AddressAdded(AddressAdded { identity_no, chain, address }) = decoded_event else {
		panic!("AddressAdded event should be emitted")
	};

	assert_eq!(identity_no, 0);
	assert_eq!(chain, polkadot.clone());
	assert_eq!(address, encoded_address);

	// Cannot add an address for the same chain twice.
	assert_eq!(
		identity.add_address(polkadot, encoded_address.clone()),
		Err(Error::AddressAlreadyAdded)
	);

	// Bob is not allowed to add an address to alice's identity.
	set_caller::<DefaultEnvironment>(bob);
	assert_eq!(identity.add_address(moonbeam, encoded_address), Err(Error::NotAllowed));
}

#[ink::test]
fn update_address_works() {
	let DefaultAccounts::<DefaultEnvironment> { alice, bob, charlie, .. } = get_default_accounts();

	let mut identity = Identity::new();

	let polkadot: ChainId = (0, Polkadot);
	let moonbeam: ChainId = (2004, Polkadot);

	assert!(identity.create_identity().is_ok());
	assert!(identity
		.add_chain(polkadot.clone(), ChainInfo { account_type: AccountId32 })
		.is_ok());
	assert!(identity
		.add_chain(moonbeam.clone(), ChainInfo { account_type: AccountId32 })
		.is_ok());

	assert_eq!(identity.owner_of.get(0), Some(alice));
	assert_eq!(
		identity.number_to_identity.get(0).unwrap(),
		IdentityInfo { addresses: Default::default() }
	);

	let polkadot_address = alice.encode();

	assert!(identity.add_address(polkadot.clone(), polkadot_address.clone()).is_ok());
	assert_eq!(
		identity.number_to_identity.get(0).unwrap(),
		IdentityInfo { addresses: vec![(polkadot.clone(), polkadot_address)] }
	);

	// Alice lost the key phrase of her old address so now she wants to use her other
	// address.
	let new_polkadot_address = bob.encode();

	assert!(identity.update_address(polkadot.clone(), new_polkadot_address.clone()).is_ok());
	assert_eq!(
		identity.number_to_identity.get(0).unwrap(),
		IdentityInfo { addresses: vec![(polkadot.clone(), new_polkadot_address.clone())] }
	);

	assert_eq!(recorded_events().count(), 5);
	let last_event = recorded_events().last().unwrap();
	let decoded_event = <Event as scale::Decode>::decode(&mut &last_event.data[..])
		.expect("Failed to decode event");

	let Event::AddressUpdated(AddressUpdated { identity_no, chain, updated_address }) =
		decoded_event
	else {
		panic!("AddressUpdated event should be emitted")
	};

	assert_eq!(identity_no, 0);
	assert_eq!(chain, polkadot.clone());
	assert_eq!(updated_address, new_polkadot_address);

	// Won't work since the identity doesn't have an address on the
	// Moonbeam parachain.
	assert_eq!(identity.update_address(moonbeam, alice.encode()), Err(Error::InvalidChain));

	// Charlie is not allowed to update to alice's identity.
	set_caller::<DefaultEnvironment>(charlie);
	assert_eq!(identity.update_address(polkadot, charlie.encode()), Err(Error::NotAllowed));
}

#[ink::test]
fn remove_address_works() {
	let DefaultAccounts::<DefaultEnvironment> { alice, bob, .. } = get_default_accounts();

	let mut identity = Identity::new();

	let polkadot: ChainId = (0, Polkadot);
	assert!(identity.create_identity().is_ok());
	assert!(identity
		.add_chain(polkadot.clone(), ChainInfo { account_type: AccountId32 })
		.is_ok());

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
		IdentityInfo { addresses: vec![(polkadot.clone(), encoded_address)] }
	);

	// Bob is not allowed to remove an address from alice's identity.
	set_caller::<DefaultEnvironment>(bob);
	assert_eq!(identity.remove_address(polkadot.clone()), Err(Error::NotAllowed));

	set_caller::<DefaultEnvironment>(alice);
	assert!(identity.remove_address(polkadot.clone()).is_ok());

	assert_eq!(recorded_events().count(), 4);
	let last_event = recorded_events().last().unwrap();
	let decoded_event = <Event as scale::Decode>::decode(&mut &last_event.data[..])
		.expect("Failed to decode event");

	let Event::AddressRemoved(AddressRemoved { identity_no, chain }) = decoded_event else {
		panic!("AddressRemoved event should be emitted")
	};

	assert_eq!(identity_no, 0);
	assert_eq!(chain, polkadot.clone());

	assert_eq!(identity.number_to_identity.get(0).unwrap(), IdentityInfo { addresses: vec![] });

	// Cannot remove an address from a chain that is not part of the
	// identity.
	assert_eq!(identity.remove_address(polkadot), Err(Error::InvalidChain));
}

#[ink::test]
fn remove_identity_works() {
	let DefaultAccounts::<DefaultEnvironment> { alice, bob, .. } = get_default_accounts();

	let mut identity = Identity::new();

	assert!(identity.create_identity().is_ok());

	let polkadot: ChainId = (0, Polkadot);
	assert!(identity
		.add_chain(polkadot.clone(), ChainInfo { account_type: AccountId32 })
		.is_ok());

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
		IdentityInfo { addresses: vec![(polkadot, encoded_address)] }
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

	let Event::IdentityRemoved(IdentityRemoved { identity_no }) = decoded_event else {
		panic!("IdentityRemoved event should be emitted")
	};

	assert_eq!(identity_no, 0);

	// Make sure all of the state got removed.
	assert_eq!(identity.owner_of.get(0), None);
	assert_eq!(identity.identity_of.get(alice), None);
	assert_eq!(identity.number_to_identity.get(0), None);
}

#[ink::test]
fn address_size_limit_works() {
	let mut identity = Identity::new();

	let polkadot = (0, Polkadot);
	assert!(identity.create_identity().is_ok());
	assert!(identity
		.add_chain(polkadot.clone(), ChainInfo { account_type: AccountId32 })
		.is_ok());

	let mut polkadot_address: Vec<u8> = vec![];
	(0..150).for_each(|n| polkadot_address.push(n));

	assert_eq!(identity.add_address(polkadot, polkadot_address), Err(Error::AddressSizeExceeded));
}

#[ink::test]
fn add_chain_works() {
	let DefaultAccounts::<DefaultEnvironment> { alice, bob, .. } = get_default_accounts();

	let mut identity = Identity::new();
	assert_eq!(identity.admin, alice);

	// Adding a chain successful
	assert!(identity.add_chain((0, Kusama), ChainInfo { account_type: AccountId32 }).is_ok());

	// Check emitted events
	assert_eq!(recorded_events().count(), 1);
	let last_event = recorded_events().last().unwrap();
	let decoded_event = <Event as scale::Decode>::decode(&mut &last_event.data[..])
		.expect("Failed to decode event");

	let Event::ChainAdded(ChainAdded { chain_id, account_type }) = decoded_event else {
		panic!("ChainAdded event should be emitted")
	};

	assert_eq!(chain_id.clone(), (0, Kusama));
	assert_eq!(account_type, AccountId32);

	let info = ChainInfo { account_type: AccountId32 };

	// Check storage items updated
	assert_eq!(identity.chain_info_of.get(chain_id.clone()), Some(info.clone()));
	assert_eq!(identity.available_chains(), vec![(chain_id, info)]);
	assert_eq!(identity.chain_ids, vec![(0, Kusama)]);

	// Only the contract creator can add a new chain
	set_caller::<DefaultEnvironment>(bob);
	assert_eq!(
		identity.add_chain((2004, Kusama), ChainInfo { account_type: AccountId32 }),
		Err(Error::NotAllowed)
	);

	set_caller::<DefaultEnvironment>(alice);
}

#[ink::test]
fn remove_chain_works() {
	let DefaultAccounts::<DefaultEnvironment> { alice, bob, .. } = get_default_accounts();
	let account_type = AccountId32;

	let mut identity = Identity::new();
	assert_eq!(identity.admin, alice);

	let chain_id = (0, Kusama);
	assert!(
		identity.add_chain(chain_id.clone(), ChainInfo { account_type }).is_ok(),
		"Failed to add chain"
	);

	// Remove chain: chain doesn't exist
	assert_eq!(identity.remove_chain((chain_id.0.clone(), Polkadot)), Err(Error::InvalidChain));

	// Only the contract owner can remove a chain
	set_caller::<DefaultEnvironment>(bob);
	assert_eq!(identity.remove_chain(chain_id.clone()), Err(Error::NotAllowed));

	// Remove chain successful
	set_caller::<DefaultEnvironment>(alice);
	assert!(identity.remove_chain(chain_id.clone()).is_ok());

	assert!(identity.chain_info_of.get(chain_id.clone()).is_none());

	assert!(identity.available_chains().is_empty());

	// Check emitted events
	let last_event = recorded_events().last().unwrap();
	let decoded_event = <Event as scale::Decode>::decode(&mut &last_event.data[..])
		.expect("Failed to decode event");

	let Event::ChainRemoved(ChainRemoved { chain_id: removed_chain_id }) = decoded_event else {
		panic!("ChainRemoved event should be emitted")
	};

	assert_eq!(removed_chain_id, chain_id);
}

#[ink::test]
fn update_chain_works() {
	let DefaultAccounts::<DefaultEnvironment> { alice, bob, .. } = get_default_accounts();

	let account_type = AccountId32;

	let mut identity = Identity::new();
	assert_eq!(identity.admin, alice);

	let polkadot_id = (0, Polkadot);
	assert!(
		identity
			.add_chain(polkadot_id.clone(), ChainInfo { account_type: account_type.clone() })
			.is_ok(),
		"Failed to add chain"
	);

	assert!(identity.add_chain((2000, Polkadot), ChainInfo { account_type }).is_ok());

	// Only the contract owner can update a chain
	set_caller::<DefaultEnvironment>(bob);
	assert_eq!(
		identity.update_chain(polkadot_id.clone(), Some(AccountKey20)),
		Err(Error::NotAllowed)
	);

	set_caller::<DefaultEnvironment>(alice);

	// Must be an existing chain.
	assert_eq!(identity.update_chain((3, Polkadot), None), Err(Error::InvalidChain));

	// Update chain success.
	assert!(identity.update_chain(polkadot_id.clone(), Some(AccountKey20)).is_ok());

	// Check the emitted events
	assert_eq!(recorded_events().count(), 3);
	let last_event = recorded_events().last().unwrap();
	let decoded_event = <Event as scale::Decode>::decode(&mut &last_event.data[..])
		.expect("Failed to decode event");

	let Event::ChainUpdated(ChainUpdated {
		chain_id: chain_updated,
		account_type: updated_account_type,
	}) = decoded_event
	else {
		panic!("ChainUpdated event should be emitted")
	};

	assert_eq!(chain_updated, polkadot_id);
	assert_eq!(updated_account_type, AccountKey20);
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
		decoded_event
	else {
		panic!("RecoveryAccountSet event should be emitted")
	};

	assert_eq!(identity_no, 0);
	assert_eq!(recovery_account, bob);

	assert_eq!(identity.recovery_account_of.get(identity_no), Some(bob));
}

#[ink::test]
fn transfer_ownership_works() {
	let DefaultAccounts::<DefaultEnvironment> { alice, bob, .. } = get_default_accounts();
	let identity_no = 0;

	let mut identity = Identity::new();

	let polkadot_id = (0, Polkadot);
	assert!(
		identity
			.add_chain(polkadot_id.clone(), ChainInfo { account_type: AccountId32 })
			.is_ok(),
		"Failed to add chain"
	);

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
		IdentityInfo { addresses: vec![(polkadot_id, encoded_address)] }
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
fn init_with_chains_works() {
	let chains = vec![
		ChainInfo { account_type: AccountId32 },
		ChainInfo { account_type: AccountId32 },
		ChainInfo { account_type: AccountKey20 },
		ChainInfo { account_type: AccountId32 },
	];
	let chain_ids = vec![(0, Polkadot), (2000, Polkadot), (2004, Polkadot), (2006, Polkadot)];
	let identity = Identity::init_with_chains(chains, chain_ids.clone());

	assert_eq!(
		identity.chain_info_of((0, Polkadot)),
		Some(ChainInfo { account_type: AccountId32 })
	);
	assert_eq!(
		identity.chain_info_of((2000, Polkadot)),
		Some(ChainInfo { account_type: AccountId32 })
	);
	assert_eq!(
		identity.chain_info_of((2004, Polkadot)),
		Some(ChainInfo { account_type: AccountKey20 })
	);
	assert_eq!(
		identity.chain_info_of((2006, Polkadot)),
		Some(ChainInfo { account_type: AccountId32 })
	);

	assert_eq!(identity.chain_ids, chain_ids);
	assert_eq!(
		identity.available_chains(),
		vec![
			((0, Polkadot), ChainInfo { account_type: AccountId32 }),
			((2000, Polkadot), ChainInfo { account_type: AccountId32 }),
			((2004, Polkadot), ChainInfo { account_type: AccountKey20 }),
			((2006, Polkadot), ChainInfo { account_type: AccountId32 })
		]
	);
}

#[ink::test]
fn getting_transaction_destination_works() {
	let DefaultAccounts::<DefaultEnvironment> { alice, .. } = get_default_accounts();
	let identity_no = 0;

	let mut identity = Identity::new();

	let polkadot_id = (0, Polkadot);
	assert!(
		identity
			.add_chain(polkadot_id.clone(), ChainInfo { account_type: AccountId32 })
			.is_ok(),
		"Failed to add chain"
	);

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

	assert_eq!(
		identity.transaction_destination(identity_no, polkadot_id.clone()),
		Ok(encoded_address)
	);

	// Fails since the provided `identity_no` does not exist.
	assert_eq!(identity.transaction_destination(42, polkadot_id), Err(Error::IdentityDoesntExist));

	let moonbeam_id = (2004, Polkadot);
	// Fails because alice does not have an address on the Moonbeam chain.
	assert!(
		identity
			.add_chain(moonbeam_id.clone(), ChainInfo { account_type: AccountId32 })
			.is_ok(),
		"Failed to add chain"
	);

	assert_eq!(
		identity.transaction_destination(identity_no, moonbeam_id),
		Err(Error::InvalidChain)
	);
}

fn get_default_accounts() -> DefaultAccounts<DefaultEnvironment> {
	default_accounts::<DefaultEnvironment>()
}
