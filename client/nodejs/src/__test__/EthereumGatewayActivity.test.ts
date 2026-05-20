import { expect, it } from 'vitest';
import { MintingGatewayEvents } from '@argonprotocol/ethereum-contracts';
import { toHex, type Hex } from 'viem';
import type { ArgonClient } from '../index';
import type { ArgonFinalizedExecutionHeader } from '../EthereumProof';
import {
  buildGatewayActivityProofPayload,
  decodeEthereumGatewayActivityLog,
  decodeEthereumTransferToArgonStartedLog,
} from '../EthereumGatewayActivity';
import {
  createArgonGatewayClient,
  createExecutionBlock,
  createExecutionClient,
  createGatewayProofConsts,
  createGlobalIssuanceCouncilRotatedBlockLog,
  createLegacyReceipt,
  repeatHex,
  createTransferToArgonStartedBlockLog,
} from './ethereumProofTestUtils';

type GatewayActivityBlockLog =
  | ReturnType<typeof createTransferToArgonStartedBlockLog>
  | ReturnType<typeof createGlobalIssuanceCouncilRotatedBlockLog>;

it('decodes TransferToArgonStarted logs into gateway activity state', async () => {
  const log = createTransferToArgonStartedBlockLog({
    gatewayAddress: repeatHex('77', 20),
    txHash: repeatHex('10', 32),
    transactionIndex: 0,
    logIndex: 0,
    nonce: 7n,
    argonAccountId: repeatHex('33', 32),
  });

  expect(
    decodeEthereumTransferToArgonStartedLog({
      topics: log.topics,
      data: log.data,
    }),
  ).toEqual({
    from: repeatHex('11', 20),
    token: repeatHex('22', 20),
    amount: 250n,
    argonAccountId: repeatHex('33', 32),
    gatewayState: {
      gatewayActivityNonce: 7n,
      argonApprovalsNonce: 0n,
      argonCirculation: 750n,
      argonotCirculation: 2_000n,
    },
  });
});

it('decodes non-transfer gateway activity logs', async () => {
  const log = createGlobalIssuanceCouncilRotatedBlockLog({
    gatewayAddress: repeatHex('77', 20),
    txHash: repeatHex('10', 32),
    transactionIndex: 0,
    logIndex: 1,
    nonce: 9n,
  });

  expect(
    decodeEthereumGatewayActivityLog({
      topics: log.topics,
      data: log.data,
    }),
  ).toEqual({
    kind: MintingGatewayEvents.GlobalIssuanceCouncilRotated.name,
    councilHash: repeatHex('61', 32),
    relayerArgonAccountId: repeatHex('62', 32),
    gatewayState: {
      gatewayActivityNonce: 9n,
      argonApprovalsNonce: 4n,
      argonCirculation: 750n,
      argonotCirculation: 2_000n,
    },
  });
});

