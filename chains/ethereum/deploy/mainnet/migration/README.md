# Historical Ethereum migration

The completed migration bundles, provenance, and deployment loader are preserved at
[commit 7eb80ba8](https://github.com/argonprotocol/mainchain/tree/7eb80ba8ea114ae91b6eb987e374a5725f88d8ec/chains/ethereum/deploy/mainnet/migration).

Fresh deployments no longer load or submit those historical balances. Existing deployment manifests
remain audit records.

The owner-only, one-time `MintingGateway.migrate` initializer and `migrationCompleted` storage
remain because Apps development setup uses them to create initial ARGN/ARGNOT balances. Proxy
storage layout and repeat-run behavior are unchanged.
