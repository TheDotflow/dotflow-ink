/// Types used in the identity contract.
use crate::{ensure, Error, ADDRESS_SIZE_LIMIT};
use ink::prelude::vec::Vec;
#[cfg(feature = "std")]
use ink::storage::traits::StorageLayout;

/// Each identity will be associated with a unique identifier called `IdentityNo`.
pub type IdentityNo = u32;

/// We want to keep the address type very generic since we want to support any
/// address format. We won't actually keep the addresses in the contract itself.
/// Before storing them, we'll encrypt them to ensure privacy.
pub type Address = Vec<u8>;

/// Used to represent any blockchain in the Polkadot, Kusama or Rococo network.
pub type NetworkId = u32;

#[derive(scale::Encode, scale::Decode, Debug, Default, PartialEq)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo, StorageLayout))]
pub struct IdentityInfo {
	/// Each address is associated with a specific blockchain.
	pub(crate) addresses: Vec<(NetworkId, Address)>,
}

impl IdentityInfo {
	/// Adds an address for the given network
	pub fn add_address(&mut self, network: NetworkId, address: Address) -> Result<(), Error> {
		ensure!(address.len() <= ADDRESS_SIZE_LIMIT, Error::AddressSizeExceeded);

		ensure!(
			!self.addresses.clone().into_iter().any(|address| address.0 == network),
			Error::AddressAlreadyAdded
		);
		self.addresses.push((network, address));

		Ok(())
	}

	/// Updates the address of the given network
	pub fn update_address(
		&mut self,
		network: NetworkId,
		new_address: Address,
	) -> Result<(), Error> {
		ensure!(new_address.len() <= ADDRESS_SIZE_LIMIT, Error::AddressSizeExceeded);

		if let Some(position) =
			self.addresses.clone().into_iter().position(|address| address.0 == network)
		{
			self.addresses[position] = (network, new_address);
			Ok(())
		} else {
			Err(Error::InvalidNetwork)
		}
	}

	/// Remove an address record by network
	pub fn remove_address(&mut self, network: NetworkId) -> Result<(), Error> {
		let old_count = self.addresses.len();
		self.addresses.retain(|(net, _)| *net != network);

		let new_count = self.addresses.len();

		if old_count == new_count {
			Err(Error::InvalidNetwork)
		} else {
			Ok(())
		}
	}
}
