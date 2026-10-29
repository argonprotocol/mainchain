import camelcaseKeys from 'camelcase-keys';
import type { Hex } from 'viem';
import type {
  EthereumBeaconConfigSpecResponse,
  EthereumBeaconGenesisResponse,
  EthereumLightClientBootstrapResponse,
} from './EthereumBeaconTypes';
import { buildExecutionHeaderProof, getBeaconJson } from './EthereumBeaconApi';
import {
  loadRetainedExecutionHeaderAnchorAtOrAfterBlock,
  type MissingGatewayActivityLocator,
} from './EthereumGatewayActivityProof';
import { getExecutionClient, type EthereumExecutionSource } from './EthereumExecution';
import { decodeExecutionHeaderRlp } from './EthereumProof';
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
  executionHeaderProof: Awaited<ReturnType<typeof buildExecutionHeaderProof>>;
  executionBlockNumber: bigint;
  executionBlockHash: Hex;
};

type EthereumBeaconHeadersResponse = {
  data: Array<{
    root: Hex;
  }>;
};

/**
 * Builds one execution-header backfill for a chosen target execution block.
 */
export async function buildGatewayExecutionHeaderBackfill(
  client: IArgonQueryable & Pick<ArgonClient, 'consts'>,
  options: EthereumExecutionSource & {
    beaconApiUrl: string;
    targetExecutionBlockNumber: bigint;
  },
): Promise<EthereumExecutionHeaderBackfillPayload | null> {
  const latestExecutionHeaderAnchorBlockHash =
    await client.query.ethereumVerifier.latestExecutionHeaderAnchorBlockHash();
  if (latestExecutionHeaderAnchorBlockHash.isNone) {
    throw new Error('No Argon finalized execution header is available yet; wait for relayer sync');
  }

  const latestExecutionHeaderAnchorBlockHashHex = latestExecutionHeaderAnchorBlockHash
    .unwrap()
    .toHex();
  const latestExecutionHeaderAnchor = await client.query.ethereumVerifier.executionHeaderAnchors(
    latestExecutionHeaderAnchorBlockHashHex,
  );
  if (latestExecutionHeaderAnchor.isNone) {
    throw new Error(
      `Argon finalized execution header ${latestExecutionHeaderAnchorBlockHashHex} is missing`,
    );
  }

  const targetExecutionBlockNumber = options.targetExecutionBlockNumber;

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
        const bootstrapHeader = bootstrap.data.header;
        if (!bootstrapHeader.execution) {
          const bootstrapSlot = BigInt(bootstrapHeader.beacon.slot);
          const capellaSlot = BigInt(spec.data.CAPELLA_FORK_EPOCH ?? '0') * slotsPerEpoch;
          if (bootstrapSlot === 0n || bootstrapSlot < capellaSlot) {
            scanSlot += 1n;
            continue;
          }
        }

        const executionHeaderProof = await buildExecutionHeaderProof(
          bootstrapHeader,
          options,
          spec.data,
        );
        const proofHeader = executionHeaderProof.executionHeader;
        let executionBlockNumber: bigint;
        if ('Rlp' in proofHeader) {
          executionBlockNumber = decodeExecutionHeaderRlp(proofHeader.Rlp).number;
        } else if (proofHeader.Deneb) {
          executionBlockNumber = BigInt(proofHeader.Deneb.blockNumber);
        } else {
          executionBlockNumber = BigInt(proofHeader.Capella.blockNumber);
        }
        let executionBlockHash: Hex;
        if (bootstrapHeader.execution) {
          executionBlockHash = bootstrapHeader.execution.block_hash.toLowerCase() as Hex;
        } else {
          executionBlockHash = bootstrapHeader.execution_block_hash.toLowerCase() as Hex;
        }

        if (executionBlockNumber >= targetExecutionBlockNumber) {
          return {
            targetExecutionBlockNumber,
            expectedBeaconRoot: checkpointHeader.root.toLowerCase() as Hex,
            header: camelcaseKeys(
              bootstrapHeader.beacon,
            ) as EthereumExecutionHeaderBackfillPayload['header'],
            executionHeaderProof,
            executionBlockNumber,
            executionBlockHash,
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

/**
 * Builds the execution-header backfills still needed for the supplied cached locator blocks.
 */
export async function buildGatewayExecutionHeaderBackfills(
  client: IArgonQueryable & Pick<ArgonClient, 'consts'>,
  options: EthereumExecutionSource & {
    beaconApiUrl: string;
    locators: MissingGatewayActivityLocator[];
  },
): Promise<EthereumExecutionHeaderBackfillPayload[]> {
  const currentGatewayState =
    await client.query.crosschainTransfer.gatewayStateBySourceChain('Ethereum');
  const minimumGatewayActivityNonce = currentGatewayState.isSome
    ? currentGatewayState.unwrap().gatewayActivityNonce.toBigInt() + 1n
    : 1n;
  const locators = [...options.locators]
    .filter(locator => locator.endGatewayActivityNonce >= minimumGatewayActivityNonce)
    .sort((left, right) =>
      left.startGatewayActivityNonce < right.startGatewayActivityNonce ? -1 : 1,
    );
  if (locators.length === 0) {
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
  for (const locator of locators) {
    if (seenTargetBlocks.has(locator.blockNumber)) {
      continue;
    }
    seenTargetBlocks.add(locator.blockNumber);
    targetExecutionBlockNumbers.push(locator.blockNumber);
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
      beaconApiUrl: options.beaconApiUrl,
      executionClient: options.executionClient,
      executionRpcUrl: options.executionRpcUrl,
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
