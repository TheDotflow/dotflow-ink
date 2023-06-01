#![cfg_attr(not(feature = "std"), no_std)]

#[ink::contract]
mod identity {
	#[ink(storage)]
	pub struct Identity {}

	impl Identity {
		#[ink(constructor)]
		pub fn new() -> Self {
			Identity {}
		}

		#[ink(message)]
		pub fn foo(&self) -> u32 {
			42
		}
	}

	#[cfg(test)]
	mod tests {
		use super::*;

		/// We test if the constructor does its job.
		#[ink::test]
		fn constructor_works() {
			let identity = Identity::new();
			assert_eq!(identity.foo(), 42);
		}
	}

	#[cfg(all(test, feature = "e2e-tests"))]
	mod e2e_tests {}
}
