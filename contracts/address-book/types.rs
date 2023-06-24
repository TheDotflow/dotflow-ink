//! Types used in the address book contract.

use ink::prelude::{string::String, vec::Vec};
#[cfg(feature = "std")]
use ink::storage::traits::StorageLayout;

use crate::*;

/// Each identity is associated with a unique identifier called `IdentityNo`.
pub type IdentityNo = u32;

pub type Nickname = String;

/// The address book struct that contains all the information that the address
/// book contract needs.
#[derive(scale::Encode, scale::Decode, Debug, Default, PartialEq, Clone)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo, StorageLayout))]
pub struct AddressBookInfo {
	/// All the identities that are part of an address book. Each identity can
	/// have an optional nickname.
	pub(crate) identities: Vec<(Option<Nickname>, IdentityNo)>,
}

impl AddressBookInfo {
	pub fn add_identity(
		&mut self,
		identity_no: IdentityNo,
		nickname: Option<Nickname>,
	) -> Result<(), Error> {
		ensure!(
			!self.identities.clone().into_iter().any(|address| address.1 == identity_no),
			Error::IdentityAlreadyAdded
		);

		self.identities.push((nickname, identity_no));

		Ok(())
	}

	pub fn remove_identity(identity_no: IdentityNo) {
		// TODO:
	}

	pub fn update_nickname(identity_no: IdentityNo, new_nickname: Option<Nickname>) {
		// TODO:
	}
}
