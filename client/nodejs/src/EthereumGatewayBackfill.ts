import camelcaseKeys from 'camelcase-keys';
import { type Hex } from 'viem';
import type {
  EthereumBeaconConfigSpecResponse,
  EthereumBeaconGenesisResponse,
  EthereumLightClientBootstrapResponse,
} from './EthereumBeaconTypes';
import { buildExecutionHeaderProof, getBeaconJson } from './EthereumBeaconApi';
import {
  discoverGatewayActivities,
  loadRetainedExecutionHeaderAnchorAtOrAfterBlock,
} from './EthereumGatewayActivityProof';
import { getExecutionClient, type EthereumExecutionSource } from './EthereumExecution';
import type { ArgonClient, IArgonQueryable } from './index';

export type EthereumExecutionHeaderBackfillPayload = {
  targetExecutionBlockNumber: bigint;
  expectedBeaconRoot: Hex;
  header: {
    slot: string;
    proposerIndex: string;
    parentRoot: Hex;
    stateRoot: Hex;
    bodyRoot: Hex;
  };
  executionHeaderProof: ReturnType<typeof buildExecutionHeaderProof>;
  executionBlockNumber: bigint;
  executionBlockHash: Hex;
};

type EthereumBeaconHeadersResponse = {
  data: Array<{
    root: Hex;
  }>;
};

export async function buildGatewayExecutionHeaderBackfill(
  client: IArgonQueryable & Pick<ArgonClient, 'consts'>,
  options: EthereumExecutionSource & {
    beaconApiUrl: string;
    gatewayAddress: Hex;
    throughExecutionBlockNumber?: bigint;
    targetExecutionBlockNumber?: bigint;
  },
): Promise<EthereumExecutionHeaderBackfillPayload | null> {
  const targetExecutionBlockNumber =
    options.targetExecutionBlockNumber ??
    (await discoverGatewayActivities(client, options))?.activities[0]?.blockNumber;
  if (targetExecutionBlockNumber == null) {
    return null;
  }

  const maxProofExecutionHeaderDepth =
    client.consts.crosschainTransfer.maxProofExecutionHeaderDepth.toNumber();
  if (!Number.isInteger(maxProofExecutionHeaderDepth) || maxProofExecutionHeaderDepth < 0) {
    throw new Error(
      'Gateway backfill requires maxProofExecutionHeaderDepth to be a non-negative integer',
    );
  }

  const retainedAnchor = await loadRetainedExecutionHeaderAnchorAtOrAfterBlock(
    client,
    targetExecutionBlockNumber,
  );
  if (
    retainedAnchor &&
    retainedAnchor.blockNumber <= targetExecutionBlockNumber + BigInt(maxProofExecutionHeaderDepth)
  ) {
    return null;
  }

  const latestFinalizedBlockRoot = (await client.query.ethereumVerifier.latestFinalizedBlockRoot())
    .toHex()
    .toLowerCase() as Hex;
  const finalizedState =
    await client.query.ethereumVerifier.finalizedBeaconState(latestFinalizedBlockRoot);
  if (finalizedState.isNone) {
    return null;
  }
  const finalizedStateValue = finalizedState.unwrap();

  const executionClient = getExecutionClient(options);
  const [spec, genesis, targetExecutionHeader] = await Promise.all([
    getBeaconJson<EthereumBeaconConfigSpecResponse>(options.beaconApiUrl, '/eth/v1/config/spec'),
    getBeaconJson<EthereumBeaconGenesisResponse>(options.beaconApiUrl, '/eth/v1/beacon/genesis'),
    executionClient.getBlock({ blockNumber: targetExecutionBlockNumber }),
  ]);
  const slotsPerEpoch = BigInt(spec.data.SLOTS_PER_EPOCH);
  const secondsPerSlot = BigInt(spec.data.SECONDS_PER_SLOT);
  if (slotsPerEpoch <= 0n) {
    throw new Error('Beacon API returned an invalid SLOTS_PER_EPOCH value');
  }
  if (secondsPerSlot <= 0n) {
    throw new Error('Beacon API returned an invalid SECONDS_PER_SLOT value');
  }

  const latestFinalizedSlot = finalizedStateValue.slot.toBigInt();
  const beaconGenesisTime = BigInt(genesis.data.genesis_time);
  let scanSlot =
    targetExecutionHeader.timestamp <= beaconGenesisTime
      ? 0n
      : (targetExecutionHeader.timestamp - beaconGenesisTime) / secondsPerSlot;

  if (scanSlot >= slotsPerEpoch) {
    scanSlot -= slotsPerEpoch;
  } else {
    scanSlot = 0n;
  }
  if (scanSlot > latestFinalizedSlot) {
    scanSlot = latestFinalizedSlot;
  }
  const maxScanSlot =
    scanSlot + slotsPerEpoch * 8n < latestFinalizedSlot
      ? scanSlot + slotsPerEpoch * 8n
      : latestFinalizedSlot;

  while (scanSlot <= maxScanSlot) {
    let headers: EthereumBeaconHeadersResponse = { data: [] };
    try {
      headers = await getBeaconJson<EthereumBeaconHeadersResponse>(
        options.beaconApiUrl,
        `/eth/v1/beacon/headers?slot=${scanSlot}`,
      );
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      if (
        !message.includes(`/eth/v1/beacon/headers?slot=${scanSlot}`) ||
        !message.includes(': 404')
      ) {
        throw error;
      }
    }

    const checkpointHeader = headers.data[0];

    if (checkpointHeader) {
      try {
        const bootstrap = await getBeaconJson<EthereumLightClientBootstrapResponse>(
          options.beaconApiUrl,
          `/eth/v1/beacon/light_client/bootstrap/${checkpointHeader.root}`,
        );
        const executionBlockNumber = BigInt(bootstrap.data.header.execution.block_number);

        if (executionBlockNumber >= targetExecutionBlockNumber) {
          return {
            targetExecutionBlockNumber,
            expectedBeaconRoot: checkpointHeader.root.toLowerCase() as Hex,
            header: camelcaseKeys(
              bootstrap.data.header.beacon,
            ) as EthereumExecutionHeaderBackfillPayload['header'],
            executionHeaderProof: buildExecutionHeaderProof(bootstrap.data.header),
            executionBlockNumber,
            executionBlockHash: bootstrap.data.header.execution.block_hash.toLowerCase() as Hex,
          };
        }
      } catch (error) {
        const message = error instanceof Error ? error.message : String(error);
        if (
          !message.includes('/eth/v1/beacon/light_client/bootstrap/') ||
          !message.includes(': 404')
        ) {
          throw error;
        }
      }
    }

    scanSlot += 1n;
  }

  throw new Error(
    `Unable to find a historical light-client bootstrap near execution block ${targetExecutionBlockNumber} using ${options.beaconApiUrl}; try a beacon API that exposes historical /eth/v1/beacon/light_client/bootstrap roots`,
  );
}

