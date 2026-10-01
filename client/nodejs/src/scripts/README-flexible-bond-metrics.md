# Flexible bond earnings metrics backfill

This attributes historical earnings already paid to vaults. It only updates metrics on bond lots
that still exist on chain; it does not transfer Argons or adjust missed collects.

Run this using an archive-capable WS endpoint for mainnet or testnet. Calls are encoded with this
checkout's client metadata, so a read-only dry run can happen before deployment:

```sh
yarn workspace @argonprotocol/mainchain backfill:flexible-bond-metrics \
  wss://ARCHIVE_ENDPOINT > flexible-bond-metrics.json
```

The script uses the latest finalized block as its anchor. It starts with the first complete frame
using the old flexible-bond snapshot (mainnet 537, testnet 565, introduced in runtime spec 158) and
finds the last old-format payout from archived frame starts. The manifest includes the frame hashes,
vault shares, per-lot attribution, expected and updated metrics, and `sudoCallData` for each batch
of up to 25 surviving lots.

Only submit after the matching runtime containing `treasury.backfillBondLotEarnings` is deployed.
Regenerate and review the manifest then, since live earnings may have changed since the dry run. For
each batch, paste `sudoCallData` into the Polkadot.js Apps extrinsic decoder on the matching
network, confirm that it decodes as `sudo.sudo(utility.batchAll([...]))` with the reviewed lot
updates, and submit through the UI with the sudo account. Check the `sudo.Sudid` result:
finalization of the outer sudo extrinsic does not prove that the inner batch succeeded. Each batch
is atomic; if one lot's metrics changed, none of that batch's updates apply. The root call also
treats an already-updated lot as a no-op, so the same batch can be retried. The script does not
accept a signing key or send transactions. Keep the reviewed manifest for retries; do not generate a
new one after partial submission, which could count already-backfilled frames twice.
