import { BlockHeader, createBlockHeaderFromRPC, type JSONRPCBlock } from '@ethereumjs/block';
import { createMerkleProof, createMPT, verifyMPTWithMerkleProof } from '@ethereumjs/mpt';
import type { IArgonQueryable } from './index';
import { bytesToHex, type Hex, hexToBytes, toHex, toRlp } from 'viem';
import {
  type EthereumExecutionClient,
  type EthereumExecutionSource,
  type EthereumReceipt,
  getExecutionClient,
  retryWhileExecutionRpcIndexing,
} from './EthereumExecution';

type EthGetBlockByHashRpc = {
  Method: 'eth_getBlockByHash';
  Parameters: [Hex, true];
  ReturnType: JSONRPCBlock;
};
type EthGetBlockByNumberRpc = {
  Method: 'eth_getBlockByNumber';
  Parameters: [Hex, true];
  ReturnType: JSONRPCBlock;
};

export type ArgonFinalizedExecutionHeader = {
  blockHash: Hex;
  blockNumber: bigint;
};

export class ArgonFinalizedExecutionHeaderPathError extends Error {}

export type EthereumEventLocator = {
  txHash: Hex;
  logIndexes?: number[];
  receipt?: EthereumReceipt;
};

export type EthereumEventLocatorBlock = EthereumEventLocator[];

export type EthereumEventLog = Pick<EthereumReceipt['logs'][number], 'address' | 'data'> & {
  topics: Hex[];
};

type LoadedEthereumEventLocator = {
  receipt: EthereumReceipt;
  requestedLogIndexes: number[];
};

export type EthereumEventProof = {
  executionBlockProof: {
    anchorBlockHash: Hex;
    targetToAnchorHeaderChain: { rlp: Hex }[];
  };
  blocks: {
    targetBlockNumber: number;
    receiptLogs: {
      transactionIndex: number;
      eventLog: EthereumEventLog;
    }[];
    receiptProof: {
      nodes: Hex[];
      receipts: {
        transactionIndex: number;
        nodeIndexes: number[];
      }[];
    };
  }[];
};

type EthereumEventProofBlock = EthereumEventProof['blocks'][number];
type EthereumReceiptProof = EthereumEventProofBlock['receiptProof'];
type EthereumReceiptLog = EthereumEventProofBlock['receiptLogs'][number];
type EthereumCombinedReceiptProofReceipt = EthereumReceiptProof['receipts'][number];

export function encodeReceiptTrieKey(transactionIndex: number): Uint8Array {
  return transactionIndex === 0 ? Uint8Array.from([0x80]) : toRlp(toHex(transactionIndex), 'bytes');
}

export function encodeEthereumReceiptForProof(receipt: EthereumReceipt): Uint8Array {
  const payload = toRlp(
    [
      receipt.root ?? (receipt.status === 'success' ? '0x1' : '0x0'),
      toHex(receipt.cumulativeGasUsed),
      receipt.logsBloom,
      receipt.logs.map(log => [log.address, [...log.topics], log.data]),
    ],
    'bytes',
  );

  switch (receipt.type) {
    case 'legacy':
      return payload;
    case 'eip2930':
      return Uint8Array.from([...hexToBytes('0x01'), ...payload]);
    case 'eip1559':
      return Uint8Array.from([...hexToBytes('0x02'), ...payload]);
    case 'eip4844':
      return Uint8Array.from([...hexToBytes('0x03'), ...payload]);
    case 'eip7702':
      return Uint8Array.from([...hexToBytes('0x04'), ...payload]);
    default:
      throw new Error(`Unsupported Ethereum receipt type ${String(receipt.type)}`);
  }
}

