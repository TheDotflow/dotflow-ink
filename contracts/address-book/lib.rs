//! The source code of the address book contract.

#![cfg_attr(not(feature = "std"), no_std, no_main)]

use common::types::*;
use ink::prelude::vec::Vec;

#[cfg(test)]
mod tests;

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

#[derive(scale::Encode, scale::Decode, Debug, PartialEq)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum Error {
	/// The user already has an address book
	AddressBookAlreadyCreated,
	/// The user doesn't have an address book yet
	AddressBookDoesntExist,
	/// The given identity no is not valid
	InvalidIdentityNo,
	/// The given identity is already added
	IdentityAlreadyAdded,
	/// The given nickname is too long
	NickNameTooLong,
}

#[ink::contract]
mod address_book {
	use super::*;
	use crate::types::*;
	use ink::{
		env::{
			call::{build_call, ExecutionInput, Selector},
			DefaultEnvironment,
		},
		storage::Mapping,
	};

	#[ink(storage)]
	pub struct AddressBook {
		/// Each address book is associated with an `AccountId`.
		///
		/// NOTE: One account can only own one address book.
		pub(crate) address_book_of: Mapping<AccountId, AddressBookInfo>,

		/// Address of the `Identity` contract. This is set during contract
		/// deployment and can't be changed later.
		pub(crate) identity_contract: AccountId,
	}

	#[ink(event)]
	pub struct AddressBookCreated {
		#[ink(topic)]
		pub(crate) owner: AccountId,
	}

	#[ink(event)]
	pub struct AddressBookRemoved {
		#[ink(topic)]
		pub(crate) owner: AccountId,
	}

	#[ink(event)]
	pub struct IdentityContractSet {
		pub(crate) address: AccountId,
	}

	impl AddressBook {
		/// Constructor
		/// Instantiate with the address of `Identity` contract
		#[ink(constructor)]
		pub fn new(identity_contract: AccountId) -> Self {
			AddressBook { address_book_of: Default::default(), identity_contract }
		}

		/// Creates an address book for a user
		#[ink(message)]
		pub fn identity_contract(&self) -> AccountId {
			self.identity_contract
		}

		/// Creates an address book for a user
		#[ink(message)]
		pub fn create_address_book(&mut self) -> Result<(), Error> {
			let caller = self.env().caller();

			ensure!(self.address_book_of.get(caller).is_none(), Error::AddressBookAlreadyCreated);
			self.address_book_of
				.insert(caller, &AddressBookInfo { identities: Default::default() });

			ink::env::emit_event::<DefaultEnvironment, _>(AddressBookCreated { owner: caller });

			Ok(())
		}

		/// Removes the address book of a user
		#[ink(message)]
		pub fn remove_address_book(&mut self) -> Result<(), Error> {
			let caller = self.env().caller();

			ensure!(self.address_book_of.get(caller).is_some(), Error::AddressBookDoesntExist);

			self.address_book_of.remove(caller);

			ink::env::emit_event::<DefaultEnvironment, _>(AddressBookRemoved { owner: caller });

			Ok(())
		}

		/// Adds an identity to the user's address book
		#[ink(message)]
		pub fn add_identity(
			&mut self,
			identity_no: IdentityNo,
			nickname: Option<Nickname>,
		) -> Result<(), Error> {
			let caller = self.env().caller();
			let mut address_book: AddressBookInfo = self
				.address_book_of
				.get(caller)
				.map_or(Err(Error::AddressBookDoesntExist), Ok)?;

			let identity = build_call::<DefaultEnvironment>()
				.call(self.identity_contract)
				.gas_limit(0)
				.exec_input(
					ExecutionInput::new(Selector::new(ink::selector_bytes!("identity")))
						.push_arg(identity_no),
				)
				.returns::<Option<()>>()
				.invoke();

			ensure!(identity.is_some(), Error::InvalidIdentityNo);

			address_book.add_identity(identity_no, nickname)?;

			Ok(())
		}

		/// Removes an identity from the user's address book
		#[ink(message)]
		pub fn remove_identity(&mut self, identity_no: IdentityNo) {
			// TODO:
		}

		/// Update nickname of an identity
		#[ink(message)]
		pub fn update_nickname(&mut self, identity_no: IdentityNo, new_nickname: Option<Nickname>) {
			// TODO:
		}

		/// Returns the identities stored in the address book of a user
		#[ink(message)]
		pub fn identities_of(&self, account: AccountId) -> Vec<IdentityRecord> {
			if let Some(address_book) = self.address_book_of.get(account) {
				address_book.identities
			} else {
				Vec::default()
			}
		}
	}

	#[cfg(all(test, feature = "e2e-tests"))]
	mod e2e_tests {
		use super::AddressBookRef;
		use identity::IdentityRef;
		use ink_e2e::build_message;

		type E2EResult<T> = std::result::Result<T, Box<dyn std::error::Error>>;

		#[ink_e2e::test]
		async fn constructor_works(mut client: ink_e2e::Client<C, E>) -> E2EResult<()> {
			let identity_constructor = IdentityRef::new();

			let identity_acc_id = client
				.instantiate("identity", &ink_e2e::alice(), identity_constructor, 0, None)
				.await
				.expect("instantiate failed")
				.account_id;

			let book_constructor = AddressBookRef::new(identity_acc_id);

			let book_acc_id = client
				.instantiate("address-book", &ink_e2e::alice(), book_constructor, 0, None)
				.await
				.expect("instantiate failed")
				.account_id;

			let get_identity_contract = build_message::<AddressBookRef>(book_acc_id.clone())
				.call(|address_book| address_book.identity_contract());

			let get_identity_contract_result = client
				.call_dry_run(&ink_e2e::bob(), &get_identity_contract, 0, None)
				.await
				.return_value();
			assert_eq!(get_identity_contract_result, identity_acc_id);

			Ok(())
		}

		#[ink_e2e::test]
		async fn add_identity_works(mut client: ink_e2e::Client<C, E>) -> E2EResult<()> {
			let identity_constructor = IdentityRef::init_with_networks(vec![]);

			let identity_acc_id = client
				.instantiate("identity", &ink_e2e::alice(), identity_constructor, 0, None)
				.await
				.expect("instantiate failed")
				.account_id;

			let book_constructor = AddressBookRef::new(identity_acc_id);

			let book_acc_id = client
				.instantiate("address-book", &ink_e2e::alice(), book_constructor, 0, None)
				.await
				.expect("instantiate failed")
				.account_id;

			Ok(())
		}
	}
}
