import { createBlockHeaderFromRPC, type JSONRPCBlock } from '@ethereumjs/block';
import { mintingGatewayActivityHashingFixture } from '@argonprotocol/ethereum-contracts/fixtures';
import { expect, it } from 'vitest';
import { createMPT, verifyMPTWithMerkleProof } from '@ethereumjs/mpt';
import {
  bytesToHex,
  concatHex,
  encodeAbiParameters,
  encodeEventTopics,
  hexToBytes,
  keccak256,
  toHex,
  type Hex,
} from 'viem';
import {
  hashMintingGatewayActivityBlockLocator,
  MintingGatewayEvents,
  mintingGatewayAbi,
} from '../EvmContracts';
import type { ArgonClient } from '../index';
import {
  buildEthereumEventProof,
  encodeEthereumReceiptForProof,
  encodeReceiptTrieKey,
} from '../EthereumProof';
import type { EthereumExecutionClient, EthereumReceipt } from '../EthereumExecution';
import {
  appendEthereumGatewayActivityRoot,
  decodeEthereumGatewayActivityLog,
  decodeEthereumTransferToArgonStartedLog,
  hashEthereumGatewayActivity,
} from '../EthereumGatewayActivity';
import {
  buildGatewayActivityProof,
  buildGatewayActivityReceiptProofPayloads,
  buildGatewayActivityStorageProofs,
  discoverMissingGatewayActivityLocators,
  supportsGatewayActivityReceiptProofs,
} from '../EthereumGatewayActivityProof';
import productionInboundReceipt from './fixtures/productionInboundReceipt.json';
import {
  createArgonGatewayClient,
  createExecutionBlock,
  createExecutionClient,
  createGatewayProofConsts,
  createGlobalIssuanceCouncilRotatedBlockLog,
  createLegacyReceipt,
  createTransferToArgonStartedBlockLog,
  repeatHex,
} from './ethereumProofTestUtils';

type EncodedReceiptInput = Parameters<typeof encodeEthereumReceiptForProof>[0];

// Mirror the runtime proof-bound consts for query-only unit tests.
const gatewayProofConsts = createGatewayProofConsts();

it('imports the shared gateway hashing fixture through the contracts fixture export', () => {
  expect(
    hashMintingGatewayActivityBlockLocator({
      blockNumber: mintingGatewayActivityHashingFixture.blockNumber,
      startGatewayActivityNonce: mintingGatewayActivityHashingFixture.startGatewayActivityNonce,
      endGatewayActivityNonce: mintingGatewayActivityHashingFixture.endGatewayActivityNonce,
      activityRoot: mintingGatewayActivityHashingFixture.finalRoot,
    }),
  ).toBe(mintingGatewayActivityHashingFixture.locatorHash);
});

it('reports receipt gateway proofs as supported before the storage-proof runtime version', () => {
  const client = {
    runtimeVersion: {
      specVersion: {
        toNumber: () => 153,
      },
    },
  } as Pick<ArgonClient, 'runtimeVersion'>;

  expect(supportsGatewayActivityReceiptProofs(client)).toBe(true);
});

it('reports receipt gateway proofs as unsupported once storage proofs are required', () => {
  const client = {
    runtimeVersion: {
      specVersion: {
        toNumber: () => 154,
      },
    },
  } as Pick<ArgonClient, 'runtimeVersion'>;

  expect(supportsGatewayActivityReceiptProofs(client)).toBe(false);
});

it('builds the legacy receipt proof payload for pre-storage-proof runtimes', async () => {
  const gatewayAddress = repeatHex('70', 20);
  const transferLog = createTransferToArgonStartedBlockLog({
    gatewayAddress,
    txHash: repeatHex('30', 32),
    transactionIndex: 0,
    logIndex: 0,
    nonce: 1n,
    argonAccountId: repeatHex('45', 32),
  });
  const receipt = createLegacyReceipt({
    txHash: transferLog.transactionHash,
    transactionIndex: transferLog.transactionIndex,
    logs: [
      {
        address: transferLog.address,
        topics: transferLog.topics as [Hex, ...Hex[]],
        data: transferLog.data,
      },
    ],
  });
  const block = await createExecutionBlock({
    number: 10n,
    receipts: [receipt],
    blockLogs: [transferLog],
  });
  const decodedActivity = {
    txHash: transferLog.transactionHash,
    transactionIndex: transferLog.transactionIndex,
    logIndex: transferLog.logIndex,
    blockHash: transferLog.blockHash,
    blockNumber: transferLog.blockNumber,
    ...decodeEthereumGatewayActivityLog(transferLog),
  };
  const argonClient = createArgonGatewayClient({
    previousGatewayActivityNonce: 0n,
    runtimeSpecVersion: 153,
    consts: createGatewayProofConsts(),
    argonFinalizedExecutionHeaders: [{ blockHash: block.hash, blockNumber: 10n }],
  });
  const executionClient = createExecutionClient({
    blocks: [block],
    receipts: [receipt],
    chainId: 1n,
    locators: [
      {
        blockNumber: 10n,
        startGatewayActivityNonce: 1n,
        endGatewayActivityNonce: 1n,
        activityRoot: appendEthereumGatewayActivityRoot(
          repeatHex('00', 32),
          hashEthereumGatewayActivity({ chainId: 1n, gatewayAddress }, decodedActivity),
        ),
      },
    ],
    logsByBlockNumber: {
      '10': [transferLog],
    },
  });

  const locators = await discoverMissingGatewayActivityLocators({
    executionClient,
    finalizedExecutionBlockNumber: 10n,
    gatewayAddress,
    minimumGatewayActivityNonce: 1n,
  });
  const { payloads } = await buildGatewayActivityProof(argonClient, {
    gatewayAddress,
    executionClient,
    locators,
  });
  const payload = payloads[0];
  if (!payload || !('blocks' in payload.proof)) {
    throw new Error('Expected the default gateway activity proof builder to choose receipt proofs');
  }

  expect(payload?.previousGatewayActivityNonce).toBe(0n);
  expect(payload?.gatewayActivityNonceRange).toEqual({ start: 1n, end: 1n });
  expect(payload?.executionBlockNumberRange).toEqual({ start: 10n, end: 10n });
  expect(payload.proof.blocks).toHaveLength(1);
  expect(payload.proof.blocks[0]?.receiptLogs).toHaveLength(1);
  expect(payload.proof.blocks[0]?.receiptProof.receipts[0]?.transactionIndex).toBe(0);
});

