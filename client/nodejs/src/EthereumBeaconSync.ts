import camelcaseKeys from 'camelcase-keys';
import type { SubmittableExtrinsic } from '@polkadot/api/promise/types';
import type {
  EthereumBeaconConfigSpecResponse,
  EthereumBeaconGenesisResponse,
  EthereumBeaconHeaderDetailsResponse,
  EthereumLightClientBootstrapResponse,
  EthereumLightClientUpdate,
  EthereumLightClientUpdatesResponse,
} from './EthereumBeaconTypes';
import { buildExecutionHeaderProof, getBeaconJson } from './EthereumBeaconApi';
import type { ArgonClient } from './index';
import { type Hex, hexToBytes } from 'viem';

type BeaconPreset = 'mainnet' | 'minimal';
type BeaconNetworkParams = {
  epochsPerSyncCommitteePeriod: bigint;
  preset: BeaconPreset;
  slotsPerEpoch: bigint;
};
type EthereumBeaconSyncState =
  | {
      isBootstrapped: false;
      hasNextSyncCommittee: boolean;
      latestFinalizedBlockRoot: string;
      latestSyncCommitteeUpdatePeriod: bigint;
      headerInterval: bigint;
    }
  | {
      isBootstrapped: true;
      hasNextSyncCommittee: boolean;
      latestFinalizedBlockRoot: string;
      latestFinalizedSlot: bigint;
      nextRecommendedFinalizedSlot: bigint;
      latestSyncCommitteeUpdatePeriod: bigint;
      headerInterval: bigint;
    };

const lightClientUpdateRetryIntervalMs = 500;
const lightClientUpdateRetryTimeoutMs = 30_000;
const expectedBeaconPresetPromises = new WeakMap<ArgonClient, Promise<BeaconPreset | undefined>>();
const beaconNetworkParamsPromises = new Map<string, Promise<BeaconNetworkParams>>();

export async function getEthereumBeaconSyncBootstrapTx(
  client: ArgonClient,
  beaconApiUrl: string,
): Promise<SubmittableExtrinsic> {
  const expectedPreset = await getExpectedBeaconPreset(client);
  const [genesis, spec] = await Promise.all([
    getBeaconJson<EthereumBeaconGenesisResponse>(beaconApiUrl, '/eth/v1/beacon/genesis'),
    getBeaconJson<EthereumBeaconConfigSpecResponse>(beaconApiUrl, '/eth/v1/config/spec'),
  ]);
  const specData = spec.data;
  parseBeaconNetworkParams(specData, expectedPreset);
  const startedAt = Date.now();
  let lastError: Error | undefined;
  let bootstrap: EthereumLightClientBootstrapResponse | undefined;

  while (Date.now() - startedAt < lightClientUpdateRetryTimeoutMs) {
    try {
      const finalizedHeader = await getBeaconJson<EthereumBeaconHeaderDetailsResponse>(
        beaconApiUrl,
        '/eth/v1/beacon/headers/finalized',
      );
      bootstrap = await getBeaconJson<EthereumLightClientBootstrapResponse>(
        beaconApiUrl,
        `/eth/v1/beacon/light_client/bootstrap/${finalizedHeader.data.root}`,
      );
      lastError = undefined;
      break;
    } catch (error) {
      if (!(error instanceof Error)) {
        throw error;
      }
      lastError = error;
      await new Promise(resolve => setTimeout(resolve, lightClientUpdateRetryIntervalMs));
    }
  }

  if (!bootstrap) {
    throw (
      lastError ??
      new Error('Unable to fetch ethereum beacon bootstrap before the retry timeout elapsed')
    );
  }

  return client.tx.ethereumVerifier.forceCheckpoint(
    {
      header: camelcaseKeys(bootstrap.data.header.beacon),
      currentSyncCommittee: camelcaseKeys(bootstrap.data.current_sync_committee),
      currentSyncCommitteeBranch: bootstrap.data.current_sync_committee_branch,
      validatorsRoot: genesis.data.genesis_validators_root,
      executionHeaderProof: buildExecutionHeaderProof(bootstrap.data.header),
    },
    {
      genesis: {
        version: genesis.data.genesis_fork_version,
        epoch: 0n,
      },
      altair: buildFork(specData, 'ALTAIR'),
      bellatrix: buildFork(specData, 'BELLATRIX'),
      capella: buildFork(specData, 'CAPELLA'),
      deneb: buildFork(specData, 'DENEB'),
      electra: buildFork(specData, 'ELECTRA'),
      fulu: buildFork(specData, 'FULU'),
    },
  );
}

