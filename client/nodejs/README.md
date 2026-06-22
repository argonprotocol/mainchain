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

## Gateway Activity Proof Flow

Gateway activity proving uses one shared discovery step and then one of two runtime-dependent
builders:

1. Call `discoverMissingGatewayActivityLocators(...)` with a finalized execution block number plus
   the minimum gateway activity nonce you care about. If you already cache finalized locators, pass
   `afterLocatorIndex` to fetch only the new suffix.
2. Cache those finalized locator blocks in the app as you collect them.
3. Call `buildGatewayActivityProof(...)` with those locators. Receipt-proof runtimes choose the
   receipt path automatically. Storage-proof runtimes also require the finalized execution header
   you want to relay against.
4. If you need runtime-specific control, use `buildGatewayActivityReceiptProofPayloads(...)` for
   receipt-proof runtimes or `buildGatewayActivityStorageProofs(...)` for storage-proof runtimes.
5. Submit each returned payload, and keep any `deferredLocators` for a later attempt.

The client library does not keep a long-lived locator cache. App-side code should own that cache,
scoped to the range between the last proven gateway locator and the latest Argon-finalized execution
anchor, and pass those cached locators into both gateway proof building and execution-header
backfill planning.
