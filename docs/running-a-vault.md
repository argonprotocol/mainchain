# Bitcoin Vaults

Vaults are a core part of the Argon network. They lend Argons to both Bitcoin holders and can claim
Mining Bid Pools by funding Mining Bonds. For Bitcoin lock-ers, Vaults commit to lock up the market
value of Bitcoin in Argons for a year. A Vault may choose the Annual Percentage Rate (APR) per Argon
they charge for this service.

## Locked Bitcoins

A LockedBitcoin is a one-year commitment to lock up the market value of Bitcoin in Argons. These
funds are not at risk as long as a Vault performs the following three functions:

1. A Vault acts as a co-signer for a Bitcoin UTXO that is locked in Argon. The Vault must respond to
   requests to co-sign LockedBitcoin Release Requests within 10 days, or they will forfeit funds.
   NOTE: a Vault must perform this function for a full year from the last Bitcoin they allow to be
   locked. In other words, a Vault commits to remaining operational for up to one year past when
   they "_close_" the vault.
2. A Vault must never move a Bitcoin on the Bitcoin network without releasing the lock first on
   Argon (ie, collude with the Bitcoin holder to bypass Argon). If they do so, they will forfeit
   funds.
3. A vault must "collect" revenue to make it a freely spendable balance. A "collect" can only be
   called when there are not pending LockedBitcoin Release Requests. Uncollected revenue is burned
   after 10 days. (NOTE: this also includes any treasury pool revenue).

### Variables for LockedBitcoins

- `Annual Percentage Rate (apr)`: An annual percentage rate charged per argon for the service of
  locking up Bitcoin. The Satoshi value in Argons is calculated by using prices of Bitcoin and Argon
  in USD. The APR multiplied by the argon value to determine the yearly fee. This fee is paid out at
  the end of the term.
- `Base Fee`: A non-refundable flat fee charged for the service of locking up Bitcoin. This fee is
  held for a day before being spendable.
- `Securitization Ratio`: The ratio of offered _Bitcoin Argons_ to securities used to cover the loss
  of Bitcoins in case of fraud.
- `Securitization`: The number of argons a Vault will lock as Securitization for Bitcoins.
  `1 / securitization ratio` worth of these argons can be used to create LockedBitcoins.

## Treasury Pools

Treasury Pools offer LockedBitcoins upfront liquidity in the case where they will not be minted due
to a below-target argon price. A treasury pool for a slot accumulates all mining bid capital on top
of Bonded Argons provided by argon holders in the network. The pool is distributed first to any
pending Bitcoins, and then to each vault based on the size of its contributed treasury pool. The
per-slot treasury pool is capped at one-tenth of a vault's activated securitization.

If a treasury pool is used to fund Bitcoins, it will be the first to be paid out once the argon
price is back at (or above) target.

`Activated securitization` is the amount of securitization currently locked into Bitcoin multiplied
by the securitization ratio. In other words, it's the capital locked into use.

To bond argons to a `Treasury Pool`, any account can submit argons to a given cohort's bid pool
using the `Treasury` pallet. The vault can control how much of the treasury pool earnings are shared
with those contributing capital.

> A vault operator can pre-bond argons to the treasury pool, which gives them first priority to
> capitalize on activated securitization. This is fully optional. The prebond api is
> `liquidyPool.vault_operator_prebond`

### Variables for Treasury Pools

- `Treasury Pool Profit Sharing`: The percent of the profit sharing a vault is offering for treasury
  pool participants.

## The Argon Bitcoin CLI

Argon publishes a CLI that allows Bitcoin Lock-ers and Vault operators to simplify their operations
on the Argon network.

> This will ultimately be replaced by an app interface, so feedback on the flow would be appreciated
> when testing the CLI.

You can find the latest release on the
[releases page](https://github.com/argonprotocol/mainchain/releases/latest).

```bash
$ argon-bitcoin-cli --help
A cli used to lock bitcoins, create and manage Vaults

Usage: argon-bitcoin-cli [OPTIONS] <COMMAND>

Commands:
  vault  List, create and manage vaults
  locks  Create, release and monitor LockedBitcoins
  xpriv  Create, secure, and manage your Bitcoin Master XPriv Key
  utils  Utilities for working with Bitcoin and Argon primitives
  help   Print this message or the help of the given subcommand(s)

Options:
  -t, --trusted-rpc-url <TRUSTED_RPC_URL>  The argon rpc url to connect to [env: TRUSTED_RPC_URL=] [default: ws://127.0.0.1:9944]
  -h, --help                               Print help
  -V, --version                            Print version
```

The `utils` commands are particularly useful for working with Polkadot.js for converting complex
numbers (such as when creating a vault).

NOTE: you must convert an Xpub from the Electrum interface into something the Polkadot.js interface
can understand. You can use the `utils` command to do this.

```bash
$ argon-bitcoin-cli utils encode-xpub tpubD8t2diXwgDwRaNt8NNY6pb19U3SwmUzxFhFtSaKb79cfkPqqWX8vSqPzsW2NkhkMsxye6fuB2wNqs5sGTZPpM63UaAb3e69LvNcFpci6JZt
```

## Choosing a Network

You'll note that all the urls and examples in this doc use the testnet. Please replace
`wss://rpc.testnet.argonprotocol.org` with a trusted mainnet rpc url if you are using the mainnet.
The testnet won't let you submit mainnet bitcoins, so you can't accidentally lose your funds, but
you can lose a lot of time :). NOTE: if you share a key across both environments, you WILL invoke
these actions against the network.

