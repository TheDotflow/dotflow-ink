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
	/// The user already has an address book
	AddressBookAlreadyCreated,
	/// The user doesn't have an address book yet
	AddressBookDoesntExist,
	/// The caller is not the contract owner
	NotContractOwner,
	/// Address of the identity contract is already set
	IdentityContractAlreadySet,
	/// Identity contract address is not set
	IdentityContractNotSet,
	/// The given identity no is not valid
	InvalidIdentityNo,
	/// The given identity is already added
	IdentityAlreadyAdded,
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
		#[ink(constructor)]
		pub fn new(identity_contract: AccountId) -> Self {
			AddressBook { address_book_of: Default::default(), identity_contract }
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

			ensure!(self.address_book_of.get(caller).is_some(), Error::AddressBookDoesntExist);

			self.address_book_of.remove(caller);

			self.env().emit_event(AddressBookRemoved { owner: caller });

			Ok(())
		}

		#[ink(message)]
		pub fn set_identity_contract(&mut self, address: AccountId) -> Result<(), Error> {
			let caller = self.env().caller();

			// Only the contract owner can set identity contract address
			ensure!(caller == self.admin, Error::NotContractOwner);
			ensure!(self.identity_contract.is_none(), Error::IdentityContractAlreadySet);

			self.identity_contract = Some(address);

			self.env().emit_event(IdentityContractSet { address });

			Ok(())
		}

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

			let identity_contract =
				self.identity_contract.map_or(Err(Error::IdentityContractNotSet), Ok)?;

			let identity = build_call::<DefaultEnvironment>()
				.call(identity_contract)
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
