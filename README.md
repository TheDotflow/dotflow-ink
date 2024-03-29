<h1 align="center">dotflow-ink</h1>

<p align="center">
 <img width="400" src="./images/w3f.jpg" />
</p>
<p align="center">
 <img src="https://raw.githubusercontent.com/paritytech/ink/master/.images/badge.svg" href="https://github.com/paritytech/ink" />
</p>

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

## Deployment

The contracts are currently deployed on [Rococo-contracts](https://use.ink/testnet/) and is used by the frontend when running locally.

Identity contract address: 5E1rtM1uNKNttdYYEXdVnfCdVrg9DGgJG1ERiytAqPNjjGRf

Address book contract address:
5ChUFfCSppd3rXqpYVJs5KAKgi9dz1zyEvWwpFvXLEvYFuVq

### Deployment on Rococo-contracts

To interact or deploy the contracts you will need to have some ROC tokens on the rococo-contracts parachain. Follow the steps on this page to get some tokens: [how to get ROC](https://use.ink/testnet/#2-get-testnet-tokens)

To deploy the contract on rococo-contracts you can follow this [documentation page](https://use.ink/testnet/)

### Deployment on Astar Shibuya

We were initially deploying our project to Shibuya, but we were running into some problems with the Shibuya RPC nodes so for that reason we switched to the rococo-contracts parachain.

To deploy the contract on the testnet you first need to obtain some Astar SBY tokens. 

The easiest way to get SBY tokens is to go to the  [Astar Portal](https://portal.astar.network/) login in with a wallet and select the Shibuya network. After that use the faucet option to get some tokens for the deployment.

After successfully getting some SBY tokens you will need to build the contract to get a WASM executable. The steps to build the contract are explained in the following section.

Finally, to deploy the contract go to [Polkadot.js](https://polkadot.js.org/) and connect to the Shibuya network. Once Shibuya network is selected you will be able to Developer -> Contract section where you can deploy the contract.

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

   **We should have `substrate-contracts-node` installed in the path to run e2e tests.**
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
