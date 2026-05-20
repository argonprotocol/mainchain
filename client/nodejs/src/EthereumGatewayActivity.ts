import {
  mintingGatewayAbi,
  type MintingGatewayActivityBlockLocator,
  MintingGatewayEvents,
  type MintingGatewayGlobalIssuanceCouncilRotated,
  type MintingGatewayMintingAuthorityActivated,
  type MintingGatewayMintingAuthorityDeactivated,
  type MintingGatewayTransferOutOfArgonCanceled,
  type MintingGatewayTransferOutOfArgonFinalized,
  type MintingGatewayTransferToArgonStarted,
} from '@argonprotocol/ethereum-contracts';
import type { ArgonClient, IArgonQueryable } from './index';
import { decodeEventLog, getAddress, type Hex } from 'viem';
import {
  ArgonFinalizedExecutionHeaderPathError,
  buildExecutionHeaderChain,
  buildEthereumEventProof,
  type ArgonFinalizedExecutionHeader,
  type EthereumEventLocator,
  type EthereumEventLocatorBlock,
  type EthereumEventLog,
  type EthereumEventProof,
  getLatestArgonFinalizedExecutionHeader,
  loadExecutionHeader,
} from './EthereumProof';
import {
  getExecutionClient,
  type EthereumExecutionClient,
  type EthereumExecutionSource,
  retryWhileExecutionRpcIndexing,
} from './EthereumExecution';

type DecodedEthereumGatewayActivity =
  | ({
      kind: typeof MintingGatewayEvents.GlobalIssuanceCouncilRotated.name;
    } & MintingGatewayGlobalIssuanceCouncilRotated)
  | ({
      kind: typeof MintingGatewayEvents.MintingAuthorityActivated.name;
    } & MintingGatewayMintingAuthorityActivated)
  | ({
      kind: typeof MintingGatewayEvents.MintingAuthorityDeactivated.name;
    } & MintingGatewayMintingAuthorityDeactivated)
  | ({
      kind: typeof MintingGatewayEvents.TransferOutOfArgonCanceled.name;
    } & MintingGatewayTransferOutOfArgonCanceled)
  | ({
      kind: typeof MintingGatewayEvents.TransferOutOfArgonFinalized.name;
    } & MintingGatewayTransferOutOfArgonFinalized)
  | ({
      kind: typeof MintingGatewayEvents.TransferToArgonStarted.name;
    } & MintingGatewayTransferToArgonStarted);

export type EthereumGatewayActivity = DecodedEthereumGatewayActivity & {
  txHash: Hex;
  transactionIndex: number;
  logIndex: number;
  blockHash: Hex;
  blockNumber: bigint;
};

export type EthereumGatewayActivityProofPayload = {
  previousGatewayActivityNonce: bigint;
  proof: EthereumEventProof;
  gatewayActivityNonceRange: {
    start: bigint;
    end: bigint;
  };
  executionBlockNumberRange: {
    start: bigint;
    end: bigint;
  };
  activities: EthereumGatewayActivity[];
};

type EthereumBlockLog = Awaited<ReturnType<EthereumExecutionClient['getLogs']>>[number];
type DiscoveredGatewayActivities = Pick<
  EthereumGatewayActivityProofPayload,
  'previousGatewayActivityNonce' | 'activities'
>;

type GatewayActivityProofChunk = {
  activities: EthereumGatewayActivity[];
  locatorBlock: EthereumEventLocatorBlock;
};

type GatewayProofBounds = {
  activitiesPerReceiptProof: number;
  receiptProofsPerExtrinsic: number;
  sharedHeaderDepth: number;
};

type ArgonFinalizedExecutionHeaderPlan = {
  argonFinalizedExecutionHeader: ArgonFinalizedExecutionHeader;
  earliestTargetHeader: Awaited<ReturnType<typeof loadExecutionHeader>>;
  targetToArgonFinalizedHeaderChain: Awaited<ReturnType<typeof buildExecutionHeaderChain>>;
};

