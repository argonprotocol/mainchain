import {
  hashMintingGatewayActivityBlockLocator,
  mintingGatewayAbi,
  type MintingGatewayHashContext,
} from './EvmContracts';
import type { ArgonClient, IArgonQueryable } from './index';
import { createMerkleProof, createMPTFromProof } from '@ethereumjs/mpt';
import {
  bytesToHex,
  concatHex,
  fromRlp,
  getAddress,
  hexToBytes,
  keccak256,
  size,
  toHex,
  type GetProofReturnType,
  type Hex,
} from 'viem';
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
  decodeExecutionHeaderRlp,
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
  executionBlockProof: EthereumEventProof['executionBlockProof'];
  accountProof: Hex[];
  storageProof: Hex[];
  slots: EthereumStorageSlotProofPayload[];
};

type GatewayActivityStorageProof = {
  activityRootSeed: Hex;
  locatorIndex: bigint;
  storageProof: EthereumAccountStorageProofPayload;
  activityLogs: GatewayActivityLog['log'][];
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
  EthereumGatewayActivityPayload<GatewayActivityStorageProof>;

export type MissingGatewayActivityLocator = {
  locatorIndex: bigint;
  blockNumber: bigint;
  startGatewayActivityNonce: bigint;
  endGatewayActivityNonce: bigint;
  previousLocatorHash: Hex;
  activityRoot: Hex;
};

/** App-owned capture of one complete locator at a particular Ethereum execution block. */
export type CapturedGatewayActivityStorageProof = MissingGatewayActivityLocator & {
  executionHeaderRlp: Hex;
  proof: GatewayActivityStorageProof;
  activities: EthereumGatewayActivity[];
};

export type EthereumGatewayActivityProofBuildResult<
  TPayload,
  TLocator extends MissingGatewayActivityLocator = MissingGatewayActivityLocator,
> = {
  payloads: TPayload[];
  deferredLocators: TLocator[];
  /** Captures that cannot reach a retained anchor; capture these locators again at a newer block. */
  recaptureLocators: TLocator[];
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

/**
 * Receipt and storage proof calls may coexist; detect each capability independently.
 */
export function supportsGatewayActivityReceiptProofs(client: Pick<ArgonClient, 'tx'>): boolean {
  return typeof client.tx.crosschainTransfer?.proveGatewayActivity === 'function';
}

export function supportsGatewayActivityStorageProofs(client: Pick<ArgonClient, 'tx'>): boolean {
  return typeof client.tx.crosschainTransfer?.proveGatewayActivityStorage === 'function';
}

/**
 * Builds gateway activity proofs using the available runtime calls, preferring storage proofs.
 *
 * Pass discovered locators for receipts, or records returned by
 * `captureGatewayActivityStorageProofs` for storage. This function never captures storage witnesses.
 */
export async function buildGatewayActivityProof<TLocator extends MissingGatewayActivityLocator>(
  client: IArgonQueryable & Pick<ArgonClient, 'consts' | 'tx'>,
  options: EthereumExecutionSource & {
    gatewayAddress: Hex;
    locators: TLocator[];
  },
): Promise<EthereumGatewayActivityProofBuildResult<EthereumGatewayActivityProofPayload, TLocator>> {
  if (!supportsGatewayActivityStorageProofs(client)) {
    if (supportsGatewayActivityReceiptProofs(client)) {
      return buildGatewayActivityReceiptProofPayloads(client, options);
    }

    throw new Error('Gateway activity proofs are not supported by this runtime');
  }

  const captures = options.locators.filter(
    (locator): locator is TLocator & CapturedGatewayActivityStorageProof => 'proof' in locator,
  );
  if (captures.length !== options.locators.length) {
    throw new Error(
      'Gateway activity storage proofs require captured locators; call captureGatewayActivityStorageProofs first',
    );
  }

  return buildGatewayActivityStorageProofs(client, {
    ...options,
    locators: captures,
  });
}

/**
 * Builds receipt-proof payloads for the cached locators that can be relayed right now.
 */
export async function buildGatewayActivityReceiptProofPayloads<
  TLocator extends MissingGatewayActivityLocator,
>(
  client: IArgonQueryable & Pick<ArgonClient, 'consts' | 'tx'>,
  options: EthereumExecutionSource & {
    gatewayAddress: Hex;
    locators: TLocator[];
  },
): Promise<
  EthereumGatewayActivityProofBuildResult<EthereumGatewayActivityReceiptProofPayload, TLocator>
> {
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
      recaptureLocators: [],
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
    let headerPlan: ArgonFinalizedExecutionHeaderPlan | null;
    try {
      headerPlan = await selectArgonFinalizedExecutionHeader(
        client,
        executionClient,
        remainingProofChunks[0],
        bounds.sharedHeaderDepth,
      );
    } catch (error) {
      if (error instanceof ArgonFinalizedExecutionHeaderPathError && payloads.length > 0) {
        break;
      }
      throw error;
    }
    if (!headerPlan) {
      break;
    }

    const acceptedProofChunks = await collectProofChunksForArgonFinalizedExecutionHeader(
      executionClient,
      remainingProofChunks,
      headerPlan,
      bounds.receiptProofsPerExtrinsic,
    );
    if (acceptedProofChunks.length === 0) {
      break;
    }

    const acceptedActivities = acceptedProofChunks.flatMap(({ activities }) => activities);
    const proof = await buildEthereumEventProof(
      options,
      headerPlan.argonFinalizedExecutionHeader,
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
    recaptureLocators: [],
  };
}

/**
 * Captures storage witnesses while the execution RPC still has recent state, before Argon anchoring.
 *
 * The app chooses an observed (not pending) execution block and owns persistence and recapture.
 * A single eth_getProof request captures all selected locator slots before loading activity logs.
 * Witnesses are checked against the captured header so a reorg or inconsistent RPC cannot poison
 * the cache with state from another block at the same height.
 */
export async function captureGatewayActivityStorageProofs(
  options: EthereumExecutionSource & {
    executionBlock: Pick<
      Awaited<ReturnType<EthereumExecutionClient['getBlock']>>,
      'hash' | 'number'
    >;
    gatewayAddress: Hex;
    locators: MissingGatewayActivityLocator[];
  },
): Promise<CapturedGatewayActivityStorageProof[]> {
  if (options.locators.length === 0) return [];
  const { hash: blockHash, number: blockNumber } = options.executionBlock;
  if (blockHash === null || blockNumber === null) {
    throw new Error('Storage proof capture requires a mined execution block');
  }
  for (const locator of options.locators) {
    if (locator.blockNumber > blockNumber) {
      throw new Error(`Gateway block ${locator.blockNumber} is newer than the capture block`);
    }
  }

  const executionClient = getExecutionClient(options);
  const gatewayAddress = getAddress(options.gatewayAddress);
  const [chainId, mappingSlot] = await Promise.all([
    executionClient.getChainId(),
    executionClient.readContract({
      abi: mintingGatewayAbi,
      address: gatewayAddress,
      blockNumber,
      functionName: 'activityBlockLocatorsMappingSlot',
    }),
  ]);
  const locatorSlots = options.locators.map(locator => {
    const base = BigInt(
      keccak256(
        concatHex([toHex(locator.locatorIndex, { size: 32 }), toHex(mappingSlot, { size: 32 })]),
      ),
    );
    return [toHex(base, { size: 32 }), toHex(base + 1n, { size: 32 })];
  });
  const witness = await retryWhileExecutionRpcIndexing(() =>
    executionClient.getProof({
      address: gatewayAddress,
      blockNumber,
      storageKeys: [...new Set(locatorSlots.flat())],
    }),
  );
  const captureHeader = await loadExecutionHeader(executionClient, blockHash);
  if (captureHeader.number !== blockNumber) {
    throw new Error('Storage proof capture block number does not match its header');
  }

  const accountTrie = await createMPTFromProof(
    witness.accountProof.map(node => hexToBytes(node)),
    { root: captureHeader.stateRoot, useKeyHashing: true },
  );
  const accountKey = hexToBytes(gatewayAddress);
  const account = await accountTrie.get(accountKey, true);
  const accountFields = account && fromRlp(bytesToHex(account), 'hex');
  if (
    !Array.isArray(accountFields) ||
    accountFields.length !== 4 ||
    typeof accountFields[2] !== 'string'
  ) {
    throw new Error('eth_getProof did not prove the gateway account at the capture block');
  }
  const storageRoot = hexToBytes(accountFields[2] as Hex);
  // Rebuild the ordered paths: the runtime rejects redundant nodes accepted by the trie library.
  const accountProof = (await createMerkleProof(accountTrie, accountKey)).map(node =>
    bytesToHex(node),
  );
  const slotProofsByKey = new Map<string, GetProofReturnType['storageProof'][number]>();
  for (const slotProof of witness.storageProof) {
    // RPCs may return quantity-style keys without the requested 32-byte padding.
    const key = toHex(BigInt(slotProof.key), { size: 32 }).toLowerCase();
    if (!slotProofsByKey.has(key)) slotProofsByKey.set(key, slotProof);
  }

  const executionHeaderRlp = bytesToHex(captureHeader.serialize());
  const hashContext = { chainId: BigInt(chainId), gatewayAddress };
  const captures: CapturedGatewayActivityStorageProof[] = [];
  for (const [index, locator] of options.locators.entries()) {
    const storageProof = await verifyLocatorStorageProof(
      locator,
      locatorSlots[index],
      storageRoot,
      slotProofsByKey,
    );
    const activities = await readAndVerifyLocatorActivities(executionClient, locator, hashContext);
    captures.push({
      ...locator,
      executionHeaderRlp,
      activities: activities.map(({ activity }) => activity),
      proof: {
        locatorIndex: locator.locatorIndex,
        activityRootSeed: locator.previousLocatorHash,
        activityLogs: activities.map(({ log }) => log),
        storageProof: {
          executionBlockProof: { anchorBlockHash: blockHash, targetToAnchorHeaderChain: [] },
          accountProof,
          ...storageProof,
        },
      },
    });
  }
  return captures;
}

/**
 * Attaches retained-anchor ancestry to app-cached captures, without refetching witnesses or logs.
 * Missing anchors/gaps are deferred; orphaned or over-distance captures must be captured again.
 * Capacity violations are errors, not deferred work: a newer anchor cannot make them fit.
 */
export async function buildGatewayActivityStorageProofs<
  TLocator extends CapturedGatewayActivityStorageProof,
>(
  client: IArgonQueryable & Pick<ArgonClient, 'consts' | 'tx'>,
  options: EthereumExecutionSource & {
    gatewayAddress: Hex;
    locators: TLocator[];
  },
): Promise<
  EthereumGatewayActivityProofBuildResult<EthereumGatewayActivityStorageProofPayload, TLocator>
> {
  if (!supportsGatewayActivityStorageProofs(client)) {
    throw new Error(
      'Gateway activity storage proofs are not supported by this runtime; use buildGatewayActivityReceiptProofPayloads instead',
    );
  }

  const maxActivitiesPerGatewayProof =
    client.consts.crosschainTransfer.maxActivitiesPerGatewayProof.toNumber();
  if (maxActivitiesPerGatewayProof < 1) {
    throw new Error('Gateway proof requires maxActivitiesPerGatewayProof to be a positive integer');
  }
  const maxGatewayActivityLogDataBytes =
    client.consts.crosschainTransfer.maxGatewayActivityLogDataBytes.toNumber();

  const executionClient = getExecutionClient(options);
  const gatewayAddress = getAddress(options.gatewayAddress);
  const previousGatewayActivityNonce = await loadPreviousGatewayActivityNonce(client);
  const maximumDepth = BigInt(
    client.consts.crosschainTransfer.maxProofExecutionHeaderDepth.toNumber(),
  );
  const pendingCaptures = [...options.locators]
    .filter(capture => capture.endGatewayActivityNonce > previousGatewayActivityNonce)
    .sort((left, right) =>
      compareBigintsAscending(left.startGatewayActivityNonce, right.startGatewayActivityNonce),
    );
  const payloads: EthereumGatewayActivityStorageProofPayload[] = [];
  const recaptureLocators: TLocator[] = [];
  // Several locators captured at one block share the same retained anchor and ancestry path.
  const ancestryByCaptureHeader = new Map<Hex, EthereumEventProof['executionBlockProof']>();
  let expectedPreviousGatewayActivityNonce = previousGatewayActivityNonce;

  for (const capture of pendingCaptures) {
    const nextGatewayActivityNonce = expectedPreviousGatewayActivityNonce + 1n;
    if (nextGatewayActivityNonce < capture.startGatewayActivityNonce) {
      break;
    }
    if (nextGatewayActivityNonce > capture.endGatewayActivityNonce) {
      continue;
    }

    const { proof } = capture;
    // These compatibility checks should never trigger with the capped contract and supported runtime.
    // Fail early on mismatched limits rather than submit a proof that cannot be accepted.
    if (proof.activityLogs.length > maxActivitiesPerGatewayProof) {
      throw new Error(`Gateway block ${capture.blockNumber} exceeds maxActivitiesPerGatewayProof`);
    }
    const activityLogDataBytes = proof.activityLogs.reduce(
      (total, log) => total + size(log.data),
      0,
    );
    if (activityLogDataBytes > maxGatewayActivityLogDataBytes) {
      throw new Error(
        `Gateway block ${capture.blockNumber} activity log data (${activityLogDataBytes} bytes) exceeds maxGatewayActivityLogDataBytes (${maxGatewayActivityLogDataBytes} bytes)`,
      );
    }

    if (
      proof.activityLogs.some(log => log.address.toLowerCase() !== gatewayAddress.toLowerCase())
    ) {
      throw new Error('Captured storage proof belongs to a different gateway');
    }
    let executionBlockProof = ancestryByCaptureHeader.get(capture.executionHeaderRlp);
    if (!executionBlockProof) {
      const captureHeader = decodeExecutionHeaderRlp(hexToBytes(capture.executionHeaderRlp));
      const relayAnchor = await loadRetainedExecutionHeaderAnchorAtOrAfterBlock(
        client,
        captureHeader.number,
      );
      if (!relayAnchor) break;
      if (relayAnchor.blockNumber - captureHeader.number > maximumDepth) {
        recaptureLocators.push(capture);
        break;
      }
      try {
        const chain = await buildExecutionHeaderChain(executionClient, captureHeader, relayAnchor);
        executionBlockProof = {
          anchorBlockHash: relayAnchor.blockHash,
          targetToAnchorHeaderChain: chain.map(({ rlp }) => ({ rlp })),
        };
      } catch (error) {
        if (!(error instanceof ArgonFinalizedExecutionHeaderPathError)) throw error;
        recaptureLocators.push(capture);
        break;
      }
      ancestryByCaptureHeader.set(capture.executionHeaderRlp, executionBlockProof);
    }

    payloads.push({
      previousGatewayActivityNonce: expectedPreviousGatewayActivityNonce,
      proof: {
        ...proof,
        storageProof: { ...proof.storageProof, executionBlockProof },
      },
      gatewayActivityNonceRange: {
        start: nextGatewayActivityNonce,
        end: capture.endGatewayActivityNonce,
      },
      executionBlockNumberRange: {
        start: capture.blockNumber,
        end: capture.blockNumber,
      },
      activities: capture.activities.filter(
        activity => activity.gatewayState.gatewayActivityNonce >= nextGatewayActivityNonce,
      ),
    });
    expectedPreviousGatewayActivityNonce = capture.endGatewayActivityNonce;
  }

  return {
    payloads,
    deferredLocators: pendingCaptures.filter(
      capture =>
        capture.endGatewayActivityNonce > expectedPreviousGatewayActivityNonce &&
        !recaptureLocators.includes(capture),
    ),
    recaptureLocators,
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

async function verifyLocatorStorageProof(
  locator: MissingGatewayActivityLocator,
  slotKeys: Hex[],
  storageRoot: Uint8Array,
  slotProofsByKey: ReadonlyMap<string, GetProofReturnType['storageProof'][number]>,
): Promise<Pick<EthereumAccountStorageProofPayload, 'storageProof' | 'slots'>> {
  const expectedRange =
    (locator.endGatewayActivityNonce << 128n) |
    (locator.startGatewayActivityNonce << 64n) |
    locator.blockNumber;
  const nodes: Hex[] = [];
  const nodeIndexes = new Map<string, number>();
  const slots: EthereumStorageSlotProofPayload[] = [];

  for (const [index, slot] of slotKeys.entries()) {
    const slotProof = slotProofsByKey.get(slot.toLowerCase());
    if (!slotProof) throw new Error(`eth_getProof did not return storage slot ${slot}`);
    const expectedValue = index === 0 ? expectedRange : BigInt(locator.activityRoot);
    if (slotProof.value !== expectedValue) {
      throw new Error(`eth_getProof returned an unexpected value for storage slot ${slot}`);
    }

    const slotTrie = await createMPTFromProof(
      slotProof.proof.map(node => hexToBytes(node)),
      { root: storageRoot, useKeyHashing: true },
    );
    const slotKey = hexToBytes(slot);
    const value = await slotTrie.get(slotKey, true);
    const encoded = value && fromRlp(bytesToHex(value), 'hex');
    if (encoded !== null && typeof encoded !== 'string') {
      throw new Error(`eth_getProof returned an invalid storage value for ${slot}`);
    }
    const actualValue = typeof encoded === 'string' && encoded !== '0x' ? BigInt(encoded) : 0n;
    if (actualValue !== expectedValue) {
      throw new Error(`eth_getProof did not prove the expected value for storage slot ${slot}`);
    }

    const path = await createMerkleProof(slotTrie, slotKey);
    const indexes = path.map(bytes => {
      const node = bytesToHex(bytes);
      const key = node.toLowerCase();
      let nodeIndex = nodeIndexes.get(key);
      if (nodeIndex === undefined) {
        nodeIndex = nodes.length;
        nodeIndexes.set(key, nodeIndex);
        nodes.push(node);
      }
      return nodeIndex;
    });
    slots.push({ slot, value: toHex(expectedValue, { size: 32 }), nodeIndexes: indexes });
  }

  return { storageProof: nodes, slots };
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
      hashContext,
    );

    activities.push(
      ...blockActivities
        .filter(
          ({ activity }) =>
            activity.gatewayState.gatewayActivityNonce >= expectedGatewayActivityNonce,
        )
        .map(({ activity }) => activity),
    );
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
  const crosschainTransferConsts = client.consts.crosschainTransfer;
  const activitiesPerReceiptProof =
    crosschainTransferConsts.maxActivitiesPerReceiptProof?.toNumber();
  if (!activitiesPerReceiptProof || activitiesPerReceiptProof < 1) {
    throw new Error(
      'Gateway receipt proof requires maxActivitiesPerReceiptProof to be a positive integer',
    );
  }

  const receiptProofsPerExtrinsic =
    crosschainTransferConsts.maxReceiptProofsPerExtrinsic?.toNumber();
  if (!receiptProofsPerExtrinsic || receiptProofsPerExtrinsic < 1) {
    throw new Error(
      'Gateway receipt proof requires maxReceiptProofsPerExtrinsic to be a positive integer',
    );
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

  return null;
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

async function readAndVerifyLocatorActivities(
  executionClient: EthereumExecutionClient,
  locator: MissingGatewayActivityLocator,
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

  return locatorActivities;
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
