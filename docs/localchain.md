# Localchains & Notaries

The Argon has a built-in aggregator/rollup service called a Notary. Notaries rollup balance changes
from personal "blockchains" called Localchains. Localchains run on a personal computer, mobile
phone, or back a cloud machine. They maintain a cryptographically linked list of balance changes
(note: it's akin to a blockchain, but absent " blocks").

## Choosing a Mainnet

A Localchain needs a mainnet. You'll note that all the urls and example images in this doc use the
testnet. Replace url parameters as needed.

You can also set an environment variable and omit the parameters by setting
`export ARGON_MAINCHAIN_URL=<URL>` to the url you'd like to use. Some example urls:

- Argon Foundation RPC: `wss://rpc.argon.network`.
- Testnet: `wss://rpc.testnet.argonprotocol.org`.

```bash
 export ARGON_MAINCHAIN_URL=wss://rpc.argon.network
```

### Balance Tip Proof

Localchains track balances as a linked history and proof of a current "tip" that has been submitted
to a Notebook that is in the Argon mainchain. The Localchain obtains proof that their tip is in the
merkle root that is published in the notebook either by building it from the balance changes in a
notebook, or requesting it from a provider (some Notaries will activate this option).

### Synchronous Changes / Jump Accounts

Localchain change are single threaded. Once you create a new "tip", you cannot add another change
until your tip is committed to a notebook. For this reason, the Localchain is expected to create any
transactions that will not immediately to commit through "Jump Accounts". These are temporary
accounts that will be destroyed once they have completed their task of sending or requesting funds.

### Tax

Localchains must "pay" tax on transactions of either 20 centagons or 20% if less than 1 argon is
exchanged. This tax is moved to a "tax" account in the user's Localchain. It can be used to vote for
which blocks to follow on the mainchain.

### Channel Holds / Ulixee Datastore Micropayments

ChannelHolds are a special type of Balance Change "note" that are used to set aside funds for high
volume transactions. ChannelHolds can settle in increments of one milligon, but can be divided up
further by micropayment implementers like Ulixee. In the case of Ulixee, per-query payments are
allowed down to on millionth of an Argon. When a ChannelHold closes, it has the right to generate
20% of the funds as tax to be used for voting used to close blocks.

### Notarizations

Localchains can bundle one or more of the following types of transactions into a "notarization",
which is simply a commitment to the shared notebook that a notary is submitting for the current
tick.

Localchains have 3 types of transactions they can submit:

1. `Balance Changes`: These are complete changes that include a signed balance change from a sender
   and a signed balance change from the recipient. All funds must be allocated.
2. `Block Votes`: Votes created through aggregated tax funds. These are used to vote on which block
   to follow in the mainchain.
3. `Domains`: Localchains can register domains that are used to establish micropayment channel
   holds. They're used in the Ulixee Data network to facilitate data query payments.

The current implementation of Localchain uses a Sqlite database to track the state of balance
changes. It must have an external source run a "sync" operation to get the latest state and proofs.

## Command Line Interface

You can interact with the Localchain using the Localchain CLI for your operating system (found on
the latest [releases page](https://github.com/argonprotocol/mainchain/releases/latest)).

```bash
$ argon-localchain --help
The Argon ecosystem is a stablecoin on top of bitcoin.

Usage: argon-localchain [OPTIONS] <COMMAND>

Commands:
  sync          Sync the localchain proofs with the latest notebooks. This will also submit votes and close/claim ChannelHolds as needed
  domains       Explore and manage Domains
  accounts      Manage local accounts
  transactions  Create and receive transactions
  help          Print this message or the help of the given subcommand(s)

Options:
  -b, --base-dir <BASE_DIR>            Where is your localchain? Defaults to a project-specific directory based on OS.
                                          Linux:   /root/.local/share/argon/localchain
                                          Windows: C:\Users\Alice\AppData\Roaming\argon\localchain
                                          macOS:   /Users/Alice/Library/Application Support/argon/localchain [env: ARGON_LOCALCHAIN_BASE_PATH=]
  -n, --name <NAME>                    The localchain name you'd like to use [env: ARGON_LOCALCHAIN_NAME=] [default: primary]
  -m, --mainchain-url <MAINCHAIN_URL>  The mainchain to connect to (this is how a notary url is looked up) [env: ARGON_MAINCHAIN_URL=] [default: ws://127.0.0.1:9944]
  -h, --help                           Print help
  -V, --version                        Print version
```

### Creating a Localchain

You can create one or many Localchains using the CLI. Each Localchain has its own Account/Address
and will correspond to the same one on the Mainchain. To import an account you created on the
Mainchain, you can create a Localchain using the same seed phrase.

You'll note the "suri" parameter here. It is a
[Substrate URI](https://polkadot.js.org/docs/keyring/start/suri/) that can define a mnemonic,
password and even derived accounts using Heuristic Deterministic (HD) paths. In the example below,
we are using the "Alice" account from the Substrate developer examples.

NOTE: You are going to collide with a lot of other testers if you use the same seed phrase as the
examples. Just expect the unexpected ;).

```bash
argon-localchain accounts create --name="alice" \
  --suri="bottom drive obey lake curtain smoke basket hold race lonely fit walk//Alice" \
  --key-password="password"
```

This will output your address and the path to your localchain. You will use the same --name
parameter to interact with this localchain in other commands

#### Transferring Funds from the Mainchain to the Localchain

You can transfer funds from the Mainchain to the Localchain using the CLI. Since your account is the
same on both, you have access to move funds between them. The following example transfers 10 Argons
from your Mainchain Account to the corresponding Localchain account.

```bash
argon-localchain transactions from-mainchain 10.0 --name="alice" --key-password="password"
```

This will initiate a transaction on the Mainchain. You need to wait for the transaction to be
confirmed before you have access to your funds. The CLI requires you to manually sync your
Localchain to check for updates. You can follow along with the mainchain transaction using the
[Polkadot.js explorer](https://polkadot.js.org/apps/#/explorer?rpc=wss://rpc.testnet.argonprotocol.org).

```bash
argon-localchain sync --name="alice"
```

#### Send Funds from Localchain to Localchain

You can send funds from one Localchain to another using the CLI. This is a simple transaction that
simply creates an Argon File you can send to your recipient.

Let's send 5 Argons in "loose cash" format, meaning anyone can claim it. This is useful for sending
funds to someone, but can be somewhat risky if it's intercepted by a malicious actor. If you know
the recipient's account, that's often a preferable way to send funds
(`argon-localchain transactions send 5 <address>`.

Since Localchains are single-threaded, this CLI will do you the favor of transacting through a "Jump
Account". This means you will have a temporary account in your Localchain that will be un-usable
until the transaction is used by a recipient. You won't directly use jump accounts in any case, but
this is what's happening behind the scenes.

```bash
argon-localchain transactions send 5.0 --name="alice"
```

You should see a file path. This file has your signed funds that can be sent to the recipient. You
can send this file over any medium you like, but it's recommended to use a secure channel
(particularly if you haven't restricted the allowed recipients).

#### Receive Funds from a Localchain

You can receive funds from another Localchain using the CLI. They will send you an Argon File that
you can import to your Localchain.

NOTE: you can use the same approach to move funds between multiple Localchains you own.

```bash
argon-localchain transactions receive <path-to-file> --name="alice"
```

#### Moving funds Back to Mainchain

You can move funds back to the Mainchain using the CLI.

```bash
argon-localchain transactions to-mainchain 5.0 --name="alice"
```

## IOS / Android SDK

The Localchain SDK is available for IOS and Android, but only inside this repository (for now). You
can also find a sample IOS wallet in the `localchain/apple` directory. The Android SDK is in the
`localchain/android` directory, but is not yet completed.

This SDK functions as a wrapper around the Localchain API using a set of FFI bindings created by
Mozilla called [Uniffi](https://github.com/mozilla/uniffi-rs).

## Ulixee Desktop

If you are a Ulixee developer, you'll find some Localchain functions built-in to the Ulixee Desktop
"wallet". The desktop can be downloaded from the latest
[releases page](https://github.com/ulixee/platform/releases/latest).
