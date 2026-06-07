# Ethereum Contracts

This package holds the Ethereum-side contracts for the replacement Argon and Argonot tokens.

At a high level, the shape is:

- the tokens are plain ERC-20s
- the tokens trust one gateway address forever
- that gateway address is a proxy, so the gateway logic can change without changing the address the
  tokens trust
- the gateway handles the protocol behavior: burns for transfer, first-council initialization, and
  the one-time migration path
- a guardian can pause immediately
- the admin Safe owns gateway actions and upgrades today

## Visual Overview

```text
Users ----------------------> MintingGateway proxy ----------------------> ArgonToken / ArgonotToken
Guardian Safe -------------> pause()
Admin Safe ---------------> proxy upgrade / migrate / unpause
Admin Safe ---------------> ProxyAdmin --------> upgrade gateway implementation

The tokens only trust the proxy address.
The proxy runs MintingGateway implementation code behind that stable address.
```

At the basic level, that is the whole shape:

- users and admins talk to the gateway
- the gateway talks to the tokens
- the tokens trust one gateway address forever
- the gateway code can still change later because that address is a proxy
- emergency pause is separated from the normal admin path

## Contracts

- `ArgonToken.sol`
- `ArgonotToken.sol`
- `MintingGateway.sol`

The two token contracts are intentionally boring. They expose `mint(...)` and `burnFrom(...)`, but
only the gateway can call them. They do not have their own role system, and they do not expose a
public self-burn path.

The gateway is where the actual protocol behavior lives:

- steady-state transfer amounts are expressed in 6-decimal Argon runtime base units
- MintingGateway runtime-unit amount and collateral fields use `uint128` balances
- `startTransferToArgon(...)` is the primary user path: the caller supplies the runtime-unit amount
  Argon should credit plus an ERC-2612 permit signature, and the gateway scales that amount into
  exact token base units before it permits, burns, and emits the outbound proof event
- `initialize(...)` sets the owner, guardian, and first council summary when the proxy is created
- `migrate(...)` is the one-time owner migration path: it loads the exact Argon and Argonot
  migration balances from the prior contracts using base-unit calldata
- `forceUpdateActiveCouncil(...)` is the owner recovery seam: it replaces the stored active council
  summary and active `microgonsPerArgonot` floor without resetting `argonApprovalsNonce` or
  `argonApprovalsHash`, so later queue items can keep chaining from the same last applied update
  under the replacement council
- every gateway activity extends one per-block `activityRoot` commitment in storage, and each new
  block locator links to the previous locator hash, so later proof systems can anchor on `stateRoot`
  while still reconstructing the rich activity payload from emitted logs
- each council rotation carries the next `microgonsPerArgonot` floor value alongside the next
  council snapshot
- transfer-out requests carry one exact `microgonsPerArgonot` quote, and Ethereum only accepts the
  request if that quoted rate is at or below the currently active council floor
- queued council-approved updates chain each signed item to the previous queue item's signed hash,
  so Ethereum only needs to verify council signatures on council-segment tips: each council rotation
  item and the last relayed item in the submitted batch
- `applyGatewayUpdates(...)` rejects batches above `100` items, and
  `finalizeTransferOutOfArgon(...)` rejects proofs above `25` minting-authority authorizations, so
  the activity payloads stay within explicit relay bounds
- `pause()` can be called by the guardian
- `unpause()` stays on the admin Safe owner path

In the app, that approval step can be hidden behind one "move" action. The user does not need to
think about allowance mechanics, but the token still enforces per-user burn authorization even if
the gateway logic changes later.

## What Can Change

There are only three things to keep in your head:

1. The token contracts are fixed. They are plain ERC-20s, and each one is deployed with one gateway
   address baked in.

2. The gateway address is stable. The tokens keep trusting that one proxy address. They do not have
   a `setGateway(...)` path.

3. The gateway code is upgradeable. The proxy points at a `MintingGateway` implementation, and that
   implementation can be replaced later.

The split is:

- guardian Safe: `pause()`
- admin Safe: `migrate(...)`, `unpause()`, and the one-time bootstrap upgrade into the final
  implementation