You can also set an environment variable and omit the parameters by setting
`export TRUSTED_RPC_URL=<URL>` to the url you'd like to use. Some example urls:

- Argon Foundation RPC: `wss://rpc.argon.network`.
- Testnet: `wss://rpc.testnet.argonprotocol.org`.

```bash
 export TRUSTED_RPC_URL=wss://rpc.argon.network
```

## Creating a Vault

To create a vault, you can use the CLI to create a new vault with the terms you'd like to offer. The
CLI will generate a url that will allow you to complete the creation process on the Polkadot.js
interface. It will also tell you how many Argons need to be in your account to create the Vault.

### 1. Create a new master XPriv key

You'll need to supply the Argon Mainchain with your Bitcoin XPub key. This is a public key that
allows the Argon network to generate addresses for your Bitcoin UTXOs. You'll want to rotate this
out occasionally as a security measure.

Your first step is to create a master XPriv key. This is a private key that will allow you to derive
1+ XPub master keys that you'll upload to Argon. As long as you derive hardened master XPubs (more
in next step), your key is impossible to uncover through the vault activities.

> Aside: this flow shows creating an XPriv key in the CLI. The flow and apps will be enhanced to use
> Hardware Keys and secure storage like the Secure Enclave on a Mac or iPhone. However, for now you
> will need to manage your XPriv key as a password encrypted file.

```bash
$ argon-bitcoin-cli xpriv master --xpriv-password-interactive --xpriv-path=/tmp/vault1.xpriv
```

> NOTE: on testnet, you must add `--bitcoin-network=signet` to the command above to create a Signet
> XPriv key.

### 2. Create an upload-able XPub key

Now you need to generate an XPub key that will be used in the Argon mainchain to auto-generate two
normal public keys for each Vaulted Bitcoin. One of those public keys will be to cosign
LockedBitcoin "release requests" with the bitcoin owner, and one is to claim the bitcoin if the
owner never releases the LockedBitcoin. These details will be retrievable during the steps you need
them.

You _do_ need to keep track of your XPriv key and password, in addition to the HD Path you use to
generate the master XPub. During a switch of keys, you'll need to pay close attention to _which_
master XPub was used for a request (and internally, which hd path you need to provide to the cli).
Let's generate the XPub key.

> NOTE: you need to use a unique HD path everytime you create a new XPub key, and ideally at the
> same "level" as the previous one, so you don't accidentally overlap keys. The last part of the
> path also needs to be hardened to ensure the key is secure.

```bash
$ argon-bitcoin-cli xpriv derive-xpub --xpriv-path=~/.xpriv/vault1.xpriv --xpriv-password-interactive --hd-path="m/84'/0'/0'"
```

### 3. Create a Vault

Use the xpub that was generated in the previous step to create a new vault. You can set the terms of
the vault to whatever you'd like to offer.

```bash
$ argon-bitcoin-cli vault create --trusted-rpc-url wss://rpc.testnet.argonprotocol.org \
  --argons=₳100 --securitization-ratio=1x \
  --treasury-profit-sharing=50% \
  --bitcoin-apr=0.5% --bitcoin-base-fee=₳0.50 \
  --bitcoin-xpub=tpubD8t2diXwgDwRaNt8NNY6pb19U3SwmUzxFhFtSaKb79cfkPqqWX8vSqPzsW2NkhkMsxye6fuB2wNqs5sGTZPpM63UaAb3e69LvNcFpci6JZt
```

This will output a URL that you can use to complete the transaction on the Polkadot.js interface.
When you load this URL, you can click over to `Submission` to submit the transaction.

```bash
Vault funds needed: ₳300
Link to create transaction:
https://polkadot.js.org/apps/?rpc=wss://rpc.testnet.argonprotocol.org#/extrinsics/decode/0x08000f0080e03779c31182841e0082841e000284d717043587cf015436d724800000008f5e2b22b8e08d61a920bc2006ccd532b6e1304ce07b3a28d86c3595db6fed2303ca6a577de236ac2477e0fc7b6e93ba5df2e4556845952446645114d002c4add213000064a7b3b6e00d
```

![Polkadot.js Vault Submission](images/pjs-vaultcreate.png)

Once your vault is created, you can look at the block it was included in to see what Vault Id you
were assigned.

