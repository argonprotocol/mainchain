import { createBlockHeaderFromRPC, type JSONRPCBlock } from '@ethereumjs/block';
import { createMPT } from '@ethereumjs/mpt';
import { MintingGatewayEvents, mintingGatewayAbi } from '../EvmContracts';
import { bytesToHex, encodeAbiParameters, encodeEventTopics, toHex, type Hex } from 'viem';
import type { ArgonClient, IArgonQueryable } from '../index';
import {
  encodeEthereumReceiptForProof,
  encodeReceiptTrieKey,
  type ArgonFinalizedExecutionHeader,
} from '../EthereumProof';
import type { EthereumExecutionClient, EthereumReceipt } from '../EthereumExecution';

const ZERO_HASH: Hex = `0x${'00'.repeat(32)}`;
const ZERO_BLOOM: Hex = `0x${'00'.repeat(256)}`;
const ZERO_ADDRESS: Hex = `0x${'00'.repeat(20)}`;
const EMPTY_UNCLES_HASH =
  '0x1dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347' as Hex;

export function repeatHex(byte: string, length: number): Hex {
  return `0x${byte.repeat(length)}`;
}

export function createGatewayProofConsts(
  args: {
    maxActivitiesPerReceiptProof?: number;
    maxReceiptProofsPerExtrinsic?: number;
    maxProofExecutionHeaderDepth?: number;
  } = {},
) {
  return {
    crosschainTransfer: {
      maxActivitiesPerReceiptProof: { toNumber: () => args.maxActivitiesPerReceiptProof ?? 16 },
      maxReceiptProofsPerExtrinsic: { toNumber: () => args.maxReceiptProofsPerExtrinsic ?? 10 },
      maxProofExecutionHeaderDepth: { toNumber: () => args.maxProofExecutionHeaderDepth ?? 64 },
    },
  } as unknown as Pick<ArgonClient, 'consts'>['consts'];
}

export function createLegacyReceipt(args: {
  txHash: Hex;
  transactionIndex: number;
  logs: Array<Pick<EthereumReceipt['logs'][number], 'address' | 'topics' | 'data'>>;
  cumulativeGasUsed?: bigint;
}): EthereumReceipt {
  return {
    type: 'legacy',
    status: 'success',
    cumulativeGasUsed: args.cumulativeGasUsed ?? 21_000n,
    logsBloom: ZERO_BLOOM,
    logs: args.logs as EthereumReceipt['logs'],
    transactionHash: args.txHash,
    transactionIndex: args.transactionIndex,
  } as unknown as EthereumReceipt;
}

export async function createExecutionBlock(args: {
  number: bigint;
  parentHash?: Hex;
  receipts?: EthereumReceipt[];
  receiptsRoot?: Hex;
  transactions?: Hex[];
  gasUsed?: bigint;
  timestamp?: bigint;
  blockLogs?: Array<{ blockHash: Hex; blockNumber: bigint }>;
}): Promise<JSONRPCBlock> {
  const receiptsRoot = args.receipts
    ? await createReceiptsRoot(args.receipts)
    : (args.receiptsRoot ?? ZERO_HASH);
  const transactions =
    args.transactions ?? args.receipts?.map(({ transactionHash }) => transactionHash) ?? [];
  const template = {
    number: toHex(args.number),
    hash: ZERO_HASH,
    parentHash: args.parentHash ?? ZERO_HASH,
    nonce: '0x0000000000000000' as Hex,
    sha3Uncles: EMPTY_UNCLES_HASH,
    logsBloom: ZERO_BLOOM,
    transactionsRoot: ZERO_HASH,
    stateRoot: ZERO_HASH,
    receiptsRoot,
    miner: ZERO_ADDRESS,
    difficulty: '0x0' as Hex,
    extraData: '0x' as Hex,
    size: '0x1' as Hex,
    gasLimit: toHex(30_000_000n),
    gasUsed: toHex(args.gasUsed ?? args.receipts?.at(-1)?.cumulativeGasUsed ?? 0n),
    timestamp: toHex(args.timestamp ?? args.number),
    transactions,
    uncles: [],
  } satisfies JSONRPCBlock;
  const hash: Hex = bytesToHex(createBlockHeaderFromRPC(template).hash());
  const block = { ...template, hash } satisfies JSONRPCBlock;

  for (const receipt of args.receipts ?? []) {
    receipt.blockHash = hash;
  }
  for (const log of args.blockLogs ?? []) {
    log.blockHash = hash;
    log.blockNumber = args.number;
  }

  return block;
}