it('requires a finalized execution header when the default builder routes to storage proofs', async () => {
  const gatewayAddress = repeatHex('70', 20);
  const transferLog = createTransferToArgonStartedBlockLog({
    gatewayAddress,
    txHash: repeatHex('33', 32),
    transactionIndex: 0,
    logIndex: 0,
    nonce: 1n,
    argonAccountId: repeatHex('48', 32),
  });
  const { argonClient, executionClient, argonFinalizedExecutionHeader } = createGatewayProofHarness(
    {
      gatewayAddress,
      runtimeSpecVersion: 154,
      latestArgonFinalizedHeader: {
        blockHash: repeatHex('ab', 32),
        blockNumber: 12n,
      },
      locators: [{ blockNumber: 10n, logs: [transferLog] }],
    },
  );
  const locators = await discoverMissingGatewayActivityLocators({
    executionClient,
    finalizedExecutionBlockNumber: argonFinalizedExecutionHeader.blockNumber,
    gatewayAddress,
    minimumGatewayActivityNonce: 1n,
  });

  await expect(
    buildGatewayActivityProof(argonClient, {
      gatewayAddress,
      executionClient,
      locators,
    }),
  ).rejects.toThrow(
    'Gateway activity storage proofs require a finalizedExecutionHeader; use discoverMissingGatewayActivityLocators and provide the execution header you want to relay against',
  );
});

it('rejects building storage gateway proofs against a receipt-proof runtime', async () => {
  const gatewayAddress = repeatHex('71', 20);
  const transferLog = createTransferToArgonStartedBlockLog({
    gatewayAddress,
    txHash: repeatHex('31', 32),
    transactionIndex: 0,
    logIndex: 0,
    nonce: 1n,
    argonAccountId: repeatHex('46', 32),
  });
  const { argonClient, executionClient, locators, argonFinalizedExecutionHeader } =
    createGatewayProofHarness({
      gatewayAddress,
      locators: [{ blockNumber: 10n, logs: [transferLog] }],
      runtimeSpecVersion: 153,
    });

  await expect(
    buildGatewayActivityStorageProofs(argonClient, {
      finalizedExecutionHeader: argonFinalizedExecutionHeader,
      gatewayAddress,
      executionClient,
      locators: [locators[0]],
    }),
  ).rejects.toThrow(
    'Gateway activity storage proofs are not supported by this runtime; use buildGatewayActivityReceiptProofPayloads instead',
  );
});

it('rejects building receipt gateway proofs against a storage-proof runtime', async () => {
  const gatewayAddress = repeatHex('72', 20);
  const transferLog = createTransferToArgonStartedBlockLog({
    gatewayAddress,
    txHash: repeatHex('32', 32),
    transactionIndex: 0,
    logIndex: 0,
    nonce: 1n,
    argonAccountId: repeatHex('47', 32),
  });
  const { argonClient, executionClient } = createGatewayProofHarness({
    gatewayAddress,
    locators: [{ blockNumber: 10n, logs: [transferLog] }],
    runtimeSpecVersion: 154,
  });

  await expect(
    buildGatewayActivityReceiptProofPayloads(argonClient, {
      gatewayAddress,
      executionClient,
      locators: [],
    }),
  ).rejects.toThrow(
    'Gateway activity receipt proofs are not supported by this runtime; use discoverMissingGatewayActivityLocators and buildGatewayActivityStorageProofs instead',
  );
});

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
  const txHash0: Hex = `0x${'aa'.repeat(32)}`;
  const txHash1: Hex = `0x${'bb'.repeat(32)}`;
  const zeroHash: Hex = `0x${'00'.repeat(32)}`;
  const zeroBloom: Hex = `0x${'00'.repeat(256)}`;
  const zeroAddress: Hex = `0x${'00'.repeat(20)}`;
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
  const receiptsRoot: Hex = bytesToHex(trie.root());
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
  const targetBlockHash: Hex = bytesToHex(createBlockHeaderFromRPC(targetBlockTemplate).hash());
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
  const anchorBlockHash: Hex = bytesToHex(createBlockHeaderFromRPC(anchorBlockTemplate).hash());
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

