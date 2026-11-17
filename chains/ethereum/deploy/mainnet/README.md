# Mainnet deployment records

[`deployment-receipt-2026-11-17.json`](deployment-receipt-2026-11-17.json) records
the deployed gateway implementation, purchase wrapper, stabilization hook,
subscriber, price quoter, and initialized pool. It retains the source revision,
contract addresses, transaction hashes, creation and runtime code hashes, and
verification-input hashes. All five deployment transactions and pool initialization
were confirmed, and all five source-verification reports recorded success.

Deployment does not imply that the gateway proxy was upgraded or the pool funded.
Safe transaction files, signing inputs, operational notes, and generated deployment
plans remain local and ignored by Git.

The older `deployment-manifest.json` and [migration reference](migration/README.md)
are historical records; their bootstrap and migration transactions must not be replayed.