export function createExecutionClient(args: {
  blocks: JSONRPCBlock[];
  receipts?: EthereumReceipt[];
  logsByBlockNumber?: Record<string, unknown[]>;
  locators?: Array<{
    blockNumber: bigint;
    startGatewayActivityNonce: bigint;
    endGatewayActivityNonce: bigint;
    previousLocatorHash?: Hex;
    activityRoot?: Hex;
  }>;
}): EthereumExecutionClient {
  const blocksByHash = Object.fromEntries(
    args.blocks.map(block => [block.hash.toLowerCase(), block]),
  );
  const blocksByNumber = Object.fromEntries(
    args.blocks.map(block => [block.number.toLowerCase(), block]),
  );
  const receiptsByHash = Object.fromEntries(
    (args.receipts ?? []).map(receipt => [receipt.transactionHash.toLowerCase(), receipt]),
  );
  const logsByBlockNumber = args.logsByBlockNumber ?? {};
  const locators = args.locators ?? [];

  return {
    readContract: async ({
      functionName,
      args: locatorArgs,
    }: {
      functionName: 'latestActivityBlockLocatorIndex' | 'activityBlockLocators';
      args?: [bigint];
    }) => {
      if (functionName === 'latestActivityBlockLocatorIndex') {
        return BigInt(locators.length);
      }

      const locator = locators[Number((locatorArgs?.[0] ?? 1n) - 1n)];
      if (!locator) {
        throw new Error(`Unexpected locator request ${String(locatorArgs?.[0])}`);
      }

      return [
        locator.blockNumber,
        locator.startGatewayActivityNonce,
        locator.endGatewayActivityNonce,
        locator.previousLocatorHash ?? ZERO_HASH,
        locator.activityRoot ?? ZERO_HASH,
      ] as const;
    },
    getLogs: async ({ fromBlock }: { fromBlock: bigint }) => {
      const logs = logsByBlockNumber[fromBlock.toString()];
      if (logs) {
        return logs as Awaited<ReturnType<EthereumExecutionClient['getLogs']>>;
      }

      throw new Error(`Unexpected getLogs block ${fromBlock}`);
    },
    getTransactionReceipt: async ({ hash }: { hash: Hex }) => {
      const receipt = receiptsByHash[hash.toLowerCase()];
      if (receipt) {
        return receipt;
      }

      throw new Error(`Unexpected receipt request for ${hash}`);
    },
    getBlock: async ({ blockHash, blockNumber }: { blockHash?: Hex; blockNumber?: bigint }) => {
      let block;
      if (blockHash !== undefined) {
        block = blocksByHash[blockHash.toLowerCase()];
      } else if (blockNumber !== undefined) {
        block = blocksByNumber[toHex(blockNumber).toLowerCase()];
      }

      if (block) {
        return {
          ...block,
          number: BigInt(block.number),
          gasLimit: BigInt(block.gasLimit),
          gasUsed: BigInt(block.gasUsed),
          timestamp: BigInt(block.timestamp),
        };
      }

      throw new Error(
        `Unexpected block request for ${blockHash ?? `blockNumber=${String(blockNumber)}`}`,
      );
    },
    request: async ({
      method,
      params,
    }: {
      method: 'eth_getBlockByHash' | 'eth_getBlockByNumber';
      params: [Hex, true];
    }) => {
      const block =
        method === 'eth_getBlockByHash'
          ? blocksByHash[params[0].toLowerCase()]
          : blocksByNumber[params[0].toLowerCase()];
      if (block) {
        return block;
      }

      throw new Error(`Unexpected header request for ${method} ${params[0]}`);
    },
  } as unknown as EthereumExecutionClient;
}

