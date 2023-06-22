//! The source code of the address book contract.

#![cfg_attr(not(feature = "std"), no_std, no_main)]

mod types;

#[macro_export]
macro_rules! ensure {
	( $x:expr, $y:expr $(,)? ) => {{
		if !$x {
			return Err($y)
		}
	}};
}

/// The maximum number of chars the nickname can hold.
const NICKNAME_LENGTH_LIMIT: u8 = 16;

#[ink::contract]
mod address_book {
	use super::*;
	use crate::types::*;
	use ink::storage::Mapping;

	#[ink(storage)]
	pub struct AddressBook {
		/// Each address book is associated with an `AccountId`.
		///
		/// NOTE: One account can only own one address book.
		address_book_of: Mapping<AccountId, AddressBookInfo>,
	}

	impl AddressBook {
		#[ink(constructor)]
		pub fn new() -> Self {
			AddressBook { address_book_of: Default::default() }
		}

		#[ink(message)]
		pub fn create_address_book(&mut self) {
			// TODO
		}

		#[ink(message)]
		pub fn remove_address_book(&mut self) {
			// TODO
		}

		#[ink(message)]
		pub fn add_identity(&mut self, identity_no: IdentityNo, nickname: Option<Nickname>) {
			// TODO
		}

		#[ink(message)]
		pub fn remove_identity(&mut self, identity_no: IdentityNo) {
			// TODO
		}

		#[ink(message)]
		pub fn update_nickname(&mut self, identity_no: IdentityNo, new_nickname: Option<Nickname>) {
			// TODO
		}
	}

	#[cfg(test)]
	mod tests {
		use super::*;
		use ink::env::{
			test::{default_accounts, DefaultAccounts},
			DefaultEnvironment,
		};

		/// We test if the constructor does its job.
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
	}

	#[cfg(all(test, feature = "e2e-tests"))]
	mod e2e_tests {}
}
