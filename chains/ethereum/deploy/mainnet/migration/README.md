# Mainnet Migration Bundle

This directory holds the checked-in final mainnet Ethereum migration bundle used by
`yarn workspace @argonprotocol/ethereum-deploy bootstrap:deploy`.

Files:

- `migrate-bundle.json`
  - the current `MintingGateway.migrate(...)` input shape used by the deploy manifest
  - copied recovery migration file hashes
  - upstream final-balance source hashes
- `restore-argon-batch-000.json`
- `restore-argon-batch-001.json`
- `restore-argonot-batch-000.json`
- `migration-plan.json`
- `manifestHashes.txt`

The copied batch files stay here for provenance. The deploy flow uses `migrate-bundle.json`.
