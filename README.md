# dotflow-ink
[![Built with ink!](https://raw.githubusercontent.com/paritytech/ink/master/.images/badge.svg)](https://github.com/paritytech/ink)

## Inspiration

Dotflow is a project that has the goal to achieve account abstraction. We want to achieve this by introducing a new entity called **identity**. Each user will be able to create their own identity which can hold all their addresses they have per each different blockchain.

Users will also be able to create their own address book which can contain the identities they most frequently communicate with.

### The benefits of this are the following:
- The user doesn't have to worry whether the recipient of a transaction changed their address over time since it is the identity owner's responsibility to keep their identity addresses up to date.
- No need to keep track of another users addresses over multiple blockchains. Their identity will hold all of the addresses they have on each different blockchain.
- When sending transactions the user doesn't have to specify the actual address of the recepient only needs to select the identity and the destination chain.

### Privacy

We don't want users to expose all of their addresses to everyone. The identity owner needs to be able to decide who gets access to their addresses.

To achieve this we encrypt all of the addresses with different ciphers per each blockchain before storing them in an identity.
To encrypt the addresses we use the AES symmetric key encryption algorithm.

Based on all of the ciphers the `IdentityKey` will be constructed which holds all of them. This will be stored in local storage.

When an identity owner wants to share their address with a user with which he interacts often he will send only the part of the `IdentityKey` that is associated with the blockchains on which he is doing transactions with the other person.
This way the other user will be able to decrypt and access his addresses. 

## Build & Test Locally 
1. Make sure to have the latest [cargo contract](https://crates.io/crates/cargo-contract).
2. Clone the GitHub repository: 
```
git clone https://github.com/TheDotflow/dotflow-ink.git
 ```
 3. Compile and run unit tests
```
cd dotflow-ink/
cargo build
cargo test
```
3. Build the contracts:
```
cd contracts/identity/
cargo contract build --release
```
4. Run e2e tests:
```
# In the root of the project
cargo test --features e2e-tests
```

## Docker
To build the docker image run the following command:
```
docker build -t dotflow-ink .
```
To run the tests:
```
docker run -it dotflow-ink
```

## W3F grant application

https://github.com/w3f/Grants-Program/pull/1657
