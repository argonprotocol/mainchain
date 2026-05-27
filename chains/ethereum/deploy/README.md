# Ethereum Deploy

This workspace owns the operator-side Ethereum bootstrap flow:

- `bootstrap:deploy`
- `bootstrap:prepare-runtime-setup`
- `gas:measure`
- generated deployment manifests under `./<env>/`
- checked-in final migration artifacts under `./mainnet/migration/`
- [`DEPLOY_CHECKLIST.md`](./DEPLOY_CHECKLIST.md)

Use:

```sh
yarn workspace @argonprotocol/ethereum-deploy bootstrap:deploy ...
yarn workspace @argonprotocol/ethereum-deploy bootstrap:prepare-runtime-setup ...
yarn workspace @argonprotocol/ethereum-deploy gas:measure
```

The contract source and tests remain in [`../contracts/`](../contracts/README.md).

Layout:

- `./index.ts`, `./prepareRuntimeSetup.ts`, `./measure.ts`: CLI entrypoints
- `./src/`: shared deploy logic
- `./<env>/deployment-manifest.json`: output from `bootstrap:deploy`
- `./mainnet/migration/`: checked-in final migration bundle and provenance files
