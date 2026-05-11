import { createBlockHeaderFromRPC, type JSONRPCBlock } from '@ethereumjs/block';
import { createMerkleProof, createMPT, verifyMPTWithMerkleProof } from '@ethereumjs/mpt';
import {
  MINTING_GATEWAY_BURN_FOR_TRANSFER_EVENT_NAME,
  mintingGatewayArtifact,
} from '@argonprotocol/ethereum-contracts';
import type { IArgonQueryable } from './index';
import {
  bytesToHex,
  createPublicClient,
  encodeEventTopics,
  getAddress,
  type Hex,
  hexToBytes,
  http,
  toHex,
  toRlp,
} from 'viem';

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

export type EthereumReceipt = Awaited<
  ReturnType<ReturnType<typeof createPublicClient>['getTransactionReceipt']>
>;
export type EthereumExecutionClient = ReturnType<typeof createPublicClient>;
export type RetainedExecutionAnchor = {
  blockHash: Hex;
  blockNumber: bigint;
};

export type EthereumEventLocator = {
  txHash: Hex;
  logIndex?: number;
  executionRpcUrl?: string;
  executionClient?: EthereumExecutionClient;
  receipt?: EthereumReceipt;
};

export type EthereumEventLog = {
  address: Hex;
  topics: Hex[];
  data: Hex;
};

export type EthereumExecutionHeaderProof = {
  rlp: Hex;
};

export type EthereumExecutionBlockProof = {
  anchorBlockHash: Hex;
  targetToAnchorHeaderChain: EthereumExecutionHeaderProof[];
};

export type EthereumReceiptProof = {
  transactionIndex: number;
  nodes: Hex[];
};

export type EthereumEventProof = {
  eventLog: EthereumEventLog;
  proof: {
    executionBlockProof: EthereumExecutionBlockProof;
    receiptProof: EthereumReceiptProof;
  };
};

const ethereumBurnForTransferTopic = encodeEventTopics({
  abi: mintingGatewayArtifact.abi,
  eventName: MINTING_GATEWAY_BURN_FOR_TRANSFER_EVENT_NAME,
})[0]?.toLowerCase();

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
  { txHash, logIndex = 0, receipt: providedReceipt, ...executionSource }: EthereumEventLocator,
): Promise<EthereumEventProof> {
  const executionClient = getExecutionClient(executionSource);
  const receipt = await loadEthereumReceipt(txHash, providedReceipt, executionClient);
  const log = receipt.logs[logIndex];

  if (!log) throw new Error(`Missing log ${logIndex} in receipt ${txHash}`);

  const [targetHeader, anchor] = await Promise.all([
    loadExecutionHeader(executionClient, receipt.blockHash),
    getLatestRetainedAnchor(client),
  ]);

  if (anchor.blockNumber < targetHeader.number) {
    throw new Error(
      `Latest retained execution anchor ${anchor.blockHash} is behind target block ${receipt.blockHash}; wait for relayer sync`,
    );
  }

  const [headerChain, receiptProofNodes] = await Promise.all([
    buildExecutionHeaderChain(executionClient, targetHeader.number, anchor.blockNumber),
    buildReceiptProofNodes(
      executionClient,
      receipt.blockHash,
      receipt.transactionIndex,
      bytesToHex(targetHeader.receiptTrie),
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

export function findEthereumBurnForTransferLogIndex(
  receipt: EthereumReceipt,
  gatewayAddress: Hex,
): number {
  const normalizedGatewayAddress = getAddress(gatewayAddress).toLowerCase();

  const index = receipt.logs.findIndex(log => {
    return (
      log.address.toLowerCase() === normalizedGatewayAddress &&
      log.topics[0]?.toLowerCase() === ethereumBurnForTransferTopic
    );
  });

  if (index === -1) {
    throw new Error(
      `Ethereum receipt ${receipt.transactionHash} did not emit BurnForTransfer from gateway ${gatewayAddress}`,
    );
  }

  return index;
}

export async function getLatestRetainedAnchor(
  client: IArgonQueryable,
): Promise<RetainedExecutionAnchor> {
  const verifierQuery = client.query.ethereumVerifier;
  const latestAnchorHash = await verifierQuery.latestExecutionHeaderAnchorBlockHash();

  if (latestAnchorHash.isNone) {
    throw new Error(
      'No retained ethereum execution anchor is available yet; wait for relayer sync',
    );
  }

  const blockHash = latestAnchorHash.unwrap().toHex();
  const anchor = await verifierQuery.executionHeaderAnchors(blockHash);

  if (anchor.isNone) {
    throw new Error(`Retained ethereum execution anchor ${blockHash} is missing`);
  }

  return {
    blockHash,
    blockNumber: anchor.unwrap().blockNumber.toBigInt(),
  };
}

export async function waitForRetainedExecutionAnchor(
  client: IArgonQueryable,
  targetBlockNumber: bigint,
  options: { pollMs?: number; timeoutMs?: number } = {},
): Promise<RetainedExecutionAnchor> {
  const pollMs = options.pollMs ?? 3_000;
  const timeoutMs = options.timeoutMs ?? 5 * 60_000;
  const startedAt = Date.now();
  let lastError: Error | undefined;

  while (Date.now() - startedAt < timeoutMs) {
    try {
      const anchor = await getLatestRetainedAnchor(client);
      if (anchor.blockNumber >= targetBlockNumber) {
        return anchor;
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
      `Retained ethereum execution anchor did not reach block ${targetBlockNumber} within ${Math.floor(timeoutMs / 1000)}s`,
    )
  );
}

function getExecutionClient(
  source: Pick<EthereumEventLocator, 'executionRpcUrl' | 'executionClient'>,
): EthereumExecutionClient {
  if (source.executionClient) {
    return source.executionClient;
  }
  if (source.executionRpcUrl) {
    return createPublicClient({ transport: http(source.executionRpcUrl) });
  }

  throw new Error('Ethereum event proof requires an execution client or execution RPC URL');
}

async function loadEthereumReceipt(
  txHash: Hex,
  providedReceipt: EthereumReceipt | undefined,
  executionClient: EthereumExecutionClient,
) {
  return (
    providedReceipt ??
    (await waitForIndexed(() => executionClient.getTransactionReceipt({ hash: txHash })))
  );
}

async function loadExecutionHeader(
  executionClient: EthereumExecutionClient,
  blockTag: Hex | bigint,
) {
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
  const header = createBlockHeaderFromRPC(blockData);

  if (
    typeof blockTag === 'string' &&
    bytesToHex(header.hash()).toLowerCase() !== blockTag.toLowerCase()
  ) {
    throw new Error(`Execution header hash mismatch for block ${blockTag}`);
  }

  return header;
}

async function buildExecutionHeaderChain(
  executionClient: EthereumExecutionClient,
  targetBlockNumber: bigint,
  anchorBlockNumber: bigint,
): Promise<Hex[]> {
  const headers: Hex[] = [];

  for (let blockNumber = targetBlockNumber; blockNumber < anchorBlockNumber; blockNumber += 1n) {
    const header = await loadExecutionHeader(executionClient, blockNumber);
    headers.push(bytesToHex(header.serialize()));
  }

  return headers;
}

async function buildReceiptProofNodes(
  executionClient: EthereumExecutionClient,
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