- admin Safe-owned `ProxyAdmin`: gateway upgrades

That is the main boundary today:

- fast emergency stop through the guardian
- normal admin actions and upgrades through the Safe

The timelock is still the intended later hardening step once the contract is ready to go with a
delayed governance flow.

## Why It Is Shaped This Way

The main decision here is that the token should not have a mutable trust list.

We do not want a token contract that can keep changing which gateway it trusts. That is too much
power in the wrong place, and it makes proofs, upgrades, and review harder to reason about. So the
token takes the gateway address in its constructor and never changes it.

At the same time, we probably will need to change gateway logic over time. That is why the gateway
itself sits behind a proxy. The token keeps trusting the same address, but the logic behind that
address can be upgraded later through governance.

That split is the point of this package:

- tokens stay simple and stable
- gateway owns the moving parts
- upgrades happen behind the gateway address, not by teaching the token to trust someone new

Migration also goes through the gateway for the same reason. It is a one-time setup capability, but
it is still part of the same authority boundary. That keeps the token surface smaller and keeps the
initial migration distribution in the same place as the long-term burn behavior.

## Deployment Shape

The deployment order still matters:

1. Deploy a bootstrap `MintingGateway` implementation with zero canonical-token immutables.
2. Deploy the gateway proxy with the admin Safe as its current `ProxyAdmin` owner and the bootstrap
   `initialize(...)` calldata already encoded into the proxy constructor. Token-bearing gateway
   entrypoints stay blocked in this bootstrap state until the final implementation is installed with
   canonical token addresses configured.
3. Deploy `ArgonToken` and `ArgonotToken` with the proxy address in their constructors.
4. Deploy the final `MintingGateway` implementation with the Argon and Argonot token addresses baked
   in as immutables.
5. Have the admin Safe call `ProxyAdmin.upgradeAndCall(...)` once to move the proxy onto that final
   implementation.
6. Have the admin Safe call `migrate(...)` once with the migrated Argon balances and migrated
   Argonot balances.

The reverse direction does not exist: the token contracts do not have a mutable `setGateway(...)`
path.

The generated bootstrap manifest records:

- the bootstrap gateway implementation address
- the final gateway implementation address
- the gateway proxy address
- the proxy admin address
- the token addresses
- the Safe calldata needed to upgrade the proxy to the final implementation with fixed token
  addresses

See [`../deploy/README.md`](../deploy/README.md) for the deploy workspace,
[`../deploy/mainnet/migration/README.md`](../deploy/mainnet/migration/README.md) for the checked-in
mainnet migration bundle, [`../deploy/DEPLOY_CHECKLIST.md`](../deploy/DEPLOY_CHECKLIST.md) for the
full activation sequence, and [TODO.md](./TODO.md) for the remaining hardening work.

## Commands

Recovery research and balance-derivation tooling still live in the separate
`argonprotocol/hyperbridge-recovery` repository. The final reviewed mainnet migration bundle used by
`bootstrap:deploy` is checked in locally under
[`../deploy/mainnet/migration/`](../deploy/mainnet/migration/).

Contract commands stay here:

```sh
yarn workspace @argonprotocol/ethereum-contracts test
yarn workspace @argonprotocol/ethereum-contracts typecheck
```

Bootstrap deploy commands live in the sibling deploy workspace at
[`../deploy/`](../deploy/README.md):

```sh
yarn workspace @argonprotocol/ethereum-deploy gas:measure
yarn workspace @argonprotocol/ethereum-deploy gas:measure --json > /tmp/ethereum-gas.json
yarn workspace @argonprotocol/ethereum-deploy bootstrap:deploy --argon-rpc-url wss://... --admin-safe 0x... --guardian-safe 0x...
yarn workspace @argonprotocol/ethereum-deploy bootstrap:prepare-runtime-setup --argon-rpc-url wss://... --deployment-manifest chains/ethereum/deploy/mainnet/deployment-manifest.json --force-set-after-nonce 0
```

The system selects the initial global council from Argon vault operators that already have an
effective Ethereum council signer registered on-chain and more than `5,000` accepted treasury bonds.

