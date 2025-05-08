This is a Node.js client for the Argon Protocol (https://argonprotocol.org). It has the following core features:

1. A typescript generated RPC client for the Argon Protocol.
2. A CLI for interacting with a subset of the Argon Protocol (Mining Bidding, Vaults, Liquidity Pools, Bitcoin)
3. A "Wage Protector" class that can be used to protect wages against inflation or deflation of the Argon.

## Installation

```bash
npm install @argonprotocol/mainchain
```

## Client Usage

To create a client, you can use the `getClient(host)` function in the main module. This will return a client object that
is typed to the argon apis.

## Wage Protector

If you want to protect wages against inflation or deflation of the Argon, there is a `WageProtector` class that can be
used. You can use it a single time:

```javascript
const { WageProtector } = require('@argonprotocol/mainchain');
const basePrice = 1_000_000n; // 1 Argon
const protector = await WageProtector.create(client);
const protectedPrice = protector.getProtectedWage(basePrice);
```

Or you can subscribe to changes (for instance, to track a series of valid cpi adjustments):

```javascript
const { WageProtector } = require('@argonprotocol/mainchain');
const basePrice = 1_000_000n; // 1 Argon
const { unsubscribe } = await WageProtector.subscribe(
  client,
  protectedPrice => {
    console.log(
      `Protected price: ${protectedPrice.getProtectedWage(basePrice)}`,
    );
  },
);
```

Each `WageProtector` instance has the details of the Argon Target Price and USD price at the time of creation.

```typescript
interface IArgonCpiSnapshot {
  // The target price of the argon as a fixed point number (18 decimals)
  argonUsdTargetPrice: bigint;
  // The current price of the argon as a fixed point number (18 decimals)
  argonUsdPrice: bigint;
  // The block hash in which the cpi was finalized
  finalizedBlock: Uint8Array;
  // The tick that the cpi applies to
  tick: bigint;
}
```

## Library Classes

The library has a few classes that are used to interact with the Argon Protocol.

`clis` - this is a collection of cli commands and helpers. Included here are two helpers to read and store encrypted
wallets from Polkadot.js or a wallet like [Talisman](https://talisman.xyz/).

`Accountset.ts` - manage subaccounts from a single keypair

```typescript
import {
  Accountset,
  getClient,
  type KeyringPair,
  mnemonicGenerate,
} from '@argonprotocol/mainchain';
import { keyringFromFile } from '@argonprotocol/mainchain/clis';
import { existsSync, promises as fs } from 'node:fs';

const mnemonicFile = './sessionKeyMnemonic.key';
// generate keys that are used to sign blocks.
if (!existsSync(mnemonicFile)) {
  const sessionKeyMnemonic = mnemonicGenerate();
  await fs.writeFile(sessionKeyMnemonic, 'utf8');
}
const sessionKeyMnemonic = await fs.readFile(mnemonicFile, 'utf8');
const seedAccount = await keyringFromFile({
  filePath: '<path to file>',
  passphrase: 'my password',
});
const mainchainUrl = 'wss://rpc.argon.network';
const client = getClient(mainchainUrl);
this.accountset = new Accountset({
  client,
  seedAccount,
  sessionKeyMnemonic,
  subaccountRange: [0, 1, 2, 3, 4, 5, 6, 7, 8, 9],
});

// register your keys on your miner
await this.accountset.registerKeys(localRpcUrl);
```

`BidPool.ts` - monitor mining bid pools

`BitcoinLocks.ts` - wait for bitcoin space, create bitcoin transactions and look up market pricing

`CohortBidder.ts` - create a bidding bot

```typescript
import { CohortBidder } from '@argonprotocol/mainchain';

// on new bidding
const subaccounts = await accountset.getAvailableMinerAccounts(5);
const cohortBidder = new CohortBidder(accountset, cohortId, subaccounts, {
  maxSeats: 5,
  // a budget not to exceed
  maxBudget: 100_000_000,
  // only spend max 1 argon per seat
  maxBid: 1_000_000,
  // start bidding at 0.5 argons
  minBid: 500_000,
  // increment by 0.01 argons each bid
  bidIncrement: 10_000,
  // wait 10 minutes between bids
  bidDelay: 10,
});
```

`MiningBids.ts` - subscribe to the next bidding cohorts

```typescript
import { MiningBids } from '@argonprotocol/mainchain';

const miningBids = new MiningBids(client);
const { unsubscribe } = await miningBids.onCohortChange({
  async onBiddingStart(cohortId) {
    // ...
  },
  async onBiddingEnd(cohortId) {
    // ...
  },
});
```

`Vault.ts + VaultMonitor.ts` - watch vaults for available funds/opportunities and calculate fees

## Cli

To use this CLI, the easiest way is to define an .env file with the following variables (you can also provide these to
each command).

```env
MAINCHAIN_URL=ws://localhost:9944
ACCOUNT_SURI=# a suri for the account
ACCOUNT_JSON_PATH=# a path to a polkadotjs extracted account json
ACCOUNT_PASSPHRASE=# a passphrase for the pjs json account
SUBACCOUNT_RANGE=# number of subaccounts to manage
KEYS_MNEMONIC=# a mnemonic for the keys to generate
```

### Setup

To perform actions like `bidding on mining seats`, this cli will need to run for a long period of time. The easiest way
to do so is to install it alongside a full node. This gives you a server that:

1. Will stay alive.
2. Can have signing keys installed without exposing apis.
3. Is a trusted source to submit your bid apis through.

You can setup an account as per the files above by running this command to start:

```bash
npx @argonprotocol/mainchain accounts create --path=.env.bob --account-suri=//Bob --register-keys=http://localhost:9944
```

Or with an exported Polkadotjs account (eg, from Talisman, Polkadot Extension, etc):

```bash
npx @argonprotocol/mainchain accounts create --path=.env.bob --account-file-path=./bob.json --account-passphrase=1234 --register-keys=http://localhost:9944
```

This will register your keys with the local node and serve as your `--env` file for the rest of the commands.

### Exporting PolkadotJs Keys

You can export encrypted json accounts from Polkadot.js to use in this tool.

1. Go to the [Polkadot.js Apps](https://polkadot.js.org/apps/#/accounts).
2. Click on the dropdown for the account you want to export.
3. Click on the "Export Account" button.
4. Enter a password to encrypt the JSON file.
5. Move the file to somewhere accessible from where this tool is run (or to mounted docker volumes).
6. Ensure your `.env.<account>` file has a relative path to this JSON file and the password you used to encrypt it.

### Security Concerns (Bid Proxy Account)

You might find the idea of putting an account on your server with the private key to be too risky. You can optionally
create a proxy account for your bidder script using `npx @argonprotocol/mainchain mining create-bid-proxy`. You will
need to run this from a machine with access to your full account keys (either via export or Polkadotjs directly). This
will create a new `env`account file with the proxy details, which you'll use as your env file for the rest of the
commands. The proxy will only be able to make `bid` calls and will not be able to transfer funds or do anything else
with the account. This is a good way to limit the exposure of your main account keys.

NOTE: you will still pay fees from your proxy account, so ensure you keep this account loaded up with fees. This is a
major downside to this approach that we are still seeking to solve.

### Commands

```
Usage: Argon CLI [options] [command]

Options:
  -e, --env <path>         The path to the account .env file to load
  -u, --mainchain-url <url>        The mainchain URL to connect to (default: "wss://rpc.argon.network", env: MAINCHAIN_URL)
  --account-file-path <jsonPath>   The path to your json seed file from polkadotjs (env: ACCOUNT_JSON_PATH)
  --account-suri <secretUri>       A secret uri (suri) to use for the account (env: ACCOUNT_SURI)
  --account-passphrase <password>  The password for your seed file (env: ACCOUNT_PASSPHRASE)
  --account-passphrase-file <path> A password for your seed file contained in a file
  -s, --subaccounts <range>        Restrict this operation to a subset of the subaccounts (eg, 0-10) (env: SUBACCOUNT_RANGE)
  -h, --help                       display help for command

Commands:
  accounts                         Manage subaccounts from a single keypair
  vaults                           Monitor vaults and manage securitization
  mining                           Watch mining seats or setup bidding
  liquidity-pools                  Monitor or bond to liquidity pools
  bitcoin                          Wait for bitcoin space
  help [command]                   display help for command
```

### Example Commands

Create a dynamic bid for a mining seat, maxed at 10 argons per seat. Without a budget, this will use up to 100 argons (
eg, 10 seats).

```bash
npx @argonprotocol/mainchain --env=accounts/.env.bob mining bid --min-bid=1 --max-bid=10 --bid-increment=0.1
```

### Subaccounts

If you want to work with subaccounts during `consolidate` or setup of an account you can use the `-s, --subaccounts`
flag to specify which subaccount to work with. `-s=0-9` is the default. You can also use `-s=0,1,2` to specify a list of
subaccounts. You can also use `-s=0-2,4` to specify a range and a list of subaccounts.