export async function getNextEthereumBeaconSyncTxs(
  client: ArgonClient,
  beaconApiUrl: string,
): Promise<SubmittableExtrinsic[]> {
  const state = await getEthereumBeaconSyncState(client);

  if (!state.isBootstrapped) {
    return [];
  }

  const expectedPreset = await getExpectedBeaconPreset(client);
  const beaconNetworkParams = await getBeaconNetworkParams(beaconApiUrl, expectedPreset);
  const storedPeriod = computePeriod(state.latestFinalizedSlot, beaconNetworkParams);
  const txs: SubmittableExtrinsic[] = [];
  const startedAt = Date.now();

  while (Date.now() - startedAt < lightClientUpdateRetryTimeoutMs) {
    let finalityUpdate: EthereumLightClientUpdate;

    try {
      finalityUpdate = (
        await getBeaconJson<{ data: EthereumLightClientUpdate }>(
          beaconApiUrl,
          '/eth/v1/beacon/light_client/finality_update',
        )
      ).data;
    } catch (error) {
      if (!isRetryableBeaconLightClientError(error)) {
        throw error;
      }

      await new Promise(resolve => setTimeout(resolve, lightClientUpdateRetryIntervalMs));
      continue;
    }

    const missingNextSyncCommittee = !state.hasNextSyncCommittee;
    const finalityFinalizedPeriod = computePeriod(
      BigInt(finalityUpdate.finalized_header.beacon.slot),
      beaconNetworkParams,
    );
    const requiredCommitteePeriod = getRequiredCommitteePeriod({
      storedPeriod,
      missingNextSyncCommittee,
      latestSyncCommitteeUpdatePeriod: state.latestSyncCommitteeUpdatePeriod,
      finalityFinalizedPeriod,
    });
    const needsCommitteeData = requiredCommitteePeriod !== undefined;
    const finalityCarriesRequiredCommittee = updateCarriesCommitteeForPeriod(
      finalityUpdate,
      requiredCommitteePeriod,
      beaconNetworkParams,
    );
    const needsCurrentPeriodUpdate =
      requiredCommitteePeriod !== undefined &&
      (missingNextSyncCommittee || !finalityCarriesRequiredCommittee);
    let submitUpdate: EthereumLightClientUpdate | undefined = finalityUpdate;

    if (needsCommitteeData && needsCurrentPeriodUpdate) {
      let periodUpdates: EthereumLightClientUpdatesResponse;

      try {
        periodUpdates = await getBeaconJson<EthereumLightClientUpdatesResponse>(
          beaconApiUrl,
          `/eth/v1/beacon/light_client/updates?count=1&start_period=${requiredCommitteePeriod}`,
        );
      } catch (error) {
        if (!isRetryableBeaconLightClientError(error)) {
          throw error;
        }

        await new Promise(resolve => setTimeout(resolve, lightClientUpdateRetryIntervalMs));
        continue;
      }

      const currentPeriodUpdate = periodUpdates[0]?.data;
      if (
        updateCarriesCommitteeForPeriod(
          currentPeriodUpdate,
          requiredCommitteePeriod,
          beaconNetworkParams,
        )
      ) {
        submitUpdate = currentPeriodUpdate;
      } else {
        submitUpdate = undefined;
      }
    }

    if (needsCommitteeData && needsCurrentPeriodUpdate && !submitUpdate) {
      return [];
    }

    // make sure the update we choose has a supermajority of sync committee participants
    if (
      submitUpdate &&
      !hasSyncCommitteeSupermajority(submitUpdate.sync_aggregate.sync_committee_bits)
    ) {
      await new Promise(resolve => setTimeout(resolve, lightClientUpdateRetryIntervalMs));
      continue;
    }

    if (submitUpdate) {
      const finalizedSlot = BigInt(submitUpdate.finalized_header.beacon.slot);
      const attestedSlot = BigInt(submitUpdate.attested_header.beacon.slot);

      const finalizedPeriod = computePeriod(finalizedSlot, beaconNetworkParams);
      const attestedPeriod = computePeriod(attestedSlot, beaconNetworkParams);

      const isNewerThanStoredFinalized = finalizedSlot > state.latestFinalizedSlot;
      const hasReachedNextFinalizedWindow = finalizedSlot >= state.nextRecommendedFinalizedSlot;
      const canAdvanceFinalized = isNewerThanStoredFinalized && hasReachedNextFinalizedWindow;
      const advancesSyncCommitteePeriod = finalizedPeriod > state.latestSyncCommitteeUpdatePeriod;
      const canUseNextSyncCommittee = missingNextSyncCommittee || advancesSyncCommitteePeriod;
      const includesNextSyncCommittee =
        hasNextSyncCommitteeUpdate(submitUpdate) && canUseNextSyncCommittee;
      const fillsMissingNextSyncCommittee =
        missingNextSyncCommittee && includesNextSyncCommittee && attestedPeriod === storedPeriod;
      const advancesKnownSyncCommitteePeriod =
        !missingNextSyncCommittee && includesNextSyncCommittee;

      if (
        canAdvanceFinalized ||
        fillsMissingNextSyncCommittee ||
        advancesKnownSyncCommitteePeriod
      ) {
        const nextSyncCommitteeUpdate = includesNextSyncCommittee
          ? {
              nextSyncCommitteeUpdate: {
                nextSyncCommittee: camelcaseKeys(submitUpdate.next_sync_committee!),
                nextSyncCommitteeBranch: submitUpdate.next_sync_committee_branch!,
              },
            }
          : {};
        txs.push(
          client.tx.ethereumVerifier.submit({
            attestedHeader: camelcaseKeys(submitUpdate.attested_header.beacon),
            syncAggregate: camelcaseKeys(submitUpdate.sync_aggregate),
            signatureSlot: submitUpdate.signature_slot,
            ...nextSyncCommitteeUpdate,
            finalizedHeader: camelcaseKeys(submitUpdate.finalized_header.beacon),
            finalityBranch: submitUpdate.finality_branch,
            executionHeaderProof: buildExecutionHeaderProof(submitUpdate.finalized_header),
          }),
        );
      }
    }

    break;
  }

  return txs;
}