it('resolves explicit Ethereum log indexes against receipt log metadata', async () => {
  const txHash: Hex = `0x${'aa'.repeat(32)}`;
  const zeroHash: Hex = `0x${'00'.repeat(32)}`;
  const zeroBloom: Hex = `0x${'00'.repeat(256)}`;
  const zeroAddress: Hex = `0x${'00'.repeat(20)}`;
  const emptyUnclesHash =
    '0x1dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347' as Hex;
  const receipt = {
    type: 'legacy',
    status: 'success',
    cumulativeGasUsed: 21_000n,
    logsBloom: zeroBloom,
    logs: [
      {
        address: `0x${'33'.repeat(20)}`,
        topics: [`0x${'44'.repeat(32)}`],
        data: '0x1234' as Hex,
        logIndex: 318,
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
  const receiptsRoot: Hex = bytesToHex(trie.root());
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
    gasUsed: toHex(21_000n),
    timestamp: toHex(1n),
    transactions: [txHash],
    uncles: [],
  } satisfies JSONRPCBlock;
  const targetBlockHash: Hex = bytesToHex(createBlockHeaderFromRPC(targetBlockTemplate).hash());
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
  const anchorBlockHash: Hex = bytesToHex(createBlockHeaderFromRPC(anchorBlockTemplate).hash());
  const anchorBlock = {
    ...anchorBlockTemplate,
    hash: anchorBlockHash,
  } satisfies JSONRPCBlock;
  receipt.blockHash = targetBlockHash;

  const executionClient = {
    getTransactionReceipt: async ({ hash }: { hash: Hex }) => {
      if (hash === txHash) return receipt;
      throw new Error(`Unexpected receipt request for ${hash}`);
    },
    getBlock: async ({ blockHash }: { blockHash: Hex }) => {
      expect(blockHash).toBe(targetBlockHash);
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
        if (params[0] === targetBlockHash) return targetBlock;
        if (params[0] === anchorBlockHash) return anchorBlock;
      }
      if (method === 'eth_getBlockByNumber' && params[0] === toHex(10n)) {
        return targetBlock;
      }

      throw new Error(`Unexpected header request for ${method} ${params[0]}`);
    },
  } as unknown as EthereumExecutionClient;
  const proof = await buildEthereumEventProof(
    { executionClient },
    { blockHash: anchorBlockHash, blockNumber: 11n },
    [[{ txHash, receipt, logIndexes: [318] }]],
  );

  expect(proof.blocks).toHaveLength(1);
  expect(proof.blocks[0].receiptLogs).toHaveLength(1);
  expect(proof.blocks[0].receiptLogs[0]?.eventLog.data).toBe('0x1234');
});

it('encodes reverted receipts with an empty status byte', async () => {
  const receipt = {
    type: 'eip1559',
    status: 'reverted',
    cumulativeGasUsed: 21_000n,
    logsBloom: `0x${'00'.repeat(256)}`,
    logs: [],
  } as unknown as EthereumReceipt;

  expect(bytesToHex(encodeEthereumReceiptForProof(receipt))).toBe(
    '0x02f9010880825208b9010000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000c0',
  );
});

it('shares one execution header chain across multiple proved execution blocks', async () => {
  const txHash0: Hex = `0x${'aa'.repeat(32)}`;
  const txHash1: Hex = `0x${'bb'.repeat(32)}`;
  const zeroHash: Hex = `0x${'00'.repeat(32)}`;
  const zeroBloom: Hex = `0x${'00'.repeat(256)}`;
  const zeroAddress: Hex = `0x${'00'.repeat(20)}`;
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
  const receiptsRoot0: Hex = bytesToHex(trie0.root());

  const trie1 = await createMPT();
  await trie1.put(
    encodeReceiptTrieKey(receipt1.transactionIndex),
    encodeEthereumReceiptForProof(receipt1),
  );
  const receiptsRoot1: Hex = bytesToHex(trie1.root());

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
  const olderBlockHash: Hex = bytesToHex(createBlockHeaderFromRPC(olderBlockTemplate).hash());
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
  const newerBlockHash: Hex = bytesToHex(createBlockHeaderFromRPC(newerBlockTemplate).hash());
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
  const anchorBlockHash: Hex = bytesToHex(createBlockHeaderFromRPC(anchorBlockTemplate).hash());
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
  const txHash: Hex = `0x${'aa'.repeat(32)}`;
  const zeroHash: Hex = `0x${'00'.repeat(32)}`;
  const zeroBloom: Hex = `0x${'00'.repeat(256)}`;
  const zeroAddress: Hex = `0x${'00'.repeat(20)}`;
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
  const receiptsRoot: Hex = bytesToHex(trie.root());
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
  const anchorBlockHash: Hex = bytesToHex(createBlockHeaderFromRPC(anchorBlockTemplate).hash());
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
  const from: Hex = `0x${'11'.repeat(20)}`;
  const token: Hex = `0x${'22'.repeat(20)}`;
  const argonAccountId: Hex = `0x${'33'.repeat(32)}`;
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
  const councilHash: Hex = `0x${'44'.repeat(32)}`;
  const approvalHash: Hex = `0x${'66'.repeat(32)}`;
  const relayerArgonAccountId: Hex = `0x${'55'.repeat(32)}`;
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
      councilHash,
      approvalHash,
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
    approvalHash,
    relayerArgonAccountId,
    gatewayState: {
      gatewayActivityNonce: 9n,
      argonApprovalsNonce: 4n,
      argonCirculation: 500n,
      argonotCirculation: 2_000n,
    },
  });
});

it('builds a gateway payload for the next uncovered locator', async () => {
  const gatewayAddress = repeatHex('77', 20);
  const latestArgonFinalizedHeader = {
    blockHash: repeatHex('fe', 32),
    blockNumber: 12n,
  };
  const transferLog = createTransferToArgonStartedBlockLog({
    gatewayAddress,
    txHash: repeatHex('10', 32),
    transactionIndex: 0,
    logIndex: 0,
    nonce: 1n,
    argonAccountId: repeatHex('41', 32),
  });
  const councilRotationLog = createGlobalIssuanceCouncilRotatedBlockLog({
    gatewayAddress,
    txHash: repeatHex('10', 32),
    transactionIndex: 0,
    logIndex: 1,
    nonce: 2n,
  });
  const { argonClient, executionClient, locators } = createGatewayProofHarness({
    gatewayAddress,
    latestArgonFinalizedHeader,
    locators: [{ blockNumber: 10n, logs: [transferLog, councilRotationLog] }],
  });

  const { payloads } = await buildGatewayActivityStorageProofs(argonClient, {
    executionClient,
    finalizedExecutionHeader: latestArgonFinalizedHeader,
    gatewayAddress,
    locators: [locators[0]],
  });
  const payload = payloads[0];

  expect(payload?.previousGatewayActivityNonce).toBe(0n);
  expect(payload?.gatewayActivityNonceRange).toEqual({ start: 1n, end: 2n });
  expect(payload?.executionBlockNumberRange).toEqual({ start: 10n, end: 10n });
  expect(payload?.activities.map(activity => activity.kind)).toEqual([
    MintingGatewayEvents.TransferToArgonStarted.name,
    MintingGatewayEvents.GlobalIssuanceCouncilRotated.name,
  ]);
  expect(payload?.proof.locatorIndex).toBe(1n);
  expect(payload?.proof.activityLogs).toHaveLength(2);
  expect(payload?.proof.storageProof.anchorBlockHash).toBe(latestArgonFinalizedHeader.blockHash);
  expect(payload?.proof.storageProof.slots).toHaveLength(2);
  expect(payload?.proof.storageProof.slots[0]?.nodeIndexes).toEqual([0, 1]);
  expect(payload?.proof.storageProof.slots[1]?.nodeIndexes).toEqual([1, 2]);
  expect(payload?.proof.storageProof.storageProof).toEqual([
    repeatHex('01', 32),
    repeatHex('02', 32),
    repeatHex('03', 32),
  ]);
});

