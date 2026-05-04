import { createProof as createSingleProof, ProofType } from '@chainsafe/persistent-merkle-tree';
import camelcaseKeys from 'camelcase-keys';
import { ForkName } from '@lodestar/params';
import type { SubmittableExtrinsic } from '@polkadot/api/promise/types';
import type {
  EthereumBeaconBlockResponse,
  EthereumBeaconConfigSpecResponse,
  EthereumBeaconExecutionPayload,
  EthereumBeaconGenesisResponse,
  EthereumBeaconHeaderDetailsResponse,
  EthereumLightClientHeader,
  EthereumLightClientBootstrapResponse,
  EthereumLightClientUpdate,
  EthereumLightClientUpdatesResponse,
} from './EthereumBeaconTypes';
import type { ArgonClient } from './index';
import { bytesToHex, hexToBytes, type Hex } from 'viem';

type BeaconPreset = 'mainnet' | 'minimal';
type BeaconForkName = ForkName.capella | ForkName.deneb | ForkName.electra | ForkName.fulu;
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

let loadedEthereumBeaconPreset: BeaconPreset | undefined;
const lightClientUpdateRetryIntervalMs = 500;
const lightClientUpdateRetryTimeoutMs = 30_000;
let lodestarModulesPromise:
  | Promise<{
      epochsPerSyncCommitteePeriod: bigint;
      slotsPerEpoch: bigint;
      sszTypesFor: typeof import('@lodestar/types').sszTypesFor;
    }>
  | undefined;
const beaconNetworkParamsPromises = new Map<string, Promise<BeaconNetworkParams>>();