it('builds a proveGatewayActivity payload from uncovered TransferToArgonStarted locators', async () => {
  const gatewayAddress = repeatHex('77', 20);
  const block10 = await createGatewayActivityBlock({
    number: 10n,
    logs: [
      createTransferToArgonStartedBlockLog({
        gatewayAddress,
        txHash: repeatHex('10', 32),
        transactionIndex: 0,
        logIndex: 0,
        nonce: 1n,
        argonAccountId: repeatHex('41', 32),
      }),
      createTransferToArgonStartedBlockLog({
        gatewayAddress,
        txHash: repeatHex('11', 32),
        transactionIndex: 1,
        logIndex: 0,
        nonce: 2n,
        argonAccountId: repeatHex('42', 32),
      }),
    ],
  });
  const block11 = await createGatewayActivityBlock({
    number: 11n,
    parentHash: block10.block.hash,
    logs: [
      createTransferToArgonStartedBlockLog({
        gatewayAddress,
        txHash: repeatHex('12', 32),
        transactionIndex: 0,
        logIndex: 0,
        nonce: 3n,
        argonAccountId: repeatHex('43', 32),
      }),
    ],
  });
  const argonFinalizedExecutionHeaderBlock = await createExecutionBlock({
    number: 12n,
    parentHash: block11.block.hash,
    transactions: [],
  });

  const proofPlan = await buildGatewayProofPlan({
    gatewayAddress,
    blocks: [block10, block11],
    locators: [
      { blockNumber: 10n, startGatewayActivityNonce: 1n, endGatewayActivityNonce: 2n },
      { blockNumber: 11n, startGatewayActivityNonce: 3n, endGatewayActivityNonce: 3n },
    ],
    argonFinalizedExecutionHeaders: [
      {
        blockHash: argonFinalizedExecutionHeaderBlock.hash,
        blockNumber: 12n,
      },
    ],
    previousGatewayActivityNonce: 1n,
    extraExecutionBlocks: [argonFinalizedExecutionHeaderBlock],
    throughExecutionBlockNumber: 11n,
  });
  const payload = proofPlan.payload;

  expect(proofPlan.latestGatewayActivityNonce).toBe(3n);
  expect(proofPlan.payloadUpToGatewayActivityNonce).toBe(3n);
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
  const gatewayAddress = repeatHex('77', 20);
  const block10 = await createGatewayActivityBlock({
    number: 10n,
    logs: [
      createTransferToArgonStartedBlockLog({
        gatewayAddress,
        txHash: repeatHex('10', 32),
        transactionIndex: 0,
        logIndex: 0,
        nonce: 1n,
        argonAccountId: repeatHex('41', 32),
      }),
      createGlobalIssuanceCouncilRotatedBlockLog({
        gatewayAddress,
        txHash: repeatHex('10', 32),
        transactionIndex: 0,
        logIndex: 1,
        nonce: 2n,
      }),
    ],
  });
  const argonFinalizedExecutionHeaderBlock = await createExecutionBlock({
    number: 11n,
    parentHash: block10.block.hash,
    transactions: [],
  });

  const proofPlan = await buildGatewayProofPlan({
    gatewayAddress,
    blocks: [block10],
    locators: [{ blockNumber: 10n, startGatewayActivityNonce: 1n, endGatewayActivityNonce: 2n }],
    argonFinalizedExecutionHeaders: [
      {
        blockHash: argonFinalizedExecutionHeaderBlock.hash,
        blockNumber: 11n,
      },
    ],
    extraExecutionBlocks: [argonFinalizedExecutionHeaderBlock],
  });
  const payload = proofPlan.payload;

  expect(proofPlan.latestGatewayActivityNonce).toBe(2n);
  expect(proofPlan.payloadUpToGatewayActivityNonce).toBe(2n);
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
  const gatewayAddress = repeatHex('91', 20);
  const block10 = await createGatewayActivityBlock({
    number: 10n,
    logs: [1n, 2n, 3n].map((nonce, index) =>
      createTransferToArgonStartedBlockLog({
        gatewayAddress,
        txHash: repeatHex('93', 32),
        transactionIndex: 0,
        logIndex: index,
        nonce,
        argonAccountId: toHex(index + 1, { size: 32 }),
      }),
    ),
  });
  const argonFinalizedExecutionHeaderBlock = await createExecutionBlock({
    number: 11n,
    parentHash: block10.block.hash,
    transactions: [],
  });

  const proofPlan = await buildGatewayProofPlan({
    gatewayAddress,
    blocks: [block10],
    locators: [{ blockNumber: 10n, startGatewayActivityNonce: 1n, endGatewayActivityNonce: 3n }],
    argonFinalizedExecutionHeaders: [
      {
        blockHash: argonFinalizedExecutionHeaderBlock.hash,
        blockNumber: 11n,
      },
    ],
    consts: createGatewayProofConsts({ maxActivitiesPerReceiptProof: 1 }),
    extraExecutionBlocks: [argonFinalizedExecutionHeaderBlock],
  });

  expect(proofPlan.latestGatewayActivityNonce).toBe(3n);
  expect(proofPlan.payloadUpToGatewayActivityNonce).toBe(3n);
  expect(proofPlan.payload?.gatewayActivityNonceRange).toEqual({ start: 1n, end: 3n });
  expect(proofPlan.payload?.proof.blocks).toHaveLength(3);
  expect(proofPlan.payload?.proof.blocks.map(block => block.receiptLogs)).toEqual([
    [expect.objectContaining({ transactionIndex: 0 })],
    [expect.objectContaining({ transactionIndex: 0 })],
    [expect.objectContaining({ transactionIndex: 0 })],
  ]);
});

