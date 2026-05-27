import {
  type AccountId32,
  type BTreeMap,
  MICROGONS_PER_ARGON,
  type ArgonClient,
  type ArgonPrimitivesVault,
  type FrameSupportTokensMiscIdAmountRuntimeHoldReason,
  type PalletPriceIndexPriceIndex,
  type u128,
  type u64,
} from '@argonprotocol/mainchain';
import { getAddress } from 'ethers';

export type VaultOperatorWithEffectiveCouncilSigner = {
  accountId: string;
  signer: string;
  vaultId: number;
};

export async function collectVaultOperatorsByEffectiveCouncilSigner(client: ArgonClient) {
  const activeEntries =
    await client.query.crosschainTransfer.councilSignerByDestinationChainAndAccountId.entries(
      'Ethereum',
    );
  const pendingEntries =
    await client.query.crosschainTransfer.pendingCouncilSignerByDestinationChainAndAccountId.entries(
      'Ethereum',
    );
  const activeByAccountId = new Map<string, string>();
  const pendingByAccountId = new Map<string, string>();

  for (const [key, signer] of activeEntries) {
    if (!signer.isSome) continue;
    activeByAccountId.set(getStorageEntryAccountId(key.args[1]), signer.unwrap().toString());
  }

  for (const [key, signer] of pendingEntries) {
    if (!signer.isSome) continue;
    pendingByAccountId.set(getStorageEntryAccountId(key.args[1]), signer.unwrap().toString());
  }

  const accountsByEffectiveSigner = new Map<string, VaultOperatorWithEffectiveCouncilSigner>();
  for (const accountId of new Set([...activeByAccountId.keys(), ...pendingByAccountId.keys()])) {
    const effectiveSigner = pendingByAccountId.get(accountId) ?? activeByAccountId.get(accountId);
    if (!effectiveSigner) continue;

    const vaultId = await client.query.vaults.vaultIdByOperator(accountId);
    if (vaultId.isNone) continue;

    const signer = getAddress(effectiveSigner);
    const signerKey = signer.toLowerCase();
    const existing = accountsByEffectiveSigner.get(signerKey);
    if (existing && existing.accountId !== accountId) {
      throw new Error(
        `Multiple vault operators have the same effective council signer ${signer}: ${existing.accountId}, ${accountId}`,
      );
    }

    accountsByEffectiveSigner.set(signerKey, {
      accountId,
      signer,
      vaultId: vaultId.unwrap().toNumber(),
    });
  }

  return accountsByEffectiveSigner;
}

export async function deriveRuntimeCouncilSnapshot(
  client: ArgonClient,
  memberAccountIds: string[],
) {
  if (!memberAccountIds.length) {
    throw new Error('Cannot derive a runtime council snapshot without council member accounts');
  }

  const byAccountId = new Map<string, VaultOperatorWithEffectiveCouncilSigner>();
  for (const member of (await collectVaultOperatorsByEffectiveCouncilSigner(client)).values()) {
    byAccountId.set(member.accountId, member);
  }

  const [
    currentTick,
    frameRewardTicksRemaining,
    nextFrameId,
    confirmedBitcoinBlockTip,
    priceIndex,
    historicArgonotFloorByFrame,
    miningConfig,
  ] = await Promise.all([
    client.query.ticks.currentTick(),
    client.query.miningSlot.frameRewardTicksRemaining(),
    client.query.miningSlot.nextFrameId(),
    client.query.bitcoinUtxos.confirmedBitcoinBlockTip(),
    client.query.priceIndex.current(),
    client.query.priceIndex.historicArgonotFloorByFrame(),
    client.query.miningSlot.miningConfig(),
  ]);

  if (confirmedBitcoinBlockTip.isNone) {
    throw new Error(
      'Cannot derive the runtime council snapshot without a confirmed Bitcoin block tip',
    );
  }
  if (priceIndex.isNone) {
    throw new Error(
      'Cannot derive the runtime council snapshot because priceIndex.current is empty',
    );
  }

  const councilRotationFrames = client.consts.crosschainTransfer.councilRotationFrames.toBigInt();
  const ticksPerBitcoinBlock = client.consts.bitcoinLocks.ticksPerBitcoinBlock.toBigInt();
  const operationalMinimumVaultSecuritization =
    client.consts.vaults.operationalMinimumVaultSecuritization.toBigInt();

  if (ticksPerBitcoinBlock === 0n) {
    throw new Error(
      'Cannot derive the runtime council snapshot because ticksPerBitcoinBlock is zero',
    );
  }

  const ticksPerFrame = miningConfig.ticksBetweenSlots.toBigInt();
  const currentTickValue = currentTick.toBigInt();
  const nextFrameTick = currentTickValue + frameRewardTicksRemaining.toBigInt();
  const currentBitcoinHeight = confirmedBitcoinBlockTip.unwrap().blockHeight.toBigInt();
  const epochMicrogonsPerArgonot = getLowestMicrogonsPerArgonot({
    councilRotationFrames,
    currentFrameId: saturatingSub(nextFrameId.toBigInt(), 1n),
    currentPriceIndex: priceIndex.unwrap(),
    historicArgonotFloorByFrame,
  });

  const members = await Promise.all(
    memberAccountIds.map(async accountId => {
      const member = byAccountId.get(accountId);
      if (!member) {
        throw new Error(
          `No vault operator account ${accountId} with a pre-registered effective council signer was found on the target Argon runtime`,
        );
      }

      const vault = await loadRequiredVault(client, member.vaultId);
      const committedArgonots = await loadCommittedArgonots(
        client,
        member.accountId,
        member.vaultId,
      );
      const weight =
        getCommittedSecuritization({
          councilRotationFrames,
          currentBitcoinHeight,
          currentTick: currentTickValue,
          epochMicrogonsPerArgonot,
          operationalMinimumVaultSecuritization,
          ticksPerBitcoinBlock,
          ticksPerFrame,
          vault,
          nextFrameTick,
        }) +
        (committedArgonots * epochMicrogonsPerArgonot) / BigInt(MICROGONS_PER_ARGON);

      if (weight === 0n) {
        throw new Error(
          `Vault operator ${member.accountId} has zero committed council weight, so it cannot be included in the runtime council`,
        );
      }

      return {
        ...member,
        weight,
      };
    }),
  );

  members.sort((left, right) =>
    left.signer.toLowerCase().localeCompare(right.signer.toLowerCase()),
  );

  return {
    epochMicrogonsPerArgonot,
    totalWeight: members.reduce((sum, member) => sum + member.weight, 0n),
    members,
  };
}