it('discovers all missing locators in order', async () => {
  const gatewayAddress = repeatHex('78', 20);
  const log10 = createTransferToArgonStartedBlockLog({
    gatewayAddress,
    txHash: repeatHex('11', 32),
    transactionIndex: 0,
    logIndex: 0,
    nonce: 1n,
    argonAccountId: repeatHex('42', 32),
  });
  const log11 = createTransferToArgonStartedBlockLog({
    gatewayAddress,
    txHash: repeatHex('12', 32),
    transactionIndex: 0,
    logIndex: 0,
    nonce: 2n,
    argonAccountId: repeatHex('43', 32),
  });
  const { executionClient, argonFinalizedExecutionHeader } = createGatewayProofHarness({
    gatewayAddress,
    latestArgonFinalizedHeader: {
      blockHash: repeatHex('fd', 32),
      blockNumber: 12n,
    },
    locators: [
      { blockNumber: 10n, logs: [log10] },
      { blockNumber: 11n, logs: [log11] },
    ],
  });

  const discovered = await discoverMissingGatewayActivityLocators({
    executionClient,
    finalizedExecutionBlockNumber: argonFinalizedExecutionHeader.blockNumber,
    gatewayAddress,
    minimumGatewayActivityNonce: 1n,
  });

  expect(
    discovered.map(locator => ({
      locatorIndex: locator.locatorIndex,
      blockNumber: locator.blockNumber,
      startGatewayActivityNonce: locator.startGatewayActivityNonce,
      endGatewayActivityNonce: locator.endGatewayActivityNonce,
    })),
  ).toEqual([
    {
      locatorIndex: 1n,
      blockNumber: 10n,
      startGatewayActivityNonce: 1n,
      endGatewayActivityNonce: 1n,
    },
    {
      locatorIndex: 2n,
      blockNumber: 11n,
      startGatewayActivityNonce: 2n,
      endGatewayActivityNonce: 2n,
    },
  ]);
});

it('builds a later locator payload using the previous locator hash seed', async () => {
  const gatewayAddress = repeatHex('7a', 20);
  const log10 = createTransferToArgonStartedBlockLog({
    gatewayAddress,
    txHash: repeatHex('17', 32),
    transactionIndex: 0,
    logIndex: 0,
    nonce: 1n,
    argonAccountId: repeatHex('51', 32),
  });
  const log11 = createTransferToArgonStartedBlockLog({
    gatewayAddress,
    txHash: repeatHex('18', 32),
    transactionIndex: 0,
    logIndex: 0,
    nonce: 2n,
    argonAccountId: repeatHex('52', 32),
  });
  const { argonClient, executionClient, locators, argonFinalizedExecutionHeader } =
    createGatewayProofHarness({
      gatewayAddress,
      previousGatewayActivityNonce: 1n,
      latestArgonFinalizedHeader: {
        blockHash: repeatHex('fb', 32),
        blockNumber: 12n,
      },
      locators: [
        { blockNumber: 10n, logs: [log10] },
        { blockNumber: 11n, logs: [log11] },
      ],
    });

  const { payloads } = await buildGatewayActivityStorageProofs(argonClient, {
    executionClient,
    finalizedExecutionHeader: argonFinalizedExecutionHeader,
    gatewayAddress,
    locators: [locators[1]],
  });
  const payload = payloads[0];

  expect(payload?.previousGatewayActivityNonce).toBe(1n);
  expect(payload?.proof.locatorIndex).toBe(2n);
  expect(payload?.gatewayActivityNonceRange).toEqual({ start: 2n, end: 2n });
  expect(payload?.proof.storageProof.slots[1]?.value).toBe(locators[1]?.activityRoot);
  expect(locators[1]?.activityRoot).toBe(
    appendEthereumGatewayActivityRoot(
      hashMintingGatewayActivityBlockLocator(locators[0]),
      hashEthereumGatewayActivity({ chainId: 1n, gatewayAddress }, payload.activities[0]),
    ),
  );
});

it('rejects a locator payload when eth_getProof returns an unexpected range slot value', async () => {
  const gatewayAddress = repeatHex('7e', 20);
  const log = createTransferToArgonStartedBlockLog({
    gatewayAddress,
    txHash: repeatHex('19', 32),
    transactionIndex: 0,
    logIndex: 0,
    nonce: 1n,
    argonAccountId: repeatHex('53', 32),
  });
  const {
    argonClient,
    executionClient,
    proofsByBlockNumber,
    locators,
    argonFinalizedExecutionHeader,
  } = createGatewayProofHarness({
    gatewayAddress,
    latestArgonFinalizedHeader: {
      blockHash: repeatHex('fa', 32),
      blockNumber: 12n,
    },
    locators: [{ blockNumber: 10n, logs: [log] }],
  });

  const rangeProof = proofsByBlockNumber['12']?.storageProof?.[0];
  if (!rangeProof) {
    throw new Error('Expected a mocked range slot proof');
  }
  rangeProof.value += 1n;

  await expect(() =>
    buildGatewayActivityStorageProofs(argonClient, {
      executionClient,
      finalizedExecutionHeader: argonFinalizedExecutionHeader,
      gatewayAddress,
      locators: [locators[0]],
    }),
  ).rejects.toThrow('eth_getProof returned an unexpected value for storage slot');
});