export async function getEthereumBeaconSyncState(
  client: ArgonClient,
): Promise<EthereumBeaconSyncState> {
  const latestFinalizedBlockRoot = (await client.query.ethereumVerifier.latestFinalizedBlockRoot())
    .toHex()
    .toLowerCase();
  const finalizedState =
    await client.query.ethereumVerifier.finalizedBeaconState(latestFinalizedBlockRoot);
  const latestSyncCommitteeUpdatePeriod = (
    await client.query.ethereumVerifier.latestSyncCommitteeUpdatePeriod()
  ).toBigInt();
  const headerInterval = client.consts.ethereumVerifier.freeHeadersInterval.toBigInt();
  const hasNextSyncCommittee = !(
    await client.query.ethereumVerifier.nextSyncCommittee.size()
  ).isZero();

  if (finalizedState.isNone) {
    return {
      isBootstrapped: false,
      hasNextSyncCommittee,
      latestFinalizedBlockRoot,
      latestSyncCommitteeUpdatePeriod,
      headerInterval,
    };
  }

  const latestFinalizedSlot = finalizedState.unwrap().slot.toBigInt();

  return {
    isBootstrapped: true,
    hasNextSyncCommittee,
    latestFinalizedBlockRoot,
    latestFinalizedSlot,
    nextRecommendedFinalizedSlot: latestFinalizedSlot + headerInterval,
    latestSyncCommitteeUpdatePeriod,
    headerInterval,
  };
}

function buildFork(spec: Record<string, string>, name: string) {
  const version = spec[`${name}_FORK_VERSION`];
  const epoch = spec[`${name}_FORK_EPOCH`];

  if (!version) throw new Error(`Missing beacon spec value for ${name}_FORK_VERSION`);
  if (!epoch) throw new Error(`Missing beacon spec value for ${name}_FORK_EPOCH`);

  return {
    version,
    epoch: BigInt(epoch),
  };
}

function hasNextSyncCommitteeUpdate(
  update?: Pick<EthereumLightClientUpdate, 'next_sync_committee' | 'next_sync_committee_branch'>,
): boolean {
  return !!update?.next_sync_committee && !!update?.next_sync_committee_branch;
}

function getRequiredCommitteePeriod({
  storedPeriod,
  missingNextSyncCommittee,
  latestSyncCommitteeUpdatePeriod,
  finalityFinalizedPeriod,
}: {
  storedPeriod: bigint;
  missingNextSyncCommittee: boolean;
  latestSyncCommitteeUpdatePeriod: bigint;
  finalityFinalizedPeriod: bigint;
}): bigint | undefined {
  if (missingNextSyncCommittee) {
    return storedPeriod;
  }

  if (finalityFinalizedPeriod > latestSyncCommitteeUpdatePeriod) {
    return latestSyncCommitteeUpdatePeriod + 1n;
  }

  return undefined;
}