Mainnet bootstrap generation uses the default public Ethereum RPC unless you override
`ETHEREUM_RPC_URL`:

```sh
ETHEREUM_DEPLOYER_PRIVATE_KEY=0x...
yarn workspace @argonprotocol/ethereum-deploy bootstrap:deploy --argon-rpc-url wss://... --network mainnet --admin-safe 0x... --guardian-safe 0x...
```

Testnet bootstrap generation uses the default public Ethereum Sepolia RPC unless you override
`TESTNET_ETHEREUM_RPC_URL`:

```sh
ETHEREUM_DEPLOYER_PRIVATE_KEY=0x...
yarn workspace @argonprotocol/ethereum-deploy bootstrap:deploy --argon-rpc-url wss://... --network testnet --admin-safe 0x... --guardian-safe 0x...
```

The `testnet` deployment lane points at Sepolia.

`bootstrap:deploy` already performs the full Ethereum-side deployment sequence for this bootstrap
flow:

- deploy bootstrap `MintingGateway` implementation
- deploy proxy and initialize first council state
- deploy `ArgonToken`
- deploy `ArgonotToken`
- deploy final `MintingGateway` implementation
- derive the bootstrap Ethereum council signers from Argon vault signer registrations
- derive the bootstrap council snapshot from the target Argon runtime council inputs
- write `deployment-manifest.json` plus the prepared Safe upgrade transaction

The remaining manual Ethereum follow-ups after that script are:

- have the admin Safe execute each prepared Safe transaction from the manifest in order

For mainnet, that manifest includes both the proxy upgrade and the one-time `migrate(...)` call.

For mainnet, the checked-in bundle at
[`../deploy/mainnet/migration/migrate-bundle.json`](../deploy/mainnet/migration/migrate-bundle.json)
already carries:

- the final Argon and Argonot migration recipient lists and amounts in current `migrate(...)` shape
- the copied recovery migration file hashes
- the upstream final-balance source hashes used to derive that bundle

The Argon-side legacy gateway balance move plus the finalized refund cases are already baked into
the runtime migration at
[`pallets/crosschain_transfer/src/migrations.rs`](../../../pallets/crosschain_transfer/src/migrations.rs).

`bootstrap:prepare-runtime-setup` defaults the execution RPC, beacon API, and
`estimatedMicrogonsPerEth` when it can:

- deployment env `mainnet` -> public Ethereum execution RPC + public Ethereum beacon API
- deployment env `testnet` -> public Ethereum Sepolia execution RPC + public Ethereum Sepolia beacon
  API
- when the deployment manifest includes initial council signers and no pricing override was
  supplied, it auto-runs a fresh `gas:measure --json`
- `estimatedMicrogonsPerEth` -> derived from Argon `priceIndex.current().argonUsdTargetPrice` and
  the public `ETH-USD` spot price
- pass `--measure-report` only when you intentionally want to pin a saved gas snapshot
- the runtime already defaults minimum minting-authority value to `10000` microgons, so pass
  `--minimum-minting-authority-value` only when you intentionally want to override it
- local/dev networks do not use those public defaults; pass `--execution-rpc-url`,
  `--beacon-api-url`, and `--estimated-microgons-per-eth` explicitly

For the normal bootstrap council path, `bootstrap:prepare-runtime-setup` uses the manifest's initial
council member accounts, force-sets that same council on Argon, and verifies that the target runtime
still derives the same signer set, weights, and `microgonsPerArgonot` floor that were used during
`bootstrap:deploy`.

## Current Costs

Refresh these sample costs with:

```sh
yarn workspace @argonprotocol/ethereum-deploy gas:measure
```

The numbers below are local measurements from `../deploy/measure.ts`. The wei and ETH columns are
sample gas-price math only at `10 gwei` and `20 gwei`.

### User Actions