it('discovers only locators after the accepted nonce and builds the next payload', async () => {
  const gatewayAddress = repeatHex('79', 20);
  const log10 = createTransferToArgonStartedBlockLog({
    gatewayAddress,
    txHash: repeatHex('13', 32),
    transactionIndex: 0,
    logIndex: 0,
    nonce: 1n,
    argonAccountId: repeatHex('44', 32),
  });
  const log11 = createTransferToArgonStartedBlockLog({
    gatewayAddress,
    txHash: repeatHex('14', 32),
    transactionIndex: 0,
    logIndex: 0,
    nonce: 2n,
    argonAccountId: repeatHex('45', 32),
  });
  const { argonClient, executionClient, locators, argonFinalizedExecutionHeader } =
    createGatewayProofHarness({
      gatewayAddress,
      previousGatewayActivityNonce: 1n,
      latestArgonFinalizedHeader: {
        blockHash: repeatHex('fc', 32),
        blockNumber: 12n,
      },
      locators: [
        { blockNumber: 10n, logs: [log10] },
        { blockNumber: 11n, logs: [log11] },
      ],
    });

  const discovered = await discoverMissingGatewayActivityLocators({
    executionClient,
    finalizedExecutionBlockNumber: argonFinalizedExecutionHeader.blockNumber,
    gatewayAddress,
    minimumGatewayActivityNonce: 2n,
  });
  const { payloads } = await buildGatewayActivityStorageProofs(argonClient, {
    executionClient,
    finalizedExecutionHeader: argonFinalizedExecutionHeader,
    gatewayAddress,
    locators: [locators[1]],
  });
  const payload = payloads[0];

  expect(discovered).toEqual([locators[1]]);
  expect(payload?.previousGatewayActivityNonce).toBe(1n);
  expect(payload?.gatewayActivityNonceRange).toEqual({ start: 2n, end: 2n });
  expect(payload?.executionBlockNumberRange).toEqual({ start: 11n, end: 11n });
  expect(payload?.proof.locatorIndex).toBe(2n);
  expect(payload?.activities.map(activity => activity.gatewayState.gatewayActivityNonce)).toEqual([
    2n,
  ]);
});

it('discovers missing locator blocks in chronological order', async () => {
  const gatewayAddress = repeatHex('7e', 20);
  const log10 = createTransferToArgonStartedBlockLog({
    gatewayAddress,
    txHash: repeatHex('21', 32),
    transactionIndex: 0,
    logIndex: 0,
    nonce: 1n,
    argonAccountId: repeatHex('46', 32),
  });
  const log11 = createTransferToArgonStartedBlockLog({
    gatewayAddress,
    txHash: repeatHex('22', 32),
    transactionIndex: 0,
    logIndex: 0,
    nonce: 2n,
    argonAccountId: repeatHex('47', 32),
  });
  const log12 = createTransferToArgonStartedBlockLog({
    gatewayAddress,
    txHash: repeatHex('23', 32),
    transactionIndex: 0,
    logIndex: 0,
    nonce: 3n,
    argonAccountId: repeatHex('48', 32),
  });
  const { argonClient, executionClient, locators, argonFinalizedExecutionHeader } =
    createGatewayProofHarness({
      gatewayAddress,
      previousGatewayActivityNonce: 1n,
      latestArgonFinalizedHeader: {
        blockHash: repeatHex('fa', 32),
        blockNumber: 13n,
      },
      locators: [
        { blockNumber: 10n, logs: [log10] },
        { blockNumber: 11n, logs: [log11] },
        { blockNumber: 12n, logs: [log12] },
      ],
    });

  const discovered = await discoverMissingGatewayActivityLocators({
    executionClient,
    finalizedExecutionBlockNumber: argonFinalizedExecutionHeader.blockNumber,
    gatewayAddress,
    minimumGatewayActivityNonce: 2n,
  });

  expect(discovered).toEqual([locators[1], locators[2]]);
});

it('reads locator discovery from the latest Argon finalized execution block', async () => {
  const gatewayAddress = repeatHex('7d', 20);
  const readContractRequests: Array<{
    functionName: string;
    blockNumber?: bigint;
    locatorIndex?: bigint;
  }> = [];
  const log10 = createTransferToArgonStartedBlockLog({
    gatewayAddress,
    txHash: repeatHex('26', 32),
    transactionIndex: 0,
    logIndex: 0,
    nonce: 1n,
    argonAccountId: repeatHex('4b', 32),
  });
  const log11 = createTransferToArgonStartedBlockLog({
    gatewayAddress,
    txHash: repeatHex('27', 32),
    transactionIndex: 0,
    logIndex: 0,
    nonce: 2n,
    argonAccountId: repeatHex('4c', 32),
  });
  const latestArgonFinalizedHeader = {
    blockHash: repeatHex('f8', 32),
    blockNumber: 12n,
  };
  const { executionClient } = createGatewayProofHarness({
    gatewayAddress,
    latestArgonFinalizedHeader,
    readContractRequests,
    locators: [
      { blockNumber: 10n, logs: [log10] },
      { blockNumber: 11n, logs: [log11] },
    ],
  });

  await discoverMissingGatewayActivityLocators({
    executionClient,
    finalizedExecutionBlockNumber: latestArgonFinalizedHeader.blockNumber,
    gatewayAddress,
    minimumGatewayActivityNonce: 1n,
  });

  const locatorRequests = readContractRequests.filter(
    ({ functionName }) =>
      functionName === 'latestActivityBlockLocatorIndex' ||
      functionName === 'activityBlockLocators',
  );

  expect(locatorRequests).not.toHaveLength(0);
  expect(
    locatorRequests.every(
      ({ blockNumber }) => blockNumber === latestArgonFinalizedHeader.blockNumber,
    ),
  ).toBe(true);
});