export async function buildEthereumEventProof(
  executionSource: EthereumExecutionSource,
  argonFinalizedExecutionHeader: ArgonFinalizedExecutionHeader,
  locatorBlocks: EthereumEventLocatorBlock[],
): Promise<EthereumEventProof> {
  if (locatorBlocks.length === 0) {
    throw new Error('At least one Ethereum event locator is required');
  }

  const executionClient = getExecutionClient(executionSource);
  const blocksWithHeaders = await Promise.all(
    locatorBlocks.map(async (locatorBlock, index) => {
      if (locatorBlock.length === 0) {
        throw new Error(`Ethereum event locator block ${index} is empty`);
      }

      const loadedLocators = await loadEthereumEventLocators(executionClient, locatorBlock);
      const firstLocator = loadedLocators[0];
      const blockHash = firstLocator.receipt.blockHash;

      for (const locator of loadedLocators.slice(1)) {
        if (locator.receipt.blockHash.toLowerCase() !== blockHash.toLowerCase()) {
          throw new Error(`Ethereum event locator block ${index} spans multiple execution blocks`);
        }
      }

      const targetHeader = await loadExecutionHeader(executionClient, blockHash);
      if (argonFinalizedExecutionHeader.blockNumber < targetHeader.number) {
        throw new Error(
          `Argon finalized execution header ${argonFinalizedExecutionHeader.blockHash} is behind target block ${blockHash}; wait for relayer sync`,
        );
      }
      return {
        blockHash,
        locators: loadedLocators,
        targetHeader,
      };
    }),
  );

  if (blocksWithHeaders.length === 0) {
    throw new Error('At least one Ethereum event locator is required');
  }

  blocksWithHeaders.sort((a, b) => Number(a.targetHeader.number - b.targetHeader.number));
  const furthestTargetBlock = blocksWithHeaders[0];

  const targetToArgonFinalizedHeaderChain = await buildExecutionHeaderChain(
    executionClient,
    furthestTargetBlock.targetHeader,
    argonFinalizedExecutionHeader,
  );

  const blocks: EthereumEventProof['blocks'] = [];
  for (const { blockHash, locators, targetHeader } of blocksWithHeaders) {
    if (targetHeader.number === argonFinalizedExecutionHeader.blockNumber) {
      if (blockHash.toLowerCase() !== argonFinalizedExecutionHeader.blockHash.toLowerCase()) {
        throw new ArgonFinalizedExecutionHeaderPathError(
          `Target block ${blockHash} is not the Argon finalized execution header ${argonFinalizedExecutionHeader.blockHash}`,
        );
      }
    } else {
      const sharedTargetHeaderOnArgonFinalizedChain =
        targetToArgonFinalizedHeaderChain[
          Number(targetHeader.number - furthestTargetBlock.targetHeader.number)
        ];
      if (
        !sharedTargetHeaderOnArgonFinalizedChain ||
        sharedTargetHeaderOnArgonFinalizedChain.blockHash.toLowerCase() !== blockHash.toLowerCase()
      ) {
        throw new ArgonFinalizedExecutionHeaderPathError(
          `Target block ${blockHash} is not on the shared execution header chain to Argon finalized execution header ${argonFinalizedExecutionHeader.blockHash}`,
        );
      }
    }

    const receiptLogs: EthereumReceiptLog[] = [];
    const transactionIndexes: number[] = [];
    const seenTransactionIndexes = new Set<number>();

    for (const { receipt, requestedLogIndexes } of locators) {
      if (!seenTransactionIndexes.has(receipt.transactionIndex)) {
        seenTransactionIndexes.add(receipt.transactionIndex);
        transactionIndexes.push(receipt.transactionIndex);
      }

      for (const index of requestedLogIndexes) {
        const log = receipt.logs[index];
        if (!log) {
          throw new Error(`Missing log ${index} in receipt ${receipt.transactionHash}`);
        }

        receiptLogs.push({
          transactionIndex: receipt.transactionIndex,
          eventLog: {
            address: log.address,
            topics: [...log.topics],
            data: log.data,
          },
        });
      }
    }

    const receiptProof = await buildCombinedReceiptProof(
      executionClient,
      blockHash,
      transactionIndexes,
      bytesToHex(targetHeader.receiptTrie),
    );

    blocks.push({
      targetBlockNumber: Number(targetHeader.number),
      receiptProof,
      receiptLogs,
    });
  }

  return {
    executionBlockProof: {
      anchorBlockHash: argonFinalizedExecutionHeader.blockHash,
      targetToAnchorHeaderChain: targetToArgonFinalizedHeaderChain.map(({ rlp }) => ({ rlp })),
    },
    blocks,
  };
}