const gatewayActivityEvents = [
  MintingGatewayEvents.GlobalIssuanceCouncilRotated,
  MintingGatewayEvents.MintingAuthorityActivated,
  MintingGatewayEvents.MintingAuthorityDeactivated,
  MintingGatewayEvents.TransferOutOfArgonCanceled,
  MintingGatewayEvents.TransferOutOfArgonFinalized,
  MintingGatewayEvents.TransferToArgonStarted,
] as const;

export function findEthereumTransferToArgonStartedLogIndexes(
  receipt: { transactionHash: Hex; logs: { address: Hex; topics: Hex[] }[] },
  gatewayAddress: Hex,
): number[] {
  const normalizedGatewayAddress = getAddress(gatewayAddress).toLowerCase();
  const indexes = receipt.logs.flatMap((log, index) =>
    log.address.toLowerCase() === normalizedGatewayAddress &&
    log.topics[0]?.toLowerCase() === MintingGatewayEvents.TransferToArgonStarted.topic
      ? [index]
      : [],
  );

  if (indexes.length === 0) {
    throw new Error(
      `Ethereum receipt ${receipt.transactionHash} did not emit TransferToArgonStarted from gateway ${gatewayAddress}`,
    );
  }

  return indexes;
}

export function decodeEthereumTransferToArgonStartedLog(
  log: Pick<EthereumEventLog, 'topics' | 'data'>,
): MintingGatewayTransferToArgonStarted {
  const decoded = decodeEthereumGatewayActivityLog(log);
  if (decoded.kind !== MintingGatewayEvents.TransferToArgonStarted.name) {
    throw new Error(
      `Expected ${MintingGatewayEvents.TransferToArgonStarted.name} gateway activity log`,
    );
  }

  const { kind: _kind, ...transfer } = decoded;
  return transfer;
}

export function decodeEthereumGatewayActivityLog(
  log: Pick<EthereumEventLog, 'topics' | 'data'>,
): DecodedEthereumGatewayActivity {
  const topic = log.topics[0]?.toLowerCase();
  if (!topic) {
    throw new Error('Gateway activity log is missing an event signature topic');
  }

  const event = gatewayActivityEvents.find(candidate => candidate.topic === topic);
  if (!event) {
    throw new Error(`Unsupported gateway activity event topic ${topic}`);
  }

  const { args } = decodeEventLog({
    abi: mintingGatewayAbi,
    eventName: event.name,
    topics: log.topics as [Hex, ...Hex[]],
    data: log.data,
    strict: true,
  });

  return {
    kind: event.name,
    ...args,
  } as DecodedEthereumGatewayActivity;
}

export async function buildGatewayActivityProofPayload(
  client: IArgonQueryable & Pick<ArgonClient, 'consts'>,
  options: EthereumExecutionSource & {
    gatewayAddress: Hex;
    throughExecutionBlockNumber?: bigint;
  },
): Promise<EthereumGatewayActivityProofPayload | null> {
  const bounds = getGatewayProofBounds(client);
  const discovered = await discoverGatewayActivities(client, options);
  if (!discovered) {
    return null;
  }

  const executionClient = getExecutionClient(options);
  const proofChunks = chunkGatewayActivitiesForProof(
    discovered.activities,
    bounds.activitiesPerReceiptProof,
  );
  const argonFinalizedExecutionHeaderPlan = await selectArgonFinalizedExecutionHeader(
    client,
    executionClient,
    proofChunks[0],
    bounds.sharedHeaderDepth,
  );
  if (!argonFinalizedExecutionHeaderPlan) {
    return null;
  }

  const acceptedProofChunks = await collectProofChunksForArgonFinalizedExecutionHeader(
    executionClient,
    proofChunks,
    argonFinalizedExecutionHeaderPlan,
    bounds.receiptProofsPerExtrinsic,
  );
  if (acceptedProofChunks.length === 0) {
    return null;
  }

  const acceptedActivities = acceptedProofChunks.flatMap(({ activities }) => activities);
  const proof = await buildEthereumEventProof(
    options,
    argonFinalizedExecutionHeaderPlan.argonFinalizedExecutionHeader,
    acceptedProofChunks.map(({ locatorBlock }) => locatorBlock),
  );

  return {
    previousGatewayActivityNonce: discovered.previousGatewayActivityNonce,
    proof,
    gatewayActivityNonceRange: {
      start: acceptedActivities[0].gatewayState.gatewayActivityNonce,
      end: acceptedActivities.at(-1)!.gatewayState.gatewayActivityNonce,
    },
    executionBlockNumberRange: {
      start: acceptedActivities[0].blockNumber,
      end: acceptedActivities.at(-1)!.blockNumber,
    },
    activities: acceptedActivities,
  };
}

