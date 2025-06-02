# Bitcoin Locks

Bitcoin Locks allow you to lock up your Bitcoin in exchange for liquidity on the Argon Network. You retain ownership of
your Bitcoin via a multi-signature script pubkey that you are a co-custodian of. You can redeem your Bitcoin at any time
by "burning" the current market rate of your Bitcoins, but notably capped at the price at "lock time" (your lock price).

## Why do I want this for my Bitcoin?

The Argon Network produces an inflation-proof currency. If you are holding your Bitcoin, locking them into an Argon
Vault allows you to continue holding but unlock the liquid worth of your Bitcoin. Since you can redeem your Bitcoin at
the market rate at time of unlock, you can protect yourself against price decreases in Bitcoin, and since it is capped
at the "lock price", you still benefit from price increases.

In addition to the Bitcoin prices, you can capitalize on variations in the Argon's price. If the Argon price falls, it
will be cheaper in relative terms to release your LockedBitcoin, allowing you to make a profit on the difference between
the market rate in local currency and the argon price. In addition, there is an early mover incentive to buy back first
when the Argon price falls, allowing an additional profit.

## Why does Argon do this?

Argon works by constantly adjusting the currency supply to match demand. In other words, there is a target Argon price
and if the price is below the target, there are too many argons in existence. If the price is above target, not enough
argons exist.

When a Bitcoin is locked into a Vault, the equivalent amount of argons are locked as collateral for a year. This reduces
the argon supply, but it nets out to zero because the Bitcoin will gain rights to mint new argons. Eg, new argons minted
are equal to argon collateral.

However, to release my LockedBitcoin, I am required to _burn_ the release price from existence (the current market rate
capped at the lock price). If many people are wanting to move out of the Argon, the price will fall, and the increased
number of argons representing the market price of my Bitcoin will remove many more argons from circulation. This removes
a proportional amount of argons from the system, while costing the same amount of fiat currency to the Bitcoin holder.
Removing the excess currency allows the price to rise.

## Lock Flow

### Bitcoin Lock Command Line

Argon has a command line interface to simplify the process of creating a Bitcoin Lock. You can find the latest release
on the [releases page](https://github.com/argonprotocol/mainchain/releases/latest).

```bash
$ argon-bitcoin-cli --help
A cli used to lock bitcoins, create and manage Vaults

Usage: argon-bitcoin-cli [OPTIONS] <COMMAND>

Commands:
  vault  List, create and manage vaults
  lock   Create, release and monitor locks
  xpriv  Create, secure, and manage your Bitcoin Master XPriv Key
  utils  Utilities for working with Bitcoin and Argon primitives
  help   Print this message or the help of the given subcommand(s)

Options:
  -t, --trusted-rpc-url <TRUSTED_RPC_URL>  The argon rpc url to connect to [env: TRUSTED_RPC_URL=] [default: ws://127.0.0.1:9944]
  -h, --help                               Print help
  -V, --version                            Print version
```

The `utils` commands are particularly useful for working with Polkadot.js for converting complex numbers.

### 0. Set up your Network

You can set an environment variable to control your network. Pick a trusted RPC url to connect to from the Argon
Network. You can replace the urls in the examples or set the environment variable below.

NOTE: the images in this document show using the Testnet, but the same general scheme applies to the Mainnet
with the appropriate URLs.

_Argon Foundation Mainnet_

```bash
$ export TRUSTED_MAINCHAIN_URL=wss://rpc.argon.network
```

_Testnet_

```bash
$ export TRUSTED_MAINCHAIN_URL=wss://rpc.testnet.argonprotocol.org
```

### 1. Create a new master XPriv key

You'll need to create a new keypair to share custody of your bitcoin. We've created a cli tool to generate a Bitcoin
XPriv, which is a keypair that can create 1 or more subkeys, called "derived" keys.

> This key is the ONLY way you'll be able to release your bitcoin, so ensure you copy the file somewhere secure. You can
> also re-use the same mnemonic you created in your [account setup](./account-setup.md).

```bash
$ argon-bitcoin-cli xpriv master --xpriv-password=supersecret --xpriv-path=~/vault1.xpriv
```

> NOTE: for testnet, you must append `--bitcoin-network=signet`

### 2. Create a Cosign Keypair

Next you'll use the XPriv from step 1 to create your first public key. For each Locked Bitcoin, you'll pick an HD path
that you must record for future use (we recommend using the date, or waiting to get your UtxoId in step 5 and recording
alongside it).

> You can use this XPriv again with a different HD Path for as many combinations as you'd like.

```bash
$ argon-bitcoin-cli xpriv derive-pubkey --xpriv-password=supersecret --xpriv-path=~/vault1.xpriv --hd-path="m/84'/0'/0'"
```

### 3. Choose a Vault

Every Vault sets terms on a fee (base fee + apr per satoshi) and the amount of collateral they're willing to put up (
securitization) to repay a Bitcoin holder in the cause of fraud. The Bitcoin holder chooses a Vault with terms they like
that has enough argons available to support the amount of Bitcoin they wish to lock up.

