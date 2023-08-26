//! Ink! integration tests convering the identity contract functionality.
use crate::{identity::*, types::*, *};
use common::types::{AccountType::*, *};

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
	assert_eq!(identity.chain_id_count, 0);
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

	assert!(identity
		.add_chain(ChainInfo {
			rpc_urls: vec!["ws://polkadot.com".to_string()],
			account_type: AccountId32,
		})
		.is_ok());
	assert!(identity
		.add_chain(ChainInfo {
			rpc_urls: vec!["ws://moonbeam.com".to_string()],
			account_type: AccountId32,
		})
		.is_ok());

	let polkadot: ChainId = 0;
	let moonbeam: ChainId = 1;

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

	let Event::AddressAdded(AddressAdded { identity_no, chain, address }) = decoded_event else {
		panic!("AddressAdded event should be emitted")
	};

	assert_eq!(identity_no, 0);
	assert_eq!(chain, polkadot);
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

	assert!(identity.create_identity().is_ok());
	assert!(identity
		.add_chain(ChainInfo {
			rpc_urls: vec!["ws://polkadot.com".to_string()],
			account_type: AccountId32,
		})
		.is_ok());
	assert!(identity
		.add_chain(ChainInfo {
			rpc_urls: vec!["ws://moonbeam.com".to_string()],
			account_type: AccountId32,
		})
		.is_ok());

	assert_eq!(identity.owner_of.get(0), Some(alice));
	assert_eq!(
		identity.number_to_identity.get(0).unwrap(),
		IdentityInfo { addresses: Default::default() }
	);

	let polkadot: ChainId = 0;
	let moonbeam: ChainId = 1;

	let polkadot_address = alice.encode();

	assert!(identity.add_address(polkadot, polkadot_address.clone()).is_ok());
	assert_eq!(
		identity.number_to_identity.get(0).unwrap(),
		IdentityInfo { addresses: vec![(polkadot, polkadot_address)] }
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

	let Event::AddressUpdated(AddressUpdated { identity_no, chain, updated_address }) =
		decoded_event
	else {
		panic!("AddressUpdated event should be emitted")
	};

	assert_eq!(identity_no, 0);
	assert_eq!(chain, polkadot);
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

	assert!(identity.create_identity().is_ok());
	assert!(identity
		.add_chain(ChainInfo {
			rpc_urls: vec!["ws://polkadot.com".to_string()],
			account_type: AccountId32,
		})
		.is_ok());

	assert_eq!(identity.owner_of.get(0), Some(alice));
	assert_eq!(
		identity.number_to_identity.get(0).unwrap(),
		IdentityInfo { addresses: Default::default() }
	);

	let polkadot: ChainId = 0;
	// In reality this address would be encrypted before storing in the contract.
	let encoded_address = alice.encode();

	assert!(identity.add_address(polkadot, encoded_address.clone()).is_ok());
	assert_eq!(
		identity.number_to_identity.get(0).unwrap(),
		IdentityInfo { addresses: vec![(polkadot, encoded_address)] }
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

	let Event::AddressRemoved(AddressRemoved { identity_no, chain }) = decoded_event else {
		panic!("AddressRemoved event should be emitted")
	};

	assert_eq!(identity_no, 0);
	assert_eq!(chain, polkadot);

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

	assert!(identity
		.add_chain(ChainInfo {
			rpc_urls: vec!["ws://polkadot.com".to_string()],
			account_type: AccountId32,
		})
		.is_ok());

	assert_eq!(identity.owner_of.get(0), Some(alice));
	assert_eq!(
		identity.number_to_identity.get(0).unwrap(),
		IdentityInfo { addresses: Default::default() }
	);

	// In reality this address would be encrypted before storing in the contract.
	let encoded_address = alice.encode();
	let polkadot: ChainId = 0;

	assert!(identity.add_address(polkadot, encoded_address.clone()).is_ok());
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

	assert!(identity.create_identity().is_ok());
	assert!(identity
		.add_chain(ChainInfo {
			rpc_urls: vec!["ws://polkadot.com".to_string()],
			account_type: AccountId32,
		})
		.is_ok());

	let polkadot = 0;

	let mut polkadot_address: Vec<u8> = vec![];
	(0..150).for_each(|n| polkadot_address.push(n));

	assert_eq!(identity.add_address(polkadot, polkadot_address), Err(Error::AddressSizeExceeded));
}

#[ink::test]
fn add_chain_works() {
	let DefaultAccounts::<DefaultEnvironment> { alice, bob, .. } = get_default_accounts();

	let mut identity = Identity::new();
	assert_eq!(identity.admin, alice);

	let polkadot_rpc_urls = vec!["ws://polkadot.com".to_string()];
	let kusama_rpc_urls = vec!["ws://polkadot.com".to_string()];

	// Adding a chain successful
	assert!(identity
		.add_chain(ChainInfo { rpc_urls: polkadot_rpc_urls.clone(), account_type: AccountId32 })
		.is_ok());

	// Check emitted events
	assert_eq!(recorded_events().count(), 1);
	let last_event = recorded_events().last().unwrap();
	let decoded_event = <Event as scale::Decode>::decode(&mut &last_event.data[..])
		.expect("Failed to decode event");

	let Event::ChainAdded(ChainAdded { chain_id, rpc_urls, account_type }) = decoded_event
	else {
		panic!("ChainAdded event should be emitted")
	};

	assert_eq!(chain_id, 0);
	assert_eq!(rpc_urls, polkadot_rpc_urls);
	assert_eq!(account_type, AccountId32);

	let info = ChainInfo { rpc_urls: polkadot_rpc_urls.clone(), account_type: AccountId32 };

	// Check storage items updated
	assert_eq!(identity.chain_info_of.get(chain_id), Some(info.clone()));
	assert_eq!(identity.available_chains(), vec![(chain_id, info)]);
	assert_eq!(identity.chain_id_count, 1);

	// Only the contract creator can add a new chain
	set_caller::<DefaultEnvironment>(bob);
	assert_eq!(
		identity.add_chain(ChainInfo { rpc_urls: kusama_rpc_urls, account_type: AccountId32 }),
		Err(Error::NotAllowed)
	);

	set_caller::<DefaultEnvironment>(alice);

	// Rpc url of the chain should not be too long
	let long_rpc_urls: Vec<String> =
		vec![String::from_utf8(vec![b'a'; CHAIN_RPC_URL_LIMIT + 1]).unwrap()];
	assert_eq!(
		identity.add_chain(ChainInfo { rpc_urls: long_rpc_urls, account_type: AccountId32 }),
		Err(Error::ChainRpcUrlTooLong)
	);
}

#[ink::test]
fn remove_chain_works() {
	let DefaultAccounts::<DefaultEnvironment> { alice, bob, .. } = get_default_accounts();
	let polkadot_rpc_urls = vec!["ws://polkadot.com".to_string()];
	let account_type = AccountId32;

	let mut identity = Identity::new();
	assert_eq!(identity.admin, alice);

	let Ok(chain_id) =
		identity.add_chain(ChainInfo { rpc_urls: polkadot_rpc_urls, account_type })
	else {
		panic!("Failed to add chain")
	};

	// Remove chain: chain doesn't exist
	assert_eq!(identity.remove_chain(chain_id + 1), Err(Error::InvalidChain));

	// Only the contract owner can remove a chain
	set_caller::<DefaultEnvironment>(bob);
	assert_eq!(identity.remove_chain(chain_id), Err(Error::NotAllowed));

	// Remove chain successful
	set_caller::<DefaultEnvironment>(alice);
	assert!(identity.remove_chain(chain_id).is_ok());

	assert!(identity.chain_info_of.get(0).is_none());

	assert!(identity.available_chains().is_empty());

	// Check emitted events
	let last_event = recorded_events().last().unwrap();
	let decoded_event = <Event as scale::Decode>::decode(&mut &last_event.data[..])
		.expect("Failed to decode event");

	let Event::ChainRemoved(ChainRemoved { chain_id: removed_chain_id }) = decoded_event
	else {
		panic!("ChainRemoved event should be emitted")
	};

	assert_eq!(removed_chain_id, chain_id);
}

#[ink::test]
fn update_chain_works() {
	let DefaultAccounts::<DefaultEnvironment> { alice, bob, .. } = get_default_accounts();
	let polkadot_rpc = "ws://pokladot.com".to_string();
	let kusama_rpc = "ws://kusama.com".to_string();
	let moonbeam_rpc = "ws://moonbeam.com".to_string();

	let account_type = AccountId32;

	let mut identity = Identity::new();
	assert_eq!(identity.admin, alice);

	let Ok(polkadot_id) = identity.add_chain(ChainInfo {
		rpc_urls: vec![polkadot_rpc.clone()],
		account_type: account_type.clone(),
	}) else {
		panic!("Failed to add chain")
	};

	assert!(identity
		.add_chain(ChainInfo { rpc_urls: vec![kusama_rpc], account_type })
		.is_ok());

	// Only the contract owner can update a chain
	set_caller::<DefaultEnvironment>(bob);
	assert_eq!(
		identity.update_chain(polkadot_id, Some(moonbeam_rpc.clone()), Some(AccountKey20)),
		Err(Error::NotAllowed)
	);

	set_caller::<DefaultEnvironment>(alice);

	// Rpc url should not be too long.
	let long_rpc_url: String = String::from_utf8(vec![b'a'; CHAIN_RPC_URL_LIMIT + 1]).unwrap();
	assert_eq!(
		identity.update_chain(polkadot_id, Some(long_rpc_url), None),
		Err(Error::ChainRpcUrlTooLong)
	);

	// Must be an existing chain.
	assert_eq!(
		identity.update_chain(3, Some(moonbeam_rpc.clone()), None),
		Err(Error::InvalidChain)
	);

	let new_rpc_url = "ws://new-chain.com".to_string();
	// Update chain success.
	assert!(identity
		.update_chain(polkadot_id, Some(new_rpc_url.clone()), Some(AccountKey20))
		.is_ok());

	// Check the emitted events
	assert_eq!(recorded_events().count(), 3);
	let last_event = recorded_events().last().unwrap();
	let decoded_event = <Event as scale::Decode>::decode(&mut &last_event.data[..])
		.expect("Failed to decode event");

	let Event::ChainUpdated(ChainUpdated {
		chain_id: chain_updated,
		rpc_urls: updated_rpc,
		account_type: updated_account_type,
	}) = decoded_event
	else {
		panic!("ChainUpdated event should be emitted")
	};

	assert_eq!(chain_updated, polkadot_id);
	assert_eq!(updated_rpc, vec![polkadot_rpc, new_rpc_url]);
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

	let Ok(polkadot_id) = identity.add_chain(ChainInfo {
		rpc_urls: vec!["ws://polkadot.com".to_string()],
		account_type: AccountId32,
	}) else {
		panic!("Failed to add chain")
	};

	assert!(identity.create_identity().is_ok());

	assert_eq!(identity.owner_of.get(0), Some(alice));
	assert_eq!(
		identity.number_to_identity.get(0).unwrap(),
		IdentityInfo { addresses: Default::default() }
	);

	// In reality this address would be encrypted before storing in the contract.
	let encoded_address = alice.encode();

	assert!(identity.add_address(polkadot_id, encoded_address.clone()).is_ok());
	assert_eq!(
		identity.number_to_identity.get(0).unwrap(),
		IdentityInfo { addresses: vec![(polkadot_id, encoded_address.clone())] }
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
		IdentityInfo { addresses: vec![(polkadot_id, encoded_address.clone())] }
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
	let polkadot_rpc = "ws://polkadot.com".to_string();
	let kusama_rpc = "ws://kusama.com".to_string();
	let moonbeam_rpc = "ws://moonbeam.com".to_string();
	let astar_rpc = "ws://astar.com".to_string();

	let chains = vec![
		ChainInfo { rpc_urls: vec![polkadot_rpc.clone()], account_type: AccountId32 },
		ChainInfo { rpc_urls: vec![kusama_rpc.clone()], account_type: AccountId32 },
		ChainInfo { rpc_urls: vec![moonbeam_rpc.clone()], account_type: AccountKey20 },
		ChainInfo { rpc_urls: vec![astar_rpc.clone()], account_type: AccountId32 },
	];
	let identity = Identity::init_with_chains(chains);

	assert_eq!(
		identity.chain_info_of(0),
		Some(ChainInfo { rpc_urls: vec![polkadot_rpc.clone()], account_type: AccountId32 })
	);
	assert_eq!(
		identity.chain_info_of(1),
		Some(ChainInfo { rpc_urls: vec![kusama_rpc.clone()], account_type: AccountId32 })
	);
	assert_eq!(
		identity.chain_info_of(2),
		Some(ChainInfo { rpc_urls: vec![moonbeam_rpc.clone()], account_type: AccountKey20 })
	);
	assert_eq!(
		identity.chain_info_of(3),
		Some(ChainInfo { rpc_urls: vec![astar_rpc.clone()], account_type: AccountId32 })
	);

	assert_eq!(identity.chain_id_count, 4);
	assert_eq!(
		identity.available_chains(),
		vec![
			(0, ChainInfo { rpc_urls: vec![polkadot_rpc], account_type: AccountId32 }),
			(1, ChainInfo { rpc_urls: vec![kusama_rpc], account_type: AccountId32 }),
			(2, ChainInfo { rpc_urls: vec![moonbeam_rpc], account_type: AccountKey20 }),
			(3, ChainInfo { rpc_urls: vec![astar_rpc], account_type: AccountId32 })
		]
	);
}

#[ink::test]
#[should_panic(expected = "Chain rpc url is too long")]
fn init_with_chains_fail() {
	let rpc_url_long = String::from_utf8(vec![b'a'; CHAIN_RPC_URL_LIMIT + 1]).unwrap();
	Identity::init_with_chains(vec![ChainInfo {
		rpc_urls: vec![rpc_url_long],
		account_type: AccountId32,
	}]);
}

#[ink::test]
fn getting_transaction_destination_works() {
	let DefaultAccounts::<DefaultEnvironment> { alice, .. } = get_default_accounts();
	let identity_no = 0;

	let mut identity = Identity::new();

	let Ok(polkadot_id) = identity.add_chain(ChainInfo {
		rpc_urls: vec!["ws://polkadot.com".to_string()],
		account_type: AccountId32,
	}) else {
		panic!("Failed to add chain")
	};

	assert!(identity.create_identity().is_ok());

	assert_eq!(identity.owner_of.get(0), Some(alice));
	assert_eq!(
		identity.number_to_identity.get(0).unwrap(),
		IdentityInfo { addresses: Default::default() }
	);

	// In reality this address would be encrypted before storing in the contract.
	let encoded_address = alice.encode();

	assert!(identity.add_address(polkadot_id, encoded_address.clone()).is_ok());
	assert_eq!(
		identity.number_to_identity.get(0).unwrap(),
		IdentityInfo { addresses: vec![(polkadot_id, encoded_address.clone())] }
	);

	assert_eq!(identity.transaction_destination(identity_no, polkadot_id), Ok(encoded_address));

	// Fails since the provided `identity_no` does not exist.
	assert_eq!(identity.transaction_destination(42, polkadot_id), Err(Error::IdentityDoesntExist));

	// Fails because alice does not have an address on the Moonbeam chain.
	let Ok(moonbeam_id) = identity.add_chain(ChainInfo {
		rpc_urls: vec!["ws://moonbeam.com".to_string()],
		account_type: AccountId32,
	}) else {
		panic!("Failed to add chain")
	};

	assert_eq!(
		identity.transaction_destination(identity_no, moonbeam_id),
		Err(Error::InvalidChain)
	);
}

fn get_default_accounts() -> DefaultAccounts<DefaultEnvironment> {
	default_accounts::<DefaultEnvironment>()
}