async function loadActivityBlockLocator(
  executionClient: EthereumExecutionClient,
  gatewayAddress: Hex,
  locatorIndex: bigint,
  cache: Map<bigint, MintingGatewayActivityBlockLocator>,
): Promise<MintingGatewayActivityBlockLocator> {
  const cached = cache.get(locatorIndex);
  if (cached) {
    return cached;
  }

  const locator = await executionClient.readContract({
    abi: mintingGatewayAbi,
    address: gatewayAddress,
    functionName: 'activityBlockLocators',
    args: [locatorIndex],
  });
  const [blockNumber, startGatewayActivityNonce, endGatewayActivityNonce] = locator;
  const loaded: MintingGatewayActivityBlockLocator = {
    blockNumber,
    startGatewayActivityNonce,
    endGatewayActivityNonce,
  };
  cache.set(locatorIndex, loaded);
  return loaded;
}

async function findFirstUncoveredActivityBlockLocatorIndex(
  executionClient: EthereumExecutionClient,
  gatewayAddress: Hex,
  latestLocatorIndex: bigint,
  previousGatewayActivityNonce: bigint,
  cache: Map<bigint, MintingGatewayActivityBlockLocator>,
): Promise<bigint | null> {
  let low = 1n;
  let high = latestLocatorIndex;
  let firstUncoveredIndex: bigint | null = null;

  while (low <= high) {
    const middle = (low + high) / 2n;
    const locator = await loadActivityBlockLocator(executionClient, gatewayAddress, middle, cache);

    if (locator.endGatewayActivityNonce > previousGatewayActivityNonce) {
      firstUncoveredIndex = middle;
      high = middle - 1n;
      continue;
    }

    low = middle + 1n;
  }

  return firstUncoveredIndex;
}

async function discoverGatewayActivities(
  client: IArgonQueryable,
  options: EthereumExecutionSource & {
    gatewayAddress: Hex;
    throughExecutionBlockNumber?: bigint;
  },
): Promise<DiscoveredGatewayActivities | null> {
  const executionClient = getExecutionClient(options);
  const gatewayAddress = getAddress(options.gatewayAddress);
  const latestLocatorIndex = await executionClient.readContract({
    abi: mintingGatewayAbi,
    address: gatewayAddress,
    functionName: 'latestActivityBlockLocatorIndex',
  });
  if (latestLocatorIndex === 0n) {
    return null;
  }

  const currentGatewayState =
    await client.query.crosschainTransfer.gatewayStateBySourceChain('Ethereum');
  const previousGatewayActivityNonce = currentGatewayState.isSome
    ? currentGatewayState.unwrap().gatewayActivityNonce.toBigInt()
    : 0n;
  const locatorCache = new Map<bigint, MintingGatewayActivityBlockLocator>();
  const firstLocatorIndex = await findFirstUncoveredActivityBlockLocatorIndex(
    executionClient,
    gatewayAddress,
    latestLocatorIndex,
    previousGatewayActivityNonce,
    locatorCache,
  );
  if (!firstLocatorIndex) {
    return null;
  }

  const activities: EthereumGatewayActivity[] = [];
  let expectedGatewayActivityNonce = previousGatewayActivityNonce + 1n;

  for (
    let locatorIndex = firstLocatorIndex;
    locatorIndex <= latestLocatorIndex;
    locatorIndex += 1n
  ) {
    const locator = await loadActivityBlockLocator(
      executionClient,
      gatewayAddress,
      locatorIndex,
      locatorCache,
    );
    if (
      options.throughExecutionBlockNumber !== undefined &&
      locator.blockNumber > options.throughExecutionBlockNumber
    ) {
      break;
    }
    if (locator.endGatewayActivityNonce < expectedGatewayActivityNonce) {
      continue;
    }

    const blockActivities = await loadGatewayActivitiesForLocator(
      executionClient,
      gatewayAddress,
      locator,
      expectedGatewayActivityNonce,
    );
    activities.push(...blockActivities);
    expectedGatewayActivityNonce = locator.endGatewayActivityNonce + 1n;
  }

  if (activities.length === 0) {
    return null;
  }

  return {
    previousGatewayActivityNonce,
    activities,
  };
}

