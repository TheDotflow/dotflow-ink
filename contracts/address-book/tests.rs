//! Ink! integration tests convering the address book contract functionality.
use crate::{address_book::*, types::*, *};
use ink::env::{
	test::{default_accounts, DefaultAccounts},
	DefaultEnvironment,
};

#[ink::test]
fn constructor_works() {
	let address_book = AddressBook::new();
	let DefaultAccounts::<DefaultEnvironment> { alice, .. } = get_default_accounts();

	// The `address_book_of` storage mapping should be empty.
	assert_eq!(address_book.address_book_of.get(alice), None);
}

fn get_default_accounts() -> DefaultAccounts<DefaultEnvironment> {
	default_accounts::<DefaultEnvironment>()
}

#[cfg(all(test, feature = "e2e-tests"))]
mod e2e_tests {}
