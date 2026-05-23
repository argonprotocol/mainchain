import { expect, it } from 'vitest';
import { createMPT, verifyMPTWithMerkleProof } from '@ethereumjs/mpt';
import { bytesToHex, hexToBytes, type Hex } from 'viem';
import {
  buildEthereumCombinedReceiptProof,
  buildEthereumEventProof,
  encodeEthereumReceiptForProof,
  encodeReceiptTrieKey,
} from '../EthereumProof';
import {
  createExecutionBlock,
  createExecutionClient,
  createLegacyReceipt,
  repeatHex,
} from './ethereumProofTestUtils';
import productionInboundReceipt from './fixtures/productionInboundReceipt.json';

type EncodedReceiptInput = Parameters<typeof encodeEthereumReceiptForProof>[0];

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
  const txHash0 = repeatHex('aa', 32);
  const txHash1 = repeatHex('bb', 32);
  const receipt0 = createLegacyReceipt({
    txHash: txHash0,
    transactionIndex: 0,
    logs: [
      {
        address: repeatHex('33', 20),
        topics: [repeatHex('44', 32)],
        data: '0x1234',
      },
    ],
  });
  const receipt1 = createLegacyReceipt({
    txHash: txHash1,
    transactionIndex: 1,
    cumulativeGasUsed: 42_000n,
    logs: [
      {
        address: repeatHex('55', 20),
        topics: [repeatHex('66', 32)],
        data: '0xabcd',
      },
    ],
  });
  const targetBlock = await createExecutionBlock({
    number: 10n,
    receipts: [receipt0, receipt1],
    gasUsed: 63_000n,
    timestamp: 1n,
  });
  const argonFinalizedExecutionHeaderBlock = await createExecutionBlock({
    number: 11n,
    parentHash: targetBlock.hash,
    transactions: [],
    timestamp: 2n,
  });
  const executionClient = createExecutionClient({
    blocks: [targetBlock, argonFinalizedExecutionHeaderBlock],
    receipts: [receipt0, receipt1],
  });
  const argonFinalizedExecutionHeader = {
    blockHash: argonFinalizedExecutionHeaderBlock.hash,
    blockNumber: 11n,
  };

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
      hexToBytes(targetBlock.receiptsRoot),
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
  const txHash0 = repeatHex('aa', 32);
  const txHash1 = repeatHex('bb', 32);
  const receipt0 = createLegacyReceipt({
    txHash: txHash0,
    transactionIndex: 0,
    logs: [
      {
        address: repeatHex('11', 20),
        topics: [repeatHex('22', 32)],
        data: '0x1234',
      },
    ],
  });
  const receipt1 = createLegacyReceipt({
    txHash: txHash1,
    transactionIndex: 0,
    cumulativeGasUsed: 42_000n,
    logs: [
      {
        address: repeatHex('33', 20),
        topics: [repeatHex('44', 32)],
        data: '0xabcd',
      },
    ],
  });
  const olderBlock = await createExecutionBlock({
    number: 10n,
    receipts: [receipt0],
    timestamp: 1n,
  });
  const newerBlock = await createExecutionBlock({
    number: 11n,
    parentHash: olderBlock.hash,
    receipts: [receipt1],
    gasUsed: 42_000n,
    timestamp: 2n,
  });
  const argonFinalizedExecutionHeaderBlock = await createExecutionBlock({
    number: 12n,
    parentHash: newerBlock.hash,
    transactions: [],
    timestamp: 3n,
  });
  const executionClient = createExecutionClient({
    blocks: [olderBlock, newerBlock, argonFinalizedExecutionHeaderBlock],
    receipts: [receipt0, receipt1],
  });

  const proof = await buildEthereumEventProof(
    { executionClient },
    {
      blockHash: argonFinalizedExecutionHeaderBlock.hash,
      blockNumber: 12n,
    },
    [[{ txHash: txHash0, receipt: receipt0 }], [{ txHash: txHash1, receipt: receipt1 }]],
  );

  expect(proof.executionBlockProof.targetToAnchorHeaderChain).toHaveLength(2);
  expect(proof.blocks).toHaveLength(2);
  expect(proof.blocks.map(block => block.targetBlockNumber)).toEqual([10, 11]);
  expect(proof.blocks[0].receiptProof.receipts[0]?.transactionIndex).toBe(0);
  expect(proof.blocks[1].receiptProof.receipts[0]?.transactionIndex).toBe(0);
});

it('uses an empty shared header path when the target block is the Argon finalized execution header', async () => {
  const txHash = repeatHex('aa', 32);
  const receipt = createLegacyReceipt({
    txHash,
    transactionIndex: 0,
    logs: [
      {
        address: repeatHex('55', 20),
        topics: [repeatHex('66', 32)],
        data: '0x1234',
      },
    ],
  });
  const argonFinalizedExecutionHeaderBlock = await createExecutionBlock({
    number: 10n,
    receipts: [receipt],
    timestamp: 1n,
  });
  const executionClient = createExecutionClient({
    blocks: [argonFinalizedExecutionHeaderBlock],
    receipts: [receipt],
  });

  const proof = await buildEthereumEventProof(
    { executionClient },
    {
      blockHash: argonFinalizedExecutionHeaderBlock.hash,
      blockNumber: 10n,
    },
    [[{ txHash, receipt }]],
  );

  expect(proof.executionBlockProof.anchorBlockHash).toBe(argonFinalizedExecutionHeaderBlock.hash);
  expect(proof.executionBlockProof.targetToAnchorHeaderChain).toHaveLength(0);
  expect(proof.blocks).toHaveLength(1);
  expect(proof.blocks[0].targetBlockNumber).toBe(10);
});

it('rejects combined receipt proofs that exceed the runtime receipt-count bound', async () => {
  const receipts = Array.from({ length: 33 }, (_, transactionIndex) =>
    createLegacyReceipt({
      txHash: repeatHex(transactionIndex.toString(16).padStart(2, '0'), 32),
      transactionIndex,
      cumulativeGasUsed: BigInt((transactionIndex + 1) * 21_000),
      logs: [
        {
          address: repeatHex('55', 20),
          topics: [repeatHex('66', 32)],
          data: '0x1234',
        },
      ],
    }),
  );
  const targetBlock = await createExecutionBlock({
    number: 10n,
    receipts,
    gasUsed: receipts.at(-1)!.cumulativeGasUsed,
    timestamp: 1n,
  });
  const executionClient = createExecutionClient({
    blocks: [targetBlock],
    receipts,
  });

  await expect(
    buildEthereumCombinedReceiptProof(
      executionClient,
      targetBlock.hash,
      receipts.map(({ transactionIndex }) => transactionIndex),
      targetBlock.receiptsRoot,
    ),
  ).rejects.toThrow('runtime receipt-count bound');
});
