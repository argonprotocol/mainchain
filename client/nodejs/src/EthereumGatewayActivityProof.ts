import {
  hashMintingGatewayActivityBlockLocator,
  mintingGatewayAbi,
  type MintingGatewayHashContext,
} from './EvmContracts';
import type { ArgonClient, IArgonQueryable } from './index';
import { concatHex, getAddress, keccak256, toHex, type Hex } from 'viem';
import {
  appendEthereumGatewayActivityRoot,
  decodeEthereumGatewayActivityLog,
  hashEthereumGatewayActivity,
  type EthereumGatewayActivity,
} from './EthereumGatewayActivity';
import {
  ArgonFinalizedExecutionHeaderPathError,
  buildExecutionHeaderChain,
  buildEthereumEventProof,
  type EthereumEventLocator,
  type EthereumEventLocatorBlock,
  type EthereumEventProof,
  getLatestArgonFinalizedExecutionHeader,
  type ArgonFinalizedExecutionHeader,
  loadExecutionHeader,
} from './EthereumProof';
import {
  getExecutionClient,
  type EthereumExecutionClient,
  type EthereumExecutionSource,
  retryWhileExecutionRpcIndexing,
} from './EthereumExecution';

type EthereumStorageSlotProofPayload = {
  slot: Hex;
  value: Hex;
  nodeIndexes: number[];
};

type EthereumAccountStorageProofPayload = {
  anchorBlockHash: Hex;
  accountProof: Hex[];
  storageProof: Hex[];
  slots: EthereumStorageSlotProofPayload[];
};

type GatewayActivityRuntimeProof = {
  locatorIndex: bigint;
  storageProof: EthereumAccountStorageProofPayload;
  activityLogs: Array<{
    address: Hex;
    topics: Hex[];
    data: Hex;
  }>;
};