export async function buildGatewayExecutionHeaderBackfills(
  client: IArgonQueryable & Pick<ArgonClient, 'consts'>,
  options: EthereumExecutionSource & {
    beaconApiUrl: string;
    gatewayAddress: Hex;
    throughExecutionBlockNumber?: bigint;
  },
): Promise<EthereumExecutionHeaderBackfillPayload[]> {
  const discovered = await discoverGatewayActivities(client, options);
  if (!discovered) {
    return [];
  }

  const maxProofExecutionHeaderDepth =
    client.consts.crosschainTransfer.maxProofExecutionHeaderDepth.toNumber();
  if (!Number.isInteger(maxProofExecutionHeaderDepth) || maxProofExecutionHeaderDepth < 0) {
    throw new Error(
      'Gateway backfill requires maxProofExecutionHeaderDepth to be a non-negative integer',
    );
  }

  const seenTargetBlocks = new Set<bigint>();
  const targetExecutionBlockNumbers: bigint[] = [];
  for (const activity of discovered.activities) {
    if (seenTargetBlocks.has(activity.blockNumber)) {
      continue;
    }
    seenTargetBlocks.add(activity.blockNumber);
    targetExecutionBlockNumbers.push(activity.blockNumber);
  }

  const payloads: EthereumExecutionHeaderBackfillPayload[] = [];
  let coveringExecutionBlockNumber: bigint | null = null;

  for (const targetExecutionBlockNumber of targetExecutionBlockNumbers) {
    if (
      coveringExecutionBlockNumber !== null &&
      coveringExecutionBlockNumber >= targetExecutionBlockNumber &&
      coveringExecutionBlockNumber <=
        targetExecutionBlockNumber + BigInt(maxProofExecutionHeaderDepth)
    ) {
      continue;
    }

    const retainedAnchor = await loadRetainedExecutionHeaderAnchorAtOrAfterBlock(
      client,
      targetExecutionBlockNumber,
    );
    if (
      retainedAnchor &&
      retainedAnchor.blockNumber <=
        targetExecutionBlockNumber + BigInt(maxProofExecutionHeaderDepth)
    ) {
      coveringExecutionBlockNumber = retainedAnchor.blockNumber;
      continue;
    }

    const payload = await buildGatewayExecutionHeaderBackfill(client, {
      ...options,
      targetExecutionBlockNumber,
    });
    if (!payload) {
      throw new Error(
        `Unable to build a gateway execution-header backfill for execution block ${targetExecutionBlockNumber}`,
      );
    }

    payloads.push(payload);
    coveringExecutionBlockNumber = payload.executionBlockNumber;
  }

  return payloads;
}
