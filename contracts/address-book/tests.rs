//! Ink! integration tests convering the address book contract functionality.
use crate::{address_book::*, types::*, *};
use ink::{
	env::{
		test::{default_accounts, DefaultAccounts},
		DefaultEnvironment,
	},
	primitives::AccountId,
};

#[ink::test]
fn constructor_works() {
	let identity_contract = get_identity_contract_address();
	let address_book = AddressBook::new(identity_contract);

	let DefaultAccounts::<DefaultEnvironment> { alice, .. } = get_default_accounts();

	// The `address_book_of` storage mapping should be empty.
	assert!(address_book.address_book_of.get(alice).is_none());

	assert_eq!(address_book.identity_contract, identity_contract);
}

#[ink::test]
fn create_address_book_works() {
	let identity_contract = get_identity_contract_address();
	let mut book = AddressBook::new(identity_contract);

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
	let identity_contract = get_identity_contract_address();
	let mut book = AddressBook::new(identity_contract);

	assert_eq!(book.remove_address_book(), Err(Error::AddressBookDoesntExist));
	assert_eq!(book.create_address_book(), Ok(()));
	assert_eq!(book.remove_address_book(), Ok(()));
}

#[ink::test]
fn add_identity_works() {
	let DefaultAccounts::<DefaultEnvironment> { alice, .. } = get_default_accounts();
	let identity_contract = get_identity_contract_address();
	let mut book = AddressBook::new(identity_contract);

	assert_eq!(book.add_identity(0, None), Err(Error::AddressBookDoesntExist));

	assert_eq!(book.create_address_book(), Ok(()));

	assert_eq!(book.add_identity(0, None), Err(Error::InvalidIdentityNo));

	let long_nickname =
		String::from_utf8(vec![b'a'; (NICKNAME_LENGTH_LIMIT + 1) as usize]).unwrap();
	assert_eq!(book.add_identity(1, Some(long_nickname)), Err(Error::NickNameTooLong));

	assert_eq!(book.add_identity(1, Some("nickname".to_string())), Ok(()));

	assert_eq!(book.identities_of(alice), vec![(Some("nickname".to_string()), 1)]);

	assert_eq!(book.add_identity(1, None), Err(Error::IdentityAlreadyAdded));
}

fn get_default_accounts() -> DefaultAccounts<DefaultEnvironment> {
	default_accounts::<DefaultEnvironment>()
}

fn get_identity_contract_address() -> AccountId {
	AccountId::from([
		0x7b, 0x02, 0xe6, 0x2d, 0x7a, 0xcc, 0x3b, 0x38, 0x35, 0x9e, 0x3d, 0x88, 0x77, 0x93, 0x60,
		0x32, 0xf9, 0x22, 0x37, 0x57, 0x5e, 0x9a, 0xa6, 0xee, 0xd7, 0x35, 0x78, 0x68, 0x0d, 0xb1,
		0x50, 0xd5,
	])
}

#[cfg(all(test, feature = "e2e-tests"))]
mod e2e_tests {}
