//! Ink! integration tests convering the address book contract functionality.
use crate::{address_book::*, types::*, *};
use ink::{
	env::{
		test::{default_accounts, set_caller, DefaultAccounts},
		DefaultEnvironment,
	},
	primitives::AccountId,
};
use ink_e2e::subxt::storage::address;

#[ink::test]
fn constructor_works() {
	let address_book = AddressBook::new();
	let DefaultAccounts::<DefaultEnvironment> { alice, .. } = get_default_accounts();

	// The `address_book_of` storage mapping should be empty.
	assert!(address_book.address_book_of.get(alice).is_none());
	assert_eq!(address_book.admin, alice);
	assert!(address_book.identity_contract.is_none());
}

#[ink::test]
fn create_address_book_works() {
	let mut book = AddressBook::new();
	let DefaultAccounts::<DefaultEnvironment> { alice, .. } = get_default_accounts();

	assert_eq!(book.create_address_book(), Ok(()));
	assert_eq!(
		book.address_book_of.get(alice),
		Some(AddressBookInfo { identities: Vec::default() })
	);

	assert_eq!(book.create_address_book(), Err(Error::AddressBookAlreadyCreated));
}

#[ink::test]
fn remove_address_book_works() {
	let mut book = AddressBook::new();

	assert_eq!(book.remove_address_book(), Err(Error::AddressBookDoesntExist));
	assert_eq!(book.create_address_book(), Ok(()));
	assert_eq!(book.remove_address_book(), Ok(()));
}

#[ink::test]
fn set_identity_contract_works() {
	let mut book = AddressBook::new();
	let DefaultAccounts::<DefaultEnvironment> { bob, .. } = get_default_accounts();

	assert!(book.identity_contract.is_none());

	let mock_address = AccountId::from([0x01; 32]);

	assert_eq!(book.set_identity_contract(mock_address), Ok(()));
	assert_eq!(book.identity_contract, Some(mock_address));

	assert_eq!(book.set_identity_contract(mock_address), Err(Error::IdentityContractAlreadySet));

	set_caller::<DefaultEnvironment>(bob);

	assert_eq!(book.set_identity_contract(mock_address), Err(Error::NotContractOwner));
}

fn get_default_accounts() -> DefaultAccounts<DefaultEnvironment> {
	default_accounts::<DefaultEnvironment>()
}

#[cfg(all(test, feature = "e2e-tests"))]
mod e2e_tests {}
