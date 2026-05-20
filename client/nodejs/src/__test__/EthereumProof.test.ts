import { createBlockHeaderFromRPC, type JSONRPCBlock } from '@ethereumjs/block';
import { expect, it } from 'vitest';
import { createMPT, verifyMPTWithMerkleProof } from '@ethereumjs/mpt';
import {
  bytesToHex,
  encodeAbiParameters,
  encodeEventTopics,
  hexToBytes,
  toHex,
  type Hex,
} from 'viem';
import { MintingGatewayEvents, mintingGatewayAbi } from '@argonprotocol/ethereum-contracts';
import type { ArgonClient, IArgonQueryable } from '../index';
import {
  buildEthereumEventProof,
  encodeEthereumReceiptForProof,
  encodeReceiptTrieKey,
} from '../EthereumProof';
import type { EthereumExecutionClient, EthereumReceipt } from '../EthereumExecution';
import {
  buildGatewayActivityProofPayload,
  decodeEthereumGatewayActivityLog,
  decodeEthereumTransferToArgonStartedLog,
} from '../EthereumGatewayActivity';
import productionInboundReceipt from './fixtures/productionInboundReceipt.json';

type EncodedReceiptInput = Parameters<typeof encodeEthereumReceiptForProof>[0];

function createGatewayProofConsts(
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

// Mirror the runtime proof-bound consts for query-only unit tests.
const gatewayProofConsts = createGatewayProofConsts();

it('encodes a production inbound receipt exactly as retained proof expects', async () => {
  const receipt = {
    ...productionInboundReceipt.receipt,
    cumulativeGasUsed: BigInt(productionInboundReceipt.receipt.cumulativeGasUsed),
  } as EncodedReceiptInput;

  expect(bytesToHex(encodeReceiptTrieKey(productionInboundReceipt.transactionIndex))).toBe(
    productionInboundReceipt.encodedTrieKey,
  );
  expect(bytesToHex(encodeEthereumReceiptForProof(receipt))).toBe(
    productionInboundReceipt.encodedReceipt,
  );

  const recoveredReceipt = await verifyMPTWithMerkleProof(
    await createMPT(),
    hexToBytes(productionInboundReceipt.receiptsRoot as Hex),
    encodeReceiptTrieKey(productionInboundReceipt.transactionIndex),
    productionInboundReceipt.receiptProofNodes.map(hex => hexToBytes(hex as Hex)),
  );

  expect(bytesToHex(recoveredReceipt!)).toBe(productionInboundReceipt.encodedReceipt);
});

it('builds one combined receipt proof for multiple receipts in the same execution block', async () => {
  const txHash0 = `0x${'aa'.repeat(32)}`;
  const txHash1 = `0x${'bb'.repeat(32)}`;
  const zeroHash = `0x${'00'.repeat(32)}`;
  const zeroBloom = `0x${'00'.repeat(256)}`;
  const zeroAddress = `0x${'00'.repeat(20)}`;
  const emptyUnclesHash =
    '0x1dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347' as Hex;
  const log0 = {
    address: `0x${'33'.repeat(20)}`,
    topics: [`0x${'44'.repeat(32)}`],
    data: '0x1234' as Hex,
  };
  const log1 = {
    address: `0x${'55'.repeat(20)}`,
    topics: [`0x${'66'.repeat(32)}`],
    data: '0xabcd' as Hex,
  };
  const receipt0 = {
    type: 'legacy',
    status: 'success',
    cumulativeGasUsed: 21_000n,
    logsBloom: zeroBloom,
    logs: [log0],
    transactionHash: txHash0,
    transactionIndex: 0,
  } as unknown as EthereumReceipt;
  const receipt1 = {
    type: 'legacy',
    status: 'success',
    cumulativeGasUsed: 42_000n,
    logsBloom: zeroBloom,
    logs: [log1],
    transactionHash: txHash1,
    transactionIndex: 1,
  } as unknown as EthereumReceipt;

  const trie = await createMPT();
  await trie.put(
    encodeReceiptTrieKey(receipt0.transactionIndex),
    encodeEthereumReceiptForProof(receipt0),
  );
  await trie.put(
    encodeReceiptTrieKey(receipt1.transactionIndex),
    encodeEthereumReceiptForProof(receipt1),
  );
  const receiptsRoot = bytesToHex(trie.root());
  const targetBlockTemplate = {
    number: toHex(10n),
    hash: zeroHash,
    parentHash: zeroHash,
    nonce: '0x0000000000000000',
    sha3Uncles: emptyUnclesHash,
    logsBloom: zeroBloom,
    transactionsRoot: zeroHash,
    stateRoot: zeroHash,
    receiptsRoot,
    miner: zeroAddress,
    difficulty: '0x0',
    extraData: '0x',
    size: '0x1',
    gasLimit: toHex(30_000_000n),
    gasUsed: toHex(63_000n),
    timestamp: toHex(1n),
    transactions: [txHash0, txHash1],
    uncles: [],
  } satisfies JSONRPCBlock;
  const targetBlockHash = bytesToHex(createBlockHeaderFromRPC(targetBlockTemplate).hash());
  const targetBlock = {
    ...targetBlockTemplate,
    hash: targetBlockHash,
  } satisfies JSONRPCBlock;
  const anchorBlockTemplate = {
    ...targetBlockTemplate,
    number: toHex(11n),
    parentHash: targetBlockHash,
    receiptsRoot: zeroHash,
    gasUsed: toHex(0n),
    timestamp: toHex(2n),
    transactions: [],
  } satisfies JSONRPCBlock;
  const anchorBlockHash = bytesToHex(createBlockHeaderFromRPC(anchorBlockTemplate).hash());
  const anchorBlock = {
    ...anchorBlockTemplate,
    hash: anchorBlockHash,
  } satisfies JSONRPCBlock;
  receipt0.blockHash = targetBlockHash;
  receipt1.blockHash = targetBlockHash;

  const executionClient = {
    getTransactionReceipt: async ({ hash }: { hash: Hex }) => {
      if (hash === txHash0) return receipt0;
      if (hash === txHash1) return receipt1;
      throw new Error(`Unexpected receipt request for ${hash}`);
    },
    getBlock: async ({ blockHash }: { blockHash: Hex }) => {
      expect(blockHash).toBe(targetBlockHash);
      return { transactions: [txHash0, txHash1] };
    },
    request: async ({
      method,
      params,
    }: {
      method: 'eth_getBlockByHash' | 'eth_getBlockByNumber';
      params: [Hex, true];
    }) => {
      if (method === 'eth_getBlockByHash') {
        if (params[0] === targetBlockHash) return targetBlock;
        if (params[0] === anchorBlockHash) return anchorBlock;
        throw new Error(`Unexpected header request for ${method} ${params[0]}`);
      } else {
        expect(params[0]).toBe(toHex(10n));
        return targetBlock;
      }
    },
  } as unknown as EthereumExecutionClient;
  const argonFinalizedExecutionHeader = { blockHash: anchorBlockHash, blockNumber: 11n };

  const combinedProof = await buildEthereumEventProof(
    { executionClient },
    argonFinalizedExecutionHeader,
    [
      [
        { txHash: txHash0, receipt: receipt0 },
        { txHash: txHash1, receipt: receipt1 },
      ],
    ],
  );
  const individualProof0 = await buildEthereumEventProof(
    { executionClient },
    argonFinalizedExecutionHeader,
    [[{ txHash: txHash0, receipt: receipt0 }]],
  );
  const individualProof1 = await buildEthereumEventProof(
    { executionClient },
    argonFinalizedExecutionHeader,
    [[{ txHash: txHash1, receipt: receipt1 }]],
  );

  expect(combinedProof.blocks).toHaveLength(1);
  expect(combinedProof.executionBlockProof.targetToAnchorHeaderChain).toHaveLength(1);
  expect(combinedProof.blocks[0].receiptLogs.map(log => log.transactionIndex)).toEqual([0, 1]);
  expect(combinedProof.blocks[0].targetBlockNumber).toBe(10);
  expect(
    combinedProof.blocks[0].receiptProof.receipts.map(receipt => receipt.transactionIndex),
  ).toEqual([0, 1]);
  expect(combinedProof.blocks[0].receiptProof.nodes.length).toBeLessThan(
    individualProof0.blocks[0].receiptProof.nodes.length +
      individualProof1.blocks[0].receiptProof.nodes.length,
  );

  for (const receiptRef of combinedProof.blocks[0].receiptProof.receipts) {
    const proofNodes = receiptRef.nodeIndexes.map(nodeIndex =>
      hexToBytes(combinedProof.blocks[0].receiptProof.nodes[nodeIndex]),
    );
    const recoveredReceipt = await verifyMPTWithMerkleProof(
      await createMPT(),
      hexToBytes(receiptsRoot),
      encodeReceiptTrieKey(receiptRef.transactionIndex),
      proofNodes,
    );

    expect(bytesToHex(recoveredReceipt!)).toBe(
      bytesToHex(
        encodeEthereumReceiptForProof(receiptRef.transactionIndex === 0 ? receipt0 : receipt1),
      ),
    );
  }
});

it('shares one execution header chain across multiple proved execution blocks', async () => {
  const txHash0 = `0x${'aa'.repeat(32)}`;
  const txHash1 = `0x${'bb'.repeat(32)}`;
  const zeroHash = `0x${'00'.repeat(32)}`;
  const zeroBloom = `0x${'00'.repeat(256)}`;
  const zeroAddress = `0x${'00'.repeat(20)}`;
  const emptyUnclesHash =
    '0x1dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347' as Hex;
  const receipt0 = {
    type: 'legacy',
    status: 'success',
    cumulativeGasUsed: 21_000n,
    logsBloom: zeroBloom,
    logs: [
      {
        address: `0x${'11'.repeat(20)}`,
        topics: [`0x${'22'.repeat(32)}`],
        data: '0x1234' as Hex,
      },
    ],
    transactionHash: txHash0,
    transactionIndex: 0,
  } as unknown as EthereumReceipt;
  const receipt1 = {
    type: 'legacy',
    status: 'success',
    cumulativeGasUsed: 42_000n,
    logsBloom: zeroBloom,
    logs: [
      {
        address: `0x${'33'.repeat(20)}`,
        topics: [`0x${'44'.repeat(32)}`],
        data: '0xabcd' as Hex,
      },
    ],
    transactionHash: txHash1,
    transactionIndex: 0,
  } as unknown as EthereumReceipt;

  const trie0 = await createMPT();
  await trie0.put(
    encodeReceiptTrieKey(receipt0.transactionIndex),
    encodeEthereumReceiptForProof(receipt0),
  );
  const receiptsRoot0 = bytesToHex(trie0.root());

  const trie1 = await createMPT();
  await trie1.put(
    encodeReceiptTrieKey(receipt1.transactionIndex),
    encodeEthereumReceiptForProof(receipt1),
  );
  const receiptsRoot1 = bytesToHex(trie1.root());

  const olderBlockTemplate = {
    number: toHex(10n),
    hash: zeroHash,
    parentHash: zeroHash,
    nonce: '0x0000000000000000',
    sha3Uncles: emptyUnclesHash,
    logsBloom: zeroBloom,
    transactionsRoot: zeroHash,
    stateRoot: zeroHash,
    receiptsRoot: receiptsRoot0,
    miner: zeroAddress,
    difficulty: '0x0',
    extraData: '0x',
    size: '0x1',
    gasLimit: toHex(30_000_000n),
    gasUsed: toHex(21_000n),
    timestamp: toHex(1n),
    transactions: [txHash0],
    uncles: [],
  } satisfies JSONRPCBlock;
  const olderBlockHash = bytesToHex(createBlockHeaderFromRPC(olderBlockTemplate).hash());
  const olderBlock = {
    ...olderBlockTemplate,
    hash: olderBlockHash,
  } satisfies JSONRPCBlock;

  const newerBlockTemplate = {
    ...olderBlockTemplate,
    number: toHex(11n),
    parentHash: olderBlockHash,
    receiptsRoot: receiptsRoot1,
    gasUsed: toHex(42_000n),
    timestamp: toHex(2n),
    transactions: [txHash1],
  } satisfies JSONRPCBlock;
  const newerBlockHash = bytesToHex(createBlockHeaderFromRPC(newerBlockTemplate).hash());
  const newerBlock = {
    ...newerBlockTemplate,
    hash: newerBlockHash,
  } satisfies JSONRPCBlock;
  const anchorBlockTemplate = {
    ...olderBlockTemplate,
    number: toHex(12n),
    parentHash: newerBlockHash,
    receiptsRoot: zeroHash,
    gasUsed: toHex(0n),
    timestamp: toHex(3n),
    transactions: [],
  } satisfies JSONRPCBlock;
  const anchorBlockHash = bytesToHex(createBlockHeaderFromRPC(anchorBlockTemplate).hash());
  const anchorBlock = {
    ...anchorBlockTemplate,
    hash: anchorBlockHash,
  } satisfies JSONRPCBlock;

  receipt0.blockHash = olderBlockHash;
  receipt1.blockHash = newerBlockHash;

  const executionClient = {
    getTransactionReceipt: async ({ hash }: { hash: Hex }) => {
      if (hash === txHash0) return receipt0;
      if (hash === txHash1) return receipt1;
      throw new Error(`Unexpected receipt request for ${hash}`);
    },
    getBlock: async ({ blockHash }: { blockHash: Hex }) => {
      if (blockHash === olderBlockHash) {
        return { transactions: [txHash0] };
      }
      if (blockHash === newerBlockHash) {
        return { transactions: [txHash1] };
      }
      throw new Error(`Unexpected block request for ${blockHash}`);
    },
    request: async ({
      method,
      params,
    }: {
      method: 'eth_getBlockByHash' | 'eth_getBlockByNumber';
      params: [Hex, true];
    }) => {
      if (method === 'eth_getBlockByHash') {
        if (params[0] === olderBlockHash) return olderBlock;
        if (params[0] === newerBlockHash) return newerBlock;
        if (params[0] === anchorBlockHash) return anchorBlock;
      }
      if (method === 'eth_getBlockByNumber') {
        if (params[0] === toHex(10n)) return olderBlock;
        if (params[0] === toHex(11n)) return newerBlock;
        if (params[0] === toHex(12n)) return anchorBlock;
      }

      throw new Error(`Unexpected header request for ${method} ${params[0]}`);
    },
  } as unknown as EthereumExecutionClient;
  const proof = await buildEthereumEventProof(
    { executionClient },
    { blockHash: anchorBlockHash, blockNumber: 12n },
    [[{ txHash: txHash0, receipt: receipt0 }], [{ txHash: txHash1, receipt: receipt1 }]],
  );

  expect(proof.executionBlockProof.targetToAnchorHeaderChain).toHaveLength(2);
  expect(proof.blocks).toHaveLength(2);
  expect(proof.blocks.map(block => block.targetBlockNumber)).toEqual([10, 11]);
  expect(proof.blocks[0].receiptProof.receipts[0]?.transactionIndex).toBe(0);
  expect(proof.blocks[1].receiptProof.receipts[0]?.transactionIndex).toBe(0);
});

it('uses an empty shared header path when the target block is the Argon finalized execution header', async () => {
  const txHash = `0x${'aa'.repeat(32)}`;
  const zeroHash = `0x${'00'.repeat(32)}`;
  const zeroBloom = `0x${'00'.repeat(256)}`;
  const zeroAddress = `0x${'00'.repeat(20)}`;
  const emptyUnclesHash =
    '0x1dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347' as Hex;
  const receipt = {
    type: 'legacy',
    status: 'success',
    cumulativeGasUsed: 21_000n,
    logsBloom: zeroBloom,
    logs: [
      {
        address: `0x${'55'.repeat(20)}`,
        topics: [`0x${'66'.repeat(32)}`],
        data: '0x1234' as Hex,
      },
    ],
    transactionHash: txHash,
    transactionIndex: 0,
  } as unknown as EthereumReceipt;

  const trie = await createMPT();
  await trie.put(
    encodeReceiptTrieKey(receipt.transactionIndex),
    encodeEthereumReceiptForProof(receipt),
  );
  const receiptsRoot = bytesToHex(trie.root());
  const anchorBlockTemplate = {
    number: toHex(10n),
    hash: zeroHash,
    parentHash: zeroHash,
    nonce: '0x0000000000000000',
    sha3Uncles: emptyUnclesHash,
    logsBloom: zeroBloom,
    transactionsRoot: zeroHash,
    stateRoot: zeroHash,
    receiptsRoot,
    miner: zeroAddress,
    difficulty: '0x0',
    extraData: '0x',
    size: '0x1',
    gasLimit: toHex(30_000_000n),
    gasUsed: toHex(21_000n),
    timestamp: toHex(1n),
    transactions: [txHash],
    uncles: [],
  } satisfies JSONRPCBlock;
  const anchorBlockHash = bytesToHex(createBlockHeaderFromRPC(anchorBlockTemplate).hash());
  const anchorBlock = {
    ...anchorBlockTemplate,
    hash: anchorBlockHash,
  } satisfies JSONRPCBlock;
  receipt.blockHash = anchorBlockHash;

  const executionClient = {
    getTransactionReceipt: async ({ hash }: { hash: Hex }) => {
      if (hash === txHash) return receipt;
      throw new Error(`Unexpected receipt request for ${hash}`);
    },
    getBlock: async ({ blockHash }: { blockHash: Hex }) => {
      expect(blockHash).toBe(anchorBlockHash);
      return { transactions: [txHash] };
    },
    request: async ({
      method,
      params,
    }: {
      method: 'eth_getBlockByHash' | 'eth_getBlockByNumber';
      params: [Hex, true];
    }) => {
      if (method === 'eth_getBlockByHash') {
        expect(params[0]).toBe(anchorBlockHash);
      } else {
        expect(params[0]).toBe(toHex(10n));
      }

      return anchorBlock;
    },
  } as unknown as EthereumExecutionClient;
  const proof = await buildEthereumEventProof(
    { executionClient },
    { blockHash: anchorBlockHash, blockNumber: 10n },
    [[{ txHash, receipt }]],
  );

  expect(proof.executionBlockProof.anchorBlockHash).toBe(anchorBlockHash);
  expect(proof.executionBlockProof.targetToAnchorHeaderChain).toHaveLength(0);
  expect(proof.blocks).toHaveLength(1);
  expect(proof.blocks[0].targetBlockNumber).toBe(10);
});

it('decodes TransferToArgonStarted logs into gateway activity state', async () => {
  const from = `0x${'11'.repeat(20)}`;
  const token = `0x${'22'.repeat(20)}`;
  const argonAccountId = `0x${'33'.repeat(32)}`;
  const topics = encodeEventTopics({
    abi: mintingGatewayAbi,
    eventName: MintingGatewayEvents.TransferToArgonStarted.name,
    args: { from, token },
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
      argonAccountId,
      {
        gatewayActivityNonce: 7n,
        argonApprovalsNonce: 3n,
        argonCirculation: 500n,
        argonotCirculation: 2_000n,
      },
    ],
  );

  expect(
    decodeEthereumTransferToArgonStartedLog({
      topics: topics as Hex[],
      data,
    }),
  ).toEqual({
    from,
    token,
    amount: 250n,
    argonAccountId,
    gatewayState: {
      gatewayActivityNonce: 7n,
      argonApprovalsNonce: 3n,
      argonCirculation: 500n,
      argonotCirculation: 2_000n,
    },
  });
});

it('decodes non-transfer gateway activity logs', async () => {
  const councilHash = `0x${'44'.repeat(32)}`;
  const relayerArgonAccountId = `0x${'55'.repeat(32)}`;
  const topics = encodeEventTopics({
    abi: mintingGatewayAbi,
    eventName: MintingGatewayEvents.GlobalIssuanceCouncilRotated.name,
  });
  const data = encodeAbiParameters(
    [
      { name: 'councilHash', type: 'bytes32' },
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
      councilHash,
      relayerArgonAccountId,
      {
        gatewayActivityNonce: 9n,
        argonApprovalsNonce: 4n,
        argonCirculation: 500n,
        argonotCirculation: 2_000n,
      },
    ],
  );

  expect(
    decodeEthereumGatewayActivityLog({
      topics: topics as Hex[],
      data,
    }),
  ).toEqual({
    kind: MintingGatewayEvents.GlobalIssuanceCouncilRotated.name,
    councilHash,
    relayerArgonAccountId,
    gatewayState: {
      gatewayActivityNonce: 9n,
      argonApprovalsNonce: 4n,
      argonCirculation: 500n,
      argonotCirculation: 2_000n,
    },
  });
});

it('builds a proveGatewayActivity payload from uncovered TransferToArgonStarted locators', async () => {
  const gatewayAddress = `0x${'77'.repeat(20)}`;
  const zeroHash = `0x${'00'.repeat(32)}`;
  const zeroBloom = `0x${'00'.repeat(256)}`;
  const zeroAddress = `0x${'00'.repeat(20)}`;
  const emptyUnclesHash =
    '0x1dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347' as Hex;
  const txHash0 = `0x${'10'.repeat(32)}`;
  const txHash1 = `0x${'11'.repeat(32)}`;
  const txHash2 = `0x${'12'.repeat(32)}`;
  const block10Logs = [
    createTransferToArgonStartedBlockLog({
      gatewayAddress,
      txHash: txHash0,
      transactionIndex: 0,
      logIndex: 0,
      blockHash: zeroHash,
      blockNumber: 10n,
      nonce: 1n,
      argonAccountId: `0x${'41'.repeat(32)}`,
    }),
    createTransferToArgonStartedBlockLog({
      gatewayAddress,
      txHash: txHash1,
      transactionIndex: 1,
      logIndex: 0,
      blockHash: zeroHash,
      blockNumber: 10n,
      nonce: 2n,
      argonAccountId: `0x${'42'.repeat(32)}`,
    }),
  ];
  const block11Logs = [
    createTransferToArgonStartedBlockLog({
      gatewayAddress,
      txHash: txHash2,
      transactionIndex: 0,
      logIndex: 0,
      blockHash: zeroHash,
      blockNumber: 11n,
      nonce: 3n,
      argonAccountId: `0x${'43'.repeat(32)}`,
    }),
  ];
  const receipt0 = {
    type: 'legacy',
    status: 'success',
    cumulativeGasUsed: 21_000n,
    logsBloom: zeroBloom,
    logs: [{ address: gatewayAddress, topics: block10Logs[0].topics, data: block10Logs[0].data }],
    transactionHash: txHash0,
    transactionIndex: 0,
  } as unknown as EthereumReceipt;
  const receipt1 = {
    type: 'legacy',
    status: 'success',
    cumulativeGasUsed: 42_000n,
    logsBloom: zeroBloom,
    logs: [{ address: gatewayAddress, topics: block10Logs[1].topics, data: block10Logs[1].data }],
    transactionHash: txHash1,
    transactionIndex: 1,
  } as unknown as EthereumReceipt;
  const receipt2 = {
    type: 'legacy',
    status: 'success',
    cumulativeGasUsed: 21_000n,
    logsBloom: zeroBloom,
    logs: [{ address: gatewayAddress, topics: block11Logs[0].topics, data: block11Logs[0].data }],
    transactionHash: txHash2,
    transactionIndex: 0,
  } as unknown as EthereumReceipt;

  const trie10 = await createMPT();
  await trie10.put(encodeReceiptTrieKey(0), encodeEthereumReceiptForProof(receipt0));
  await trie10.put(encodeReceiptTrieKey(1), encodeEthereumReceiptForProof(receipt1));
  const receiptsRoot10 = bytesToHex(trie10.root());

  const trie11 = await createMPT();
  await trie11.put(encodeReceiptTrieKey(0), encodeEthereumReceiptForProof(receipt2));
  const receiptsRoot11 = bytesToHex(trie11.root());

  const block10Template = {
    number: toHex(10n),
    hash: zeroHash,
    parentHash: zeroHash,
    nonce: '0x0000000000000000',
    sha3Uncles: emptyUnclesHash,
    logsBloom: zeroBloom,
    transactionsRoot: zeroHash,
    stateRoot: zeroHash,
    receiptsRoot: receiptsRoot10,
    miner: zeroAddress,
    difficulty: '0x0',
    extraData: '0x',
    size: '0x1',
    gasLimit: toHex(30_000_000n),
    gasUsed: toHex(63_000n),
    timestamp: toHex(1n),
    transactions: [txHash0, txHash1],
    uncles: [],
  } satisfies JSONRPCBlock;
  const block10Hash = bytesToHex(createBlockHeaderFromRPC(block10Template).hash());
  const block10 = { ...block10Template, hash: block10Hash } satisfies JSONRPCBlock;

  const block11Template = {
    ...block10Template,
    number: toHex(11n),
    parentHash: block10Hash,
    receiptsRoot: receiptsRoot11,
    gasUsed: toHex(21_000n),
    timestamp: toHex(2n),
    transactions: [txHash2],
  } satisfies JSONRPCBlock;
  const block11Hash = bytesToHex(createBlockHeaderFromRPC(block11Template).hash());
  const block11 = { ...block11Template, hash: block11Hash } satisfies JSONRPCBlock;
  const block12Template = {
    ...block10Template,
    number: toHex(12n),
    parentHash: block11Hash,
    receiptsRoot: zeroHash,
    gasUsed: toHex(0n),
    timestamp: toHex(3n),
    transactions: [],
  } satisfies JSONRPCBlock;
  const block12Hash = bytesToHex(createBlockHeaderFromRPC(block12Template).hash());
  const block12 = { ...block12Template, hash: block12Hash } satisfies JSONRPCBlock;

  block10Logs.forEach(log => {
    log.blockHash = block10Hash;
  });
  block11Logs.forEach(log => {
    log.blockHash = block11Hash;
  });
  receipt0.blockHash = block10Hash;
  receipt1.blockHash = block10Hash;
  receipt2.blockHash = block11Hash;

  const executionClient = {
    readContract: async ({
      functionName,
      args,
    }: {
      functionName: 'latestActivityBlockLocatorIndex' | 'activityBlockLocators';
      args?: [bigint];
    }) => {
      if (functionName === 'latestActivityBlockLocatorIndex') {
        return 2n;
      }

      if (args?.[0] === 1n) {
        return [10n, 1n, 2n];
      }
      if (args?.[0] === 2n) {
        return [11n, 3n, 3n];
      }

      throw new Error(`Unexpected locator request ${String(args?.[0])}`);
    },
    getLogs: async ({ fromBlock }: { fromBlock: bigint }) => {
      if (fromBlock === 10n) return block10Logs;
      if (fromBlock === 11n) return block11Logs;
      throw new Error(`Unexpected logs request for block ${fromBlock}`);
    },
    getTransactionReceipt: async ({ hash }: { hash: Hex }) => {
      if (hash === txHash0) return receipt0;
      if (hash === txHash1) return receipt1;
      if (hash === txHash2) return receipt2;
      throw new Error(`Unexpected receipt request for ${hash}`);
    },
    getBlock: async ({ blockHash }: { blockHash: Hex }) => {
      if (blockHash === block10Hash) return { transactions: [txHash0, txHash1] };
      if (blockHash === block11Hash) return { transactions: [txHash2] };
      throw new Error(`Unexpected block request for ${blockHash}`);
    },
    request: async ({
      method,
      params,
    }: {
      method: 'eth_getBlockByHash' | 'eth_getBlockByNumber';
      params: [Hex, true];
    }) => {
      if (method === 'eth_getBlockByHash') {
        if (params[0] === block10Hash) return block10;
        if (params[0] === block11Hash) return block11;
        if (params[0] === block12Hash) return block12;
      }
      if (method === 'eth_getBlockByNumber') {
        if (params[0] === toHex(10n)) return block10;
        if (params[0] === toHex(11n)) return block11;
        if (params[0] === toHex(12n)) return block12;
      }

      throw new Error(`Unexpected header request for ${method} ${params[0]}`);
    },
  } as unknown as EthereumExecutionClient;
  const argonClient = {
    query: {
      crosschainTransfer: {
        gatewayStateBySourceChain: async () => ({
          isSome: true,
          unwrap: () => ({
            gatewayActivityNonce: {
              toBigInt: () => 1n,
            },
          }),
        }),
      },
      ethereumVerifier: {
        latestExecutionHeaderAnchorBlockHash: async () => ({
          isNone: false,
          unwrap: () => ({ toHex: () => block12Hash }),
        }),
        executionHeaderAnchors: async (blockHash: Hex) => {
          expect(blockHash).toBe(block12Hash);

          return {
            isNone: false,
            unwrap: () => ({
              blockNumber: {
                toBigInt: () => 12n,
              },
            }),
          };
        },
      },
    },
    consts: gatewayProofConsts,
  } as unknown as IArgonQueryable & Pick<ArgonClient, 'consts'>;

  const payload = await buildGatewayActivityProofPayload(argonClient, {
    gatewayAddress,
    executionClient,
    throughExecutionBlockNumber: 11n,
  });

  expect(payload?.previousGatewayActivityNonce).toBe(1n);
  expect(payload?.gatewayActivityNonceRange).toEqual({ start: 2n, end: 3n });
  expect(payload?.executionBlockNumberRange).toEqual({ start: 10n, end: 11n });
  expect(payload?.activities.map(activity => activity.kind)).toEqual([
    MintingGatewayEvents.TransferToArgonStarted.name,
    MintingGatewayEvents.TransferToArgonStarted.name,
  ]);
  expect(payload?.proof.blocks).toHaveLength(2);
  expect(payload?.proof.blocks.map(block => block.targetBlockNumber)).toEqual([10, 11]);
  expect(payload?.proof.blocks[0].receiptLogs.map(log => log.transactionIndex)).toEqual([1]);
  expect(payload?.proof.blocks[1].receiptLogs.map(log => log.transactionIndex)).toEqual([0]);
});

it('builds a gateway activity payload from mixed gateway events', async () => {
  const gatewayAddress = `0x${'77'.repeat(20)}`;
  const zeroHash = `0x${'00'.repeat(32)}`;
  const zeroBloom = `0x${'00'.repeat(256)}`;
  const zeroAddress = `0x${'00'.repeat(20)}`;
  const emptyUnclesHash =
    '0x1dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347' as Hex;
  const txHash = `0x${'10'.repeat(32)}`;
  const transferLog = createTransferToArgonStartedBlockLog({
    gatewayAddress,
    txHash,
    transactionIndex: 0,
    logIndex: 0,
    blockHash: zeroHash,
    blockNumber: 10n,
    nonce: 1n,
    argonAccountId: `0x${'41'.repeat(32)}`,
  });
  const councilRotationLog = createGlobalIssuanceCouncilRotatedBlockLog({
    gatewayAddress,
    txHash,
    transactionIndex: 0,
    logIndex: 1,
    blockHash: zeroHash,
    blockNumber: 10n,
    nonce: 2n,
  });
  const receipt = {
    type: 'legacy',
    status: 'success',
    cumulativeGasUsed: 42_000n,
    logsBloom: zeroBloom,
    logs: [
      { address: gatewayAddress, topics: transferLog.topics, data: transferLog.data },
      { address: gatewayAddress, topics: councilRotationLog.topics, data: councilRotationLog.data },
    ],
    transactionHash: txHash,
    transactionIndex: 0,
  } as unknown as EthereumReceipt;

  const trie = await createMPT();
  await trie.put(encodeReceiptTrieKey(0), encodeEthereumReceiptForProof(receipt));
  const receiptsRoot = bytesToHex(trie.root());

  const blockTemplate = {
    number: toHex(10n),
    hash: zeroHash,
    parentHash: zeroHash,
    nonce: '0x0000000000000000',
    sha3Uncles: emptyUnclesHash,
    logsBloom: zeroBloom,
    transactionsRoot: zeroHash,
    stateRoot: zeroHash,
    receiptsRoot,
    miner: zeroAddress,
    difficulty: '0x0',
    extraData: '0x',
    size: '0x1',
    gasLimit: toHex(30_000_000n),
    gasUsed: toHex(42_000n),
    timestamp: toHex(1n),
    transactions: [txHash],
    uncles: [],
  } satisfies JSONRPCBlock;
  const blockHash = bytesToHex(createBlockHeaderFromRPC(blockTemplate).hash());
  const block = { ...blockTemplate, hash: blockHash } satisfies JSONRPCBlock;
  const block11Template = {
    ...blockTemplate,
    number: toHex(11n),
    parentHash: blockHash,
    receiptsRoot: zeroHash,
    gasUsed: toHex(0n),
    timestamp: toHex(2n),
    transactions: [],
  } satisfies JSONRPCBlock;
  const block11Hash = bytesToHex(createBlockHeaderFromRPC(block11Template).hash());
  const block11 = { ...block11Template, hash: block11Hash } satisfies JSONRPCBlock;
  transferLog.blockHash = blockHash;
  councilRotationLog.blockHash = blockHash;
  receipt.blockHash = blockHash;

  const executionClient = {
    readContract: async ({
      functionName,
      args,
    }: {
      functionName: 'latestActivityBlockLocatorIndex' | 'activityBlockLocators';
      args?: [bigint];
    }) => {
      if (functionName === 'latestActivityBlockLocatorIndex') {
        return 1n;
      }

      if (args?.[0] === 1n) {
        return [10n, 1n, 2n];
      }

      throw new Error(`Unexpected locator request ${String(args?.[0])}`);
    },
    getLogs: async () => [transferLog, councilRotationLog],
    getTransactionReceipt: async ({ hash }: { hash: Hex }) => {
      if (hash === txHash) return receipt;
      throw new Error(`Unexpected receipt request for ${hash}`);
    },
    getBlock: async ({ blockHash: queriedBlockHash }: { blockHash: Hex }) => {
      expect(queriedBlockHash).toBe(blockHash);
      return { transactions: [txHash] };
    },
    request: async ({
      method,
      params,
    }: {
      method: 'eth_getBlockByHash' | 'eth_getBlockByNumber';
      params: [Hex, true];
    }) => {
      if (method === 'eth_getBlockByHash') {
        if (params[0] === blockHash) return block;
        if (params[0] === block11Hash) return block11;
      }
      if (method === 'eth_getBlockByNumber') {
        if (params[0] === toHex(10n)) return block;
        if (params[0] === toHex(11n)) return block11;
      }

      throw new Error(`Unexpected header request for ${method} ${params[0]}`);
    },
  } as unknown as EthereumExecutionClient;
  const argonClient = {
    query: {
      crosschainTransfer: {
        gatewayStateBySourceChain: async () => ({
          isSome: false,
        }),
      },
      ethereumVerifier: {
        latestExecutionHeaderAnchorBlockHash: async () => ({
          isNone: false,
          unwrap: () => ({ toHex: () => block11Hash }),
        }),
        executionHeaderAnchors: async (queriedBlockHash: Hex) => {
          expect(queriedBlockHash).toBe(block11Hash);

          return {
            isNone: false,
            unwrap: () => ({
              blockNumber: {
                toBigInt: () => 11n,
              },
            }),
          };
        },
      },
    },
    consts: gatewayProofConsts,
  } as unknown as IArgonQueryable & Pick<ArgonClient, 'consts'>;

  const payload = await buildGatewayActivityProofPayload(argonClient, {
    gatewayAddress,
    executionClient,
  });

  expect(payload?.previousGatewayActivityNonce).toBe(0n);
  expect(payload?.gatewayActivityNonceRange).toEqual({ start: 1n, end: 2n });
  expect(payload?.executionBlockNumberRange).toEqual({ start: 10n, end: 10n });
  expect(payload?.activities.map(activity => activity.kind)).toEqual([
    MintingGatewayEvents.TransferToArgonStarted.name,
    MintingGatewayEvents.GlobalIssuanceCouncilRotated.name,
  ]);
  expect(payload?.proof.blocks).toHaveLength(1);
  expect(payload?.proof.blocks[0].receiptLogs.map(log => log.transactionIndex)).toEqual([0, 0]);
});

it('splits one execution block into multiple proof blocks when the activity bound is lower', async () => {
  const gatewayAddress = `0x${'91'.repeat(20)}`;
  const zeroHash = `0x${'00'.repeat(32)}`;
  const zeroBloom = `0x${'00'.repeat(256)}`;
  const zeroAddress = `0x${'00'.repeat(20)}`;
  const emptyUnclesHash =
    '0x1dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347' as Hex;
  const txHash = `0x${'93'.repeat(32)}`;
  const logs = [1n, 2n, 3n].map((nonce, index) =>
    createTransferToArgonStartedBlockLog({
      gatewayAddress,
      txHash,
      transactionIndex: 0,
      logIndex: index,
      blockHash: zeroHash,
      blockNumber: 10n,
      nonce,
      argonAccountId: toHex(index + 1, { size: 32 }),
    }),
  );
  const receipt = {
    type: 'legacy',
    status: 'success',
    cumulativeGasUsed: 63_000n,
    logsBloom: zeroBloom,
    logs: logs.map(log => ({ address: gatewayAddress, topics: log.topics, data: log.data })),
    transactionHash: txHash,
    transactionIndex: 0,
  } as unknown as EthereumReceipt;

  const trie = await createMPT();
  await trie.put(encodeReceiptTrieKey(0), encodeEthereumReceiptForProof(receipt));
  const receiptsRoot = bytesToHex(trie.root());
  const blockTemplate = {
    number: toHex(10n),
    hash: zeroHash,
    parentHash: zeroHash,
    nonce: '0x0000000000000000',
    sha3Uncles: emptyUnclesHash,
    logsBloom: zeroBloom,
    transactionsRoot: zeroHash,
    stateRoot: zeroHash,
    receiptsRoot,
    miner: zeroAddress,
    difficulty: '0x0',
    extraData: '0x',
    size: '0x1',
    gasLimit: toHex(30_000_000n),
    gasUsed: toHex(63_000n),
    timestamp: toHex(1n),
    transactions: [txHash],
    uncles: [],
  } satisfies JSONRPCBlock;
  const blockHash = bytesToHex(createBlockHeaderFromRPC(blockTemplate).hash());
  const block = { ...blockTemplate, hash: blockHash } satisfies JSONRPCBlock;
  const block11Template = {
    ...blockTemplate,
    number: toHex(11n),
    parentHash: blockHash,
    receiptsRoot: zeroHash,
    gasUsed: toHex(0n),
    timestamp: toHex(2n),
    transactions: [],
  } satisfies JSONRPCBlock;
  const block11Hash = bytesToHex(createBlockHeaderFromRPC(block11Template).hash());
  const block11 = { ...block11Template, hash: block11Hash } satisfies JSONRPCBlock;
  for (const log of logs) {
    log.blockHash = blockHash;
  }
  receipt.blockHash = blockHash;

  const executionClient = {
    readContract: async ({
      functionName,
      args,
    }: {
      functionName: 'latestActivityBlockLocatorIndex' | 'activityBlockLocators';
      args?: [bigint];
    }) => {
      if (functionName === 'latestActivityBlockLocatorIndex') {
        return 1n;
      }

      if (args?.[0] === 1n) {
        return [10n, 1n, 3n];
      }

      throw new Error(`Unexpected locator request ${String(args?.[0])}`);
    },
    getLogs: async () => logs,
    getTransactionReceipt: async ({ hash }: { hash: Hex }) => {
      if (hash === txHash) return receipt;
      throw new Error(`Unexpected receipt request for ${hash}`);
    },
    getBlock: async ({ blockHash: queriedBlockHash }: { blockHash: Hex }) => {
      expect(queriedBlockHash).toBe(blockHash);
      return { transactions: [txHash] };
    },
    request: async ({
      method,
      params,
    }: {
      method: 'eth_getBlockByHash' | 'eth_getBlockByNumber';
      params: [Hex, true];
    }) => {
      if (method === 'eth_getBlockByHash') {
        if (params[0] === blockHash) return block;
        if (params[0] === block11Hash) return block11;
      }
      if (method === 'eth_getBlockByNumber') {
        if (params[0] === toHex(10n)) return block;
        if (params[0] === toHex(11n)) return block11;
      }

      throw new Error(`Unexpected header request for ${method} ${params[0]}`);
    },
  } as unknown as EthereumExecutionClient;
  const argonClient = {
    query: {
      crosschainTransfer: {
        gatewayStateBySourceChain: async () => ({
          isSome: false,
        }),
      },
      ethereumVerifier: {
        latestExecutionHeaderAnchorBlockHash: async () => ({
          isNone: false,
          unwrap: () => ({ toHex: () => block11Hash }),
        }),
        executionHeaderAnchors: async () => ({
          isNone: false,
          unwrap: () => ({
            blockNumber: {
              toBigInt: () => 11n,
            },
          }),
        }),
      },
    },
    consts: createGatewayProofConsts({ maxActivitiesPerReceiptProof: 1 }),
  } as unknown as IArgonQueryable & Pick<ArgonClient, 'consts'>;

  const payload = await buildGatewayActivityProofPayload(argonClient, {
    gatewayAddress,
    executionClient,
  });

  expect(payload?.gatewayActivityNonceRange).toEqual({ start: 1n, end: 3n });
  expect(payload?.proof.blocks).toHaveLength(3);
  expect(payload?.proof.blocks.map(block => block.receiptLogs)).toEqual([
    [expect.objectContaining({ transactionIndex: 0 })],
    [expect.objectContaining({ transactionIndex: 0 })],
    [expect.objectContaining({ transactionIndex: 0 })],
  ]);
});

it('resumes from the next gateway activity when a capped payload stops mid block', async () => {
  const gatewayAddress = `0x${'94'.repeat(20)}`;
  const zeroHash = `0x${'00'.repeat(32)}`;
  const zeroBloom = `0x${'00'.repeat(256)}`;
  const zeroAddress = `0x${'00'.repeat(20)}`;
  const emptyUnclesHash =
    '0x1dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347' as Hex;
  const txHash = `0x${'96'.repeat(32)}`;
  const logs = [1n, 2n, 3n].map((nonce, index) =>
    createTransferToArgonStartedBlockLog({
      gatewayAddress,
      txHash,
      transactionIndex: 0,
      logIndex: index,
      blockHash: zeroHash,
      blockNumber: 10n,
      nonce,
      argonAccountId: toHex(index + 1, { size: 32 }),
    }),
  );
  const receipt = {
    type: 'legacy',
    status: 'success',
    cumulativeGasUsed: 63_000n,
    logsBloom: zeroBloom,
    logs: logs.map(log => ({ address: gatewayAddress, topics: log.topics, data: log.data })),
    transactionHash: txHash,
    transactionIndex: 0,
  } as unknown as EthereumReceipt;

  const trie = await createMPT();
  await trie.put(encodeReceiptTrieKey(0), encodeEthereumReceiptForProof(receipt));
  const receiptsRoot = bytesToHex(trie.root());
  const blockTemplate = {
    number: toHex(10n),
    hash: zeroHash,
    parentHash: zeroHash,
    nonce: '0x0000000000000000',
    sha3Uncles: emptyUnclesHash,
    logsBloom: zeroBloom,
    transactionsRoot: zeroHash,
    stateRoot: zeroHash,
    receiptsRoot,
    miner: zeroAddress,
    difficulty: '0x0',
    extraData: '0x',
    size: '0x1',
    gasLimit: toHex(30_000_000n),
    gasUsed: toHex(63_000n),
    timestamp: toHex(1n),
    transactions: [txHash],
    uncles: [],
  } satisfies JSONRPCBlock;
  const blockHash = bytesToHex(createBlockHeaderFromRPC(blockTemplate).hash());
  const block = { ...blockTemplate, hash: blockHash } satisfies JSONRPCBlock;
  const block11Template = {
    ...blockTemplate,
    number: toHex(11n),
    parentHash: blockHash,
    receiptsRoot: zeroHash,
    gasUsed: toHex(0n),
    timestamp: toHex(2n),
    transactions: [],
  } satisfies JSONRPCBlock;
  const block11Hash = bytesToHex(createBlockHeaderFromRPC(block11Template).hash());
  const block11 = { ...block11Template, hash: block11Hash } satisfies JSONRPCBlock;
  for (const log of logs) {
    log.blockHash = blockHash;
  }
  receipt.blockHash = blockHash;

  const executionClient = {
    readContract: async ({
      functionName,
      args,
    }: {
      functionName: 'latestActivityBlockLocatorIndex' | 'activityBlockLocators';
      args?: [bigint];
    }) => {
      if (functionName === 'latestActivityBlockLocatorIndex') {
        return 1n;
      }

      if (args?.[0] === 1n) {
        return [10n, 1n, 3n];
      }

      throw new Error(`Unexpected locator request ${String(args?.[0])}`);
    },
    getLogs: async () => logs,
    getTransactionReceipt: async ({ hash }: { hash: Hex }) => {
      if (hash === txHash) return receipt;
      throw new Error(`Unexpected receipt request for ${hash}`);
    },
    getBlock: async ({ blockHash: queriedBlockHash }: { blockHash: Hex }) => {
      expect(queriedBlockHash).toBe(blockHash);
      return { transactions: [txHash] };
    },
    request: async ({
      method,
      params,
    }: {
      method: 'eth_getBlockByHash' | 'eth_getBlockByNumber';
      params: [Hex, true];
    }) => {
      if (method === 'eth_getBlockByHash') {
        if (params[0] === blockHash) return block;
        if (params[0] === block11Hash) return block11;
      }
      if (method === 'eth_getBlockByNumber') {
        if (params[0] === toHex(10n)) return block;
        if (params[0] === toHex(11n)) return block11;
      }

      throw new Error(`Unexpected header request for ${method} ${params[0]}`);
    },
  } as unknown as EthereumExecutionClient;
  const argonClient = {
    query: {
      crosschainTransfer: {
        gatewayStateBySourceChain: async () => ({
          isSome: true,
          unwrap: () => ({
            gatewayActivityNonce: {
              toBigInt: () => 1n,
            },
          }),
        }),
      },
      ethereumVerifier: {
        latestExecutionHeaderAnchorBlockHash: async () => ({
          isNone: false,
          unwrap: () => ({ toHex: () => block11Hash }),
        }),
        executionHeaderAnchors: async () => ({
          isNone: false,
          unwrap: () => ({
            blockNumber: {
              toBigInt: () => 11n,
            },
          }),
        }),
      },
    },
    consts: createGatewayProofConsts({
      maxActivitiesPerReceiptProof: 1,
      maxReceiptProofsPerExtrinsic: 1,
    }),
  } as unknown as IArgonQueryable & Pick<ArgonClient, 'consts'>;

  const payload = await buildGatewayActivityProofPayload(argonClient, {
    gatewayAddress,
    executionClient,
  });

  expect(payload?.gatewayActivityNonceRange).toEqual({ start: 2n, end: 2n });
  expect(payload?.executionBlockNumberRange).toEqual({ start: 10n, end: 10n });
  expect(payload?.activities.map(activity => activity.gatewayState.gatewayActivityNonce)).toEqual([
    2n,
  ]);
  expect(payload?.proof.blocks).toHaveLength(1);
  expect(payload?.proof.blocks[0].receiptLogs.map(log => log.transactionIndex)).toEqual([0]);
});

it('limits a gateway activity payload to the runtime proof-block bound', async () => {
  const gatewayAddress = `0x${'77'.repeat(20)}`;
  const zeroHash = `0x${'00'.repeat(32)}`;
  const zeroBloom = `0x${'00'.repeat(256)}`;
  const zeroAddress = `0x${'00'.repeat(20)}`;
  const emptyUnclesHash =
    '0x1dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347' as Hex;
  const txHash10 = `0x${'10'.repeat(32)}`;
  const txHash11 = `0x${'11'.repeat(32)}`;
  const log10 = createTransferToArgonStartedBlockLog({
    gatewayAddress,
    txHash: txHash10,
    transactionIndex: 0,
    logIndex: 0,
    blockHash: zeroHash,
    blockNumber: 10n,
    nonce: 1n,
    argonAccountId: `0x${'41'.repeat(32)}`,
  });
  const log11 = createTransferToArgonStartedBlockLog({
    gatewayAddress,
    txHash: txHash11,
    transactionIndex: 0,
    logIndex: 0,
    blockHash: zeroHash,
    blockNumber: 11n,
    nonce: 2n,
    argonAccountId: `0x${'42'.repeat(32)}`,
  });
  const receipt10 = {
    type: 'legacy',
    status: 'success',
    cumulativeGasUsed: 21_000n,
    logsBloom: zeroBloom,
    logs: [{ address: gatewayAddress, topics: log10.topics, data: log10.data }],
    transactionHash: txHash10,
    transactionIndex: 0,
  } as unknown as EthereumReceipt;
  const receipt11 = {
    type: 'legacy',
    status: 'success',
    cumulativeGasUsed: 21_000n,
    logsBloom: zeroBloom,
    logs: [{ address: gatewayAddress, topics: log11.topics, data: log11.data }],
    transactionHash: txHash11,
    transactionIndex: 0,
  } as unknown as EthereumReceipt;

  const trie10 = await createMPT();
  await trie10.put(encodeReceiptTrieKey(0), encodeEthereumReceiptForProof(receipt10));
  const receiptsRoot10 = bytesToHex(trie10.root());
  const trie11 = await createMPT();
  await trie11.put(encodeReceiptTrieKey(0), encodeEthereumReceiptForProof(receipt11));
  const receiptsRoot11 = bytesToHex(trie11.root());

  const block10Template = {
    number: toHex(10n),
    hash: zeroHash,
    parentHash: zeroHash,
    nonce: '0x0000000000000000',
    sha3Uncles: emptyUnclesHash,
    logsBloom: zeroBloom,
    transactionsRoot: zeroHash,
    stateRoot: zeroHash,
    receiptsRoot: receiptsRoot10,
    miner: zeroAddress,
    difficulty: '0x0',
    extraData: '0x',
    size: '0x1',
    gasLimit: toHex(30_000_000n),
    gasUsed: toHex(21_000n),
    timestamp: toHex(1n),
    transactions: [txHash10],
    uncles: [],
  } satisfies JSONRPCBlock;
  const block10Hash = bytesToHex(createBlockHeaderFromRPC(block10Template).hash());
  const block10 = { ...block10Template, hash: block10Hash } satisfies JSONRPCBlock;
  const block11Template = {
    ...block10Template,
    number: toHex(11n),
    parentHash: block10Hash,
    hash: zeroHash,
    receiptsRoot: receiptsRoot11,
    timestamp: toHex(2n),
    transactions: [txHash11],
  } satisfies JSONRPCBlock;
  const block11Hash = bytesToHex(createBlockHeaderFromRPC(block11Template).hash());
  const block11 = { ...block11Template, hash: block11Hash } satisfies JSONRPCBlock;
  const block12Template = {
    ...block10Template,
    number: toHex(12n),
    parentHash: block11Hash,
    receiptsRoot: zeroHash,
    gasUsed: toHex(0n),
    timestamp: toHex(3n),
    transactions: [],
  } satisfies JSONRPCBlock;
  const block12Hash = bytesToHex(createBlockHeaderFromRPC(block12Template).hash());
  const block12 = { ...block12Template, hash: block12Hash } satisfies JSONRPCBlock;
  log10.blockHash = block10Hash;
  log11.blockHash = block11Hash;
  receipt10.blockHash = block10Hash;
  receipt11.blockHash = block11Hash;

  const executionClient = {
    readContract: async ({
      functionName,
      args,
    }: {
      functionName: 'latestActivityBlockLocatorIndex' | 'activityBlockLocators';
      args?: [bigint];
    }) => {
      if (functionName === 'latestActivityBlockLocatorIndex') {
        return 2n;
      }

      if (args?.[0] === 1n) {
        return [10n, 1n, 1n];
      }
      if (args?.[0] === 2n) {
        return [11n, 2n, 2n];
      }

      throw new Error(`Unexpected locator request ${String(args?.[0])}`);
    },
    getLogs: async ({ fromBlock }: { fromBlock: bigint }) => {
      if (fromBlock === 10n) return [log10];
      if (fromBlock === 11n) return [log11];
      throw new Error(`Unexpected getLogs block ${fromBlock}`);
    },
    getTransactionReceipt: async ({ hash }: { hash: Hex }) => {
      if (hash === txHash10) return receipt10;
      if (hash === txHash11) return receipt11;
      throw new Error(`Unexpected receipt request for ${hash}`);
    },
    getBlock: async ({ blockHash }: { blockHash: Hex }) => {
      if (blockHash === block10Hash) return { transactions: [txHash10] };
      if (blockHash === block11Hash) return { transactions: [txHash11] };
      throw new Error(`Unexpected block request for ${blockHash}`);
    },
    request: async ({
      method,
      params,
    }: {
      method: 'eth_getBlockByHash' | 'eth_getBlockByNumber';
      params: [Hex, true];
    }) => {
      if (method === 'eth_getBlockByHash') {
        if (params[0] === block10Hash) return block10;
        if (params[0] === block11Hash) return block11;
        if (params[0] === block12Hash) return block12;
      }
      if (method === 'eth_getBlockByNumber') {
        if (params[0] === toHex(10n)) return block10;
        if (params[0] === toHex(11n)) return block11;
        if (params[0] === toHex(12n)) return block12;
      }

      throw new Error(`Unexpected header request for ${method} ${params[0]}`);
    },
  } as unknown as EthereumExecutionClient;
  const argonClient = {
    query: {
      crosschainTransfer: {
        gatewayStateBySourceChain: async () => ({
          isSome: false,
        }),
      },
      ethereumVerifier: {
        latestExecutionHeaderAnchorBlockHash: async () => ({
          isNone: false,
          unwrap: () => ({ toHex: () => block12Hash }),
        }),
        executionHeaderAnchors: async () => ({
          isNone: false,
          unwrap: () => ({
            blockNumber: {
              toBigInt: () => 12n,
            },
          }),
        }),
      },
    },
    consts: createGatewayProofConsts({ maxReceiptProofsPerExtrinsic: 1 }),
  } as unknown as IArgonQueryable & Pick<ArgonClient, 'consts'>;

  const payload = await buildGatewayActivityProofPayload(argonClient, {
    gatewayAddress,
    executionClient,
  });

  expect(payload?.gatewayActivityNonceRange).toEqual({ start: 1n, end: 1n });
  expect(payload?.executionBlockNumberRange).toEqual({ start: 10n, end: 10n });
  expect(payload?.proof.blocks).toHaveLength(1);
});

it('stops a gateway activity payload at the Argon finalized execution header', async () => {
  const gatewayAddress = `0x${'67'.repeat(20)}`;
  const zeroHash = `0x${'00'.repeat(32)}`;
  const zeroBloom = `0x${'00'.repeat(256)}`;
  const zeroAddress = `0x${'00'.repeat(20)}`;
  const emptyUnclesHash =
    '0x1dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347' as Hex;
  const txHash10 = `0x${'12'.repeat(32)}`;
  const txHash11 = `0x${'13'.repeat(32)}`;
  const log10 = createTransferToArgonStartedBlockLog({
    gatewayAddress,
    txHash: txHash10,
    transactionIndex: 0,
    logIndex: 0,
    blockHash: zeroHash,
    blockNumber: 10n,
    nonce: 1n,
    argonAccountId: `0x${'51'.repeat(32)}`,
  });
  const log11 = createTransferToArgonStartedBlockLog({
    gatewayAddress,
    txHash: txHash11,
    transactionIndex: 0,
    logIndex: 0,
    blockHash: zeroHash,
    blockNumber: 11n,
    nonce: 2n,
    argonAccountId: `0x${'52'.repeat(32)}`,
  });
  const receipt10 = {
    type: 'legacy',
    status: 'success',
    cumulativeGasUsed: 21_000n,
    logsBloom: zeroBloom,
    logs: [{ address: gatewayAddress, topics: log10.topics, data: log10.data }],
    transactionHash: txHash10,
    transactionIndex: 0,
  } as unknown as EthereumReceipt;
  const receipt11 = {
    type: 'legacy',
    status: 'success',
    cumulativeGasUsed: 21_000n,
    logsBloom: zeroBloom,
    logs: [{ address: gatewayAddress, topics: log11.topics, data: log11.data }],
    transactionHash: txHash11,
    transactionIndex: 0,
  } as unknown as EthereumReceipt;

  const trie10 = await createMPT();
  await trie10.put(encodeReceiptTrieKey(0), encodeEthereumReceiptForProof(receipt10));
  const receiptsRoot10 = bytesToHex(trie10.root());
  const trie11 = await createMPT();
  await trie11.put(encodeReceiptTrieKey(0), encodeEthereumReceiptForProof(receipt11));
  const receiptsRoot11 = bytesToHex(trie11.root());

  const block10Template = {
    number: toHex(10n),
    hash: zeroHash,
    parentHash: zeroHash,
    nonce: '0x0000000000000000',
    sha3Uncles: emptyUnclesHash,
    logsBloom: zeroBloom,
    transactionsRoot: zeroHash,
    stateRoot: zeroHash,
    receiptsRoot: receiptsRoot10,
    miner: zeroAddress,
    difficulty: '0x0',
    extraData: '0x',
    size: '0x1',
    gasLimit: toHex(30_000_000n),
    gasUsed: toHex(21_000n),
    timestamp: toHex(1n),
    transactions: [txHash10],
    uncles: [],
  } satisfies JSONRPCBlock;
  const block10Hash = bytesToHex(createBlockHeaderFromRPC(block10Template).hash());
  const block10 = { ...block10Template, hash: block10Hash } satisfies JSONRPCBlock;
  const block11Template = {
    ...block10Template,
    number: toHex(11n),
    hash: zeroHash,
    receiptsRoot: receiptsRoot11,
    timestamp: toHex(2n),
    transactions: [txHash11],
  } satisfies JSONRPCBlock;
  const block11Hash = bytesToHex(createBlockHeaderFromRPC(block11Template).hash());
  const block11 = { ...block11Template, hash: block11Hash } satisfies JSONRPCBlock;
  log10.blockHash = block10Hash;
  log11.blockHash = block11Hash;
  receipt10.blockHash = block10Hash;
  receipt11.blockHash = block11Hash;

  const executionClient = {
    readContract: async ({
      functionName,
      args,
    }: {
      functionName: 'latestActivityBlockLocatorIndex' | 'activityBlockLocators';
      args?: [bigint];
    }) => {
      if (functionName === 'latestActivityBlockLocatorIndex') {
        return 2n;
      }

      if (args?.[0] === 1n) {
        return [10n, 1n, 1n];
      }
      if (args?.[0] === 2n) {
        return [11n, 2n, 2n];
      }

      throw new Error(`Unexpected locator request ${String(args?.[0])}`);
    },
    getLogs: async ({ fromBlock }: { fromBlock: bigint }) => {
      if (fromBlock === 10n) return [log10];
      if (fromBlock === 11n) return [log11];
      throw new Error(`Unexpected getLogs block ${fromBlock}`);
    },
    getTransactionReceipt: async ({ hash }: { hash: Hex }) => {
      if (hash === txHash10) return receipt10;
      if (hash === txHash11) return receipt11;
      throw new Error(`Unexpected receipt request for ${hash}`);
    },
    getBlock: async ({ blockHash }: { blockHash: Hex }) => {
      if (blockHash === block10Hash) return { transactions: [txHash10] };
      if (blockHash === block11Hash) return { transactions: [txHash11] };
      throw new Error(`Unexpected block request for ${blockHash}`);
    },
    request: async ({
      method,
      params,
    }: {
      method: 'eth_getBlockByHash' | 'eth_getBlockByNumber';
      params: [Hex, true];
    }) => {
      if (method === 'eth_getBlockByHash') {
        if (params[0] === block10Hash) return block10;
        if (params[0] === block11Hash) return block11;
      }
      if (method === 'eth_getBlockByNumber') {
        if (params[0] === toHex(10n)) return block10;
        if (params[0] === toHex(11n)) return block11;
      }

      throw new Error(`Unexpected header request for ${method} ${params[0]}`);
    },
  } as unknown as EthereumExecutionClient;
  const argonClient = {
    query: {
      crosschainTransfer: {
        gatewayStateBySourceChain: async () => ({
          isSome: false,
        }),
      },
      ethereumVerifier: {
        latestExecutionHeaderAnchorBlockHash: async () => ({
          isNone: false,
          unwrap: () => ({ toHex: () => block10Hash }),
        }),
        executionHeaderAnchors: async () => ({
          isNone: false,
          unwrap: () => ({
            blockNumber: {
              toBigInt: () => 10n,
            },
          }),
        }),
      },
    },
    consts: gatewayProofConsts,
  } as unknown as IArgonQueryable & Pick<ArgonClient, 'consts'>;

  const payload = await buildGatewayActivityProofPayload(argonClient, {
    gatewayAddress,
    executionClient,
  });

  expect(payload?.gatewayActivityNonceRange).toEqual({ start: 1n, end: 1n });
  expect(payload?.executionBlockNumberRange).toEqual({ start: 10n, end: 10n });
  expect(payload?.proof.blocks).toHaveLength(1);
});

it('stops a gateway activity payload before a later proof chunk leaves the Argon finalized header chain', async () => {
  const gatewayAddress = `0x${'57'.repeat(20)}`;
  const zeroHash = `0x${'00'.repeat(32)}`;
  const zeroBloom = `0x${'00'.repeat(256)}`;
  const zeroAddress = `0x${'00'.repeat(20)}`;
  const emptyUnclesHash = '0x1dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347';
  const txHash10 = `0x${'14'.repeat(32)}`;
  const txHash11 = `0x${'15'.repeat(32)}`;
  const log10 = createTransferToArgonStartedBlockLog({
    gatewayAddress,
    txHash: txHash10,
    transactionIndex: 0,
    logIndex: 0,
    blockHash: zeroHash,
    blockNumber: 10n,
    nonce: 1n,
    argonAccountId: `0x${'61'.repeat(32)}`,
  });
  const log11 = createTransferToArgonStartedBlockLog({
    gatewayAddress,
    txHash: txHash11,
    transactionIndex: 0,
    logIndex: 0,
    blockHash: zeroHash,
    blockNumber: 11n,
    nonce: 2n,
    argonAccountId: `0x${'62'.repeat(32)}`,
  });
  const receipt10 = {
    type: 'legacy',
    status: 'success',
    cumulativeGasUsed: 21_000n,
    logsBloom: zeroBloom,
    logs: [{ address: gatewayAddress, topics: log10.topics, data: log10.data }],
    transactionHash: txHash10,
    transactionIndex: 0,
  } as unknown as EthereumReceipt;
  const receipt11 = {
    type: 'legacy',
    status: 'success',
    cumulativeGasUsed: 21_000n,
    logsBloom: zeroBloom,
    logs: [{ address: gatewayAddress, topics: log11.topics, data: log11.data }],
    transactionHash: txHash11,
    transactionIndex: 0,
  } as unknown as EthereumReceipt;

  const trie10 = await createMPT();
  await trie10.put(encodeReceiptTrieKey(0), encodeEthereumReceiptForProof(receipt10));
  const receiptsRoot10 = bytesToHex(trie10.root());
  const trie11 = await createMPT();
  await trie11.put(encodeReceiptTrieKey(0), encodeEthereumReceiptForProof(receipt11));
  const receiptsRoot11 = bytesToHex(trie11.root());

  const block10Template = {
    number: toHex(10n),
    hash: zeroHash,
    parentHash: zeroHash,
    nonce: '0x0000000000000000',
    sha3Uncles: emptyUnclesHash,
    logsBloom: zeroBloom,
    transactionsRoot: zeroHash,
    stateRoot: zeroHash,
    receiptsRoot: receiptsRoot10,
    miner: zeroAddress,
    difficulty: '0x0',
    extraData: '0x',
    size: '0x1',
    gasLimit: toHex(30_000_000n),
    gasUsed: toHex(21_000n),
    timestamp: toHex(1n),
    transactions: [txHash10],
    uncles: [],
  } satisfies JSONRPCBlock;
  const block10Hash = bytesToHex(createBlockHeaderFromRPC(block10Template).hash());
  const block10 = { ...block10Template, hash: block10Hash } satisfies JSONRPCBlock;

  const block11CanonicalTemplate = {
    ...block10Template,
    number: toHex(11n),
    parentHash: block10Hash,
    hash: zeroHash,
    receiptsRoot: receiptsRoot11,
    timestamp: toHex(2n),
    transactions: [txHash11],
  } satisfies JSONRPCBlock;
  const block11CanonicalHash = bytesToHex(
    createBlockHeaderFromRPC(block11CanonicalTemplate).hash(),
  );
  const block11Canonical = {
    ...block11CanonicalTemplate,
    hash: block11CanonicalHash,
  } satisfies JSONRPCBlock;
  const block12Template = {
    ...block10Template,
    number: toHex(12n),
    parentHash: block11CanonicalHash,
    receiptsRoot: zeroHash,
    gasUsed: toHex(0n),
    timestamp: toHex(3n),
    transactions: [],
  } satisfies JSONRPCBlock;
  const block12Hash = bytesToHex(createBlockHeaderFromRPC(block12Template).hash());
  const block12 = { ...block12Template, hash: block12Hash } satisfies JSONRPCBlock;
  const block11Wrong = {
    ...block11CanonicalTemplate,
    parentHash: `0x${'ff'.repeat(32)}`,
    hash: zeroHash,
  } satisfies JSONRPCBlock;
  const block11WrongHash = bytesToHex(createBlockHeaderFromRPC(block11Wrong).hash());
  const block11 = { ...block11Wrong, hash: block11WrongHash } satisfies JSONRPCBlock;

  log10.blockHash = block10Hash;
  log11.blockHash = block11WrongHash;
  receipt10.blockHash = block10Hash;
  receipt11.blockHash = block11WrongHash;

  const executionClient = {
    readContract: async ({
      functionName,
      args,
    }: {
      functionName: 'latestActivityBlockLocatorIndex' | 'activityBlockLocators';
      args?: [bigint];
    }) => {
      if (functionName === 'latestActivityBlockLocatorIndex') {
        return 2n;
      }

      if (args?.[0] === 1n) {
        return [10n, 1n, 1n];
      }
      if (args?.[0] === 2n) {
        return [11n, 2n, 2n];
      }

      throw new Error(`Unexpected locator request ${String(args?.[0])}`);
    },
    getLogs: async ({ fromBlock }: { fromBlock: bigint }) => {
      if (fromBlock === 10n) return [log10];
      if (fromBlock === 11n) return [log11];
      throw new Error(`Unexpected getLogs block ${fromBlock}`);
    },
    getTransactionReceipt: async ({ hash }: { hash: Hex }) => {
      if (hash === txHash10) return receipt10;
      if (hash === txHash11) return receipt11;
      throw new Error(`Unexpected receipt request for ${hash}`);
    },
    getBlock: async ({ blockHash }: { blockHash: Hex }) => {
      if (blockHash === block10Hash) return { transactions: [txHash10] };
      if (blockHash === block11WrongHash) return { transactions: [txHash11] };
      throw new Error(`Unexpected block request for ${blockHash}`);
    },
    request: async ({
      method,
      params,
    }: {
      method: 'eth_getBlockByHash' | 'eth_getBlockByNumber';
      params: [Hex, true];
    }) => {
      if (method === 'eth_getBlockByHash') {
        if (params[0] === block10Hash) return block10;
        if (params[0] === block11CanonicalHash) return block11Canonical;
        if (params[0] === block11WrongHash) return block11;
        if (params[0] === block12Hash) return block12;
      }
      if (method === 'eth_getBlockByNumber') {
        if (params[0] === toHex(10n)) return block10;
        if (params[0] === toHex(11n)) return block11Canonical;
        if (params[0] === toHex(12n)) return block12;
      }

      throw new Error(`Unexpected header request for ${method} ${params[0]}`);
    },
  } as unknown as EthereumExecutionClient;
  const argonClient = {
    query: {
      crosschainTransfer: {
        gatewayStateBySourceChain: async () => ({
          isSome: false,
        }),
      },
      ethereumVerifier: {
        latestExecutionHeaderAnchorBlockHash: async () => ({
          isNone: false,
          unwrap: () => ({ toHex: () => block12Hash }),
        }),
        executionHeaderAnchors: async () => ({
          isNone: false,
          unwrap: () => ({
            blockNumber: {
              toBigInt: () => 12n,
            },
          }),
        }),
      },
    },
    consts: gatewayProofConsts,
  } as unknown as IArgonQueryable & Pick<ArgonClient, 'consts'>;

  const payload = await buildGatewayActivityProofPayload(argonClient, {
    gatewayAddress,
    executionClient,
  });

  expect(payload?.gatewayActivityNonceRange).toEqual({ start: 1n, end: 1n });
  expect(payload?.executionBlockNumberRange).toEqual({ start: 10n, end: 10n });
  expect(payload?.proof.blocks).toHaveLength(1);
});

it('rejects zero receipt-proof bounds before building a gateway payload', async () => {
  const argonClient = {
    consts: createGatewayProofConsts({ maxReceiptProofsPerExtrinsic: 0 }),
  } as unknown as Pick<ArgonClient, 'consts'>;

  await expect(() =>
    buildGatewayActivityProofPayload(argonClient as never, {
      gatewayAddress: `0x${'77'.repeat(20)}`,
    }),
  ).rejects.toThrow('Gateway proof requires maxReceiptProofsPerExtrinsic to be a positive integer');
});

function createTransferToArgonStartedBlockLog(args: {
  gatewayAddress: Hex;
  txHash: Hex;
  transactionIndex: number;
  logIndex: number;
  blockHash: Hex;
  blockNumber: bigint;
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
      from: `0x${'11'.repeat(20)}`,
      token: `0x${'22'.repeat(20)}`,
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
    blockHash: args.blockHash,
    blockNumber: args.blockNumber,
  };
}

function createGlobalIssuanceCouncilRotatedBlockLog(args: {
  gatewayAddress: Hex;
  txHash: Hex;
  transactionIndex: number;
  logIndex: number;
  blockHash: Hex;
  blockNumber: bigint;
  nonce: bigint;
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
      `0x${'61'.repeat(32)}`,
      `0x${'62'.repeat(32)}`,
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
    blockHash: args.blockHash,
    blockNumber: args.blockNumber,
  };
}