async function loadGatewayActivitiesForLocator(
  executionClient: EthereumExecutionClient,
  gatewayAddress: Hex,
  locator: MintingGatewayActivityBlockLocator,
  expectedGatewayActivityNonce: bigint,
): Promise<EthereumGatewayActivity[]> {
  const blockLogs = await retryWhileExecutionRpcIndexing(() =>
    executionClient.getLogs({
      address: gatewayAddress,
      fromBlock: locator.blockNumber,
      toBlock: locator.blockNumber,
    }),
  );
  const activities = blockLogs
    .flatMap(log => {
      try {
        return [toEthereumGatewayActivity(log)];
      } catch {
        return [];
      }
    })
    .sort((left, right) =>
      left.gatewayState.gatewayActivityNonce < right.gatewayState.gatewayActivityNonce ? -1 : 1,
    );
  const remainingActivities = activities.filter(
    activity =>
      activity.gatewayState.gatewayActivityNonce >= expectedGatewayActivityNonce &&
      activity.gatewayState.gatewayActivityNonce <= locator.endGatewayActivityNonce,
  );
  if (
    remainingActivities.length === 0 ||
    remainingActivities[0].gatewayState.gatewayActivityNonce !== expectedGatewayActivityNonce
  ) {
    throw new Error(`Gateway block ${locator.blockNumber} contains uncovered gateway activity`);
  }

  let previousNonce = expectedGatewayActivityNonce - 1n;
  for (const activity of remainingActivities) {
    if (activity.gatewayState.gatewayActivityNonce !== previousNonce + 1n) {
      throw new Error(
        `Gateway block ${locator.blockNumber} contains a gap in gateway activity coverage`,
      );
    }
    previousNonce = activity.gatewayState.gatewayActivityNonce;
  }

  if (previousNonce !== locator.endGatewayActivityNonce) {
    throw new Error(`Gateway block ${locator.blockNumber} contains uncovered gateway activity`);
  }

  return remainingActivities;
}

function toEthereumGatewayActivity(log: EthereumBlockLog): EthereumGatewayActivity {
  if (
    !log.transactionHash ||
    log.transactionIndex == null ||
    log.logIndex == null ||
    !log.blockHash ||
    log.blockNumber == null
  ) {
    throw new Error('Execution log is missing indexed receipt metadata');
  }

  return {
    txHash: log.transactionHash,
    transactionIndex: Number(log.transactionIndex),
    logIndex: Number(log.logIndex),
    blockHash: log.blockHash,
    blockNumber: log.blockNumber,
    ...decodeEthereumGatewayActivityLog(log),
  };
}