it('discovers only newly finalized locator blocks when given the last cached locator index', async () => {
  const gatewayAddress = repeatHex('7e', 20);
  const readContractRequests: Array<{
    functionName: string;
    blockNumber?: bigint;
    locatorIndex?: bigint;
  }> = [];
  const log10 = createTransferToArgonStartedBlockLog({
    gatewayAddress,
    txHash: repeatHex('24', 32),
    transactionIndex: 0,
    logIndex: 0,
    nonce: 1n,
    argonAccountId: repeatHex('49', 32),
  });
  const log11 = createTransferToArgonStartedBlockLog({
    gatewayAddress,
    txHash: repeatHex('25', 32),
    transactionIndex: 0,
    logIndex: 0,
    nonce: 2n,
    argonAccountId: repeatHex('4a', 32),
  });
  const log12 = createTransferToArgonStartedBlockLog({
    gatewayAddress,
    txHash: repeatHex('28', 32),
    transactionIndex: 0,
    logIndex: 0,
    nonce: 3n,
    argonAccountId: repeatHex('4d', 32),
  });
  const { executionClient, locators, argonFinalizedExecutionHeader } = createGatewayProofHarness({
    gatewayAddress,
    latestArgonFinalizedHeader: {
      blockHash: repeatHex('f9', 32),
      blockNumber: 13n,
    },
    readContractRequests,
    locators: [
      { blockNumber: 10n, logs: [log10] },
      { blockNumber: 11n, logs: [log11] },
      { blockNumber: 12n, logs: [log12] },
    ],
  });

  const discovered = await discoverMissingGatewayActivityLocators({
    afterLocatorIndex: 1n,
    executionClient,
    finalizedExecutionBlockNumber: argonFinalizedExecutionHeader.blockNumber,
    gatewayAddress,
    minimumGatewayActivityNonce: 1n,
  });

  expect(discovered).toEqual([locators[1], locators[2]]);
  expect(
    readContractRequests.filter(({ functionName }) => functionName === 'activityBlockLocators'),
  ).toEqual([
    {
      functionName: 'activityBlockLocators',
      blockNumber: argonFinalizedExecutionHeader.blockNumber,
      locatorIndex: 1n,
    },
    {
      functionName: 'activityBlockLocators',
      blockNumber: argonFinalizedExecutionHeader.blockNumber,
      locatorIndex: 2n,
    },
    {
      functionName: 'activityBlockLocators',
      blockNumber: argonFinalizedExecutionHeader.blockNumber,
      locatorIndex: 3n,
    },
  ]);
});

it('builds gateway activity payloads from cached locator blocks across chained roots', async () => {
  const gatewayAddress = repeatHex('7f', 20);
  const log10 = createTransferToArgonStartedBlockLog({
    gatewayAddress,
    txHash: repeatHex('24', 32),
    transactionIndex: 0,
    logIndex: 0,
    nonce: 1n,
    argonAccountId: repeatHex('49', 32),
  });
  const log11 = createTransferToArgonStartedBlockLog({
    gatewayAddress,
    txHash: repeatHex('25', 32),
    transactionIndex: 0,
    logIndex: 0,
    nonce: 2n,
    argonAccountId: repeatHex('4a', 32),
  });
  const { argonClient, executionClient, argonFinalizedExecutionHeader } = createGatewayProofHarness(
    {
      gatewayAddress,
      latestArgonFinalizedHeader: {
        blockHash: repeatHex('f9', 32),
        blockNumber: 12n,
      },
      locators: [
        { blockNumber: 10n, logs: [log10] },
        { blockNumber: 11n, logs: [log11] },
      ],
    },
  );

  const locators = await discoverMissingGatewayActivityLocators({
    executionClient,
    finalizedExecutionBlockNumber: argonFinalizedExecutionHeader.blockNumber,
    gatewayAddress,
    minimumGatewayActivityNonce: 1n,
  });
  const { payloads } = await buildGatewayActivityStorageProofs(argonClient, {
    executionClient,
    finalizedExecutionHeader: argonFinalizedExecutionHeader,
    gatewayAddress,
    locators,
  });

  expect(payloads).toHaveLength(2);
  expect(
    payloads.flatMap(payload =>
      payload.activities.map(activity => activity.gatewayState.gatewayActivityNonce),
    ),
  ).toEqual([1n, 2n]);
});

