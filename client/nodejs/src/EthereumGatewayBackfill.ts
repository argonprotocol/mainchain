import camelcaseKeys from 'camelcase-keys';
import { type Hex } from 'viem';
import type {
  EthereumBeaconConfigSpecResponse,
  EthereumBeaconHeadersResponse,
  EthereumLightClientBootstrapResponse,
} from './EthereumBeaconTypes';
import { buildExecutionHeaderProof, getBeaconJson } from './EthereumBeaconApi';
import {
  discoverGatewayActivities,
  loadRetainedExecutionHeaderAnchorAtOrAfterBlock,
} from './EthereumGatewayActivityProof';
import type { EthereumExecutionSource } from './EthereumExecution';
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

  const spec = await getBeaconJson<EthereumBeaconConfigSpecResponse>(
    options.beaconApiUrl,
    '/eth/v1/config/spec',
  );
  const slotsPerEpoch = BigInt(spec.data.SLOTS_PER_EPOCH);
  if (slotsPerEpoch <= 0n) {
    throw new Error('Beacon API returned an invalid SLOTS_PER_EPOCH value');
  }

  let scanSlot = finalizedStateValue.slot.toBigInt();
  scanSlot -= scanSlot % slotsPerEpoch;
  let closestCoveringPayload: EthereumExecutionHeaderBackfillPayload | null = null;

  while (true) {
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
          closestCoveringPayload = {
            targetExecutionBlockNumber,
            expectedBeaconRoot: checkpointHeader.root.toLowerCase() as Hex,
            header: camelcaseKeys(
              bootstrap.data.header.beacon,
            ) as EthereumExecutionHeaderBackfillPayload['header'],
            executionHeaderProof: buildExecutionHeaderProof(bootstrap.data.header),
            executionBlockNumber,
            executionBlockHash: bootstrap.data.header.execution.block_hash.toLowerCase() as Hex,
          };
        } else if (closestCoveringPayload) {
          return closestCoveringPayload;
        } else {
          return null;
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

    if (scanSlot < slotsPerEpoch) {
      return closestCoveringPayload;
    }

    scanSlot -= slotsPerEpoch;
  }
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
