# Argon

An inflation-proof stablecoin backed by [Bitcoin](https://bitcoin.org) and the [Ulixee](https://ulixee.org) data
network. The Argon is built as a transactional
currency that can scale to micropayments as low as 1 millionth of a dollar. It is built on the
Polkadot [Substrate](https://substrate.dev) framework.

## Overview

The Argon mainchain is the stabilization layer of the Argon network. Argon stabilizes the value of the Argon by removing
excess currency from supply when the price of argon falls below target (ie, less is desired by the market) and minting
new supply when price is above target (ie, more is in demand than exists).

Minting can be done through two mechanisms. The first is via mining for the Argon, where miners are given permission to
mint new Argons when the price is above target. The second is through a "Liquid Locking" offering for Bitcoins, where
Bitcoins are able to mint liquidity equal to their Bitcoin's market value by committing to locking their Bitcoins and an
equivalent amount of Argons for a year.

The Bitcoin bonds transform into "options" that allow a holder to profit from the volatility of both the Bitcoin price
and the Argon price. To call an option, a Bitcoin holder must issue a "system burn" of argons equivalent to the current
market price of Bitcoin. This creates a dynamically adjusting "burn rate" to forcefully remove unwanted currency from
the system (unwanted, meaning, the market price of the Argon is below target, ergo less desirable).

The Argon's second layer of excess currency removal (ie, stabilization) occurs through the collection of a tax on
all transactions that occur on the L2 of the Argon network, the Localchains. This second layer tax will dynamically
adjust to the price of the Argon, ensuring sufficient supply can be removed quickly from the system.

Localchains are the person-to-person Layer 2 of the Argon network. They are a personal "blockchain" that keeps a
cryptographic proof of the "current balance" of each user, which can be signed and sent to another user for exchanging
funds. Localchains are expected to be used for high-frequency transactions, such as micropayments for queries in
[Ulixee Datastores](https://ulixee.org/docs/datastore). Localchains submit balance changes in "notarizations" to the
Argon mainchain via Notaries,
which are network approved entities that audit the balance changes and batch them to the mainchain.

## Consensus

The Argon mainchain uses a hybrid consensus model. The first form of consensus is
a [Randomx](https://github.com/tevador/RandomX) proof-of-compute
algorithm that is active during "Slot Zero" before miners register for slots. Randomx was developed as part of the
Monero blockchain to create a proof of work that does not favor GPUs, allowing regular desktops to compete. The argon
has no funds minted in the genesis block, so this period can be viewed as the bootstrapping period for the network.

The second form of consensus is a registered mining phase, where miners bid for slots to mine blocks. They must bid by
using bonded argons from a Vault in the system. All mining argons must have an equivalent amount of Bitcoin bonded,
which will bring liquidity to the system. Miners are given priority to register blocks in the system over proof of
compute blocks, so once this phase activates, the network will be secured by the miners and only use proof of compute
when no miners are registered or active. Miners are matched to votes provided by Localchain users using an XOR closest
algorithm (more details to come in the whitepaper).

## Technology Brief

The Argon mainchain is built using the [Substrate](https://substrate.dev) framework. Substrate is a modular blockchain
framework that allows developers to build custom blockchains that operate independently, but can connect into a shared
ecosystem called "Parachains". Substrate is built in Rust and has a WebAssembly runtime that allows for upgrades to the
blockchain without forks. Argon runs as a "solochain", which means our consensus is not shared with other chains. It
will gain integration through a "Transferchain" that allows for the transfer of Argons between chains.

The Localchain is a Rust library that can be run in a standalone cli, within Node.js via bindings created
using [napi-rs](https://napi.rs) or as a library in iOS/Android using [uniffi](https://github.com/mozilla/uniffi-rs). It
uses a Sqlite database to store the balance changes and notarizations.

The Notary is a Rust-based API that uses Json-RPC to communicate with the Localchains and the Argon mainchain. It uses a
PostgresSQL database to keep track of its history and users' balance tips.

## Repository Structure

- `bitcoin`: This module has a cli for bitcoin bonds and vaults, and shared code around the Bitcoin miniscript used for
  the vaulted bitcoins.
- `client`: Mainchain client library in rust and for nodejs.
- `end-to-end`: Tests intending to be run using the end-user tools.
- `localchain`: Localchain is the layer 2 of Argon run by individual wallets. This repo includes bindings for nodejs, a
  standalone cli, and code to run the localchain inside iOS/Android. More about the
  localchain [here](./docs/localchain.md).
- `node`: The node implementation is the entry point for the Argon blockchain. It is responsible for networking,
  consensus, and running the blockchain.
    - `randomx`: Argon has two forms of block consensus. The first is a randomx proof-of-work algorithm. It is eligible
      for creating all blocks during "Slot 0", before miners register for slots.
    - `consensus`: The second form of consensus comes from votes submitted by localchains to the notaries. This module
      has logic for retrieving notebooks from notaries and prioritizing blocks.
    - `bitcoin_utxo_tracker`: Each block created and validated by the Argon blockchain must track all Bitcoin UTXOs that
      back the Argon via bonds. This is done by tracking BlockFilters in bitcoin against the bonded UTXOs.
- `notary`: The notary validates localchain transactions and confirms they are operating on their latest tip. A notary
  runs on a Postgres database. It must submit notebooks rolling up all the localchain balance changes, votes and
  registered domains. Notebooks are submitted for each system "tick".
- `runtime`: The runtime implementation is the core logic of the blockchain. It defines the state transition function
  and the blockchain's logic. The runtime is built as a wasm binary and can be upgraded without a hard fork.
- `oracle`: The code for running the oracles that submit price data and the bitcoin confirmed tip to the blockchain.
- `pallets`: The pallets are the various "tables" that make up the blockchain. They define the storage, dispatchables,
  events, and errors of the blockchain.
    - `bitcoin_utxos`: Tracks the Bitcoin UTXOs that back the Argon.
    - `block_rewards`: Allocates and unlocks block rewards (they are frozen for a period before being allowed to be
      spent).
    - `block_seal`: Verifies the type of block seal used to secure the blockchain matches eligible work.
    - `block_seal_spec`: Tracks and adjust difficulty of compute and vote "power" for block seals.
    - `bond`: Allows users to lock and unlock bitcoin bonds, and tracks the mining bonds (pairs with the vaults pallet,
      and the mining slots).
    - `chain_transfer`: Allows users to transfer Argon between chains. Currently supports Localchain and Mainchain.
    - `domains`: Registers and tracks domains. Domains are used to establish micropayment channel holds with ip routing
      akin to a dns lookup. They're prominently used for [Ulixee Datastores](https://ulixee.org/docs/datastore).
    - `mining_slot`: Allows users to register for mining slots. Mining slots are used to determine who is eligible to
      mine blocks created by the notebook commit reveal scheme.
    - `mint`: Mints Argons to bonded bitcoins and miners when the Argon price is above target
    - `notaries`: Registers the notaries and metadata to connect to them
    - `notebook`: Tracks the recent notebooks submitted by the notaries, as well as the account change roots for each
      notebook. Also tracks the last changed notebook per localchain.
    - `price_index`: Tracks Argon-USD and Bitcoin USD pricing, as well as the current Argon Target price given CPI
      inflation since the starting time.
    - `ticks`: Tracks system-wide "ticks" or minutes since genesis as whole units.
    - `vaults`: Register and manage vaults that offer bonded bitcoins to miners and bitcoin holders.
- `primitives`: Shared models and types for the Argon mainchain, localchain and notaries.

## Runtime Pallets

Argon makes use of a few runtime pallets that are useful to know about as a user of the system.

- `Balances`: This is an instance of
  the [balances](https://paritytech.github.io/polkadot-sdk/master/pallet_balances/index.html) pallet that is used to
  track the
  balances of the Argon
  currency. NOTE: balances are stored in the System Account, not the pallet. This can make some api calls confusing.
- `Ownership`: This is an instance of
  the [balances](https://paritytech.github.io/polkadot-sdk/master/pallet_balances/index.html) pallet that is used to
  track the Ownership Tokens
  of an account. Ownership tokens allow you to bid on mining slots.
- `Sessions`: This is an instance of
  the [session](https://paritytech.github.io/polkadot-sdk/master/pallet_session/index.html)
  pallet that is used to track the session keys of active miners. You must submit session keys to activate your mining
  slot. NOTE: this is usually done on your miner itself, not via rpc to a public rpc host.
- `Grandpa`: Argon uses the [Grandpa](https://github.com/w3f/consensus/blob/master/pdf/grandpa.pdf) Finality gadget to
  finalize blocks. This pallet is used to track the finality authorities (which correspond to the active miners) and
  equivocations (which are penalties for not following grandpa rules).
- `Multisig`: This [pallet](https://paritytech.github.io/polkadot-sdk/master/pallet_multisig/index.html) allows you to
  create a multi-signature account. Substrate's multisig is a little different from other systems, as it requires each
  party to submit their portion of the signature in a separate transaction.
- `Proxy`: This [pallet](https://paritytech.github.io/polkadot-sdk/master/pallet_proxy/index.html) allows you to create
  a proxy account that can submit transactions on behalf of another account. This is useful for creating a cold wallet
  that can still submit transactions. The cold wallet will pay for the transaction fees but doesn't have to have keys
  loaded into memory.

## Using the Testnet

The Argon testnet is a network that is intended to be used for testing and development. You can connect to
the [Polkadot Developer Portal](https://polkadot.js.org/apps/#/explorer?rpc=wss://rpc.testnet.argonprotocol.org) to
interact with the testnet. We are also publishing binary versions of the localchain and bitcoin cli that are useful to
test out connecting to the testnet. Those versions can be found on
the [releases page](https://github.com/argonprotocol/argon/releases/latest).

Useful Urls:

- RPC: `wss://rpc.testnet.argonprotocol.org`
- Testnet Notary: `wss://notary1.testnet.argonprotocol.org`
- Testnet Bootnode: `wss://bootnode0.testnet.argonprotocol.org`
- [Polkadot/Substrate Portal](https://polkadot.js.org/apps/#/explorer?rpc=wss://rpc.testnet.argonprotocol.org)
- [Argon Discord](https://discord.gg/ChyAhFtD)

Here are some tutorials to get you started:

- [How to set up an Account](./docs/account-setup.md)
- [How to use the Argon Localchain CLI](./docs/localchain.md#command-line-interface)
- [How to bond Bitcoins using the Argon Bitcoin CLI](./docs/bitcoin-bond.md)
- [How to create and manage a Vault using the Argon Bitcoin CLI](./docs/running-a-vault.md)
- [How to run a testnet miner](./docs/run-a-miner.md)

## Running Locally

Depending on your operating system and Rust version, there might be additional packages required to compile this
project.
Check the [Substrate Install](https://docs.substrate.io/install/) instructions for your platform for the most common
dependencies.

### Build

Use the following command to build the tools without launching it:

```sh
cargo build --release
```

### Launch a local test network

The following command starts a local test network that will not persist state. NOTE: you need postgres sql installed and
available.

```sh
./scripts/local_testnet.sh
```

### Connect with Polkadot-JS Developer Admin Front-End

After you start the network locally, you can interact with it using the hosted version of
the [Polkadot/Substrate Portal](https://polkadot.js.org/apps/#/explorer?rpc=ws://localhost:9944).

You can also find the source code and instructions for hosting your own instance on
the [polkadot-js/apps](https://github.com/polkadot-js/apps) repository.