export async function getLatestArgonFinalizedExecutionHeader(
  client: IArgonQueryable,
): Promise<ArgonFinalizedExecutionHeader> {
  const verifierQuery = client.query.ethereumVerifier;
  const latestArgonFinalizedExecutionHeaderHash =
    await verifierQuery.latestExecutionHeaderAnchorBlockHash();

  if (latestArgonFinalizedExecutionHeaderHash.isNone) {
    throw new Error('No Argon finalized execution header is available yet; wait for relayer sync');
  }

  const blockHash = latestArgonFinalizedExecutionHeaderHash.unwrap().toHex();
  const argonFinalizedExecutionHeaderEntry = await verifierQuery.executionHeaderAnchors(blockHash);

  if (argonFinalizedExecutionHeaderEntry.isNone) {
    throw new Error(`Argon finalized execution header ${blockHash} is missing`);
  }

  return {
    blockHash,
    blockNumber: argonFinalizedExecutionHeaderEntry.unwrap().blockNumber.toBigInt(),
  };
}

export async function waitForArgonFinalizedExecutionHeader(
  client: IArgonQueryable,
  targetBlockNumber: bigint,
  options: { pollMs?: number; timeoutMs?: number } = {},
): Promise<ArgonFinalizedExecutionHeader> {
  const pollMs = options.pollMs ?? 3_000;
  const timeoutMs = options.timeoutMs ?? 5 * 60_000;
  const startedAt = Date.now();
  let lastError: Error | undefined;

  while (Date.now() - startedAt < timeoutMs) {
    try {
      const argonFinalizedExecutionHeader = await getLatestArgonFinalizedExecutionHeader(client);
      if (argonFinalizedExecutionHeader.blockNumber >= targetBlockNumber) {
        return argonFinalizedExecutionHeader;
      }
    } catch (error) {
      if (!(error instanceof Error)) {
        throw error;
      }
      lastError = error;
    }

    await new Promise(resolve => setTimeout(resolve, pollMs));
  }

  throw (
    lastError ??
    new Error(
      `Argon finalized execution header did not reach block ${targetBlockNumber} within ${Math.floor(timeoutMs / 1000)}s`,
    )
  );
}

async function loadEthereumEventLocators(
  executionClient: EthereumExecutionClient,
  locators: EthereumEventLocator[],
) {
  return await retryWhileExecutionRpcIndexing(() => {
    return Promise.all(
      locators.map(async ({ txHash, logIndexes, receipt }) => {
        receipt ??= await executionClient.getTransactionReceipt({ hash: txHash });
        const requestedLogIndexes = logIndexes ?? [0];
        if (requestedLogIndexes.length === 0) {
          throw new Error(`At least one log index is required for receipt ${txHash}`);
        }

        return {
          receipt,
          requestedLogIndexes,
        } satisfies LoadedEthereumEventLocator;
      }),
    );
  });
}

export async function loadExecutionHeader(
  executionClient: EthereumExecutionClient,
  blockTag: Hex | bigint,
) {
  const blockData = await retryWhileExecutionRpcIndexing(() =>
    typeof blockTag === 'string'
      ? executionClient.request<EthGetBlockByHashRpc>({
          method: 'eth_getBlockByHash',
          params: [blockTag, true],
        })
      : executionClient.request<EthGetBlockByNumberRpc>({
          method: 'eth_getBlockByNumber',
          params: [toHex(blockTag), true],
        }),
  );
  const header = createBlockHeaderFromRPC(blockData);

  if (
    typeof blockTag === 'string' &&
    bytesToHex(header.hash()).toLowerCase() !== blockTag.toLowerCase()
  ) {
    throw new Error(`Execution header hash mismatch for block ${blockTag}`);
  }

  return header;
}

