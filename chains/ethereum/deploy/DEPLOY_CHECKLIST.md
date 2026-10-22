# Deploy Checklist

This is the ordered remaining work to deploy and activate the Ethereum gateway against an existing
Argon runtime.

## 1. Prepare Ownership And Bootstrap Council

- [ ] Create the Ethereum admin Safe.
- [ ] Create the Ethereum guardian Safe.
- [ ] Fund the deployer and the Safe execution path on the target EVM network.
- [ ] Make sure each vault that should qualify for initial global council selection pre-registers
      its Ethereum council signer key on Argon.
- [ ] Make sure each vault that should qualify for initial global council selection has more than
      `5,000` accepted treasury bonds.

## 2. Deploy Ethereum Contracts

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

## Purchase activation

- [ ] Upgrade the Argon runtime and relaying clients to recognize purchase activities, retain price
      and amount evidence, and derive the purchase tick from its verified Ethereum block timestamp.
- [ ] Enable gateway activity-root storage-proof syncing before activation; the 4 KiB receipt-node
      bound cannot cover every supported purchase receipt.
- [ ] Verify the chain's native Universal Router 2.1.2, Permit2, v4 PoolManager and canonical USDC
      addresses in the purchase configuration.
- [ ] Prepare a gateway upgrade with `--purchase-config path.json`, or explicitly reuse the
      authorized purchase address with `--swap-contract`.
- [ ] Review the immutable wiring and Safe calldata before executing the upgrade.