it('splits one execution block when the combined receipt proof would exceed the runtime receipt bound', async () => {
  const gatewayAddress = repeatHex('a1', 20);
  const activities = Array.from({ length: 33 }, (_, index) =>
    createTransferToArgonStartedBlockLog({
      gatewayAddress,
      txHash: repeatHex((index + 1).toString(16).padStart(2, '0'), 32),
      transactionIndex: index,
      logIndex: 0,
      nonce: BigInt(index + 1),
      argonAccountId: toHex(index + 1, { size: 32 }),
    }),
  );
  const block10 = await createGatewayActivityBlock({
    number: 10n,
    logs: activities,
  });
  const argonFinalizedExecutionHeaderBlock = await createExecutionBlock({
    number: 11n,
    parentHash: block10.block.hash,
    transactions: [],
  });

  const proofPlan = await buildGatewayProofPlan({
    gatewayAddress,
    blocks: [block10],
    locators: [{ blockNumber: 10n, startGatewayActivityNonce: 1n, endGatewayActivityNonce: 33n }],
    argonFinalizedExecutionHeaders: [
      {
        blockHash: argonFinalizedExecutionHeaderBlock.hash,
        blockNumber: 11n,
      },
    ],
    consts: createGatewayProofConsts({ maxActivitiesPerReceiptProof: 40 }),
    extraExecutionBlocks: [argonFinalizedExecutionHeaderBlock],
  });

  expect(proofPlan.latestGatewayActivityNonce).toBe(33n);
  expect(proofPlan.payloadUpToGatewayActivityNonce).toBe(33n);
  expect(proofPlan.payload?.proof.blocks).toHaveLength(2);
  expect(proofPlan.payload?.proof.blocks[0]?.receiptProof.receipts).toHaveLength(32);
  expect(proofPlan.payload?.proof.blocks[1]?.receiptProof.receipts).toHaveLength(1);
});

