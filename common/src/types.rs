//! Types used in the dotflow contracts.

use ink::prelude::{string::String, vec::Vec};

#[cfg(feature = "std")]
use ink::storage::traits::StorageLayout;

/// Each identity is associated with a unique identifier called `IdentityNo`.
pub type IdentityNo = u32;

/// We want to keep the address type very generic since we want to support any
/// address format. We won't actually keep the addresses in the contract itself.
/// Before storing them, we'll encrypt them to ensure privacy.
pub type ChainAddress = Vec<u8>;

/// Used to represent any blockchain in the Polkadot, Kusama or Rococo chain.
pub type ChainId = u32;

/// We currently support these two address types since XCM is also supporting
/// only these ones.
#[derive(scale::Encode, scale::Decode, Debug, PartialEq, Clone)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo, StorageLayout))]
pub enum AccountType {
	AccountId32,
	AccountKey20,
}

#[derive(scale::Encode, scale::Decode, Debug, PartialEq, Clone)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo, StorageLayout))]
pub struct ChainInfo {
	/// We need to know the rpc url of each chain otherwise we won't know how
	/// to communicate with it.
	pub rpc_urls: Vec<String>,
	/// We need to know the address type when making XCM transfers.
	pub account_type: AccountType,
}

impl ChainInfo {
	// Makes sure none of the chain's urls exceed the size limit.
	pub fn ensure_rpc_url_size_limit(&self, limit: usize) -> bool {
		self.rpc_urls.iter().all(|url| url.len() <= limit)
	}
}
