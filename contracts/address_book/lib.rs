//! The source code of the address book contract.

#![cfg_attr(not(feature = "std"), no_std, no_main)]

use common::{ensure, types::*};
use ink::prelude::vec::Vec;

#[cfg(test)]
mod tests;

mod types;

/// The maximum number of chars the nickname can hold.
const NICKNAME_LENGTH_LIMIT: u8 = 16;

#[derive(scale::Encode, scale::Decode, Debug, PartialEq)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum Error {
	/// The user already has an address book.
	AddressBookAlreadyCreated,
	/// The user doesn't have an address book yet.
	AddressBookDoesntExist,
	/// The given `IdentityNo` is not valid.
	IdentityDoesntExist,
	/// The given `IdentityNo` is not added yet.
	IdentityNotAdded,
	/// The given identity is already added.
	IdentityAlreadyAdded,
	/// The given nickname is too long.
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
		/// The owner of the newly created address book.
		#[ink(topic)]
		pub(crate) owner: AccountId,
	}

	#[ink(event)]
	pub struct AddressBookRemoved {
		/// The owner of the removed address book.
		#[ink(topic)]
		pub(crate) owner: AccountId,
	}

	#[ink(event)]
	pub struct IdentityAdded {
		/// The owner of the address book.
		#[ink(topic)]
		pub(crate) owner: AccountId,
		/// The identity added to the address book.
		pub(crate) identity: IdentityNo,
	}

	#[ink(event)]
	pub struct NickNameUpdated {
		/// The owner of the address book.
		#[ink(topic)]
		pub(crate) owner: AccountId,
		/// The identity that received a new nickname.
		pub(crate) identity_no: IdentityNo,
		/// The new nickname.
		pub(crate) new_nickname: Option<Nickname>,
	}

	#[ink(event)]
	pub struct IdentityRemoved {
		pub(crate) owner: AccountId,
		pub(crate) identity: IdentityNo,
	}

	impl AddressBook {
		/// Constructor
		/// Instantiate with the address of `Identity` contract.
		#[ink(constructor)]
		pub fn new(identity_contract: AccountId) -> Self {
			AddressBook { address_book_of: Default::default(), identity_contract }
		}

		/// Returns the address of the identity contract.
		#[ink(message)]
		pub fn identity_contract(&self) -> AccountId {
			self.identity_contract
		}

		/// Creates an address book for the caller.
		#[ink(message)]
		pub fn create_address_book(&mut self) -> Result<(), Error> {
			let caller = self.env().caller();

			// Only one address book per user.
			ensure!(self.address_book_of.get(caller).is_none(), Error::AddressBookAlreadyCreated);
			self.address_book_of
				.insert(caller, &AddressBookInfo { identities: Default::default() });

			ink::env::emit_event::<DefaultEnvironment, _>(AddressBookCreated { owner: caller });

			Ok(())
		}

		/// Removes the address book of the caller.
		#[ink(message)]
		pub fn remove_address_book(&mut self) -> Result<(), Error> {
			let caller = self.env().caller();

			ensure!(self.address_book_of.get(caller).is_some(), Error::AddressBookDoesntExist);

			self.address_book_of.remove(caller);

			ink::env::emit_event::<DefaultEnvironment, _>(AddressBookRemoved { owner: caller });

			Ok(())
		}

		/// Adds an identity to the user's address book.
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

			// Ensure that the provided `identity_no` is existent by calling the
			// identity contract.
			let identity = build_call::<DefaultEnvironment>()
				.call(self.identity_contract)
				.gas_limit(0)
				.exec_input(
					ExecutionInput::new(Selector::new(ink::selector_bytes!("identity")))
						.push_arg(identity_no),
				)
				.returns::<Option<()>>()
				.invoke();

			ensure!(identity.is_some(), Error::IdentityDoesntExist);

			address_book.add_identity(identity_no, nickname)?;
			self.address_book_of.insert(caller, &address_book);

			ink::env::emit_event::<DefaultEnvironment, _>(IdentityAdded {
				owner: caller,
				identity: identity_no,
			});

			Ok(())
		}

		/// Removes an identity from the user's address book.
		#[ink(message)]
		pub fn remove_identity(&mut self, identity_no: IdentityNo) -> Result<(), Error> {
			let caller = self.env().caller();

			let mut address_book: AddressBookInfo = self
				.address_book_of
				.get(caller)
				.map_or(Err(Error::AddressBookDoesntExist), Ok)?;

			address_book.remove_identity(identity_no)?;
			self.address_book_of.insert(caller, &address_book);

			ink::env::emit_event::<DefaultEnvironment, _>(IdentityRemoved {
				owner: caller,
				identity: identity_no,
			});

			Ok(())
		}

		/// Update nickname of an identity.
		#[ink(message)]
		pub fn update_nickname(
			&mut self,
			identity_no: IdentityNo,
			new_nickname: Option<Nickname>,
		) -> Result<(), Error> {
			let caller = self.env().caller();
			let mut address_book = self
				.address_book_of
				.get(caller)
				.map_or(Err(Error::AddressBookDoesntExist), Ok)?;

			address_book.update_nickname(identity_no, new_nickname.clone())?;
			self.address_book_of.insert(caller, &address_book);

			ink::env::emit_event::<DefaultEnvironment, _>(NickNameUpdated {
				owner: caller,
				identity_no,
				new_nickname,
			});

			Ok(())
		}

		/// Returns the identities stored in the address book of a user.
		#[ink(message)]
		pub fn identities_of(&self, account: AccountId) -> Vec<IdentityRecord> {
			self.address_book_of.get(account).unwrap_or_default().identities
		}

		/// Returns whether the user has created an address book or not
		#[ink(message)]
		pub fn has_address_book(&self) -> bool {
			let caller = self.env().caller();
			self.address_book_of.get(caller).is_some()
		}
	}

	#[cfg(all(test, feature = "e2e-tests"))]
	mod e2e_tests {
		use super::*;
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

			// Alice creates an address book
			let create_address_book_call = build_message::<AddressBookRef>(book_acc_id)
				.call(|address_book| address_book.create_address_book());
			client
				.call(&ink_e2e::alice(), create_address_book_call, 0, None)
				.await
				.expect("failed to create an address book");

			// Error: Alice tries to add an identity that hasn't been created yet.

			let add_identity_call = build_message::<AddressBookRef>(book_acc_id)
				.call(|address_book| address_book.add_identity(0, Some("bob".to_string())));
			assert_eq!(
				client
					.call_dry_run(&ink_e2e::alice(), &add_identity_call, 0, None)
					.await
					.return_value(),
				Err(Error::IdentityDoesntExist)
			);

			// Bob creates his identity
			let create_identity_call = build_message::<IdentityRef>(identity_acc_id)
				.call(|identity| identity.create_identity());
			client
				.call(&ink_e2e::bob(), create_identity_call, 0, None)
				.await
				.expect("failed to create an identity");

			// Alice adds an identity with too long nickname
			let add_identity_with_too_long_nickname_call =
				build_message::<AddressBookRef>(book_acc_id).call(|address_book| {
					address_book.add_identity(
						0, // identityNo
						Some(
							String::from_utf8(vec![b'a'; (NICKNAME_LENGTH_LIMIT + 1) as usize])
								.unwrap(),
						),
					)
				});

			// The nickname of the identity has to be less or equal to the `NICKNAME_LENGTH_LIMIT`.
			assert_eq!(
				client
					.call_dry_run(
						&ink_e2e::alice(),
						&add_identity_with_too_long_nickname_call,
						0,
						None
					)
					.await
					.return_value(),
				Err(Error::NickNameTooLong)
			);

			// Now Alice can successfully add Bob's identity to the address book.
			client
				.call(&ink_e2e::alice(), add_identity_call, 0, None)
				.await
				.expect("Failed to add an identity into an address book");

			// Check contract storage
			let call_identities_of_alice =
				build_message::<AddressBookRef>(book_acc_id).call(|address_book| {
					address_book.identities_of(ink_e2e::account_id(ink_e2e::AccountKeyring::Alice))
				});

			let identities = client
				.call(&ink_e2e::alice(), call_identities_of_alice, 0, None)
				.await
				.expect("Failed to get identities of alice")
				.return_value();

			assert_eq!(identities, vec![(0, Some("bob".to_string()))]);

			// Error: Cannot add the same identity twice.
			let call_add_same_identity_twice = build_message::<AddressBookRef>(book_acc_id)
				.call(|address_book| address_book.add_identity(0, Some("bob".to_string())));
			assert_eq!(
				client
					.call_dry_run(&ink_e2e::alice(), &call_add_same_identity_twice, 0, None)
					.await
					.return_value(),
				Err(Error::IdentityAlreadyAdded)
			);

			Ok(())
		}

		#[ink_e2e::test]
		async fn remove_identity_works(mut client: ink_e2e::Client<C, E>) -> E2EResult<()> {
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

			// Alice creates an address book
			let create_address_book_call = build_message::<AddressBookRef>(book_acc_id)
				.call(|address_book| address_book.create_address_book());
			client
				.call(&ink_e2e::alice(), create_address_book_call, 0, None)
				.await
				.expect("failed to create an address book");

			// Bob creates his identity
			let create_identity_call = build_message::<IdentityRef>(identity_acc_id)
				.call(|identity| identity.create_identity());
			client
				.call(&ink_e2e::bob(), create_identity_call, 0, None)
				.await
				.expect("failed to create an identity");

			// Now Alice can successfully add Bob's identity to the address book.
			let add_identity_call = build_message::<AddressBookRef>(book_acc_id)
				.call(|address_book| address_book.add_identity(0, Some("bob".to_string())));
			client
				.call(&ink_e2e::alice(), add_identity_call, 0, None)
				.await
				.expect("Failed to add an identity into an address book");

			// Check contract storage
			let call_identities_of_alice =
				build_message::<AddressBookRef>(book_acc_id).call(|address_book| {
					address_book.identities_of(ink_e2e::account_id(ink_e2e::AccountKeyring::Alice))
				});

			let identities = client
				.call(&ink_e2e::alice(), call_identities_of_alice.clone(), 0, None)
				.await
				.expect("Failed to get identities of alice")
				.return_value();

			assert_eq!(identities, vec![(0, Some("bob".to_string()))]);

			// Fails. Cannot remove an identity that is not part of the address book.

			let remove_invalid_identity_call = build_message::<AddressBookRef>(book_acc_id)
				.call(|address_book| address_book.remove_identity(1));

			assert!(client
				.call(&ink_e2e::alice(), remove_invalid_identity_call, 0, None)
				.await
				.is_err());

			// Success. Alice removes bob from her address book.
			let remove_identity_call = build_message::<AddressBookRef>(book_acc_id)
				.call(|address_book| address_book.remove_identity(0));

			client
				.call(&ink_e2e::alice(), remove_identity_call, 0, None)
				.await
				.expect("Failed to remove an identity from an address book");

			assert_eq!(
				client
					.call(&ink_e2e::alice(), call_identities_of_alice, 0, None)
					.await
					.expect("Failed to get identities of alice")
					.return_value(),
				vec![]
			);

			Ok(())
		}

		#[ink_e2e::test]
		async fn update_nickname_works(mut client: ink_e2e::Client<C, E>) -> E2EResult<()> {
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

			// Alice creates an address book
			let create_address_book_call = build_message::<AddressBookRef>(book_acc_id)
				.call(|address_book| address_book.create_address_book());
			client
				.call(&ink_e2e::alice(), create_address_book_call, 0, None)
				.await
				.expect("failed to create an address book");

			// Bob creates his identity
			let create_identity_call = build_message::<IdentityRef>(identity_acc_id)
				.call(|identity| identity.create_identity());
			client
				.call(&ink_e2e::bob(), create_identity_call, 0, None)
				.await
				.expect("failed to create an identity");

			// Alice adds Bob's identity to the address book.
			let add_identity_call = build_message::<AddressBookRef>(book_acc_id)
				.call(|address_book| address_book.add_identity(0, Some("bob".to_string())));
			client
				.call(&ink_e2e::alice(), add_identity_call, 0, None)
				.await
				.expect("Failed to add an identity into an address book");

			// Error: Cannot update the nickname of an identity not added to the address book
			let update_nick_name_invalid_identity = build_message::<AddressBookRef>(book_acc_id)
				.call(|address_book| {
					address_book.update_nickname(1, Some("new_nickname".to_string()))
				});

			assert_eq!(
				client
					.call_dry_run(&ink_e2e::alice(), &update_nick_name_invalid_identity, 0, None)
					.await
					.return_value(),
				Err(Error::IdentityNotAdded)
			);

			// Error: Length of the new nickname is too long
			let update_nick_name_too_long =
				build_message::<AddressBookRef>(book_acc_id).call(|address_book| {
					address_book.update_nickname(
						0,
						Some(
							String::from_utf8(vec![b'a'; (NICKNAME_LENGTH_LIMIT + 1) as usize])
								.unwrap(),
						),
					)
				});

			assert_eq!(
				client
					.call_dry_run(&ink_e2e::alice(), &update_nick_name_too_long, 0, None)
					.await
					.return_value(),
				Err(Error::NickNameTooLong)
			);

			// Success: update nickname
			let call_update_nickname =
				build_message::<AddressBookRef>(book_acc_id).call(|address_book| {
					address_book.update_nickname(0, Some("new_nickname".to_string()))
				});

			client
				.call(&ink_e2e::alice(), call_update_nickname, 0, None)
				.await
				.expect("Failed to update the nickname");

			// Check contract storage
			let call_identities_of_alice =
				build_message::<AddressBookRef>(book_acc_id).call(|address_book| {
					address_book.identities_of(ink_e2e::account_id(ink_e2e::AccountKeyring::Alice))
				});

			let identities = client
				.call(&ink_e2e::alice(), call_identities_of_alice, 0, None)
				.await
				.expect("Failed to get identities of alice")
				.return_value();

			assert_eq!(identities, vec![(0, Some("new_nickname".to_string()))]);

			Ok(())
		}
	}
}