it('resumes from the next gateway activity when a receipt-bound split stops mid block', async () => {
  const gatewayAddress = repeatHex('a2', 20);
  const activities = Array.from({ length: 33 }, (_, index) =>
    createTransferToArgonStartedBlockLog({
      gatewayAddress,
      txHash: repeatHex((index + 1).toString(16).padStart(2, '0'), 32),
      transactionIndex: index,
      logIndex: 0,
      nonce: BigInt(index + 1),
      argonAccountId: toHex(index + 1, { size: 32 }),
    }),
  );
  const block10 = await createGatewayActivityBlock({
    number: 10n,
    logs: activities,
  });
  const argonFinalizedExecutionHeaderBlock = await createExecutionBlock({
    number: 11n,
    parentHash: block10.block.hash,
    transactions: [],
  });

  const firstProofPlan = await buildGatewayProofPlan({
    gatewayAddress,
    blocks: [block10],
    locators: [{ blockNumber: 10n, startGatewayActivityNonce: 1n, endGatewayActivityNonce: 33n }],
    argonFinalizedExecutionHeaders: [
      {
        blockHash: argonFinalizedExecutionHeaderBlock.hash,
        blockNumber: 11n,
      },
    ],
    consts: createGatewayProofConsts({
      maxActivitiesPerReceiptProof: 40,
      maxReceiptProofsPerExtrinsic: 1,
    }),
    extraExecutionBlocks: [argonFinalizedExecutionHeaderBlock],
  });

  expect(firstProofPlan.latestGatewayActivityNonce).toBe(33n);
  expect(firstProofPlan.payloadUpToGatewayActivityNonce).toBe(32n);
  expect(firstProofPlan.payload?.gatewayActivityNonceRange).toEqual({ start: 1n, end: 32n });
  expect(firstProofPlan.payload?.proof.blocks).toHaveLength(1);
  expect(firstProofPlan.payload?.proof.blocks[0]?.receiptProof.receipts).toHaveLength(32);

  const secondProofPlan = await buildGatewayProofPlan({
    gatewayAddress,
    blocks: [block10],
    locators: [{ blockNumber: 10n, startGatewayActivityNonce: 1n, endGatewayActivityNonce: 33n }],
    argonFinalizedExecutionHeaders: [
      {
        blockHash: argonFinalizedExecutionHeaderBlock.hash,
        blockNumber: 11n,
      },
    ],
    previousGatewayActivityNonce: 32n,
    consts: createGatewayProofConsts({
      maxActivitiesPerReceiptProof: 40,
      maxReceiptProofsPerExtrinsic: 1,
    }),
    extraExecutionBlocks: [argonFinalizedExecutionHeaderBlock],
  });

  expect(secondProofPlan.latestGatewayActivityNonce).toBe(33n);
  expect(secondProofPlan.payloadUpToGatewayActivityNonce).toBe(33n);
  expect(secondProofPlan.payload?.gatewayActivityNonceRange).toEqual({ start: 33n, end: 33n });
  expect(
    secondProofPlan.payload?.activities.map(activity => activity.gatewayState.gatewayActivityNonce),
  ).toEqual([33n]);
  expect(secondProofPlan.payload?.proof.blocks).toHaveLength(1);
  expect(secondProofPlan.payload?.proof.blocks[0]?.receiptProof.receipts).toHaveLength(1);
});

it('resumes from the next gateway activity when a capped payload stops mid block', async () => {
  const gatewayAddress = repeatHex('94', 20);
  const block10 = await createGatewayActivityBlock({
    number: 10n,
    logs: [1n, 2n, 3n].map((nonce, index) =>
      createTransferToArgonStartedBlockLog({
        gatewayAddress,
        txHash: repeatHex('96', 32),
        transactionIndex: 0,
        logIndex: index,
        nonce,
        argonAccountId: toHex(index + 1, { size: 32 }),
      }),
    ),
  });
  const argonFinalizedExecutionHeaderBlock = await createExecutionBlock({
    number: 11n,
    parentHash: block10.block.hash,
    transactions: [],
  });

  const proofPlan = await buildGatewayProofPlan({
    gatewayAddress,
    blocks: [block10],
    locators: [{ blockNumber: 10n, startGatewayActivityNonce: 1n, endGatewayActivityNonce: 3n }],
    argonFinalizedExecutionHeaders: [
      {
        blockHash: argonFinalizedExecutionHeaderBlock.hash,
        blockNumber: 11n,
      },
    ],
    previousGatewayActivityNonce: 1n,
    consts: createGatewayProofConsts({
      maxActivitiesPerReceiptProof: 1,
      maxReceiptProofsPerExtrinsic: 1,
    }),
    extraExecutionBlocks: [argonFinalizedExecutionHeaderBlock],
  });
  const payload = proofPlan.payload;

  expect(proofPlan.latestGatewayActivityNonce).toBe(3n);
  expect(proofPlan.payloadUpToGatewayActivityNonce).toBe(2n);
  expect(payload?.gatewayActivityNonceRange).toEqual({ start: 2n, end: 2n });
  expect(payload?.executionBlockNumberRange).toEqual({ start: 10n, end: 10n });
  expect(payload?.activities.map(activity => activity.gatewayState.gatewayActivityNonce)).toEqual([
    2n,
  ]);
  expect(payload?.proof.blocks).toHaveLength(1);
  expect(payload?.proof.blocks[0].receiptLogs.map(log => log.transactionIndex)).toEqual([0]);
});