it('builds a gateway payload for a chosen missing locator block', async () => {
  const gatewayAddress = repeatHex('7f', 20);
  const log10 = createTransferToArgonStartedBlockLog({
    gatewayAddress,
    txHash: repeatHex('24', 32),
    transactionIndex: 0,
    logIndex: 0,
    nonce: 1n,
    argonAccountId: repeatHex('49', 32),
  });
  const log11 = createTransferToArgonStartedBlockLog({
    gatewayAddress,
    txHash: repeatHex('25', 32),
    transactionIndex: 0,
    logIndex: 0,
    nonce: 2n,
    argonAccountId: repeatHex('4a', 32),
  });
  const { argonClient, executionClient, argonFinalizedExecutionHeader } = createGatewayProofHarness(
    {
      gatewayAddress,
      previousGatewayActivityNonce: 1n,
      latestArgonFinalizedHeader: {
        blockHash: repeatHex('f9', 32),
        blockNumber: 12n,
      },
      locators: [
        { blockNumber: 10n, logs: [log10] },
        { blockNumber: 11n, logs: [log11] },
      ],
    },
  );

  const discovered = await discoverMissingGatewayActivityLocators({
    executionClient,
    finalizedExecutionBlockNumber: argonFinalizedExecutionHeader.blockNumber,
    gatewayAddress,
    minimumGatewayActivityNonce: 2n,
  });
  const nextLocator = discovered[0];
  const { payloads } = await buildGatewayActivityStorageProofs(argonClient, {
    executionClient,
    finalizedExecutionHeader: argonFinalizedExecutionHeader,
    gatewayAddress,
    locators: [nextLocator],
  });
  const payload = payloads[0];

  expect(payload?.previousGatewayActivityNonce).toBe(1n);
  expect(payload?.gatewayActivityNonceRange).toEqual({ start: 2n, end: 2n });
  expect(payload?.executionBlockNumberRange).toEqual({ start: 11n, end: 11n });
  expect(payload?.proof.locatorIndex).toBe(nextLocator?.locatorIndex);
  expect(payload?.activities.map(activity => activity.gatewayState.gatewayActivityNonce)).toEqual([
    2n,
  ]);
});

it('defers a partial locator resume because whole locators must be proven atomically', async () => {
  const gatewayAddress = repeatHex('79', 20);
  const logs = [1n, 2n].map((nonce, index) =>
    createTransferToArgonStartedBlockLog({
      gatewayAddress,
      txHash: repeatHex('13', 32),
      transactionIndex: 0,
      logIndex: index,
      nonce,
      argonAccountId: toHex(index + 1, { size: 32 }),
    }),
  );
  const { argonClient, executionClient, locators, argonFinalizedExecutionHeader } =
    createGatewayProofHarness({
      gatewayAddress,
      previousGatewayActivityNonce: 1n,
      locators: [{ blockNumber: 10n, logs }],
    });

  await expect(
    buildGatewayActivityStorageProofs(argonClient, {
      executionClient,
      finalizedExecutionHeader: argonFinalizedExecutionHeader,
      gatewayAddress,
      locators: [locators[0]],
    }),
  ).resolves.toEqual({
    payloads: [],
    deferredLocators: [locators[0]],
  });
});

it('rejects a locator that exceeds maxActivitiesPerGatewayProof instead of splitting it', async () => {
  const gatewayAddress = repeatHex('7a', 20);
  const logs = [1n, 2n, 3n].map((nonce, index) =>
    createTransferToArgonStartedBlockLog({
      gatewayAddress,
      txHash: repeatHex('14', 32),
      transactionIndex: 0,
      logIndex: index,
      nonce,
      argonAccountId: toHex(index + 1, { size: 32 }),
    }),
  );
  const { argonClient, executionClient, locators, argonFinalizedExecutionHeader } =
    createGatewayProofHarness({
      gatewayAddress,
      consts: createGatewayProofConsts({ maxActivitiesPerGatewayProof: 2 }),
      locators: [{ blockNumber: 10n, logs }],
    });

  await expect(() =>
    buildGatewayActivityStorageProofs(argonClient, {
      executionClient,
      finalizedExecutionHeader: argonFinalizedExecutionHeader,
      gatewayAddress,
      locators: [locators[0]],
    }),
  ).rejects.toThrow('exceeds maxActivitiesPerGatewayProof');
});

it('defers locators when the latest finalized execution header is still behind the locator block', async () => {
  const gatewayAddress = repeatHex('7b', 20);
  const log = createTransferToArgonStartedBlockLog({
    gatewayAddress,
    txHash: repeatHex('15', 32),
    transactionIndex: 0,
    logIndex: 0,
    nonce: 1n,
    argonAccountId: repeatHex('44', 32),
  });
  const { argonClient, executionClient, locators, argonFinalizedExecutionHeader } =
    createGatewayProofHarness({
      gatewayAddress,
      latestArgonFinalizedHeader: {
        blockHash: repeatHex('fc', 32),
        blockNumber: 9n,
      },
      locators: [{ blockNumber: 10n, logs: [log] }],
    });

  await expect(
    buildGatewayActivityStorageProofs(argonClient, {
      executionClient,
      finalizedExecutionHeader: argonFinalizedExecutionHeader,
      gatewayAddress,
      locators: [locators[0]],
    }),
  ).resolves.toEqual({
    payloads: [],
    deferredLocators: [locators[0]],
  });
});

it('rejects zero gateway-proof bounds before building a gateway payload', async () => {
  const gatewayAddress = repeatHex('7d', 20);
  const log = createTransferToArgonStartedBlockLog({
    gatewayAddress,
    txHash: repeatHex('17', 32),
    transactionIndex: 0,
    logIndex: 0,
    nonce: 1n,
    argonAccountId: repeatHex('46', 32),
  });
  const { argonClient, executionClient, locators, argonFinalizedExecutionHeader } =
    createGatewayProofHarness({
      gatewayAddress,
      consts: createGatewayProofConsts({ maxActivitiesPerGatewayProof: 0 }),
      locators: [{ blockNumber: 10n, logs: [log] }],
    });

  await expect(() =>
    buildGatewayActivityStorageProofs(argonClient, {
      executionClient,
      finalizedExecutionHeader: argonFinalizedExecutionHeader,
      gatewayAddress,
      locators: [locators[0]],
    }),
  ).rejects.toThrow('Gateway proof requires maxActivitiesPerGatewayProof to be a positive integer');
});

type TestGatewayBlockLog =
  | ReturnType<typeof createTransferToArgonStartedBlockLog>
  | ReturnType<typeof createGlobalIssuanceCouncilRotatedBlockLog>;

