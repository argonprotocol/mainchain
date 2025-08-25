This is a Node.js client for the Argon Protocol (https://argonprotocol.org). It has the following
core features:

1. A typescript generated RPC client for the Argon Protocol.
2. A "Wage Protector" class that can be used to protect wages against inflation or deflation of the
   Argon.

## Installation

```bash
npm install @argonprotocol/mainchain
```

## Client Usage

To create a client, you can use the `getClient(host)` function in the main module. This will return
a client object that is typed to the argon apis.

## Wage Protector

If you want to protect wages against inflation or deflation of the Argon, there is a `WageProtector`
class that can be used. You can use it a single time:

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
const { unsubscribe } = await WageProtector.subscribe(client, protectedPrice => {
  console.log(`Protected price: ${protectedPrice.getProtectedWage(basePrice)}`);
});
```

Each `WageProtector` instance has the details of the Argon Target Price and USD price at the time of
creation.

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

## Bitcoin Locks

The client also has a `BitcoinLock` class that can be used to create, ratchet and release bitcoin
locks

## Vaults

The client also has a `Vault` class that can be used to create and manage vaults, as well as
calculate bitcoin fees.