export function createArgonGatewayClient(
  args: {
    previousGatewayActivityNonce?: bigint;
    argonFinalizedExecutionHeaders?: ArgonFinalizedExecutionHeader[];
    consts?: Pick<ArgonClient, 'consts'>['consts'];
  } = {},
): IArgonQueryable & Pick<ArgonClient, 'consts'> {
  const previousGatewayActivityNonce = args.previousGatewayActivityNonce ?? 0n;
  const argonFinalizedExecutionHeaders = args.argonFinalizedExecutionHeaders ?? [];
  const latestArgonFinalizedExecutionHeader = argonFinalizedExecutionHeaders.at(-1);
  const retainedExecutionHeaderAnchors = [...argonFinalizedExecutionHeaders].sort((left, right) =>
    Number(left.blockNumber - right.blockNumber),
  );
  const toExecutionHeaderAnchorCodec = (header: ArgonFinalizedExecutionHeader) => ({
    blockHash: {
      toHex: () => header.blockHash,
    },
    blockNumber: {
      toBigInt: () => header.blockNumber,
    },
  });
  const toExecutionHeaderAnchorOption = (header: ArgonFinalizedExecutionHeader) => ({
    isSome: true,
    unwrap: () => toExecutionHeaderAnchorCodec(header),
  });

  return {
    query: {
      crosschainTransfer: {
        gatewayStateBySourceChain: async () =>
          previousGatewayActivityNonce > 0n
            ? {
                isSome: true,
                unwrap: () => ({
                  gatewayActivityNonce: {
                    toBigInt: () => previousGatewayActivityNonce,
                  },
                }),
              }
            : {
                isSome: false,
              },
      },
      ethereumVerifier: {
        latestExecutionHeaderAnchorBlockHash: async () =>
          latestArgonFinalizedExecutionHeader
            ? {
                isNone: false,
                unwrap: () => ({ toHex: () => latestArgonFinalizedExecutionHeader.blockHash }),
              }
            : {
                isNone: true,
              },
        executionHeaderAnchors: async (blockHash: Hex) => {
          const header = argonFinalizedExecutionHeaders.find(
            entry => entry.blockHash.toLowerCase() === blockHash.toLowerCase(),
          );

          return header
            ? {
                isNone: false,
                unwrap: () => ({
                  blockNumber: {
                    toBigInt: () => header.blockNumber,
                  },
                }),
              }
            : {
                isNone: true,
              };
        },
        executionHeaderAnchorsByBlockNumber: Object.assign(
          async (scanKey: Hex) => {
            const header = retainedExecutionHeaderAnchors.find(
              entry =>
                toHex(entry.blockNumber, { size: 8 }).toLowerCase() === scanKey.toLowerCase(),
            );

            return header
              ? {
                  isSome: true,
                  unwrap: () => toExecutionHeaderAnchorCodec(header),
                }
              : {
                  isSome: false,
                };
          },
          {
            key: (scanKey: Hex) => `storage:${scanKey.toLowerCase()}`,
            entriesPaged: async ({ pageSize, startKey }: { pageSize: number; startKey?: string }) =>
              retainedExecutionHeaderAnchors
                .filter(
                  entry =>
                    toHex(entry.blockNumber, { size: 8 }).toLowerCase() >
                    (startKey?.replace('storage:', '') ?? `0x${'00'.repeat(8)}`),
                )
                .slice(0, pageSize)
                .map(
                  entry =>
                    [
                      `storage:${toHex(entry.blockNumber, { size: 8 }).toLowerCase()}`,
                      toExecutionHeaderAnchorOption(entry),
                    ] as const,
                ),
          },
        ),
      },
    },
    consts: args.consts ?? createGatewayProofConsts(),
  } as unknown as IArgonQueryable & Pick<ArgonClient, 'consts'>;
}

