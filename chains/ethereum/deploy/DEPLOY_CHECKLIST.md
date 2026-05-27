# Deploy Checklist

This is the ordered remaining work to deploy and activate the Ethereum bridge against an existing
Argon runtime.

## 1. Prepare Ownership And Bootstrap Council

- [ ] Create the Ethereum admin Safe.
- [ ] Create the Ethereum guardian Safe.
- [ ] Fund the deployer and the Safe execution path on the target EVM network.
- [ ] Make sure each vault that should qualify for initial global council selection pre-registers
      its Ethereum council signer key on Argon.
- [ ] Make sure each vault that should qualify for initial global council selection has more than
      `5,000` accepted treasury bonds.

## 2. Deploy Ethereum And Run Ethereum Migration

- [ ] Run `bootstrap:deploy`, for example:

```sh
yarn workspace @argonprotocol/ethereum-deploy bootstrap:deploy \
  --argon-rpc-url wss://your-argon-rpc \
  --network mainnet \
  --admin-safe 0x... \
  --guardian-safe 0x...
```

- [ ] Have the admin Safe execute each entry in `safeTransactions[]` from the manifest in order.

## 3. Apply Argon Root Setup

- [ ] Run `bootstrap:prepare-runtime-setup`, for example:

```sh
yarn workspace @argonprotocol/ethereum-deploy bootstrap:prepare-runtime-setup \
  --argon-rpc-url wss://your-argon-rpc \
  --deployment-manifest chains/ethereum/deploy/mainnet/deployment-manifest.json \
  --force-set-after-nonce 0 \
  --output /tmp/ethereum-runtime-setup.json
```

- [ ] Submit the prepared `sudo(batchAll(...))` root transaction from
      `/tmp/ethereum-runtime-setup.json`.