#### Using the Polkadot.js interface:

You can view the Vaults and their terms using the Polkadot.js interface
![Polkadot Js Vaults](images/pjs-vaults.png)

#### Using the CLI:

You can view the Vaults and their terms using the `vaults` command. The convenient function here is it will
also show you how much the fee will be for the amount of Bitcoin you wish to lock up (and which vaults can support
that).

```bash
$ argon-bitcoin-cli vault list --btc=0.00005

Showing for: 5e-5 btc
Current mint value: ₳2.79 argons
╭────┬──────────────────┬─────────────────┬───────┬────────╮
│ Id ┆ Free argons      ┆ Securitization  ┆ Fee   │ State  │
╞════╪══════════════════╪═════════════════╪═══════╪════════╡
│ 1  ┆ ₳97.09           ┆ 2x              ┆ ₳0.51 │ Active │
╰────┴──────────────────┴─────────────────┴───────┴────────╯
```

#### Fee Calculation

The fee is calculated as `Base Fee` + `Annual Percent Rate` \* `Market rate of Satoshis in Argons`. The parameters can
be
found in the Vault Storage and Price Index Storage (plug-in your `satoshis` and `chosen vault`):

- _Bitcoin USD Price:_ _Storage:_ `PriceIndex -> current -> btcUsdPrice`
- _Argon USD Price:_ _Storage:_ `PriceIndex -> current -> argonUsdPrice`
- _Market Rate of Satoshis in Argons:_ `[satoshis] * bitcoinUsdPrice / 100_000_000 / argonUsdPrice`
- _Base Fee:_ _Storage:_ `[Chosen Vault] -> bitcoinArgons -> baseFee`
- _Annual Percent Rate:_ _Storage:_ `[Chosen Vault] -> bitcoinArgons -> apr`

### 4. Submit a lock request

You need to set-up an account on the [Argon Network](./account-setup.md) to submit a `BitcoinLocks.request` request.
Based on the Vault terms, you will need enough `balance` (eg, _Storage_: `System -> Account -> data -> free`) in your
account to cover the Vault Fee as well as the Fee to submit the transaction to the network.

> Testnet: for use of the testnet, you'll want to obtain funds as [below](#testnet)

You'll need to fill in your Satoshis, the Vault ID and your Pubkey from step 2.

#### Using the Polkadot.js interface:

> Replace your wss:// url below as appropriate for your network