function createGatewayProofHarness(args: {
  gatewayAddress: Hex;
  locators: Array<{
    blockNumber: bigint;
    logs: TestGatewayBlockLog[];
  }>;
  previousGatewayActivityNonce?: bigint;
  latestArgonFinalizedHeader?: {
    blockHash: Hex;
    blockNumber: bigint;
  };
  consts?: Pick<ArgonClient, 'consts'>['consts'];
  chainId?: bigint;
  mappingSlot?: bigint;
  readContractRequests?: Array<{
    functionName: string;
    blockNumber?: bigint;
    locatorIndex?: bigint;
  }>;
  runtimeSpecVersion?: number;
}) {
  const chainId = args.chainId ?? 1n;
  const mappingSlot = args.mappingSlot ?? 11n;
  const previousGatewayActivityNonce = args.previousGatewayActivityNonce ?? 0n;
  const latestArgonFinalizedHeader = args.latestArgonFinalizedHeader ?? {
    blockHash: repeatHex('ff', 32),
    blockNumber: args.locators.at(-1)?.blockNumber ?? 0n,
  };
  let previousLocatorHash = repeatHex('00', 32);
  const locatorFixtures = args.locators.map(({ blockNumber, logs }, index) => {
    const blockHash = repeatHex((index + 1).toString(16).padStart(2, '0'), 32);
    for (const log of logs) {
      log.blockHash = blockHash;
      log.blockNumber = blockNumber;
    }

    const locator = createGatewayLocator(
      args.gatewayAddress,
      chainId,
      blockNumber,
      logs,
      previousLocatorHash,
    );
    previousLocatorHash = hashMintingGatewayActivityBlockLocator(locator);

    return {
      logs,
      locator: {
        ...locator,
        locatorIndex: BigInt(index + 1),
      },
    };
  });
  const proofsByBlockNumber =
    locatorFixtures.length === 0
      ? {}
      : {
          [latestArgonFinalizedHeader.blockNumber.toString()]: {
            accountProof: [repeatHex('aa', 32)],
            storageProof: locatorFixtures.flatMap(
              ({ locator }, index) =>
                createGatewayStorageProof(BigInt(index + 1), locator, mappingSlot).storageProof,
            ),
          },
        };

  return {
    argonClient: createArgonGatewayClient({
      previousGatewayActivityNonce,
      argonFinalizedExecutionHeaders: [latestArgonFinalizedHeader],
      consts: args.consts,
      runtimeSpecVersion: args.runtimeSpecVersion,
    }),
    argonFinalizedExecutionHeader: latestArgonFinalizedHeader,
    previousGatewayActivityNonce,
    executionClient: createExecutionClient({
      blocks: [],
      chainId,
      activityBlockLocatorsMappingSlot: mappingSlot,
      readContractRequests: args.readContractRequests,
      locators: locatorFixtures.map(({ locator }) => locator),
      logsByBlockNumber: Object.fromEntries(
        locatorFixtures.map(({ locator, logs }) => [locator.blockNumber.toString(), logs]),
      ),
      proofsByBlockNumber,
    }),
    locators: locatorFixtures.map(({ locator }) => locator),
    proofsByBlockNumber,
  };
}

function createGatewayLocator(
  gatewayAddress: Hex,
  chainId: bigint,
  blockNumber: bigint,
  logs: TestGatewayBlockLog[],
  previousLocatorHash: Hex,
) {
  const activities = [...logs]
    .map(log => ({
      txHash: log.transactionHash,
      transactionIndex: log.transactionIndex,
      logIndex: log.logIndex,
      blockHash: log.blockHash,
      blockNumber: log.blockNumber,
      ...decodeEthereumGatewayActivityLog(log),
    }))
    .sort((left, right) =>
      left.gatewayState.gatewayActivityNonce < right.gatewayState.gatewayActivityNonce ? -1 : 1,
    );
  let activityRoot = previousLocatorHash;

  for (const activity of activities) {
    activityRoot = appendEthereumGatewayActivityRoot(
      activityRoot,
      hashEthereumGatewayActivity({ chainId, gatewayAddress }, activity),
    );
  }

  return {
    blockNumber,
    startGatewayActivityNonce: activities[0].gatewayState.gatewayActivityNonce,
    endGatewayActivityNonce: activities.at(-1)!.gatewayState.gatewayActivityNonce,
    previousLocatorHash,
    activityRoot,
  };
}

function createGatewayStorageProof(
  locatorIndex: bigint,
  locator: {
    blockNumber: bigint;
    startGatewayActivityNonce: bigint;
    endGatewayActivityNonce: bigint;
    activityRoot: Hex;
  },
  mappingSlot: bigint,
) {
  const [rangeSlot, rootSlot] = gatewayActivityLocatorSlots(locatorIndex, mappingSlot);

  return {
    storageProof: [
      {
        key: rangeSlot,
        value: BigInt(gatewayActivityLocatorRangeSlotValue(locator)),
        proof: [repeatHex('01', 32), repeatHex('02', 32)],
      },
      {
        key: rootSlot,
        value: BigInt(locator.activityRoot),
        proof: [repeatHex('02', 32), repeatHex('03', 32)],
      },
    ],
  };
}

function gatewayActivityLocatorSlots(locatorIndex: bigint, mappingSlot: bigint): [Hex, Hex] {
  const baseSlot = BigInt(
    keccak256(concatHex([toHex(locatorIndex, { size: 32 }), toHex(mappingSlot, { size: 32 })])),
  );

  return [toHex(baseSlot, { size: 32 }), toHex(baseSlot + 1n, { size: 32 })];
}

function gatewayActivityLocatorRangeSlotValue(locator: {
  blockNumber: bigint;
  startGatewayActivityNonce: bigint;
  endGatewayActivityNonce: bigint;
}): Hex {
  return toHex(
    (locator.endGatewayActivityNonce << 128n) |
      (locator.startGatewayActivityNonce << 64n) |
      locator.blockNumber,
    { size: 32 },
  );
}