it('does not load later gateway blocks once the runtime proof-block bound is filled', async () => {
  const gatewayAddress = repeatHex('77', 20);
  const block10 = await createGatewayActivityBlock({
    number: 10n,
    logs: [
      createTransferToArgonStartedBlockLog({
        gatewayAddress,
        txHash: repeatHex('10', 32),
        transactionIndex: 0,
        logIndex: 0,
        nonce: 1n,
        argonAccountId: repeatHex('41', 32),
      }),
    ],
  });
  const argonFinalizedExecutionHeaderBlock = await createExecutionBlock({
    number: 11n,
    parentHash: block10.block.hash,
    transactions: [],
  });

  const proofPlan = await buildGatewayProofPlan({
    gatewayAddress,
    blocks: [block10],
    locators: [
      { blockNumber: 10n, startGatewayActivityNonce: 1n, endGatewayActivityNonce: 1n },
      { blockNumber: 11n, startGatewayActivityNonce: 2n, endGatewayActivityNonce: 2n },
    ],
    argonFinalizedExecutionHeaders: [
      {
        blockHash: argonFinalizedExecutionHeaderBlock.hash,
        blockNumber: 11n,
      },
    ],
    consts: createGatewayProofConsts({ maxReceiptProofsPerExtrinsic: 1 }),
    extraExecutionBlocks: [argonFinalizedExecutionHeaderBlock],
  });
  const payload = proofPlan.payload;

  expect(proofPlan.latestGatewayActivityNonce).toBe(2n);
  expect(proofPlan.payloadUpToGatewayActivityNonce).toBe(1n);
  expect(payload?.gatewayActivityNonceRange).toEqual({ start: 1n, end: 1n });
  expect(payload?.executionBlockNumberRange).toEqual({ start: 10n, end: 10n });
  expect(payload?.proof.blocks).toHaveLength(1);
});

it('falls back to an older Argon finalized execution header when the latest one is on another branch', async () => {
  const gatewayAddress = repeatHex('87', 20);
  const block10 = await createGatewayActivityBlock({
    number: 10n,
    logs: [
      createTransferToArgonStartedBlockLog({
        gatewayAddress,
        txHash: repeatHex('21', 32),
        transactionIndex: 0,
        logIndex: 0,
        nonce: 1n,
        argonAccountId: repeatHex('71', 32),
      }),
    ],
  });
  const canonicalBlock11 = await createExecutionBlock({
    number: 11n,
    parentHash: block10.block.hash,
    transactions: [],
  });
  const canonicalBlock12 = await createExecutionBlock({
    number: 12n,
    parentHash: canonicalBlock11.hash,
    transactions: [],
  });
  const alternateBlock10 = await createExecutionBlock({
    number: 10n,
    parentHash: repeatHex('aa', 32),
    transactions: [],
  });
  const alternateBlock11 = await createExecutionBlock({
    number: 11n,
    parentHash: alternateBlock10.hash,
    transactions: [],
  });
  const alternateBlock12 = await createExecutionBlock({
    number: 12n,
    parentHash: alternateBlock11.hash,
    transactions: [],
  });

  const proofPlan = await buildGatewayProofPlan({
    gatewayAddress,
    blocks: [block10],
    locators: [{ blockNumber: 10n, startGatewayActivityNonce: 1n, endGatewayActivityNonce: 1n }],
    argonFinalizedExecutionHeaders: [
      { blockHash: canonicalBlock12.hash, blockNumber: 12n },
      { blockHash: alternateBlock12.hash, blockNumber: 12n },
    ],
    extraExecutionBlocks: [
      canonicalBlock11,
      canonicalBlock12,
      alternateBlock10,
      alternateBlock11,
      alternateBlock12,
    ],
  });

  expect(proofPlan.latestGatewayActivityNonce).toBe(1n);
  expect(proofPlan.payloadUpToGatewayActivityNonce).toBe(1n);
  expect(proofPlan.payload?.proof.executionBlockProof.anchorBlockHash).toBe(canonicalBlock12.hash);
});