type EthereumGatewayActivityPayload<TProof> = {
  previousGatewayActivityNonce: bigint;
  proof: TProof;
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

export type EthereumGatewayActivityReceiptProofPayload =
  EthereumGatewayActivityPayload<EthereumEventProof>;

export type EthereumGatewayActivityStorageProofPayload =
  EthereumGatewayActivityPayload<GatewayActivityRuntimeProof>;

export type MissingGatewayActivityLocator = {
  locatorIndex: bigint;
  blockNumber: bigint;
  startGatewayActivityNonce: bigint;
  endGatewayActivityNonce: bigint;
  previousLocatorHash: Hex;
  activityRoot: Hex;
};

export type EthereumGatewayActivityProofBuildResult<TPayload> = {
  payloads: TPayload[];
  deferredLocators: MissingGatewayActivityLocator[];
};

export type EthereumGatewayActivityProofPayload =
  | EthereumGatewayActivityReceiptProofPayload
  | EthereumGatewayActivityStorageProofPayload;

type EthereumBlockLog = Awaited<ReturnType<EthereumExecutionClient['getLogs']>>[number];

type GatewayActivityLog = {
  activity: EthereumGatewayActivity;
  log: Pick<EthereumBlockLog, 'address' | 'topics' | 'data'>;
};

type GatewayActivityReceiptProofChunk = {
  activities: EthereumGatewayActivity[];
  locatorBlock: EthereumEventLocatorBlock;
};

type GatewayReceiptProofBounds = {
  activitiesPerReceiptProof: number;
  receiptProofsPerExtrinsic: number;
  sharedHeaderDepth: number;
};

type ArgonFinalizedExecutionHeaderPlan = {
  argonFinalizedExecutionHeader: ArgonFinalizedExecutionHeader;
  earliestTargetHeader: Awaited<ReturnType<typeof loadExecutionHeader>>;
  targetToArgonFinalizedHeaderChain: Awaited<ReturnType<typeof buildExecutionHeaderChain>>;
};

const ZERO_HASH: Hex = `0x${'00'.repeat(32)}`;
const STORAGE_PROOF_GATEWAY_ACTIVITY_SPEC_VERSION = 154;

/**
 * Returns `true` for runtimes that still accept the legacy receipt-proof gateway activity flow.
 *
 * App-side callers should branch on this once and then stick to the matching flow:
 * - default flow: discover locators, then call `buildGatewayActivityProof`
 * - explicit receipt/storage builders remain available for narrower call sites
 */
export function supportsGatewayActivityReceiptProofs(
  client: Pick<ArgonClient, 'runtimeVersion'>,
): boolean {
  return client.runtimeVersion.specVersion.toNumber() < STORAGE_PROOF_GATEWAY_ACTIVITY_SPEC_VERSION;
}

/**
 * Builds gateway activity proofs for the current runtime version.
 *
 * Receipt-proof runtimes ignore `finalizedExecutionHeader`. Storage-proof runtimes require it.
 */
export async function buildGatewayActivityProof(
  client: IArgonQueryable & Pick<ArgonClient, 'consts' | 'runtimeVersion'>,
  options: EthereumExecutionSource & {
    gatewayAddress: Hex;
    locators: MissingGatewayActivityLocator[];
    finalizedExecutionHeader?: ArgonFinalizedExecutionHeader;
  },
): Promise<EthereumGatewayActivityProofBuildResult<EthereumGatewayActivityProofPayload>> {
  if (supportsGatewayActivityReceiptProofs(client)) {
    return buildGatewayActivityReceiptProofPayloads(client, options);
  }

  const { finalizedExecutionHeader } = options;
  if (!finalizedExecutionHeader) {
    throw new Error(
      'Gateway activity storage proofs require a finalizedExecutionHeader; use discoverMissingGatewayActivityLocators and provide the execution header you want to relay against',
    );
  }

  return buildGatewayActivityStorageProofs(client, {
    ...options,
    finalizedExecutionHeader,
  });
}

/**
 * Builds receipt-proof payloads for the cached locators that can be relayed right now.
 */
export async function buildGatewayActivityReceiptProofPayloads(
  client: IArgonQueryable & Pick<ArgonClient, 'consts' | 'runtimeVersion'>,
  options: EthereumExecutionSource & {
    gatewayAddress: Hex;
    locators: MissingGatewayActivityLocator[];
  },
): Promise<EthereumGatewayActivityProofBuildResult<EthereumGatewayActivityReceiptProofPayload>> {
  if (!supportsGatewayActivityReceiptProofs(client)) {
    throw new Error(
      'Gateway activity receipt proofs are not supported by this runtime; use discoverMissingGatewayActivityLocators and buildGatewayActivityStorageProofs instead',
    );
  }

  const bounds = getGatewayReceiptProofBounds(client);
  const executionClient = getExecutionClient(options);
  const gatewayAddress = getAddress(options.gatewayAddress);
  const [previousGatewayActivityNonce, chainId] = await Promise.all([
    loadPreviousGatewayActivityNonce(client),
    executionClient.getChainId(),
  ]);
  const hashContext = {
    chainId: BigInt(chainId),
    gatewayAddress,
  };
  const relayableLocators = [...options.locators]
    .filter(locator => locator.endGatewayActivityNonce > previousGatewayActivityNonce)
    .sort((left, right) =>
      compareBigintsAscending(left.startGatewayActivityNonce, right.startGatewayActivityNonce),
    );
  const activities = await collectActivitiesFromLocators(
    executionClient,
    relayableLocators,
    previousGatewayActivityNonce + 1n,
    hashContext,
  );
  if (activities.length === 0) {
    return {
      payloads: [],
      deferredLocators: relayableLocators,
    };
  }

  const proofChunks = chunkGatewayActivitiesForReceiptProof(
    activities,
    bounds.activitiesPerReceiptProof,
  );
  const payloads: EthereumGatewayActivityReceiptProofPayload[] = [];
  let nextPreviousGatewayActivityNonce = previousGatewayActivityNonce;
  let remainingProofChunks = proofChunks;

  while (remainingProofChunks.length > 0) {
    const argonFinalizedExecutionHeaderPlan = await selectArgonFinalizedExecutionHeader(
      client,
      executionClient,
      remainingProofChunks[0],
      bounds.sharedHeaderDepth,
    );
    if (!argonFinalizedExecutionHeaderPlan) {
      break;
    }

    const acceptedProofChunks = await collectProofChunksForArgonFinalizedExecutionHeader(
      executionClient,
      remainingProofChunks,
      argonFinalizedExecutionHeaderPlan,
      bounds.receiptProofsPerExtrinsic,
    );
    if (acceptedProofChunks.length === 0) {
      break;
    }

    const acceptedActivities = acceptedProofChunks.flatMap(({ activities }) => activities);
    const proof = await buildEthereumEventProof(
      options,
      argonFinalizedExecutionHeaderPlan.argonFinalizedExecutionHeader,
      acceptedProofChunks.map(({ locatorBlock }) => locatorBlock),
    );

    payloads.push({
      previousGatewayActivityNonce: nextPreviousGatewayActivityNonce,
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
    });
    nextPreviousGatewayActivityNonce = acceptedActivities.at(-1)!.gatewayState.gatewayActivityNonce;
    remainingProofChunks = remainingProofChunks.slice(acceptedProofChunks.length);
  }

  return {
    payloads,
    deferredLocators: relayableLocators.filter(
      locator => locator.endGatewayActivityNonce > nextPreviousGatewayActivityNonce,
    ),
  };
}

/**
 * Builds storage-proof payloads for the cached locators that can be relayed right now.
 *
 * Callers are expected to collect locators separately. This step uses the current Argon gateway
 * nonce plus the supplied finalized execution header to determine which cached locators are
 * contiguous and anchorable now, and returns the rest as deferred locators.
 */
export async function buildGatewayActivityStorageProofs(
  client: IArgonQueryable & Pick<ArgonClient, 'consts' | 'runtimeVersion'>,
  options: EthereumExecutionSource & {
    finalizedExecutionHeader: ArgonFinalizedExecutionHeader;
    gatewayAddress: Hex;
    locators: MissingGatewayActivityLocator[];
  },
): Promise<EthereumGatewayActivityProofBuildResult<EthereumGatewayActivityStorageProofPayload>> {
  if (supportsGatewayActivityReceiptProofs(client)) {
    throw new Error(
      'Gateway activity storage proofs are not supported by this runtime; use buildGatewayActivityReceiptProofPayloads instead',
    );
  }

  const maxActivitiesPerGatewayProof =
    client.consts.crosschainTransfer.maxActivitiesPerGatewayProof.toNumber();
  if (maxActivitiesPerGatewayProof < 1) {
    throw new Error('Gateway proof requires maxActivitiesPerGatewayProof to be a positive integer');
  }

  const executionClient = getExecutionClient(options);
  const gatewayAddress = getAddress(options.gatewayAddress);
  const [previousGatewayActivityNonce, chainId, activityBlockLocatorsMappingSlot] =
    await Promise.all([
      loadPreviousGatewayActivityNonce(client),
      executionClient.getChainId(),
      executionClient.readContract({
        abi: mintingGatewayAbi,
        address: gatewayAddress,
        functionName: 'activityBlockLocatorsMappingSlot',
      }),
    ]);
  const hashContext = {
    chainId: BigInt(chainId),
    gatewayAddress,
  };
  const relayableLocators = [...options.locators]
    .filter(locator => locator.endGatewayActivityNonce > previousGatewayActivityNonce)
    .sort((left, right) =>
      compareBigintsAscending(left.startGatewayActivityNonce, right.startGatewayActivityNonce),
    );
  const payloads: EthereumGatewayActivityStorageProofPayload[] = [];
  let expectedPreviousGatewayActivityNonce = previousGatewayActivityNonce;

  for (const locator of relayableLocators) {
    if (locator.startGatewayActivityNonce !== expectedPreviousGatewayActivityNonce + 1n) {
      break;
    }

    if (options.finalizedExecutionHeader.blockNumber < locator.blockNumber) {
      break;
    }

    const nextGatewayActivityNonce = expectedPreviousGatewayActivityNonce + 1n;
    const activities = await readAndVerifyLocatorActivities(
      executionClient,
      locator,
      nextGatewayActivityNonce,
      hashContext,
    );
    if (activities.length > maxActivitiesPerGatewayProof) {
      throw new Error(`Gateway block ${locator.blockNumber} exceeds maxActivitiesPerGatewayProof`);
    }

    const proof = await buildLocatorStorageProof(
      executionClient,
      hashContext.gatewayAddress,
      activityBlockLocatorsMappingSlot,
      options.finalizedExecutionHeader,
      locator,
      activities,
    );

    payloads.push({
      previousGatewayActivityNonce: expectedPreviousGatewayActivityNonce,
      proof,
      gatewayActivityNonceRange: {
        start: locator.startGatewayActivityNonce,
        end: locator.endGatewayActivityNonce,
      },
      executionBlockNumberRange: {
        start: locator.blockNumber,
        end: locator.blockNumber,
      },
      activities: activities.map(({ activity }) => activity),
    });
    expectedPreviousGatewayActivityNonce = locator.endGatewayActivityNonce;
  }

  return {
    payloads,
    deferredLocators: relayableLocators.filter(
      locator => locator.endGatewayActivityNonce > expectedPreviousGatewayActivityNonce,
    ),
  };
}

/**
 * Discovers finalized locator blocks whose activity range reaches the caller's minimum gateway
 * activity nonce.
 *
 * This is the intended first step for both receipt-proof and storage-proof runtimes and can be
 * called repeatedly while a client continues collecting unproven locator blocks. When a caller is
 * already caching finalized locators, `afterLocatorIndex` lets it fetch only the newly finalized
 * suffix.
 */
export async function discoverMissingGatewayActivityLocators(
  options: EthereumExecutionSource & {
    afterLocatorIndex?: bigint;
    finalizedExecutionBlockNumber: bigint;
    gatewayAddress: Hex;
    minimumGatewayActivityNonce: bigint;
  },
): Promise<MissingGatewayActivityLocator[]> {
  const executionClient = getExecutionClient(options);
  const gatewayAddress = getAddress(options.gatewayAddress);
  const readLocatorRecord = async (locatorIndex: bigint) => {
    const [blockNumber, startGatewayActivityNonce, endGatewayActivityNonce, activityRoot] =
      await executionClient.readContract({
        abi: mintingGatewayAbi,
        address: gatewayAddress,
        blockNumber: options.finalizedExecutionBlockNumber,
        functionName: 'activityBlockLocators',
        args: [locatorIndex],
      });

    return {
      locatorIndex,
      blockNumber,
      startGatewayActivityNonce,
      endGatewayActivityNonce,
      activityRoot,
    };
  };
  const latestLocatorIndex = await executionClient.readContract({
    abi: mintingGatewayAbi,
    address: gatewayAddress,
    blockNumber: options.finalizedExecutionBlockNumber,
    functionName: 'latestActivityBlockLocatorIndex',
  });
  if (latestLocatorIndex === 0n) {
    return [];
  }

  if (options.afterLocatorIndex !== undefined) {
    if (options.afterLocatorIndex >= latestLocatorIndex) {
      return [];
    }

    const locators: MissingGatewayActivityLocator[] = [];
    const firstUncachedLocatorIndex = options.afterLocatorIndex + 1n;
    let previousLocator: Awaited<ReturnType<typeof readLocatorRecord>> | undefined;

    if (options.afterLocatorIndex > 0n) {
      previousLocator = await readLocatorRecord(options.afterLocatorIndex);
    }

    for (
      let locatorIndex = firstUncachedLocatorIndex;
      locatorIndex <= latestLocatorIndex;
      locatorIndex += 1n
    ) {
      const locatorRecord = await readLocatorRecord(locatorIndex);
      const locator = {
        ...locatorRecord,
        previousLocatorHash: previousLocator
          ? hashMintingGatewayActivityBlockLocator(previousLocator)
          : ZERO_HASH,
      };
      if (locator.endGatewayActivityNonce >= options.minimumGatewayActivityNonce) {
        locators.push(locator);
      }
      previousLocator = locatorRecord;
    }

    return locators;
  }

  const locators: MissingGatewayActivityLocator[] = [];
  let currentLocator = await readLocatorRecord(latestLocatorIndex);

  for (let locatorIndex = latestLocatorIndex; locatorIndex >= 1n; locatorIndex -= 1n) {
    if (currentLocator.endGatewayActivityNonce < options.minimumGatewayActivityNonce) {
      break;
    }

    let previousLocator: Awaited<ReturnType<typeof readLocatorRecord>> | undefined;

    if (locatorIndex > 1n) {
      previousLocator = await readLocatorRecord(locatorIndex - 1n);
    }

    locators.push({
      ...currentLocator,
      previousLocatorHash: previousLocator
        ? hashMintingGatewayActivityBlockLocator(previousLocator)
        : ZERO_HASH,
    });

    if (!previousLocator) {
      break;
    }
    currentLocator = previousLocator;
  }

  locators.reverse();
  return locators;
}

async function collectActivitiesFromLocators(
  executionClient: EthereumExecutionClient,
  locators: MissingGatewayActivityLocator[],
  minimumGatewayActivityNonce: bigint,
  hashContext: MintingGatewayHashContext,
): Promise<EthereumGatewayActivity[]> {
  const activities: EthereumGatewayActivity[] = [];
  let expectedGatewayActivityNonce = minimumGatewayActivityNonce;

  for (const locator of locators) {
    if (expectedGatewayActivityNonce < locator.startGatewayActivityNonce) {
      break;
    }
    if (expectedGatewayActivityNonce > locator.endGatewayActivityNonce) {
      continue;
    }

    const blockActivities = await readAndVerifyLocatorActivities(
      executionClient,
      locator,
      expectedGatewayActivityNonce,
      hashContext,
    );
    if (blockActivities.length === 0) {
      continue;
    }

    activities.push(...blockActivities.map(({ activity }) => activity));
    expectedGatewayActivityNonce = locator.endGatewayActivityNonce + 1n;
  }

  return activities;
}

function compareBigintsAscending(left: bigint, right: bigint): number {
  if (left === right) {
    return 0;
  }

  return left < right ? -1 : 1;
}

function chunkGatewayActivitiesForReceiptProof(
  activities: EthereumGatewayActivity[],
  maxActivitiesPerReceiptProof: number,
): GatewayActivityReceiptProofChunk[] {
  const chunks: GatewayActivityReceiptProofChunk[] = [];
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

function getGatewayReceiptProofBounds(
  client: Pick<ArgonClient, 'consts'>,
): GatewayReceiptProofBounds {
  const crosschainTransferConsts = client.consts.crosschainTransfer as unknown as {
    maxActivitiesPerReceiptProof?: { toNumber(): number };
    maxReceiptProofsPerExtrinsic?: { toNumber(): number };
  };
  const rawActivitiesPerReceiptProof =
    crosschainTransferConsts.maxActivitiesPerReceiptProof?.toNumber();
  if (
    rawActivitiesPerReceiptProof === undefined ||
    !Number.isInteger(rawActivitiesPerReceiptProof) ||
    rawActivitiesPerReceiptProof < 1
  ) {
    throw new Error(
      'Gateway receipt proof requires maxActivitiesPerReceiptProof to be a positive integer',
    );
  }
  const activitiesPerReceiptProof = rawActivitiesPerReceiptProof;

  const rawReceiptProofsPerExtrinsic =
    crosschainTransferConsts.maxReceiptProofsPerExtrinsic?.toNumber();
  if (
    rawReceiptProofsPerExtrinsic === undefined ||
    !Number.isInteger(rawReceiptProofsPerExtrinsic) ||
    rawReceiptProofsPerExtrinsic < 1
  ) {
    throw new Error(
      'Gateway receipt proof requires maxReceiptProofsPerExtrinsic to be a positive integer',
    );
  }
  const receiptProofsPerExtrinsic = rawReceiptProofsPerExtrinsic;

  return {
    activitiesPerReceiptProof,
    receiptProofsPerExtrinsic,
    sharedHeaderDepth: client.consts.crosschainTransfer.maxProofExecutionHeaderDepth.toNumber(),
  };
}

async function selectArgonFinalizedExecutionHeader(
  client: IArgonQueryable,
  executionClient: EthereumExecutionClient,
  earliestProofChunk: GatewayActivityReceiptProofChunk,
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

  const scannedArgonFinalizedExecutionHeader =
    await loadRetainedExecutionHeaderAnchorAtOrAfterBlock(client, earliestBlockNumber);
  if (
    scannedArgonFinalizedExecutionHeader &&
    scannedArgonFinalizedExecutionHeader.blockNumber <= maxArgonFinalizedExecutionHeaderBlockNumber
  ) {
    return {
      argonFinalizedExecutionHeader: scannedArgonFinalizedExecutionHeader,
      earliestTargetHeader,
      targetToArgonFinalizedHeaderChain: await buildExecutionHeaderChain(
        executionClient,
        earliestTargetHeader,
        scannedArgonFinalizedExecutionHeader,
      ),
    };
  }

  throw new Error(
    'Oldest uncovered gateway activity exceeds the Argon finalized execution-header window',
  );
}

async function collectProofChunksForArgonFinalizedExecutionHeader(
  executionClient: EthereumExecutionClient,
  proofChunks: GatewayActivityReceiptProofChunk[],
  plan: ArgonFinalizedExecutionHeaderPlan,
  receiptProofsPerExtrinsic: number,
): Promise<GatewayActivityReceiptProofChunk[]> {
  const acceptedProofChunks: GatewayActivityReceiptProofChunk[] = [];
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

async function buildLocatorStorageProof(
  executionClient: EthereumExecutionClient,
  gatewayAddress: Hex,
  activityBlockLocatorsMappingSlot: bigint,
  argonFinalizedExecutionHeader: ArgonFinalizedExecutionHeader,
  locator: MissingGatewayActivityLocator,
  activities: GatewayActivityLog[],
): Promise<GatewayActivityRuntimeProof> {
  const baseSlot = BigInt(
    keccak256(
      concatHex([
        toHex(locator.locatorIndex, { size: 32 }),
        toHex(activityBlockLocatorsMappingSlot, { size: 32 }),
      ]),
    ),
  );
  const [rangeSlot, rootSlot] = [toHex(baseSlot, { size: 32 }), toHex(baseSlot + 1n, { size: 32 })];
  const expectedRangeValue = toHex(
    (locator.endGatewayActivityNonce << 128n) |
      (locator.startGatewayActivityNonce << 64n) |
      locator.blockNumber,
    { size: 32 },
  );
  const requestedSlots = [rangeSlot, rootSlot] as const;
  const accountStorageProof = await retryWhileExecutionRpcIndexing(() =>
    executionClient.getProof({
      address: gatewayAddress,
      blockNumber: argonFinalizedExecutionHeader.blockNumber,
      storageKeys: [...requestedSlots],
    }),
  );
  const sharedStorageProofNodes: Hex[] = [];
  const sharedNodeIndexes = new Map<string, number>();
  const storageSlotProofs = requestedSlots.map((slot, index) => {
    const slotProof = accountStorageProof.storageProof.find(
      candidate => candidate.key.toLowerCase() === slot.toLowerCase(),
    );
    if (!slotProof) {
      throw new Error(`eth_getProof did not return storage slot ${slot}`);
    }

    const nodeIndexes = slotProof.proof.map(node => {
      const key = node.toLowerCase();
      const existingIndex = sharedNodeIndexes.get(key);
      if (existingIndex !== undefined) {
        return existingIndex;
      }

      const nextIndex = sharedStorageProofNodes.length;
      sharedStorageProofNodes.push(node);
      sharedNodeIndexes.set(key, nextIndex);
      return nextIndex;
    });
    const value = toHex(slotProof.value, { size: 32 });
    const expectedValue = index === 0 ? expectedRangeValue : locator.activityRoot;

    if (value.toLowerCase() !== expectedValue.toLowerCase()) {
      throw new Error(`eth_getProof returned an unexpected value for storage slot ${slot}`);
    }

    return {
      slot,
      value,
      nodeIndexes,
    };
  });

  return {
    locatorIndex: locator.locatorIndex,
    storageProof: {
      anchorBlockHash: argonFinalizedExecutionHeader.blockHash,
      accountProof: accountStorageProof.accountProof,
      storageProof: sharedStorageProofNodes,
      slots: storageSlotProofs,
    },
    activityLogs: activities.map(({ log }) => ({
      address: log.address,
      topics: log.topics,
      data: log.data,
    })),
  };
}

async function readAndVerifyLocatorActivities(
  executionClient: EthereumExecutionClient,
  locator: MissingGatewayActivityLocator,
  minimumGatewayActivityNonce: bigint,
  hashContext: MintingGatewayHashContext,
): Promise<GatewayActivityLog[]> {
  const blockLogs = await retryWhileExecutionRpcIndexing(() =>
    executionClient.getLogs({
      address: hashContext.gatewayAddress,
      fromBlock: locator.blockNumber,
      toBlock: locator.blockNumber,
    }),
  );
  const activities = blockLogs
    .flatMap(log => {
      try {
        if (
          !log.transactionHash ||
          log.transactionIndex == null ||
          log.logIndex == null ||
          !log.blockHash ||
          log.blockNumber == null
        ) {
          throw new Error('Execution log is missing indexed receipt metadata');
        }

        return [
          {
            activity: {
              txHash: log.transactionHash,
              transactionIndex: Number(log.transactionIndex),
              logIndex: Number(log.logIndex),
              blockHash: log.blockHash,
              blockNumber: log.blockNumber,
              ...decodeEthereumGatewayActivityLog(log),
            },
            log: {
              address: log.address,
              topics: log.topics,
              data: log.data,
            },
          },
        ];
      } catch {
        return [];
      }
    })
    .sort((left, right) =>
      compareBigintsAscending(
        left.activity.gatewayState.gatewayActivityNonce,
        right.activity.gatewayState.gatewayActivityNonce,
      ),
    );
  const locatorActivities = activities.filter(
    ({ activity }) =>
      activity.gatewayState.gatewayActivityNonce >= locator.startGatewayActivityNonce &&
      activity.gatewayState.gatewayActivityNonce <= locator.endGatewayActivityNonce,
  );
  if (
    locatorActivities.length === 0 ||
    locatorActivities[0]?.activity.gatewayState.gatewayActivityNonce !==
      locator.startGatewayActivityNonce
  ) {
    throw new Error(`Gateway block ${locator.blockNumber} contains uncovered gateway activity`);
  }

  let previousNonce = locator.startGatewayActivityNonce - 1n;
  let activityRoot = locator.previousLocatorHash;

  for (const { activity } of locatorActivities) {
    if (activity.gatewayState.gatewayActivityNonce !== previousNonce + 1n) {
      throw new Error(
        `Gateway block ${locator.blockNumber} contains a gap in gateway activity coverage`,
      );
    }

    previousNonce = activity.gatewayState.gatewayActivityNonce;
    activityRoot = appendEthereumGatewayActivityRoot(
      activityRoot,
      hashEthereumGatewayActivity(hashContext, activity),
    );
  }

  if (previousNonce !== locator.endGatewayActivityNonce) {
    throw new Error(`Gateway block ${locator.blockNumber} contains uncovered gateway activity`);
  }
  if (activityRoot.toLowerCase() !== locator.activityRoot.toLowerCase()) {
    throw new Error(`Gateway block ${locator.blockNumber} activity root did not match storage`);
  }

  return locatorActivities.filter(
    ({ activity }) => activity.gatewayState.gatewayActivityNonce >= minimumGatewayActivityNonce,
  );
}

async function loadPreviousGatewayActivityNonce(client: IArgonQueryable): Promise<bigint> {
  const currentGatewayState =
    await client.query.crosschainTransfer.gatewayStateBySourceChain('Ethereum');

  return currentGatewayState.isSome
    ? currentGatewayState.unwrap().gatewayActivityNonce.toBigInt()
    : 0n;
}

export async function loadRetainedExecutionHeaderAnchorAtOrAfterBlock(
  client: IArgonQueryable,
  earliestBlockNumber: bigint,
): Promise<ArgonFinalizedExecutionHeader | null> {
  // The runtime stores this index under `block_number.to_be_bytes()`, so a fixed-width hex key
  // preserves the same big-endian ordering here. We first try the exact block number, then seek
  // to the first later retained key when that exact anchor is not present.
  const scanBlockNumberKey = toHex(earliestBlockNumber, { size: 8 });
  const executionHeaderAnchorsByBlockNumber =
    client.query.ethereumVerifier.executionHeaderAnchorsByBlockNumber;
  const exactAnchor = await executionHeaderAnchorsByBlockNumber(scanBlockNumberKey);
  const matchingEntry = exactAnchor.isSome
    ? exactAnchor
    : (
        await executionHeaderAnchorsByBlockNumber.entriesPaged({
          args: [],
          pageSize: 1,
          startKey: executionHeaderAnchorsByBlockNumber.key(scanBlockNumberKey),
        })
      )[0]?.[1];
  const scannedAnchor = matchingEntry?.isSome ? matchingEntry.unwrap() : null;

  if (!scannedAnchor) {
    return null;
  }

  return {
    blockHash: scannedAnchor.blockHash.toHex(),
    blockNumber: scannedAnchor.blockNumber.toBigInt(),
  };
}