export function createTransferToArgonStartedBlockLog(args: {
  gatewayAddress: Hex;
  txHash: Hex;
  transactionIndex: number;
  logIndex: number;
  nonce: bigint;
  argonAccountId: Hex;
}): {
  address: Hex;
  topics: Hex[];
  data: Hex;
  transactionHash: Hex;
  transactionIndex: number;
  logIndex: number;
  blockHash: Hex;
  blockNumber: bigint;
} {
  const topics = encodeEventTopics({
    abi: mintingGatewayAbi,
    eventName: MintingGatewayEvents.TransferToArgonStarted.name,
    args: {
      from: repeatHex('11', 20),
      token: repeatHex('22', 20),
    },
  });
  const data = encodeAbiParameters(
    [
      { name: 'amount', type: 'uint128' },
      { name: 'argonAccountId', type: 'bytes32' },
      {
        name: 'gatewayState',
        type: 'tuple',
        components: [
          { name: 'gatewayActivityNonce', type: 'uint64' },
          { name: 'argonApprovalsNonce', type: 'uint64' },
          { name: 'argonCirculation', type: 'uint128' },
          { name: 'argonotCirculation', type: 'uint128' },
        ],
      },
    ],
    [
      250n,
      args.argonAccountId,
      {
        gatewayActivityNonce: args.nonce,
        argonApprovalsNonce: 0n,
        argonCirculation: 750n,
        argonotCirculation: 2_000n,
      },
    ],
  );

  return {
    address: args.gatewayAddress,
    topics: topics as Hex[],
    data,
    transactionHash: args.txHash,
    transactionIndex: args.transactionIndex,
    logIndex: args.logIndex,
    blockHash: ZERO_HASH,
    blockNumber: 0n,
  };
}

export function createGlobalIssuanceCouncilRotatedBlockLog(args: {
  gatewayAddress: Hex;
  txHash: Hex;
  transactionIndex: number;
  logIndex: number;
  nonce: bigint;
  approvalHash?: Hex;
}): {
  address: Hex;
  topics: Hex[];
  data: Hex;
  transactionHash: Hex;
  transactionIndex: number;
  logIndex: number;
  blockHash: Hex;
  blockNumber: bigint;
} {
  const topics = encodeEventTopics({
    abi: mintingGatewayAbi,
    eventName: MintingGatewayEvents.GlobalIssuanceCouncilRotated.name,
  });
  const data = encodeAbiParameters(
    [
      { name: 'councilHash', type: 'bytes32' },
      { name: 'approvalHash', type: 'bytes32' },
      { name: 'relayerArgonAccountId', type: 'bytes32' },
      {
        name: 'gatewayState',
        type: 'tuple',
        components: [
          { name: 'gatewayActivityNonce', type: 'uint64' },
          { name: 'argonApprovalsNonce', type: 'uint64' },
          { name: 'argonCirculation', type: 'uint128' },
          { name: 'argonotCirculation', type: 'uint128' },
        ],
      },
    ],
    [
      repeatHex('61', 32),
      args.approvalHash ?? repeatHex('63', 32),
      repeatHex('62', 32),
      {
        gatewayActivityNonce: args.nonce,
        argonApprovalsNonce: 4n,
        argonCirculation: 750n,
        argonotCirculation: 2_000n,
      },
    ],
  );

  return {
    address: args.gatewayAddress,
    topics: topics as Hex[],
    data,
    transactionHash: args.txHash,
    transactionIndex: args.transactionIndex,
    logIndex: args.logIndex,
    blockHash: ZERO_HASH,
    blockNumber: 0n,
  };
}

async function createReceiptsRoot(receipts: EthereumReceipt[]): Promise<Hex> {
  const trie = await createMPT();

  for (const receipt of receipts) {
    await trie.put(
      encodeReceiptTrieKey(receipt.transactionIndex),
      encodeEthereumReceiptForProof(receipt),
    );
  }

  return bytesToHex(trie.root());
}