it('stops a gateway activity payload at the Argon finalized execution header', async () => {
  const gatewayAddress = repeatHex('67', 20);
  const block10 = await createGatewayActivityBlock({
    number: 10n,
    logs: [
      createTransferToArgonStartedBlockLog({
        gatewayAddress,
        txHash: repeatHex('12', 32),
        transactionIndex: 0,
        logIndex: 0,
        nonce: 1n,
        argonAccountId: repeatHex('51', 32),
      }),
    ],
  });
  const block11 = await createGatewayActivityBlock({
    number: 11n,
    logs: [
      createTransferToArgonStartedBlockLog({
        gatewayAddress,
        txHash: repeatHex('13', 32),
        transactionIndex: 0,
        logIndex: 0,
        nonce: 2n,
        argonAccountId: repeatHex('52', 32),
      }),
    ],
  });

  const proofPlan = await buildGatewayProofPlan({
    gatewayAddress,
    blocks: [block10, block11],
    locators: [
      { blockNumber: 10n, startGatewayActivityNonce: 1n, endGatewayActivityNonce: 1n },
      { blockNumber: 11n, startGatewayActivityNonce: 2n, endGatewayActivityNonce: 2n },
    ],
    argonFinalizedExecutionHeaders: [{ blockHash: block10.block.hash, blockNumber: 10n }],
  });
  const payload = proofPlan.payload;

  expect(proofPlan.latestGatewayActivityNonce).toBe(2n);
  expect(proofPlan.payloadUpToGatewayActivityNonce).toBe(1n);
  expect(payload?.gatewayActivityNonceRange).toEqual({ start: 1n, end: 1n });
  expect(payload?.executionBlockNumberRange).toEqual({ start: 10n, end: 10n });
  expect(payload?.proof.blocks).toHaveLength(1);
});

it('stops a gateway activity payload before a later proof chunk leaves the Argon finalized header chain', async () => {
  const gatewayAddress = repeatHex('57', 20);
  const block10 = await createGatewayActivityBlock({
    number: 10n,
    logs: [
      createTransferToArgonStartedBlockLog({
        gatewayAddress,
        txHash: repeatHex('14', 32),
        transactionIndex: 0,
        logIndex: 0,
        nonce: 1n,
        argonAccountId: repeatHex('61', 32),
      }),
    ],
  });
  const wrongBlock11 = await createGatewayActivityBlock({
    number: 11n,
    parentHash: repeatHex('ff', 32),
    logs: [
      createTransferToArgonStartedBlockLog({
        gatewayAddress,
        txHash: repeatHex('15', 32),
        transactionIndex: 0,
        logIndex: 0,
        nonce: 2n,
        argonAccountId: repeatHex('62', 32),
      }),
    ],
  });
  const canonicalBlock11 = await createExecutionBlock({
    number: 11n,
    parentHash: block10.block.hash,
    transactions: wrongBlock11.receipts.map(({ transactionHash }) => transactionHash),
    receipts: wrongBlock11.receipts.map(receipt =>
      createLegacyReceipt({
        txHash: receipt.transactionHash,
        transactionIndex: receipt.transactionIndex,
        cumulativeGasUsed: receipt.cumulativeGasUsed,
        logs: receipt.logs.map(({ address, topics, data }) => ({ address, topics, data })),
      }),
    ),
    timestamp: 11n,
  });
  const argonFinalizedExecutionHeaderBlock = await createExecutionBlock({
    number: 12n,
    parentHash: canonicalBlock11.hash,
    transactions: [],
  });

  const proofPlan = await buildGatewayProofPlan({
    gatewayAddress,
    blocks: [block10, wrongBlock11],
    locators: [
      { blockNumber: 10n, startGatewayActivityNonce: 1n, endGatewayActivityNonce: 1n },
      { blockNumber: 11n, startGatewayActivityNonce: 2n, endGatewayActivityNonce: 2n },
    ],
    argonFinalizedExecutionHeaders: [
      {
        blockHash: argonFinalizedExecutionHeaderBlock.hash,
        blockNumber: 12n,
      },
    ],
    extraExecutionBlocks: [canonicalBlock11, argonFinalizedExecutionHeaderBlock],
  });
  const payload = proofPlan.payload;

  expect(proofPlan.latestGatewayActivityNonce).toBe(2n);
  expect(proofPlan.payloadUpToGatewayActivityNonce).toBe(1n);
  expect(payload?.gatewayActivityNonceRange).toEqual({ start: 1n, end: 1n });
  expect(payload?.executionBlockNumberRange).toEqual({ start: 10n, end: 10n });
  expect(payload?.proof.blocks).toHaveLength(1);
});