export async function getEthereumBeaconSyncBootstrapTx(
  client: ArgonClient,
  beaconApiUrl: string,
): Promise<SubmittableExtrinsic> {
  const [genesis, spec, finalizedHeader] = await Promise.all([
    getBeaconJson<EthereumBeaconGenesisResponse>(beaconApiUrl, '/eth/v1/beacon/genesis'),
    getBeaconJson<EthereumBeaconConfigSpecResponse>(beaconApiUrl, '/eth/v1/config/spec'),
    getBeaconJson<EthereumBeaconHeaderDetailsResponse>(
      beaconApiUrl,
      '/eth/v1/beacon/headers/finalized',
    ),
  ]);
  const bootstrap = await getBeaconJson<EthereumLightClientBootstrapResponse>(
    beaconApiUrl,
    `/eth/v1/beacon/light_client/bootstrap/${finalizedHeader.data.root}`,
  );
  const specData = spec.data;

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

  const beaconNetworkParams = await getBeaconNetworkParams(beaconApiUrl);

  const update = await getNextLightClientUpdate(beaconApiUrl, state, beaconNetworkParams);
  const txs: SubmittableExtrinsic[] = [];

  if (update) {
    const finalizedSlot = BigInt(update.finalized_header.beacon.slot);
    const finalizedPeriod = computePeriod(finalizedSlot, beaconNetworkParams);
    const attestedPeriod = computePeriod(
      BigInt(update.attested_header.beacon.slot),
      beaconNetworkParams,
    );
    const storePeriod = computePeriod(state.latestFinalizedSlot, beaconNetworkParams);
    const canAdvanceFinalized =
      finalizedSlot >= state.nextRecommendedFinalizedSlot &&
      finalizedSlot > state.latestFinalizedSlot;
    const shouldIncludeNextSyncCommitteeUpdate =
      !!update.next_sync_committee &&
      !!update.next_sync_committee_branch &&
      (!state.hasNextSyncCommittee || finalizedPeriod > state.latestSyncCommitteeUpdatePeriod);
    const canSetMissingNextSync =
      !state.hasNextSyncCommittee &&
      shouldIncludeNextSyncCommitteeUpdate &&
      attestedPeriod === storePeriod;
    const submitTx =
      canAdvanceFinalized || canSetMissingNextSync
        ? client.tx.ethereumVerifier.submit({
            attestedHeader: camelcaseKeys(update.attested_header.beacon),
            syncAggregate: camelcaseKeys(update.sync_aggregate),
            signatureSlot: update.signature_slot,
            ...(shouldIncludeNextSyncCommitteeUpdate
              ? {
                  nextSyncCommitteeUpdate: {
                    nextSyncCommittee: camelcaseKeys(update.next_sync_committee!),
                    nextSyncCommitteeBranch: update.next_sync_committee_branch!,
                  },
                }
              : {}),
            finalizedHeader: camelcaseKeys(update.finalized_header.beacon),
            finalityBranch: update.finality_branch,
          })
        : undefined;
    if (submitTx) {
      txs.push(submitTx);

      const beaconBlockRoot = await getBeaconHeaderRoot(
        beaconApiUrl,
        update.finalized_header.beacon.slot,
        update.finalized_header.beacon,
      );
      const anchorTx = await createExecutionHeaderAnchorTx(client, beaconApiUrl, beaconBlockRoot);
      if (anchorTx) {
        txs.push(anchorTx);
      }

      return txs;
    }
  }

  const anchorTx = await createExecutionHeaderAnchorTx(
    client,
    beaconApiUrl,
    state.latestFinalizedBlockRoot,
  );
  if (anchorTx) {
    txs.push(anchorTx);
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

async function buildExecutionHeaderAnchorProof(beaconApiUrl: string, beaconBlockRoot: string) {
  const [beaconNetworkParams, header, block] = await Promise.all([
    getBeaconNetworkParams(beaconApiUrl),
    getBeaconJson<EthereumBeaconHeaderDetailsResponse>(
      beaconApiUrl,
      `/eth/v1/beacon/headers/${beaconBlockRoot}`,
    ),
    getBeaconJson<EthereumBeaconBlockResponse>(
      beaconApiUrl,
      `/eth/v2/beacon/blocks/${beaconBlockRoot}`,
    ),
  ]);
  const { sszTypesFor } = await loadLodestarModules(beaconNetworkParams.preset);
  const forkName = getBeaconForkName(block.version);
  const { BeaconBlockBody } = sszTypesFor(forkName);
  const ExecutionPayload = sszTypesFor(forkName, 'ExecutionPayload');
  const body = BeaconBlockBody.fromJson(block.data.message.body);
  const bodyRoot = bytesToHex(BeaconBlockBody.hashTreeRoot(body)).toLowerCase();
  const beaconHeader = header.data.header.message;

  if (bodyRoot !== beaconHeader.body_root.toLowerCase()) {
    throw new Error(
      `Beacon block body root mismatch at slot ${beaconHeader.slot}: expected ${beaconHeader.body_root}, got ${bodyRoot}`,
    );
  }

  const { gindex } = BeaconBlockBody.getPathInfo(['executionPayload']);
  const proof = createSingleProof(BeaconBlockBody.toView(body).node, {
    type: ProofType.single,
    gindex,
  }) as { witnesses: Uint8Array[] };
  const executionPayload = ExecutionPayload.fromJson(block.data.message.body.execution_payload);
  const transactionsRoot = bytesToHex(
    ExecutionPayload.getPropertyType('transactions').hashTreeRoot(executionPayload.transactions),
  ).toLowerCase();
  const withdrawalsRoot = bytesToHex(
    ExecutionPayload.getPropertyType('withdrawals').hashTreeRoot(executionPayload.withdrawals),
  ).toLowerCase();

  return {
    header: camelcaseKeys(beaconHeader),
    executionHeader: buildExecutionPayloadHeader(
      block.data.message.body.execution_payload,
      transactionsRoot,
      withdrawalsRoot,
    ),
    executionBranch: proof.witnesses.map(witness => bytesToHex(witness).toLowerCase() as Hex),
  };
}

async function createExecutionHeaderAnchorTx(
  client: ArgonClient,
  beaconApiUrl: string,
  beaconBlockRoot: string,
): Promise<SubmittableExtrinsic | undefined> {
  const executionProof = await buildExecutionHeaderAnchorProof(beaconApiUrl, beaconBlockRoot);
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

async function getBeaconHeaderRoot(
  beaconApiUrl: string,
  slot: string,
  expectedHeader: EthereumLightClientHeader['beacon'],
): Promise<string> {
  const response = await getBeaconJson<EthereumBeaconHeaderDetailsResponse>(
    beaconApiUrl,
    `/eth/v1/beacon/headers/${slot}`,
  );

  const actualHeader = response.data.header.message;
  if (
    actualHeader.slot !== expectedHeader.slot ||
    actualHeader.proposer_index !== expectedHeader.proposer_index ||
    actualHeader.parent_root.toLowerCase() !== expectedHeader.parent_root.toLowerCase() ||
    actualHeader.state_root.toLowerCase() !== expectedHeader.state_root.toLowerCase() ||
    actualHeader.body_root.toLowerCase() !== expectedHeader.body_root.toLowerCase()
  ) {
    throw new Error(`Beacon header mismatch at slot ${slot}`);
  }

  return response.data.root.toLowerCase();
}

async function getNextLightClientUpdate(
  beaconApiUrl: string,
  state: Extract<EthereumBeaconSyncState, { isBootstrapped: true }>,
  beaconNetworkParams: BeaconNetworkParams,
): Promise<EthereumLightClientUpdate | undefined> {
  const startedAt = Date.now();

  while (Date.now() - startedAt < lightClientUpdateRetryTimeoutMs) {
    const finalityUpdate = (
      await getBeaconJson<{ data: EthereumLightClientUpdate }>(
        beaconApiUrl,
        '/eth/v1/beacon/light_client/finality_update',
      )
    ).data;

    if (!hasSyncCommitteeSupermajority(finalityUpdate.sync_aggregate.sync_committee_bits)) {
      await new Promise(resolve => setTimeout(resolve, lightClientUpdateRetryIntervalMs));
      continue;
    }

    const finalizedPeriod = computePeriod(
      BigInt(finalityUpdate.finalized_header.beacon.slot),
      beaconNetworkParams,
    );
    const needsSyncCommitteeUpdate =
      !state.hasNextSyncCommittee || finalizedPeriod > state.latestSyncCommitteeUpdatePeriod;
    const finalityUpdateIncludesNextSyncCommittee =
      !!finalityUpdate.next_sync_committee && !!finalityUpdate.next_sync_committee_branch;

    if (!needsSyncCommitteeUpdate || finalityUpdateIncludesNextSyncCommittee) {
      return finalityUpdate;
    }

    const periodUpdate = (
      await getBeaconJson<EthereumLightClientUpdatesResponse>(
        beaconApiUrl,
        `/eth/v1/beacon/light_client/updates?count=1&start_period=${computePeriod(state.latestFinalizedSlot, beaconNetworkParams)}`,
      )
    ).data?.[0];

    if (periodUpdate?.next_sync_committee && periodUpdate.next_sync_committee_branch) {
      return periodUpdate;
    }

    if (finalizedPeriod > state.latestSyncCommitteeUpdatePeriod) {
      return;
    }

    return finalityUpdate;
  }

  return;
}

function buildExecutionPayloadHeader(
  header: EthereumBeaconExecutionPayload,
  transactionsRoot: string,
  withdrawalsRoot: string,
) {
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
    transactionsRoot,
    withdrawalsRoot,
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

function getBeaconForkName(version: string): BeaconForkName {
  if (version === 'capella') {
    return ForkName.capella;
  }
  if (version === 'deneb') {
    return ForkName.deneb;
  }
  if (version === 'electra') {
    return ForkName.electra;
  }
  if (version === 'fulu') {
    return ForkName.fulu;
  }

  throw new Error(`Unsupported beacon block version ${version}`);
}

async function getBeaconNetworkParams(beaconApiUrl: string): Promise<BeaconNetworkParams> {
  let paramsPromise = beaconNetworkParamsPromises.get(beaconApiUrl);

  if (!paramsPromise) {
    paramsPromise = getBeaconJson<EthereumBeaconConfigSpecResponse>(
      beaconApiUrl,
      '/eth/v1/config/spec',
    ).then(({ data }) => parseBeaconNetworkParams(data));
    beaconNetworkParamsPromises.set(beaconApiUrl, paramsPromise);
  }

  return paramsPromise;
}

async function loadLodestarModules(preset: BeaconPreset) {
  if (loadedEthereumBeaconPreset && loadedEthereumBeaconPreset !== preset) {
    throw new Error(
      `Ethereum beacon preset already initialized as ${loadedEthereumBeaconPreset}, cannot switch to ${preset} in the same process`,
    );
  }

  if (!lodestarModulesPromise) {
    lodestarModulesPromise = (async () => {
      if (preset === 'minimal') {
        const { PresetName, setActivePreset } = await import('@lodestar/params/setPreset');

        try {
          setActivePreset(PresetName.minimal);
        } catch (error) {
          throw new Error(
            `Minimal beacon preset must be selected before Lodestar params are loaded: ${error instanceof Error ? error.message : String(error)}`,
          );
        }
      }

      const [{ EPOCHS_PER_SYNC_COMMITTEE_PERIOD, SLOTS_PER_EPOCH }, { sszTypesFor }] =
        await Promise.all([import('@lodestar/params'), import('@lodestar/types')]);

      loadedEthereumBeaconPreset = preset;
      return {
        epochsPerSyncCommitteePeriod: BigInt(EPOCHS_PER_SYNC_COMMITTEE_PERIOD),
        slotsPerEpoch: BigInt(SLOTS_PER_EPOCH),
        sszTypesFor,
      };
    })();
  }

  return lodestarModulesPromise;
}

function parseBeaconNetworkParams(spec: Record<string, string>): BeaconNetworkParams {
  const slotsPerEpoch = spec.SLOTS_PER_EPOCH;
  const epochsPerSyncCommitteePeriod = spec.EPOCHS_PER_SYNC_COMMITTEE_PERIOD;

  if (!slotsPerEpoch) {
    throw new Error('Missing beacon spec value for SLOTS_PER_EPOCH');
  }
  if (!epochsPerSyncCommitteePeriod) {
    throw new Error('Missing beacon spec value for EPOCHS_PER_SYNC_COMMITTEE_PERIOD');
  }

  return {
    preset: detectBeaconPreset(spec),
    slotsPerEpoch: BigInt(slotsPerEpoch),
    epochsPerSyncCommitteePeriod: BigInt(epochsPerSyncCommitteePeriod),
  };
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
