# Ethereum Deploy

This workspace owns the operator-side Ethereum bootstrap flow:

- `bootstrap:deploy`
- `bootstrap:prepare-runtime-setup`
- `runtime-pricing:prepare`
- `prepare:gateway-upgrade`
- `gas:measure`
- generated deployment manifests under `./<env>/`
- historical migration reference under `./mainnet/migration/README.md`
- [`DEPLOY_CHECKLIST.md`](./DEPLOY_CHECKLIST.md)

Use:

```sh
yarn workspace @argonprotocol/ethereum-deploy bootstrap:deploy ...
yarn workspace @argonprotocol/ethereum-deploy bootstrap:prepare-runtime-setup ...
yarn workspace @argonprotocol/ethereum-deploy runtime-pricing:prepare ...
yarn workspace @argonprotocol/ethereum-deploy prepare:gateway-upgrade --network testnet --swap-contract 0x0000000000000000000000000000000000000000
yarn workspace @argonprotocol/ethereum-deploy etherscan:output-files --deployment-manifest path/to/current-manifest.json
yarn workspace @argonprotocol/ethereum-deploy gas:measure
```

`gas:measure` runs the local Hardhat deployment flow, exercises the current
`finalizeTransferOutOfArgon` request/proof shape, and prints the gas figures that
`bootstrap:prepare-runtime-setup` uses for its pricing recommendation.

The gateway-capacity report defaults to a 60M block gas budget. Use
`gas:measure --json --block-gas-limit 60000000` to evaluate an explicit budget. Modeled throughput
also respects the contract's per-block activity cap. The client capacity test checks that this
on-chain cap fits the bundled runtime's activity and aggregate-data bounds even if every activity
contains the maximum number of collateral rows; gas estimates are not the correctness bound.

The contract source and tests remain in [`../contracts/`](../contracts/README.md).

Layout:

- `./index.ts`, `./prepareRuntimeSetup.ts`, `./prepareMintingGatewayUpgrade.ts`, `./measure.ts`: CLI
  entrypoints
- `./prepareRuntimePricing.ts`: pricing-only runtime update CLI
- `./outputEtherscanFiles.ts`: rewrite the Etherscan standard-json files and constructor args beside
  the explicitly selected current deployment or upgrade manifest
- `./src/`: shared deploy logic
- `./<env>/deployment-manifest.json`: output from `bootstrap:deploy`
- `./mainnet/migration/README.md`: pinned history for the completed migration

## Buy and transfer

`bootstrap:deploy` accepts `--purchase-config path.json`. The file supplies the deployed native
`usdc`, `router` (Universal Router 2.1.2), and `permit2` addresses. The purchase contract reads its
v4 PoolManager from the router during deployment. Each purchase selects one pool using a native v4
PoolKey. The wrapper constructs the native single-pool swap and records that pool's before/after
prices. Deployment does not require an existing pool, StateView or oracle hook. Reward-pool
eligibility belongs on Argon. The script deploys the Argon wrapper and binds the final gateway
implementation to it.

Enable storage-proof syncing before activating purchases. Hooks and contract callers can create
receipts above the 4 KiB receipt-node limit even when the swap and gateway burn succeed. The shared
gateway activity-root proof authenticates these purchases without carrying their full receipts.

For an existing gateway, use `prepare:gateway-upgrade --purchase-config path.json`. To reuse an
existing purchase contract, pass `--swap-contract 0x...` instead. An explicit zero disables
purchases. Both paths prepare Safe upgrade calldata; they do not execute the upgrade.

Rehearse an upgrade against the deployed proxy on a local fork from the repository root:

```sh
yarn workspace @argonprotocol/ethereum-contracts build
GATEWAY_UPGRADE_NETWORK=mainnet \
GATEWAY_UPGRADE_RPC_URL=https://ethereum.publicnode.com \
GATEWAY_UPGRADE_BLOCK=26293554 \
yarn exec vitest run --config vitest.config.ts --project ethereum-contracts \
  chains/ethereum/contracts/test/MintingGatewayUpgrade.test.ts
```

Use `testnet`, its Sepolia RPC and a Sepolia block for the testnet gateway. The test reads the
actual proxy and admin from that fork, upgrades locally, compares existing state, and checks that
the only storage write is the implementation pointer. Without `GATEWAY_UPGRADE_RPC_URL`, the fork
test is skipped. It never submits transactions to the upstream network.

Activate runtime and client recognition of `TransferToArgonFromSwapStarted` through both proof paths
**before** executing the gateway upgrade. Deploy the wrapper against each chain's matching official
router version and addresses; do not substitute the older npm router artifact. The contract package
pins the exact 2.1.2 interfaces. No custom router or Permit2 deployment is part of this flow.

Historical migration balances are no longer loaded by bootstrap. The one-time owner initializer
remains for Apps development balances and rejects a second mint.

Etherscan export uses the exact build-info referenced by each compiled artifact. Select a current
manifest explicitly; export historical deployments from their original Git revision.
