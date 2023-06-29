//! Types used in the dotflow contracts.

use ink::prelude::{string::String, vec::Vec};

#[cfg(feature = "std")]
use ink::storage::traits::StorageLayout;

/// Each identity is associated with a unique identifier called `IdentityNo`.
pub type IdentityNo = u32;

/// We want to keep the address type very generic since we want to support any
/// address format. We won't actually keep the addresses in the contract itself.
/// Before storing them, we'll encrypt them to ensure privacy.
pub type NetworkAddress = Vec<u8>;

/// Used to represent any blockchain in the Polkadot, Kusama or Rococo network.
pub type NetworkId = u32;

/// Used to represent the Ss58 Prefix of a Substrate chain.
pub type Ss58Prefix = u16;

#[derive(scale::Encode, scale::Decode, Debug, Default, PartialEq, Clone)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo, StorageLayout))]
pub struct NetworkInfo {
	/// Each address is associated with a specific blockchain.
	pub name: String,
	/// This is used on the frontend to ensure the user does not add an address
	/// that is not valid on the network he specified.
	pub ss58_prefix: Ss58Prefix,
	/// We need to know the rpc url of each network otherwise we won't know how
	/// to communicate with it.
	pub rpc_url: String,
}