it('rejects zero receipt-proof bounds before building a gateway payload', async () => {
  const argonClient = createArgonGatewayClient({
    consts: createGatewayProofConsts({ maxReceiptProofsPerExtrinsic: 0 }),
  }) as unknown as Pick<ArgonClient, 'consts'>;

  await expect(() =>
    buildGatewayActivityProofPayload(argonClient as never, {
      gatewayAddress: repeatHex('77', 20),
    }),
  ).rejects.toThrow('Gateway proof requires maxReceiptProofsPerExtrinsic to be a positive integer');
});

async function createGatewayActivityBlock(args: {
  number: bigint;
  logs: GatewayActivityBlockLog[];
  parentHash?: Hex;
}): Promise<{
  block: Awaited<ReturnType<typeof createExecutionBlock>>;
  receipts: ReturnType<typeof createLegacyReceipt>[];
  logs: GatewayActivityBlockLog[];
}> {
  const receipts = [...new Map(args.logs.map(log => [log.transactionHash, log])).values()]
    .sort((left, right) => left.transactionIndex - right.transactionIndex)
    .map(log =>
      createLegacyReceipt({
        txHash: log.transactionHash,
        transactionIndex: log.transactionIndex,
        cumulativeGasUsed: 21_000n * BigInt(log.transactionIndex + 1),
        logs: args.logs
          .filter(candidate => candidate.transactionHash === log.transactionHash)
          .map(({ address, topics, data }) => ({
            address,
            topics: [...topics] as [] | [Hex, ...Hex[]],
            data,
          })),
      }),
    );
  const block = await createExecutionBlock({
    number: args.number,
    parentHash: args.parentHash,
    receipts,
    blockLogs: args.logs,
    timestamp: args.number,
  });

  return { block, receipts, logs: args.logs };
}

async function buildGatewayProofPlan(args: {
  gatewayAddress: Hex;
  blocks: Array<Awaited<ReturnType<typeof createGatewayActivityBlock>>>;
  locators: Array<{
    blockNumber: bigint;
    startGatewayActivityNonce: bigint;
    endGatewayActivityNonce: bigint;
  }>;
  argonFinalizedExecutionHeaders: ArgonFinalizedExecutionHeader[];
  previousGatewayActivityNonce?: bigint;
  consts?: Pick<ArgonClient, 'consts'>['consts'];
  throughExecutionBlockNumber?: bigint;
  extraExecutionBlocks?: Awaited<ReturnType<typeof createExecutionBlock>>[];
}) {
  const executionClient = createExecutionClient({
    blocks: [...args.blocks.map(({ block }) => block), ...(args.extraExecutionBlocks ?? [])],
    receipts: args.blocks.flatMap(({ receipts }) => receipts),
    logsByBlockNumber: Object.fromEntries(
      args.blocks.map(({ block, logs }) => [BigInt(block.number).toString(), logs]),
    ),
    locators: args.locators,
  });
  const argonClient = createArgonGatewayClient({
    previousGatewayActivityNonce: args.previousGatewayActivityNonce,
    argonFinalizedExecutionHeaders: args.argonFinalizedExecutionHeaders,
    consts: args.consts,
  });

  return buildGatewayActivityProofPayload(argonClient, {
    gatewayAddress: args.gatewayAddress,
    executionClient,
    throughExecutionBlockNumber: args.throughExecutionBlockNumber,
  });
}
