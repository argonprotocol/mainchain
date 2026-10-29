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

## Gateway Activity Proof Flow

Discovery, storage-witness capture, and relay preparation are separate operations. Apps own caching,
scheduling, RPC fallback, and submission:

1. Call `discoverMissingGatewayActivityLocators(...)` with a finalized execution block number plus
   the minimum gateway activity nonce you care about. If you already cache finalized locators, pass
   `afterLocatorIndex` to fetch only the new suffix.
2. Cache those finalized locator blocks in the app as you collect them.
3. If `supportsGatewayActivityStorageProofs(client)` is true, call
   `captureGatewayActivityStorageProofs(...)` against a recent, mined execution block while the RPC
   still has its state. This operation needs no Argon client or retained anchor. It captures the
   selected slots in one `eth_getProof` request and verifies them against that block's state root.
4. Cache the returned enriched locator records in the app. Their witness block may not be finalized
   yet; keep the finalized locator records separately so a witness can be replaced after a reorg.
5. Call `buildGatewayActivityProof(...)` with cached captures for storage runtimes, or plain
   discovered locators for receipt-only runtimes. It prefers storage when supported, selects the
   closest retained anchor, and attaches the bounded ancestry path. Storage relay preparation does
   not refetch witnesses, logs, or contract state. Each payload proves one complete locator.
6. Submit receipt payloads to `proveGatewayActivity(...)` and storage payloads (identified by
   `locatorIndex` in `payload.proof`) to `proveGatewayActivityStorage(...)`. Keep any
   `deferredLocators` for a later attempt. Replace captures listed in `recaptureLocators` using a
   newer execution block: they are orphaned or outside the retained anchor's ancestry bound.

For explicit proof selection, use `buildGatewayActivityReceiptProofPayloads(...)` with discovered
locators or `buildGatewayActivityStorageProofs(...)` with captured locators. Capabilities are
independent; a runtime may accept either format.

```ts
const captures = await captureGatewayActivityStorageProofs({
  executionClient,
  executionBlock: await executionClient.getBlock({ blockTag: 'latest' }),
  gatewayAddress,
  locators,
});
// Persist captures in the app, then prepare relay transactions when anchors become available.
const result = await buildGatewayActivityProof(client, {
  executionClient,
  gatewayAddress,
  locators: captures,
});
```

Storage proofs include the full locator's activity logs and its `activityRootSeed`, even when the
last proven nonce falls inside that locator. Only `payload.activities` and
`payload.gatewayActivityNonceRange` describe the newly applied suffix. No bootstrap call or stored
last-locator hash is needed.

Receipt inclusion alone does not make a log the next gateway activity. Undecodable or unrelated
gateway logs are rejected without pausing synchronization; decoded activities must have the next
contiguous nonce. A valid next activity that conflicts with local business state can pause syncing.
After repairing that state, an operator can unpause and rebuild from the current nonce. Storage
proofs still authenticate the full locator, but do not apply its already-consumed prefix again. If
expiry cleanup failed after the final activity was committed, unpause alone does not retry it: the
next successfully processed activity block retries cleanup at its authenticated block boundary.

Deferred locators are waiting for missing predecessors or an execution anchor. A locator that
exceeds the runtime's proof capacity is a configuration/support error, not deferred work: waiting
cannot make it fit. This flow targets the contract with its 128-activity per-block cap. The pallet's
integrity checks cover that hard cap, maximum-size events, encoded proof bytes, and provider
weights.

The client library does not keep a long-lived locator or witness cache. App-side code should own it,
scoped to the range between the last proven gateway locator and the latest collected finalized
Ethereum block. Keep locators whose blocks are not yet anchored on Argon, and pass the cached
locators into both gateway proof building and execution-header backfill planning.