![Polkadot.js Vault Id](images/pjs-vaultid.png)

### 4. Wait for your Vault to be Active

Your vault will be active in the next Mining Slot (every day at noon EST). NOTE: this rule only
applies once bidding has begun for Slot 1. The field to look at is `opened_tick`. The current tick
is available under `Storage` -> `Ticks` -> `CurrentTick` in the Polkadotjs UI.

## Monitoring Cosign Requests

As a Vault operator, you need to monitor the Argon mainchain for LockedBitcoin `release requests`.
The simplest option is to use the CLI to monitor the mainchain for these requests.

```bash
$ argon-bitcoin-cli vault pending-release --vault-id=1 --trusted-rpc-url wss://rpc.testnet.argonprotocol.org
Pending as of block #15167

NOTE: does not include eligible for reclaim by vault.

╭─────────┬───────────────┬──────────────────────┬────────────────┬──────────────────╮
│ Utxo Id ┆ Obligation Id ┆ Expiration Due Block ┆ Type           ┆ Redemption Price │
╞═════════╪═══════════════╪══════════════════════╪════════════════╪══════════════════╡
│ 1       ┆ 1             ┆ 2903776              ┆ Cosign Request ┆ ₳2.80            │
╰─────────┴───────────────┴──────────────────────┴────────────────┴──────────────────╯
```

## Cosigning Release Requests

When you see a request to release a LockedBitcoin, you need to cosign the request to allow it to be
released. You can use the CLI to cosign the request.

### 1. Provide your cosignature

When you encounter a LockedBitcoin Cosign Request, you will need to know:

1. The `UtxoId` - looked up in the previous step
2. The `XPriv Path` and `Decryption Password` - used to generate the XPub key
3. The `HD Path` used to create the master XPub key that was used by Argon to generate this bitcoin
   cosign script.

```bash
$ argon-bitcoin-cli lock vault-cosign-release --utxo-id=1 \
  --xpriv-path=~/.xpriv/vault.xpriv --xpriv-password-interactive --hd-path="m/84'/0'/0'" \
  --trusted-rpc-url wss://rpc.testnet.argonprotocol.org
```

### 2. Complete the transaction in Polkadot.js

This will output a URL that you can use to complete the transaction on the Polkadot.js interface. It
will provide your half of the co-signature to the LockedBitcoin Cosign request, and you have now
fulfilled all your duties for this LockedBitcoin.

## Reclaiming Bitcoin

If you get to the end of the year and the Bitcoin owner has not requested a LockedBitcoin Release in
Argon, you will lose possession of the vault securitization at the ratio you set. As compensation,
you can claim the Bitcoin.

### 1. Sign the "Claim Bitcoin" Partially Signed Bitcoin Transaction

You'll need to provide a destination script pubkey to send the Bitcoin to. You can also submit the
fee rate (sats/vbyte) you'd like to use for the transaction. You can find current rates here:
https://mempool.space/testnet.

```bash
$ argon-bitcoin-cli lock claim-utxo-psbt --utxo-id=1 \
  --xpriv-path=~/.xpriv/vault.xpriv --xpriv-password-interactive --hd-path="m/84'/0'/0'" \
  --dest_pubkey=tb1q3hkkt02k975ddxzeeeupy9cpysr2cy929ck4qp \
  --fee-rate-sats-per-vb=3 \
  --trusted-rpc-url wss://rpc.testnet.argonprotocol.org \
  --bitcoin-rpc-url=https://btc.getblock.io/mainnet/?api_key=your_api_key
```

This will submit the transaction to the bitcoin rpc node of your choice. For some examples, check
this [list](./bitcoin-lock.md#2-wait-for-the-vault-to-cosign-the-release).

## Vault Rules

The following are a few rules around how and when you can add funding to a vault:

- Vault funding can be added at any time
- Vault funding can be removed at any time down to the level of activated securitization
- Vaults must cosign any BitcoinLock release requests within 10 days of the request, or they will
  forfeit the market value of the Bitcoins
- Vaults must cosign any BitcoinLock which has a mismatch making it unable to be locked into Argon,
  but which make it stuck in bitcoin without further action.
- Vaults must claim revenue every 10 days after emptying all pending LockedBitcoin Release Requests.
  If they do not, the revenue will be burned.
- Vaults must maintain or exceed the promised additional securitization percentage for the duration
  of any LockedBitcoins
- Vault terms will take effect in the next Mining Slot (every day at noon EST). This also applies to
  new vaults. The exception is that prior to bidding for Slot 1, there are no delays.
- Vaults must never move a Bitcoin on the Bitcoin network without releasing the lock first on Argon.
  If they do so, they will forfeit funds.
- Bitcoin fees are a non-refundable fee charged to the LockedBitcoin creator. Any ratchets are
  charged as base fee + a prorated apr for the remainder of the year term (down-ratchets have no apr
  fee)