Submit
a [lock initialize](https://polkadot.js.org/apps/?rpc=wss%3A%2F%2Frpc.testnet.argonprotocol.org#/extrinsics/decode/0x080001000000214e000000000000000000000000000000000000000000000000000000000000000000)
request to the mainchain.

NOTE: you'll need to prefix it with 0x to paste into the UI.

#### Using the CLI:

```bash
$ argon-bitcoin-cli lock initialize --btc=0.00005 --vault-id=1 --owner-pubkey=03b1fb97851d1cee35df30e0b5ac5be87c608c71b383b1c3b97a9704335acf83c1
```

This will generate a link to complete the transaction on the Polkadot.js interface along with the fee that needs to be
available in your account.

![Lock Apply](images/pjs-lockbitcoin.png)

### $. Wait for your Utxo Id

Once you've submitted your lock application, you'll need to wait for your lock to be accepted (a green checkmark will
appear on Polkadot.js).

![Lock Approved](images/pjs-lockcheck.png)

Now head to the block Explorer and find your lock in the most recent Block. You can find the `utxoId` in the event:

![UtxoId](images/pjs-utxoid.png)

### 6. Send funds to the Bitcoin UTXO

The information in the lock event will tell you the details needed to recreate the UTXO you need to send your Bitcoin
to. The easiest way to generate the UTXO address is to use the bitcoin CLI command (replace your utxo id):

NOTE: The resulting utxo must have the same amount of Satoshis as you specify in the command. Ensure your fees are set
to add on top of the amount you specify.

```bash
$ argon-bitcoin-cli lock send-to-address --utxo-id=1
```

This will output a Pay to Address that you need to send the EXACT funds into. You can use any tool holding your BTC to
send the funds to the MultiSig address.

> IMPORTANT: The resulting utxo must have the same amount of Satoshis as you requested to lock. Ensure your fees
> are set to add on top of the amount you specify.

### 7. Wait for Argon Verification

Argon will sync your UTXO once it has 6 confirmations. You can use the CLI to check the verification status of your
lock:

```bash
$ argon-bitcoin-cli lock status --utxo-id=1
```

You can also use Polkadot.js to verify your Bitcoin UTXO by looking at the
_Storage_: `BitcoinUtxos -> utxosPendingConfirmation()`.
![Utxo Pending](images/pjs-utxopending.png)

### 8. Monitoring for Minting

You can monitor your Bitcoin Lock status using the `lock status` cli command.

```bash
$ argon-bitcoin-cli lock status --utxo-id=1
```

![Argon Cli Lock-Status](images/cli-lockstatus.png)

You can also use the Polkadot.js interface to look at the MintQueue (_Storage_: `Mint -> pendingMintUtxos`). The result
is a list of UTXOs that are waiting to be minted (`UtxoId`, `AccountId`, `Amount Remaining`). Find the entry with your
UtxoId in as the first parameter.

## Releasing your LockedBitcoin

To release your LockedBitcoin, you'll need to have enough Argons in your account to cover the release fee. You can see
the "redemption price" using the `lock status` command.

> Ensure you set-up your RPC from [step 0](#0-set-up-your-network)

```bash
$ argon-bitcoin-cli lock status --utxo-id=1
```

### 1. Submit a release request

The first step is to submit a release request to the mainchain. Your release request requires the `UtxoId`, an address
you'd like to send the Bitcoin to, and the `Network Fee` you are willing to pay to release the LockedBitcoin.

NOTE: the network fee will normally change from when you submit the request to when it is processed. However, in the
Argon testnet, fees are stable. The Vault will co-sign the transaction so that you can add additional inputs to
cover the fee. You can see recent fees using https://mempool.space. The CLI `fee-rate-sats-per-vb` is in satoshis per
virtual byte (vByte).

```bash
$ argon-bitcoin-cli lock request-release --utxo-id=1 --dest-pubkey=tb1qq0jnaqfkaf298yhx2v02azznk6a6yu8y5deqlv --fee-rate-sats-per-vb=1
```

This will provide a link to complete the transaction on the Polkadot.js interface.

### 2. Wait for the Vault to Cosign the Release

Once you've submitted your release request, you'll need to wait for the Vault to Cosign the release of your
LockedBitcoin.

You can monitor the Polkadot.js interface for the Vault to sign the transaction, or you can pre-sign the transaction and
use the "wait"parameter on the CLI to wait for the transaction to be included in a block.

This step requires you to input the `XPriv` path and HD Path from Bitcoin Lock, [step 2](#2-create-a-cosign-keypair).
You need to supply a trusted bitcoin rpc url in this step.

This must be a trusted JSON-RPC endpoint (not REST or WebSocket). Example providers include:

- [Chainstack](https://chainstack.com)
- [GetBlock](https://getblock.io)
- [QuickNode](https://www.quicknode.com/)
- Your own node running [Bitcoin Core](https://bitcoincore.org/en/download/) with `rpcuser`/`rpcpassword` configured

For testnet, you can use `https://bitcoin:bitcoin@electrs.testnet.argonprotocol.org`

```bash
$ argon-bitcoin-cli lock owner-cosign-release --utxo-id=1 --xpriv-password=supersecret --xpriv-path=~/vault1.xpriv \
  --bitcoin-rpc-url=https://btc.getblock.io/mainnet/?api_key=your_api_key \
  --hd-path="m/84'/0'/0'" --wait
```

### Testnet

#### Wallet connected to Signet

To make transactions more reliable on Bitcoin and faucet tokens available, we have set up a custom Signet. It currently
only has a single node with blocks timed to 10 minutes. You can connect to it using the following parameters:

To launch in Signet:

- [Bitcoin Core](https://bitcoincore.org/en/download/): Modify bitcoin.conf to include `signet=1`
  and `addnode=bitcoin-node0.testnet.argonprotocol.org:38333`
- [Electrum](https://electrum.org/#home): Open with --signet. Eg, in Mac
  OS: `open -a Electrum.app --args --oneserver --signet --server=electrs.testnet.argonprotocol.org:50002:s`
  ***
  ![Electrum signet.png](images/electrum-testnet.png)

#### Get Faucet BTC

The Argon testnet is a place to experiment with the Argon Network. It is connected to a custom Bitcoin Signet. You will
likely need to acquire testnet Argons and Bitcoins to experiment with this feature.

- [Argon Testnet Faucet](./account-setup.md#requesting-testnet-funds)
- The Testnet Faucet will also grant you Signet Bitcoins (good only on the Argon Testnet). To use it, go to
  our [Discord](https://discord.gg/6JxjCNvu6x) `testnet` channel and type `/drip-bitcoin`. Enter your address at the
  prompt:
  ![Discord Faucet.png](images/discord-faucet-bitcoin.png)
