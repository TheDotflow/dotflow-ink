//! The source code of the address book contract.

#![cfg_attr(not(feature = "std"), no_std, no_main)]

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
	AddressBookAlreadyCreated,
	AddressBookNotExist,
}

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
		pub(crate) address_book_of: Mapping<AccountId, AddressBookInfo>,
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

	impl AddressBook {
		#[ink(constructor)]
		pub fn new() -> Self {
			AddressBook { address_book_of: Default::default() }
		}

		#[ink(message)]
		pub fn create_address_book(&mut self) -> Result<(), Error> {
			let caller = self.env().caller();

			ensure!(self.address_book_of.get(caller).is_none(), Error::AddressBookAlreadyCreated);
			self.address_book_of
				.insert(caller, &AddressBookInfo { identities: Default::default() });

			self.env().emit_event(AddressBookCreated { owner: caller });

			Ok(())
		}

		#[ink(message)]
		pub fn remove_address_book(&mut self) -> Result<(), Error> {
			let caller = self.env().caller();

			ensure!(self.address_book_of.get(caller).is_some(), Error::AddressBookNotExist);

			self.address_book_of.remove(caller);

			self.env().emit_event(AddressBookRemoved { owner: caller });

			Ok(())
		}

		#[ink(message)]
		pub fn add_identity(&mut self, identity_no: IdentityNo, nickname: Option<Nickname>) {
			// TODO:
		}

		#[ink(message)]
		pub fn remove_identity(&mut self, identity_no: IdentityNo) {
			// TODO:
		}

		#[ink(message)]
		pub fn update_nickname(&mut self, identity_no: IdentityNo, new_nickname: Option<Nickname>) {
			// TODO:
		}
	}
}
