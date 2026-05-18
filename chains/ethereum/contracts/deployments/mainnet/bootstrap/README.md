# Mainnet Bootstrap Artifacts

This directory is for the Ethereum-side bootstrap record that lines up the deployed contracts with
the later Argon runtime migration.

Expected generated artifacts:

- `deployment-manifest.json` from
  `yarn workspace @argonprotocol/ethereum-contracts bootstrap:deploy`
- deployment transaction hashes and constructor arguments
- MintingGateway bootstrap implementation address, final implementation address, proxy address, and
  proxy admin address
- Safe calldata to upgrade the MintingGateway proxy to the final implementation with fixed Argon and
  Argonot token addresses
- the Ethereum `chainId`, `MintingGateway` address, and canonical token addresses that the future
  Argon-side runtime seeding step will need

The current bootstrap slice is intentionally narrow:

- `ArgonToken`
- `ArgonotToken`
- `MintingGatewayV2` with the user transfer-start path, first-council initialization, and one-time
  `migrate(...)`

The bootstrap implementation only stores the first council summary. Token-bearing gateway
entrypoints and `migrate(...)` reject until the proxy is upgraded to the final implementation with
canonical token addresses baked in.

This branch does not yet include the runtime-side `pallet_crosschain_transfer` migration, so the
bootstrap manifest here is the handoff artifact for that later step.