function chunkGatewayActivitiesForProof(
  activities: EthereumGatewayActivity[],
  maxActivitiesPerReceiptProof: number,
): GatewayActivityProofChunk[] {
  const chunks: GatewayActivityProofChunk[] = [];
  let currentBlockHash: Hex | undefined;
  let currentBlockActivities: EthereumGatewayActivity[] = [];

  const flushBlock = () => {
    if (!currentBlockHash || currentBlockActivities.length === 0) {
      return;
    }

    for (
      let start = 0;
      start < currentBlockActivities.length;
      start += maxActivitiesPerReceiptProof
    ) {
      const chunkActivities = currentBlockActivities.slice(
        start,
        start + maxActivitiesPerReceiptProof,
      );
      const locatorsByReceipt = new Map<Hex, EthereumEventLocator>();

      for (const activity of chunkActivities) {
        const existing = locatorsByReceipt.get(activity.txHash);
        if (existing) {
          existing.logIndexes!.push(activity.logIndex);
          continue;
        }

        locatorsByReceipt.set(activity.txHash, {
          txHash: activity.txHash,
          logIndexes: [activity.logIndex],
        });
      }

      chunks.push({
        activities: chunkActivities,
        locatorBlock: [...locatorsByReceipt.values()],
      });
    }
  };

  for (const activity of activities) {
    if (currentBlockHash && activity.blockHash.toLowerCase() !== currentBlockHash.toLowerCase()) {
      flushBlock();
      currentBlockActivities = [];
    }

    currentBlockHash = activity.blockHash;
    currentBlockActivities.push(activity);
  }

  flushBlock();
  return chunks;
}

function getGatewayProofBounds(client: Pick<ArgonClient, 'consts'>): GatewayProofBounds {
  const activitiesPerReceiptProof =
    client.consts.crosschainTransfer.maxActivitiesPerReceiptProof.toNumber();
  if (!Number.isInteger(activitiesPerReceiptProof) || activitiesPerReceiptProof < 1) {
    throw new Error('Gateway proof requires maxActivitiesPerReceiptProof to be a positive integer');
  }

  const receiptProofsPerExtrinsic =
    client.consts.crosschainTransfer.maxReceiptProofsPerExtrinsic.toNumber();
  if (!Number.isInteger(receiptProofsPerExtrinsic) || receiptProofsPerExtrinsic < 1) {
    throw new Error('Gateway proof requires maxReceiptProofsPerExtrinsic to be a positive integer');
  }

  return {
    activitiesPerReceiptProof,
    receiptProofsPerExtrinsic,
    sharedHeaderDepth: client.consts.crosschainTransfer.maxProofExecutionHeaderDepth.toNumber(),
  };
}

async function selectArgonFinalizedExecutionHeader(
  client: IArgonQueryable,
  executionClient: EthereumExecutionClient,
  earliestProofChunk: GatewayActivityProofChunk,
  sharedHeaderDepth: number,
): Promise<ArgonFinalizedExecutionHeaderPlan | null> {
  const earliestBlockHash = earliestProofChunk.activities[0].blockHash;
  const earliestBlockNumber = earliestProofChunk.activities[0].blockNumber;
  const earliestTargetHeader = await loadExecutionHeader(executionClient, earliestBlockHash);
  const latestArgonFinalizedExecutionHeader = await getLatestArgonFinalizedExecutionHeader(client);

  if (earliestBlockNumber > latestArgonFinalizedExecutionHeader.blockNumber) {
    return null;
  }

  const maxArgonFinalizedExecutionHeaderBlockNumber =
    earliestBlockNumber + BigInt(sharedHeaderDepth);

  if (
    latestArgonFinalizedExecutionHeader.blockNumber <= maxArgonFinalizedExecutionHeaderBlockNumber
  ) {
    try {
      return {
        argonFinalizedExecutionHeader: latestArgonFinalizedExecutionHeader,
        earliestTargetHeader,
        targetToArgonFinalizedHeaderChain: await buildExecutionHeaderChain(
          executionClient,
          earliestTargetHeader,
          latestArgonFinalizedExecutionHeader,
        ),
      };
    } catch (error) {
      if (!(error instanceof ArgonFinalizedExecutionHeaderPathError)) {
        throw error;
      }
    }
  }

  const verifierQuery = client.query.ethereumVerifier;
  const beaconPreset = await verifierQuery.beaconPreset();
  const argonFinalizedExecutionHeaderCapacity = beaconPreset.isMainnet
    ? 256 * 20
    : beaconPreset.isMinimal
      ? 8 * 20
      : 0;
  if (argonFinalizedExecutionHeaderCapacity === 0) {
    throw new Error(`Unknown ethereum verifier beacon preset: ${beaconPreset.toString()}`);
  }

  const zeroHash = `0x${'00'.repeat(32)}`;
  let index = (await verifierQuery.executionHeaderAnchorIndex()).toNumber();

  for (let scanned = 0; scanned < argonFinalizedExecutionHeaderCapacity; scanned += 1) {
    const argonFinalizedExecutionHeaderHash = (
      await verifierQuery.executionHeaderAnchorMapping(index)
    ).toHex();
    if (argonFinalizedExecutionHeaderHash === zeroHash) {
      break;
    }

    const argonFinalizedExecutionHeaderEntry = await verifierQuery.executionHeaderAnchors(
      argonFinalizedExecutionHeaderHash,
    );
    if (argonFinalizedExecutionHeaderEntry.isNone) {
      break;
    }

    const blockNumber = argonFinalizedExecutionHeaderEntry.unwrap().blockNumber.toBigInt();
    if (blockNumber > maxArgonFinalizedExecutionHeaderBlockNumber) {
      index = index === 0 ? argonFinalizedExecutionHeaderCapacity - 1 : index - 1;
      continue;
    }
    if (blockNumber < earliestBlockNumber) {
      break;
    }

    const argonFinalizedExecutionHeader = {
      blockHash: argonFinalizedExecutionHeaderHash,
      blockNumber,
    };

    try {
      return {
        argonFinalizedExecutionHeader,
        earliestTargetHeader,
        targetToArgonFinalizedHeaderChain: await buildExecutionHeaderChain(
          executionClient,
          earliestTargetHeader,
          argonFinalizedExecutionHeader,
        ),
      };
    } catch (error) {
      if (!(error instanceof ArgonFinalizedExecutionHeaderPathError)) {
        throw error;
      }
    }

    index = index === 0 ? argonFinalizedExecutionHeaderCapacity - 1 : index - 1;
  }

  throw new Error(
    'Oldest uncovered gateway activity exceeds the Argon finalized execution-header window',
  );
}

