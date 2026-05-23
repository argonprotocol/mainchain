import camelcaseKeys from 'camelcase-keys';
import type { SubmittableExtrinsic } from '@polkadot/api/promise/types';
import type {
  EthereumBeaconConfigSpecResponse,
  EthereumBeaconGenesisResponse,
  EthereumBeaconHeaderDetailsResponse,
  EthereumLightClientBootstrapResponse,
  EthereumLightClientHeader,
  EthereumLightClientUpdate,
  EthereumLightClientUpdatesResponse,
} from './EthereumBeaconTypes';
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
  let anchorHeader: EthereumLightClientHeader | undefined;
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
    const finalityUpdateHasNextSyncCommittee = hasNextSyncCommitteeUpdate(finalityUpdate);
    const finalityFinalizedSlot = BigInt(finalityUpdate.finalized_header.beacon.slot);
    const finalityFinalizedPeriod = computePeriod(finalityFinalizedSlot, beaconNetworkParams);
    const needsNewSyncCommitteePeriod =
      finalityFinalizedPeriod > state.latestSyncCommitteeUpdatePeriod;
    const needsCommitteeData = missingNextSyncCommittee || needsNewSyncCommitteePeriod;
    const nextCommitteePeriod = missingNextSyncCommittee
      ? storedPeriod
      : state.latestSyncCommitteeUpdatePeriod + 1n;
    const needsCurrentPeriodUpdate =
      missingNextSyncCommittee ||
      (needsNewSyncCommitteePeriod && !finalityUpdateHasNextSyncCommittee);
    let submitUpdate: EthereumLightClientUpdate | undefined = finalityUpdate;

    if (needsCommitteeData && needsCurrentPeriodUpdate) {
      let periodUpdates: EthereumLightClientUpdatesResponse;

      try {
        periodUpdates = await getBeaconJson<EthereumLightClientUpdatesResponse>(
          beaconApiUrl,
          `/eth/v1/beacon/light_client/updates?count=1&start_period=${nextCommitteePeriod}`,
        );
      } catch (error) {
        if (!isRetryableBeaconLightClientError(error)) {
          throw error;
        }

        await new Promise(resolve => setTimeout(resolve, lightClientUpdateRetryIntervalMs));
        continue;
      }

      const currentPeriodUpdate = periodUpdates[0]?.data;
      const currentPeriodUpdateHasNextSyncCommittee =
        hasNextSyncCommitteeUpdate(currentPeriodUpdate);

      if (currentPeriodUpdateHasNextSyncCommittee) {
        submitUpdate = currentPeriodUpdate;
      } else if (!missingNextSyncCommittee && !finalityUpdateHasNextSyncCommittee) {
        submitUpdate = undefined;
      }
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
          }),
        );
      }

      if (canAdvanceFinalized) {
        anchorHeader = submitUpdate.finalized_header;
      }
    }

    break;
  }

  anchorHeader ??= await getFinalizedHeaderByRoot(beaconApiUrl, state.latestFinalizedBlockRoot);

  if (anchorHeader) {
    const anchorTx = await createExecutionHeaderAnchorTx(client, anchorHeader);
    if (anchorTx) {
      txs.push(anchorTx);
    }
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

function buildExecutionHeaderAnchorProof(finalizedHeader: EthereumLightClientHeader) {
  return {
    header: camelcaseKeys(finalizedHeader.beacon),
    executionHeader: buildExecutionPayloadHeader(finalizedHeader.execution),
    executionBranch: finalizedHeader.execution_branch.map(witness => witness.toLowerCase() as Hex),
  };
}

async function createExecutionHeaderAnchorTx(
  client: ArgonClient,
  finalizedHeader: EthereumLightClientHeader,
): Promise<SubmittableExtrinsic | undefined> {
  const executionProof = buildExecutionHeaderAnchorProof(finalizedHeader);
  const executionHeader = executionProof.executionHeader;
  const blockHash =
    'Deneb' in executionHeader && executionHeader.Deneb
      ? executionHeader.Deneb.blockHash
      : executionHeader.Capella.blockHash;
  const existingAnchor = await client.query.ethereumVerifier.executionHeaderAnchors(blockHash);

  if (!existingAnchor.isNone) {
    return;
  }

  return client.tx.ethereumVerifier.importExecutionHeaderAnchor(executionProof);
}

async function getFinalizedHeaderByRoot(
  beaconApiUrl: string,
  finalizedBlockRoot: string,
): Promise<EthereumLightClientHeader | undefined> {
  const bootstrap = await getBeaconJson<EthereumLightClientBootstrapResponse>(
    beaconApiUrl,
    `/eth/v1/beacon/light_client/bootstrap/${finalizedBlockRoot}`,
  );

  return bootstrap.data.header;
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

function buildExecutionPayloadHeader(header: EthereumLightClientHeader['execution']) {
  const executionHeader = {
    parentHash: header.parent_hash,
    feeRecipient: header.fee_recipient,
    stateRoot: header.state_root,
    receiptsRoot: header.receipts_root,
    logsBloom: header.logs_bloom,
    prevRandao: header.prev_randao,
    blockNumber: header.block_number,
    gasLimit: header.gas_limit,
    gasUsed: header.gas_used,
    timestamp: header.timestamp,
    extraData: header.extra_data,
    baseFeePerGas: header.base_fee_per_gas,
    blockHash: header.block_hash,
    transactionsRoot: header.transactions_root,
    withdrawalsRoot: header.withdrawals_root,
  };

  if (header.blob_gas_used && header.excess_blob_gas) {
    return {
      Deneb: {
        ...executionHeader,
        blobGasUsed: header.blob_gas_used,
        excessBlobGas: header.excess_blob_gas,
      },
    };
  }

  return { Capella: executionHeader };
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

  if (beaconNetworkParams.preset !== expectedPreset) {
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

async function getBeaconJson<T>(beaconApiUrl: string, path: string): Promise<T> {
  const response = await fetch(
    new URL(path, beaconApiUrl.endsWith('/') ? beaconApiUrl : `${beaconApiUrl}/`),
  );

  if (!response.ok) {
    throw new Error(
      `Beacon API request failed for ${path}: ${response.status} ${response.statusText}`,
    );
  }

  return (await response.json()) as T;
}

function isRetryableBeaconLightClientError(error: unknown) {
  const message = error instanceof Error ? error.message : String(error);

  return (
    message.includes('/eth/v1/beacon/light_client/') &&
    (message.includes(': 404') || message.includes(': 500'))
  );
}
