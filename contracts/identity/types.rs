//! Types used in the identity contract.

use crate::{ensure, Error, ADDRESS_SIZE_LIMIT};
use common::types::*;
use ink::prelude::vec::Vec;

#[cfg(feature = "std")]
use ink::storage::traits::StorageLayout;

#[derive(scale::Encode, scale::Decode, Debug, Default, PartialEq)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo, StorageLayout))]
pub struct IdentityInfo {
	/// Each address is associated with a specific blockchain.
	pub(crate) addresses: Vec<(ChainId, ChainAddress)>,
}

impl IdentityInfo {
	/// Adds an address for the given chain
	pub fn add_address(&mut self, chain: ChainId, address: ChainAddress) -> Result<(), Error> {
		ensure!(address.len() <= ADDRESS_SIZE_LIMIT, Error::AddressSizeExceeded);

		ensure!(
			!self.addresses.clone().into_iter().any(|address| address.0 == chain),
			Error::AddressAlreadyAdded
		);
		self.addresses.push((chain, address));

		Ok(())
	}

	/// Updates the address of the given chain
	pub fn update_address(
		&mut self,
		chain: ChainId,
		new_address: ChainAddress,
	) -> Result<(), Error> {
		ensure!(new_address.len() <= ADDRESS_SIZE_LIMIT, Error::AddressSizeExceeded);

		if let Some(position) =
			self.addresses.clone().into_iter().position(|address| address.0 == chain)
		{
			self.addresses[position] = (chain, new_address);
			Ok(())
		} else {
			Err(Error::InvalidChain)
		}
	}

	/// Remove an address record by chain
	pub fn remove_address(&mut self, chain: ChainId) -> Result<(), Error> {
		let old_count = self.addresses.len();
		self.addresses.retain(|(net, _)| *net != chain);

		let new_count = self.addresses.len();

		if old_count == new_count {
			Err(Error::InvalidChain)
		} else {
			Ok(())
		}
	}
}