function getActivatedSecuritization(vault: ArgonPrimitivesVault) {
  return vault.securitizationLocked.toBigInt() - vault.securitizationPendingActivation.toBigInt();
}

function getCommittedSecuritization(args: {
  councilRotationFrames: bigint;
  currentBitcoinHeight: bigint;
  currentTick: bigint;
  epochMicrogonsPerArgonot: bigint;
  operationalMinimumVaultSecuritization: bigint;
  ticksPerBitcoinBlock: bigint;
  ticksPerFrame: bigint;
  vault: ArgonPrimitivesVault;
  nextFrameTick: bigint;
}) {
  const commitmentHorizonTick =
    args.councilRotationFrames === 0n
      ? args.currentTick
      : args.nextFrameTick + args.ticksPerFrame * saturatingSub(args.councilRotationFrames, 1n);
  const ticksUntilHorizon = saturatingSub(commitmentHorizonTick, args.currentTick);
  const bitcoinReleaseHorizonHeight =
    args.currentBitcoinHeight +
    (ticksUntilHorizon + saturatingSub(args.ticksPerBitcoinBlock, 1n)) / args.ticksPerBitcoinBlock;
  const relockCapacity = [...args.vault.securitizationReleaseSchedule.entries()]
    .filter(([height]) => height.toBigInt() > bitcoinReleaseHorizonHeight)
    .reduce((sum, [, amount]) => sum + amount.toBigInt(), 0n);
  const minimumReducibleSecuritization =
    args.vault.operationalMinimumReleaseTick.isSome &&
    args.vault.operationalMinimumReleaseTick.unwrap().toBigInt() > commitmentHorizonTick
      ? args.operationalMinimumVaultSecuritization
      : 0n;

  return maxBigInt(
    getActivatedSecuritization(args.vault) + relockCapacity,
    minimumReducibleSecuritization,
  );
}

function getLowestMicrogonsPerArgonot(args: {
  councilRotationFrames: bigint;
  currentFrameId: bigint;
  currentPriceIndex: PalletPriceIndexPriceIndex;
  historicArgonotFloorByFrame: BTreeMap<u64, u128>;
}) {
  const currentMicrogonsPerArgonot = getCurrentMicrogonsPerArgonot(args.currentPriceIndex);
  const oldestAllowed = saturatingSub(
    args.currentFrameId,
    saturatingSub(args.councilRotationFrames, 1n),
  );

  let lowest = currentMicrogonsPerArgonot;
  for (const [frameId, floor] of args.historicArgonotFloorByFrame.entries()) {
    if (frameId.toBigInt() < oldestAllowed) continue;
    lowest = minBigInt(floor.toBigInt(), lowest);
  }

  if (lowest === 0n) {
    throw new Error(
      'Cannot derive the runtime council snapshot because microgonsPerArgonot is zero',
    );
  }

  return lowest;
}

function getCurrentMicrogonsPerArgonot(priceIndex: PalletPriceIndexPriceIndex) {
  const argonUsdPrice = priceIndex.argonUsdPrice.toBigInt();
  const argonotUsdPrice = priceIndex.argonotUsdPrice.toBigInt();

  if (argonUsdPrice === 0n || argonotUsdPrice === 0n) {
    throw new Error(
      'Cannot derive the runtime council snapshot because the current Argon or Argonot USD price is zero',
    );
  }

  return (argonotUsdPrice * BigInt(MICROGONS_PER_ARGON)) / argonUsdPrice;
}

async function loadCommittedArgonots(client: ArgonClient, accountId: string, vaultId: number) {
  const commitment = await client.query.vaults.argonotCommitmentByVaultId(vaultId);
  if (commitment.isSome) {
    return commitment.unwrap().committedMicronots.toBigInt();
  }

  const holds = await client.query.ownership.holds(accountId);
  return holds.find(isEnterVaultHold)?.amount.toBigInt() ?? 0n;
}

async function loadRequiredVault(client: ArgonClient, vaultId: number) {
  const vault = await client.query.vaults.vaultsById(vaultId);
  if (vault.isNone) {
    throw new Error(`Vault ${vaultId} was not found on the target Argon runtime`);
  }

  return vault.unwrap();
}

function isEnterVaultHold(hold: FrameSupportTokensMiscIdAmountRuntimeHoldReason) {
  return hold.id.isVaults && hold.id.asVaults.isEnterVault;
}

function getStorageEntryAccountId(accountId: AccountId32) {
  return accountId.toString();
}

function maxBigInt(left: bigint, right: bigint) {
  return left > right ? left : right;
}

function minBigInt(left: bigint, right: bigint) {
  return left < right ? left : right;
}

function saturatingSub(left: bigint, right: bigint) {
  return left > right ? left - right : 0n;
}