export async function buildExecutionHeaderChain(
  executionClient: EthereumExecutionClient,
  targetHeader: BlockHeader,
  argonFinalizedExecutionHeader: ArgonFinalizedExecutionHeader,
): Promise<{ blockHash: Hex; rlp: Hex }[]> {
  const targetBlockHash = bytesToHex(targetHeader.hash());

  if (targetHeader.number === argonFinalizedExecutionHeader.blockNumber) {
    if (targetBlockHash.toLowerCase() !== argonFinalizedExecutionHeader.blockHash.toLowerCase()) {
      throw new ArgonFinalizedExecutionHeaderPathError(
        `Target block ${targetBlockHash} is not the Argon finalized execution header ${argonFinalizedExecutionHeader.blockHash}`,
      );
    }

    return [];
  }

  const headers: { blockHash: Hex; rlp: Hex }[] = [];
  let header = await loadExecutionHeader(executionClient, argonFinalizedExecutionHeader.blockHash);

  if (header.number !== argonFinalizedExecutionHeader.blockNumber) {
    throw new Error(
      `Execution header ${argonFinalizedExecutionHeader.blockHash} is not block ${argonFinalizedExecutionHeader.blockNumber}`,
    );
  }

  while (header.number > targetHeader.number) {
    header = await loadExecutionHeader(executionClient, bytesToHex(header.parentHash));
    headers.push({
      blockHash: bytesToHex(header.hash()),
      rlp: bytesToHex(header.serialize()),
    });
  }

  if (
    header.number !== targetHeader.number ||
    bytesToHex(header.hash()).toLowerCase() !== targetBlockHash.toLowerCase()
  ) {
    throw new ArgonFinalizedExecutionHeaderPathError(
      `Target block ${targetBlockHash} is not on the shared execution header chain to Argon finalized execution header ${argonFinalizedExecutionHeader.blockHash}`,
    );
  }

  return headers.reverse();
}

async function buildCombinedReceiptProof(
  executionClient: EthereumExecutionClient,
  blockHash: Hex,
  transactionIndexes: number[],
  receiptsRoot: Hex,
): Promise<EthereumReceiptProof> {
  const { receipts } = await retryWhileExecutionRpcIndexing(async () => {
    const block = await executionClient.getBlock({ blockHash });
    const receipts = await Promise.all(
      block.transactions.map(hash => executionClient.getTransactionReceipt({ hash })),
    );

    return { receipts };
  });

  const trie = await createMPT();

  for (const receipt of receipts) {
    await trie.put(
      encodeReceiptTrieKey(receipt.transactionIndex),
      encodeEthereumReceiptForProof(receipt),
    );
  }

  if (bytesToHex(trie.root()).toLowerCase() !== receiptsRoot.toLowerCase()) {
    throw new Error(`Receipt trie root mismatch for block ${blockHash}`);
  }

  const nodeIndexesByHex = new Map<Hex, number>();
  const sharedNodes: Hex[] = [];
  const proofReceipts = await Promise.all(
    transactionIndexes.map(async transactionIndex => {
      const key = encodeReceiptTrieKey(transactionIndex);
      const proof = await createMerkleProof(trie, key);
      const verifiedReceipt = await verifyMPTWithMerkleProof(trie, trie.root(), key, proof);

      if (!verifiedReceipt) {
        throw new Error(
          `Receipt proof verification failed for transaction index ${transactionIndex}`,
        );
      }

      return {
        transactionIndex,
        nodeIndexes: proof.map(node => {
          const hexNode = bytesToHex(node);
          const existingIndex = nodeIndexesByHex.get(hexNode);
          if (existingIndex !== undefined) {
            return existingIndex;
          }

          const nextIndex = sharedNodes.length;
          sharedNodes.push(hexNode);
          nodeIndexesByHex.set(hexNode, nextIndex);
          return nextIndex;
        }),
      } satisfies EthereumCombinedReceiptProofReceipt;
    }),
  );

  return {
    nodes: sharedNodes,
    receipts: proofReceipts,
  };
}
