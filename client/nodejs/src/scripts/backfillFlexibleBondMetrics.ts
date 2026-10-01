import { ApiPromise, WsProvider } from '@polkadot/api';
import { Metadata } from '@polkadot/types';
import { expandMetadata } from '@polkadot/types/metadata/decorate';
import type { BTreeMap, Compact, Struct, u32, u64, u128 } from '@polkadot/types-codec';
import type { HexString } from '@polkadot/util/types';
import { resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { result as metadataBytes } from '../../metadata.json';
import { getOfflineRegistry } from '../index';

const FIXED_U128_SCALE = 10n ** 18n;
const BOND_LOTS_PER_BATCH = 25;
const FIRST_FULL_FRAME_BY_GENESIS: Record<string, number> = {
  // The old flexible-bond snapshot format first appeared in runtime spec 158.
  '0xee11bf2ff8838fcb0832c09085c5319a08ba6111c225ecd899fe659872d9d45d': 537,
  '0x66dc4e5ff85faddb0311b768eb73a58d9a00c65f06056ffaa370a1b1354d7411': 565,
};

type HistoricalLot = { bondLotId: number; bonds: bigint };

type LotMetrics = {
  participatedFrames: number;
  lastFrameEarningsFrameId: number | null;
  lastFrameEarnings: bigint | null;
  cumulativeEarnings: bigint;
};

type LotAttribution = {
  bondLotId: number;
  expected: LotMetrics;
  updated: LotMetrics;
  frames: { frameId: number; vaultId: number; earnings: bigint }[];
};

type Manifest = {
  kind: 'argon-flexible-bond-metrics-v1';
  genesisHash: string;
  anchorBlockHash: string;
  firstFrameId: number;
  lastFrameId: number;
  frameBlocks: {
    frameId: number;
    startHash: string;
    payoutHash: string;
    vaultShares: {
      vaultId: number;
      grossEarnings: bigint;
      flexibleYield: bigint;
      eligibleFlexibleBonds: bigint;
      totalFlexibleBonds: bigint;
    }[];
  }[];
  lots: LotAttribution[];
  batches: { bondLotIds: number[]; sudoCallData: string }[];
};

/** Split the actual old flexible slice completely, with deterministic dust assignment. */
function allocateFlexibleYield(
  grossVaultEarnings: bigint,
  flexibleProrata: bigint,
  lots: HistoricalLot[],
): Map<number, bigint> {
  if (flexibleProrata > FIXED_U128_SCALE) throw new Error('invalid flexible prorata');
  const totalBonds = lots.reduce((total, lot) => total + lot.bonds, 0n);
  if (totalBonds === 0n) throw new Error('eligible flexible slice has no lots');

  const flexibleYield = (grossVaultEarnings * flexibleProrata) / FIXED_U128_SCALE;
  const shares = lots.map(lot => ({
    bondLotId: lot.bondLotId,
    earnings: (flexibleYield * lot.bonds) / totalBonds,
    remainder: (flexibleYield * lot.bonds) % totalBonds,
  }));

  let dust = flexibleYield - shares.reduce((total, share) => total + share.earnings, 0n);
  shares.sort((a, b) => {
    if (a.remainder !== b.remainder) return a.remainder > b.remainder ? -1 : 1;
    return a.bondLotId - b.bondLotId;
  });

  for (const share of shares) {
    if (dust === 0n) break;
    share.earnings += 1n;
    dust -= 1n;
  }

  return new Map(shares.map(share => [share.bondLotId, share.earnings]));
}

async function prepare(api: ApiPromise): Promise<Manifest> {
  const firstFrameId = FIRST_FULL_FRAME_BY_GENESIS[api.genesisHash.toHex()];
  if (!firstFrameId) throw new Error('only Argon mainnet and testnet are supported');

  const anchorBlockHash = (await api.rpc.chain.getFinalizedHead()).toHex();
  const anchor = await api.at(anchorBlockHash);

  const frameStartHashes = new Map<number, string>();
  let historyHash = anchorBlockHash;
  while (!frameStartHashes.has(firstFrameId)) {
    const history = await api.at(historyHash);
    const nextFrameId = (await history.query.miningSlot.nextFrameId()).toNumber();
    const starts = await history.query.miningSlot.frameStartBlockNumbers();
    if (starts.length === 0) throw new Error('mining frame starts are unavailable');

    for (let index = 0; index < starts.length; index++) {
      const frameId = nextFrameId - index - 1;
      if (frameId < firstFrameId) continue;
      frameStartHashes.set(
        frameId,
        (await api.rpc.chain.getBlockHash(starts[index].toNumber())).toHex(),
      );
    }

    if (frameStartHashes.has(firstFrameId)) break;
    const oldestStart = starts[starts.length - 1].toNumber();
    if (oldestStart === 0) throw new Error(`frame ${firstFrameId} is before recorded history`);

    historyHash = (await api.rpc.chain.getBlockHash(oldestStart - 1)).toHex();
  }

  // A frame start pays the preceding frame. Find the latest start still using the old snapshot.
  let lastFrameId = firstFrameId - 1;
  for (const payoutFrameId of [...frameStartHashes.keys()].sort((a, b) => b - a)) {
    if (payoutFrameId <= firstFrameId) break;
    const payoutStart = await api.at(frameStartHashes.get(payoutFrameId)!);
    const snapshot = await payoutStart.query.treasury.currentFrameVaultCapital();
    if (snapshot.isSome && snapshot.unwrap().has('vaults')) {
      lastFrameId = payoutFrameId - 1;
      break;
    }
  }
  if (lastFrameId < firstFrameId) throw new Error('no completed old-format payout frames found');

  const frameBlocks: Manifest['frameBlocks'] = [];
  const attributed = new Map<number, LotAttribution['frames']>();
  for (let frameId = firstFrameId; frameId <= lastFrameId; frameId++) {
    const startHash = frameStartHashes.get(frameId);
    const payoutHash = frameStartHashes.get(frameId + 1);
    if (!startHash || !payoutHash) throw new Error(`frame ${frameId} has no recorded start`);

    // MiningSlot rotates in on_finalize, so this post-block state contains the locked positions.
    const start = await api.at(startHash);
    const payout = await api.at(payoutHash);
    const snapshotOption = await start.query.treasury.currentFrameVaultCapital();
    if (snapshotOption.isNone) throw new Error(`frame ${frameId} has no locked capital`);
    const snapshot = snapshotOption.unwrap();
    if (snapshot.frameId.toNumber() !== frameId) {
      throw new Error(`frame ${frameId} has no matching locked capital`);
    }

    // The archived snapshot has a `vaults` field that the new runtime type no longer has.
    if (!snapshot.has('vaults')) throw new Error(`frame ${frameId} is not an old capital snapshot`);
    const vaults = snapshot.getT<BTreeMap<u32, Struct>>('vaults');

    const payoutEvents = await payout.query.system.events();
    if (
      !payoutEvents.some(
        ({ event }) =>
          event.section === 'treasury' &&
          event.method === 'FrameEarningsDistributed' &&
          event.data.length === 7 &&
          (event.data[0] as u64).toNumber() === frameId,
      )
    ) {
      throw new Error(`frame ${frameId} was not paid under the old reward model`);
    }

    const vaultShares: Manifest['frameBlocks'][number]['vaultShares'] = [];
    for (const [id, capital] of vaults) {
      const vaultId = id.toNumber();
      const eligibleBonds = capital.getT<Compact<u32>>('flexibleBondsEligible').toBigInt();
      if (eligibleBonds === 0n) continue;

      if (
        payoutEvents.some(
          ({ event }) =>
            event.section === 'vaults' &&
            event.method === 'TreasuryRecordingError' &&
            (event.data[0] as u32).toNumber() === vaultId &&
            (event.data[1] as u64).toNumber() === frameId,
        )
      ) {
        throw new Error(`frame ${frameId} vault ${vaultId} treasury payout failed`);
      }

      const oldVault = await start.query.vaults.vaultsById(vaultId);
      if (oldVault.isNone) throw new Error(`frame ${frameId} vault ${vaultId} is missing`);

      const operator = oldVault.unwrap().operatorAccountId.toString();
      const lotKeys = await start.query.treasury.bondLotIdsByAccount.entries(operator);
      const flexibleLots: HistoricalLot[] = [];
      const lotIds = lotKeys.map(([key]) => key.args[1].toNumber());

      for (let offset = 0; offset < lotIds.length; offset += 100) {
        const batch = lotIds.slice(offset, offset + 100);
        const oldLots = await start.query.treasury.bondLotById.multi(batch);
        for (let index = 0; index < batch.length; index++) {
          const bondLotId = batch[index];
          const oldLot = oldLots[index];
          if (oldLot.isNone) throw new Error(`old lot ${bondLotId} is missing`);
          const lot = oldLot.unwrap();
          if (!lot.isFlexible.isTrue || lot.releaseReason.isSome || !lot.program.isVault) continue;
          if (lot.program.asVault.vaultId.toNumber() !== vaultId) continue;
          flexibleLots.push({ bondLotId, bonds: lot.bonds.toBigInt() });
        }
      }

      const bondState = await start.query.treasury.bondLotsByVault(vaultId);
      const totalFlexibleBonds = flexibleLots.reduce((total, lot) => total + lot.bonds, 0n);
      if (
        totalFlexibleBonds !== bondState.flexibleBonds.toBigInt() ||
        eligibleBonds > totalFlexibleBonds
      ) {
        throw new Error(
          `frame ${frameId} vault ${vaultId} flexible lots do not match locked capital`,
        );
      }

      const revenues = await payout.query.vaults.revenuePerFrameByVault(vaultId);
      const revenue = revenues.find(entry => entry.frameId.toNumber() === frameId);
      if (!revenue) throw new Error(`frame ${frameId} vault ${vaultId} has no recorded payout`);

      const gross = revenue.treasuryTotalEarnings.toBigInt();
      const shares = allocateFlexibleYield(
        gross,
        capital.getT<u128>('flexibleProrata').toBigInt(),
        flexibleLots,
      );
      const flexibleYield = [...shares.values()].reduce((total, share) => total + share, 0n);
      if (flexibleYield > revenue.treasuryVaultEarnings.toBigInt()) {
        throw new Error(
          `frame ${frameId} vault ${vaultId} flexible yield exceeds recorded vault earnings`,
        );
      }

      vaultShares.push({
        vaultId,
        grossEarnings: gross,
        flexibleYield,
        eligibleFlexibleBonds: eligibleBonds,
        totalFlexibleBonds,
      });

      for (const [bondLotId, earnings] of shares) {
        const frames = attributed.get(bondLotId) ?? [];
        frames.push({ frameId, vaultId, earnings });
        attributed.set(bondLotId, frames);
      }
    }

    frameBlocks.push({ frameId, startHash, payoutHash, vaultShares });
  }

  const lots: LotAttribution[] = [];
  const candidates = [...attributed.entries()];

  for (let offset = 0; offset < candidates.length; offset += 100) {
    const batch = candidates.slice(offset, offset + 100);
    const currentLots = await anchor.query.treasury.bondLotById.multi(
      batch.map(([bondLotId]) => bondLotId),
    );
    for (let index = 0; index < batch.length; index++) {
      const currentLot = currentLots[index];
      if (currentLot.isNone) continue;
      const [bondLotId, frames] = batch[index];
      const lot = currentLot.unwrap();
      const expected: LotMetrics = {
        participatedFrames: lot.participatedFrames.toNumber(),
        lastFrameEarningsFrameId: lot.lastFrameEarningsFrameId.isSome
          ? lot.lastFrameEarningsFrameId.unwrap().toNumber()
          : null,
        lastFrameEarnings: lot.lastFrameEarnings.isSome
          ? lot.lastFrameEarnings.unwrap().toBigInt()
          : null,
        cumulativeEarnings: lot.cumulativeEarnings.toBigInt(),
      };
      const latest = frames.at(-1);
      const replaceLast =
        latest &&
        (expected.lastFrameEarningsFrameId === null ||
          latest.frameId > expected.lastFrameEarningsFrameId);
      const updated: LotMetrics = {
        participatedFrames: expected.participatedFrames + frames.length,
        cumulativeEarnings:
          expected.cumulativeEarnings + frames.reduce((total, frame) => total + frame.earnings, 0n),
        lastFrameEarningsFrameId: replaceLast ? latest.frameId : expected.lastFrameEarningsFrameId,
        lastFrameEarnings: replaceLast ? latest.earnings : expected.lastFrameEarnings,
      };
      lots.push({ bondLotId, expected, updated, frames });
    }
  }

  lots.sort((a, b) => a.bondLotId - b.bondLotId);
  // Encode the upcoming runtime's calls without changing the archive query registry.
  const registry = getOfflineRegistry();
  const { tx } = expandMetadata(registry, new Metadata(registry, metadataBytes as HexString));
  const batches: Manifest['batches'] = [];
  for (let offset = 0; offset < lots.length; offset += BOND_LOTS_PER_BATCH) {
    const batch = lots.slice(offset, offset + BOND_LOTS_PER_BATCH);
    const calls = batch.map(({ bondLotId, expected, updated }) =>
      tx.treasury.backfillBondLotEarnings(bondLotId, expected, updated),
    );
    batches.push({
      bondLotIds: batch.map(({ bondLotId }) => bondLotId),
      sudoCallData: tx.sudo.sudo(tx.utility.batchAll(calls)).toHex(),
    });
  }

  return {
    kind: 'argon-flexible-bond-metrics-v1',
    genesisHash: api.genesisHash.toHex(),
    anchorBlockHash,
    firstFrameId,
    lastFrameId,
    frameBlocks,
    lots,
    batches,
  };
}

async function main(): Promise<void> {
  const [endpoint, ...extra] = process.argv.slice(2);
  if (!endpoint || extra.length) {
    throw new Error('usage: backfillFlexibleBondMetrics <ws-endpoint>');
  }

  const api = await ApiPromise.create({ provider: new WsProvider(endpoint), noInitWarn: true });
  try {
    console.log(
      JSON.stringify(
        await prepare(api),
        (_, value: unknown) => (typeof value === 'bigint' ? value.toString() : value),
        2,
      ),
    );
  } finally {
    await api.disconnect();
  }
}

if (process.argv[1] && fileURLToPath(import.meta.url) === resolve(process.argv[1])) {
  main().catch((error: unknown) => {
    console.error(error);
    process.exitCode = 1;
  });
}