async function collectProofChunksForArgonFinalizedExecutionHeader(
  executionClient: EthereumExecutionClient,
  proofChunks: GatewayActivityProofChunk[],
  plan: ArgonFinalizedExecutionHeaderPlan,
  receiptProofsPerExtrinsic: number,
): Promise<GatewayActivityProofChunk[]> {
  const acceptedProofChunks: GatewayActivityProofChunk[] = [];
  const loadedTargetHeaders = [
    {
      blockHash: proofChunks[0].activities[0].blockHash,
      targetHeader: plan.earliestTargetHeader,
    },
  ];

  for (const proofChunk of proofChunks) {
    if (acceptedProofChunks.length === receiptProofsPerExtrinsic) {
      break;
    }

    const blockHash = proofChunk.activities[0].blockHash;
    const blockNumber = proofChunk.activities[0].blockNumber;

    if (blockNumber > plan.argonFinalizedExecutionHeader.blockNumber) {
      break;
    }

    let targetHeader = loadedTargetHeaders.find(
      loadedHeader => loadedHeader.blockHash.toLowerCase() === blockHash.toLowerCase(),
    )?.targetHeader;
    if (!targetHeader) {
      targetHeader = await loadExecutionHeader(executionClient, blockHash);
      loadedTargetHeaders.push({ blockHash, targetHeader });
    }

    if (targetHeader.number === plan.argonFinalizedExecutionHeader.blockNumber) {
      if (blockHash.toLowerCase() !== plan.argonFinalizedExecutionHeader.blockHash.toLowerCase()) {
        break;
      }
    } else {
      const expectedTargetHeaderOnArgonFinalizedChain =
        plan.targetToArgonFinalizedHeaderChain[
          Number(targetHeader.number - plan.earliestTargetHeader.number)
        ];
      if (
        !expectedTargetHeaderOnArgonFinalizedChain ||
        expectedTargetHeaderOnArgonFinalizedChain.blockHash.toLowerCase() !==
          blockHash.toLowerCase()
      ) {
        break;
      }
    }

    acceptedProofChunks.push(proofChunk);
  }

  return acceptedProofChunks;
}
