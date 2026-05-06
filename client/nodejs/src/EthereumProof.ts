import { createBlockFromRPC, type JSONRPCBlock } from '@ethereumjs/block';
import { Mainnet, createCustomCommon } from '@ethereumjs/common';
import { createMerkleProof, createMPT, verifyMPTWithMerkleProof } from '@ethereumjs/mpt';
import type { IArgonQueryable } from './index';
import { bytesToHex, createPublicClient, type Hex, hexToBytes, http, toHex, toRlp } from 'viem';

type VerifyEventLog = IArgonQueryable['call']['ethereumApis']['verifyEventLog'];
export type EthereumVerifyEventLogResult = Awaited<ReturnType<VerifyEventLog>>;
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

type EthereumReceipt = Awaited<
  ReturnType<ReturnType<typeof createPublicClient>['getTransactionReceipt']>
>;
type ExecutionClient = ReturnType<typeof createPublicClient>;
type RetainedExecutionAnchor = {
  blockHash: Hex;
  blockNumber: bigint;
};
const executionCommonPromises = new WeakMap<
  ExecutionClient,
  Promise<ReturnType<typeof createCustomCommon>>
>();

export type EthereumEventLocator = {
  txHash: Hex;
  logIndex?: number;
  executionRpcUrl: string;
};

export type EthereumEventProof = {
  eventLog: Parameters<VerifyEventLog>[0];
  proof: Parameters<VerifyEventLog>[1];
};

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
  client: IArgonQueryable,
  { txHash, logIndex = 0, executionRpcUrl }: EthereumEventLocator,
): Promise<EthereumEventProof> {
  const executionClient = createPublicClient({ transport: http(executionRpcUrl) });
  const receipt = await waitForIndexed(() =>
    executionClient.getTransactionReceipt({ hash: txHash }),
  );
  const log = receipt.logs[logIndex];

  if (!log) throw new Error(`Missing log ${logIndex} in receipt ${txHash}`);

  const [targetBlock, anchor] = await Promise.all([
    loadExecutionBlock(executionClient, receipt.blockHash),
    getLatestRetainedAnchor(client),
  ]);

  if (anchor.blockNumber < targetBlock.header.number) {
    throw new Error(
      `Latest retained execution anchor ${anchor.blockHash} is behind target block ${receipt.blockHash}; wait for relayer sync`,
    );
  }

  const [headerChain, receiptProofNodes] = await Promise.all([
    buildExecutionHeaderChain(executionClient, targetBlock.header.number, anchor.blockNumber),
    buildReceiptProofNodes(
      executionClient,
      receipt.blockHash,
      receipt.transactionIndex,
      bytesToHex(targetBlock.header.receiptTrie),
    ),
  ]);

  const eventLog: EthereumEventProof['eventLog'] = {
    address: log.address,
    topics: [...log.topics],
    data: log.data,
  };
  const proof: EthereumEventProof['proof'] = {
    executionBlockProof: {
      anchorBlockHash: anchor.blockHash,
      targetToAnchorHeaderChain: headerChain.map(rlp => ({ rlp })),
    },
    receiptProof: {
      transactionIndex: receipt.transactionIndex,
      nodes: receiptProofNodes,
    },
  };

  return { eventLog, proof };
}

async function getLatestRetainedAnchor(client: IArgonQueryable): Promise<RetainedExecutionAnchor> {
  const verifierQuery = client.query.ethereumVerifier;
  const latestAnchorHash = await verifierQuery.latestExecutionHeaderAnchorBlockHash();

  if (latestAnchorHash.isNone) {
    throw new Error(
      'No retained ethereum execution anchor is available yet; wait for relayer sync',
    );
  }

  const blockHash = latestAnchorHash.unwrap().toHex().toLowerCase() as Hex;
  const anchor = await verifierQuery.executionHeaderAnchors(blockHash);

  if (anchor.isNone) {
    throw new Error(`Retained ethereum execution anchor ${blockHash} is missing`);
  }

  return {
    blockHash,
    blockNumber: anchor.unwrap().blockNumber.toBigInt(),
  };
}

async function loadExecutionBlock(executionClient: ExecutionClient, blockTag: Hex | bigint) {
  const blockData = await waitForIndexed(() =>
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
  const common = await getExecutionCommon(executionClient);
  const block = createBlockFromRPC(blockData, [], { common });

  if (
    typeof blockTag === 'string' &&
    bytesToHex(block.hash()).toLowerCase() !== blockTag.toLowerCase()
  ) {
    throw new Error(`Execution header hash mismatch for block ${blockTag}`);
  }

  return block;
}

function getExecutionCommon(executionClient: ExecutionClient) {
  let commonPromise = executionCommonPromises.get(executionClient);

  if (!commonPromise) {
    commonPromise = waitForIndexed(() => executionClient.getChainId()).then(chainId =>
      createCustomCommon(
        {
          chainId,
        },
        Mainnet,
      ),
    );
    executionCommonPromises.set(executionClient, commonPromise);
  }

  return commonPromise;
}

async function buildExecutionHeaderChain(
  executionClient: ExecutionClient,
  targetBlockNumber: bigint,
  anchorBlockNumber: bigint,
): Promise<Hex[]> {
  const headers: Hex[] = [];

  for (let blockNumber = targetBlockNumber; blockNumber < anchorBlockNumber; blockNumber += 1n) {
    const block = await loadExecutionBlock(executionClient, blockNumber);
    headers.push(bytesToHex(block.header.serialize()));
  }

  return headers;
}

async function buildReceiptProofNodes(
  executionClient: ExecutionClient,
  blockHash: Hex,
  transactionIndex: number,
  receiptsRoot: Hex,
): Promise<Hex[]> {
  const block = await waitForIndexed(() => executionClient.getBlock({ blockHash }));
  const receipts = await Promise.all(
    block.transactions.map(hash =>
      waitForIndexed(() => executionClient.getTransactionReceipt({ hash })),
    ),
  );

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

  const key = encodeReceiptTrieKey(transactionIndex);
  const proof = await createMerkleProof(trie, key);
  const verifiedReceipt = await verifyMPTWithMerkleProof(trie, trie.root(), key, proof);

  if (!verifiedReceipt) {
    throw new Error(`Receipt proof verification failed for transaction index ${transactionIndex}`);
  }

  return proof.map(node => bytesToHex(node));
}
async function waitForIndexed<TResult>(request: () => Promise<TResult>): Promise<TResult> {
  const startedAt = Date.now();
  let lastError: Error | undefined;

  while (Date.now() - startedAt < 30_000) {
    try {
      return await request();
    } catch (error) {
      if (!(error instanceof Error) || !error.message.includes('indexing is in progress')) {
        throw error;
      }

      lastError = error;
      await new Promise(resolve => setTimeout(resolve, 500));
    }
  }

  throw lastError ?? new Error('Timed out waiting for execution RPC indexing');
}