function updateCarriesCommitteeForPeriod(
  update: EthereumLightClientUpdate | undefined,
  requiredCommitteePeriod: bigint | undefined,
  beaconNetworkParams: BeaconNetworkParams,
): boolean {
  if (!update || requiredCommitteePeriod === undefined || !hasNextSyncCommitteeUpdate(update)) {
    return false;
  }

  const attestedPeriod = computePeriod(
    BigInt(update.attested_header.beacon.slot),
    beaconNetworkParams,
  );
  const finalizedPeriod = computePeriod(
    BigInt(update.finalized_header.beacon.slot),
    beaconNetworkParams,
  );

  return attestedPeriod === requiredCommitteePeriod && finalizedPeriod === requiredCommitteePeriod;
}

function detectBeaconPreset(spec: Record<string, string>): BeaconPreset {
  return spec.SLOTS_PER_HISTORICAL_ROOT === '64' ? 'minimal' : 'mainnet';
}

async function getBeaconNetworkParams(
  beaconApiUrl: string,
  expectedPreset?: BeaconPreset,
): Promise<BeaconNetworkParams> {
  let paramsPromise = beaconNetworkParamsPromises.get(beaconApiUrl);

  if (!paramsPromise) {
    paramsPromise = getBeaconJson<EthereumBeaconConfigSpecResponse>(
      beaconApiUrl,
      '/eth/v1/config/spec',
    ).then(({ data }) => parseBeaconNetworkParams(data));
    beaconNetworkParamsPromises.set(beaconApiUrl, paramsPromise);
  }

  const beaconNetworkParams = await paramsPromise;

  if (expectedPreset && beaconNetworkParams.preset !== expectedPreset) {
    throw new Error(
      `Beacon preset mismatch: chain expects ${expectedPreset}, but endpoint reports ${beaconNetworkParams.preset}`,
    );
  }

  return beaconNetworkParams;
}

function parseBeaconNetworkParams(
  spec: Record<string, string>,
  expectedPreset?: BeaconPreset,
): BeaconNetworkParams {
  const slotsPerEpoch = spec.SLOTS_PER_EPOCH;
  const epochsPerSyncCommitteePeriod = spec.EPOCHS_PER_SYNC_COMMITTEE_PERIOD;

  if (!slotsPerEpoch) {
    throw new Error('Missing beacon spec value for SLOTS_PER_EPOCH');
  }
  if (!epochsPerSyncCommitteePeriod) {
    throw new Error('Missing beacon spec value for EPOCHS_PER_SYNC_COMMITTEE_PERIOD');
  }

  const preset = detectBeaconPreset(spec);

  if (expectedPreset && preset !== expectedPreset) {
    throw new Error(
      `Beacon preset mismatch: chain expects ${expectedPreset}, but endpoint reports ${preset}`,
    );
  }

  return {
    preset,
    slotsPerEpoch: BigInt(slotsPerEpoch),
    epochsPerSyncCommitteePeriod: BigInt(epochsPerSyncCommitteePeriod),
  };
}

async function getExpectedBeaconPreset(client: ArgonClient): Promise<BeaconPreset | undefined> {
  let presetPromise = expectedBeaconPresetPromises.get(client);

  if (!presetPromise) {
    const beaconPresetQuery = (
      client.query.ethereumVerifier as {
        beaconPreset?: () => Promise<{
          isMainnet: boolean;
          isMinimal: boolean;
          toString(): string;
        }>;
      }
    ).beaconPreset;

    if (!beaconPresetQuery) {
      return;
    }

    presetPromise = beaconPresetQuery.call(client.query.ethereumVerifier).then(preset => {
      if (preset.isMainnet) {
        return 'mainnet';
      }
      if (preset.isMinimal) {
        return 'minimal';
      }

      throw new Error(`Unknown ethereum verifier beacon preset: ${preset.toString()}`);
    });
    expectedBeaconPresetPromises.set(client, presetPromise);
  }

  try {
    return await presetPromise;
  } catch (error) {
    expectedBeaconPresetPromises.delete(client);
    throw error;
  }
}

function computePeriod(slot: bigint, beaconNetworkParams: BeaconNetworkParams) {
  return (
    slot / beaconNetworkParams.slotsPerEpoch / beaconNetworkParams.epochsPerSyncCommitteePeriod
  );
}

function hasSyncCommitteeSupermajority(syncCommitteeBits: string) {
  const bytes = hexToBytes(syncCommitteeBits as Hex);
  const totalBits = bytes.length * 8;
  const participants = bytes.reduce((total, byte) => {
    let count = 0;

    for (let value = byte; value > 0; value >>= 1) {
      count += value & 1;
    }

    return total + count;
  }, 0);

  return participants * 3 >= totalBits * 2;
}

function isRetryableBeaconLightClientError(error: unknown) {
  const message = error instanceof Error ? error.message : String(error);

  return (
    message.includes('/eth/v1/beacon/light_client/') &&
    (message.includes(': 404') || message.includes(': 500'))
  );
}