| Action                                          |     Gas |         Wei @ 10 gwei | ETH @ 10 gwei |         Wei @ 20 gwei | ETH @ 20 gwei |
| ----------------------------------------------- | ------: | --------------------: | ------------: | --------------------: | ------------: |
| `startTransferToArgon`                          | 151,608 | 1,516,080,000,000,000 |  0.001516 ETH | 3,032,160,000,000,000 |  0.003032 ETH |
| `finalizeTransferOutOfArgon` (1 authorization)  | 213,061 | 2,130,610,000,000,000 |  0.002131 ETH | 4,261,220,000,000,000 |  0.004261 ETH |
| `finalizeTransferOutOfArgon` (3 authorizations) | 250,035 | 2,500,350,000,000,000 |  0.002500 ETH | 5,000,700,000,000,000 |  0.005001 ETH |
| `cancelTransferOutOfArgon`                      | 130,337 | 1,303,370,000,000,000 |  0.001303 ETH | 2,606,740,000,000,000 |  0.002607 ETH |

### Admin And Council Actions

| Action                                                                              |       Gas |          Wei @ 10 gwei | ETH @ 10 gwei |          Wei @ 20 gwei | ETH @ 20 gwei |
| ----------------------------------------------------------------------------------- | --------: | ---------------------: | ------------: | ---------------------: | ------------: |
| Proxy deploy + `initialize` (4 council members)                                     |   817,423 |  8,174,230,000,000,000 |  0.008174 ETH | 16,348,460,000,000,000 |  0.016348 ETH |
| Proxy deploy + `initialize` (100 council members)                                   |   817,423 |  8,174,230,000,000,000 |  0.008174 ETH | 16,348,460,000,000,000 |  0.016348 ETH |
| Upgrade to final implementation (4 council members)                                 |    37,782 |    377,820,000,000,000 |  0.000378 ETH |    755,640,000,000,000 |  0.000756 ETH |
| Upgrade to final implementation (100 council members)                               |    37,794 |    377,940,000,000,000 |  0.000378 ETH |    755,880,000,000,000 |  0.000756 ETH |
| Minting authority activation (4 members, 3 signatures)                              |   215,941 |  2,159,410,000,000,000 |  0.002159 ETH |  4,318,820,000,000,000 |  0.004319 ETH |
| Minting authority activation (100 members, 90 signatures)                           |   890,351 |  8,903,510,000,000,000 |  0.008904 ETH | 17,807,020,000,000,000 |  0.017807 ETH |
| Minting authority activation batch (100 members, 3 activations, 90 signatures once) |   968,836 |  9,688,360,000,000,000 |  0.009688 ETH | 19,376,720,000,000,000 |  0.019377 ETH |
| Council rotation (4 -> 4 members, 3 signatures)                                     |   200,141 |  2,001,410,000,000,000 |  0.002001 ETH |  4,002,820,000,000,000 |  0.004003 ETH |
| Council rotation (100 -> 100 members, 90 signatures)                                | 1,048,534 | 10,485,340,000,000,000 |  0.010485 ETH | 20,970,680,000,000,000 |  0.020971 ETH |

The batched `100`-member activation row is where the chained queue hash pays off: three activations
land for about `968,836` gas total, or about `322,945` gas per activation, because the council
quorum is only verified once at the segment tip.

`startTransferToArgon(...)` includes the ERC-2612 permit directly. The caller signs for the
runtime-unit amount they want Argon to credit, and the gateway scales that amount into exact token
base units before it permits and burns in one transaction.

## Current Scope

This package covers the bootstrap slice, not the full long-term mint-authority system.

That means:

- the gateway contract shape is in place
- the stable proxy address is in place
- the guardian pause path is in place
- the owner recovery seam for the active council summary is in place
- the current Safe-owned admin / upgrade path is in place
- the one-time migration path is in place

## Deferred Controls

This package does not include the follow-on control layer.

Not included in this package:

- per-authority, per-chain, per-epoch, or rolling daily issuance caps
- delayed Ethereum-side activation for large mint paths that should allow emergency review
- automatic pause triggers on abnormal circulation growth, authority loss, or state drift
- fast authority disable / suspend and replacement-authorization handling for suspicious operators
- the restoration batch generator in the standalone recovery repo
- the temporary migration path
- the restoration forensics work that blocks a mainnet mint run
