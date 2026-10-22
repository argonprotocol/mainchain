import { createBlockHeaderFromRPC, type JSONRPCBlock } from '@ethereumjs/block';
import { mintingGatewayActivityHashingFixture } from '@argonprotocol/ethereum-contracts/fixtures';
import { expect, it, vi } from 'vitest';
import { createMPT, createMerkleProof, verifyMPTWithMerkleProof } from '@ethereumjs/mpt';
import {
  bytesToHex,
  concatHex,
  fromRlp,
  sliceHex,
  encodeAbiParameters,
  encodeEventTopics,
  getAddress,
  hexToBytes,
  keccak256,
  toHex,
  toRlp,
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
  captureGatewayActivityStorageProofs,
  discoverMissingGatewayActivityLocators,
  supportsGatewayActivityReceiptProofs,
  supportsGatewayActivityStorageProofs,
  type MissingGatewayActivityLocator,
} from '../EthereumGatewayActivityProof';
import productionInboundReceipt from './fixtures/productionInboundReceipt.json';
import {
  createArgonGatewayClient,
  createExecutionBlock,
  createExecutionClient,
  createGatewayProofConsts,
  createGlobalIssuanceCouncilRotatedBlockLog,
  createLegacyReceipt,
  createTransferOutOfArgonFinalizedBlockLog,
  createTransferToArgonStartedBlockLog,
  repeatHex,
} from './ethereumProofTestUtils';
import v4Purchase from '../../../../chains/ethereum/fixtures/uniswap-v4-purchase.json';
import v4Refund from '../../../../chains/ethereum/fixtures/uniswap-v4-purchase-refund.json';

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

it('supports the base spec-161 receipt call without requiring storage proofs', () => {
  const client = createArgonGatewayClient({ runtimeSpecVersion: 161, storageProofs: false });
  expect(supportsGatewayActivityReceiptProofs(client)).toBe(true);
  expect(supportsGatewayActivityStorageProofs(client)).toBe(false);
});

it('does not report receipt support when only the storage call is available', () => {
  const client = createArgonGatewayClient({ receiptProofs: false });
  expect(supportsGatewayActivityReceiptProofs(client)).toBe(false);
  expect(supportsGatewayActivityStorageProofs(client)).toBe(true);
});

it('builds default and explicit receipt proofs on the base spec-161 receipt runtime', async () => {
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
    runtimeSpecVersion: 161,
    storageProofs: false,
    consts: createGatewayProofConsts(),
    argonFinalizedExecutionHeaders: [{ blockHash: block.hash, blockNumber: 10n }],
  });
  // The deployed receipt-only runtime has neither of the storage-proof constants.
  Reflect.deleteProperty(argonClient.consts.crosschainTransfer, 'maxActivitiesPerGatewayProof');
  Reflect.deleteProperty(argonClient.consts.crosschainTransfer, 'maxGatewayActivityLogDataBytes');
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

  const bothCapabilitiesClient = createArgonGatewayClient({
    argonFinalizedExecutionHeaders: [{ blockHash: block.hash, blockNumber: 10n }],
  });
  expect(supportsGatewayActivityReceiptProofs(bothCapabilitiesClient)).toBe(true);
  expect(supportsGatewayActivityStorageProofs(bothCapabilitiesClient)).toBe(true);
  const explicitReceipt = await buildGatewayActivityReceiptProofPayloads(bothCapabilitiesClient, {
    gatewayAddress,
    executionClient,
    locators,
  });
  expect(explicitReceipt.payloads).toEqual(payloads);
});

it('proves all 25 finalization collateral rows through storage and receipt proofs', async () => {
  const gatewayAddress = repeatHex('70', 20);
  const mintingCollateral = Array.from({ length: 25 }, (_, index) => ({
    signingKey: getAddress(toHex(index + 1, { size: 20 })),
    microgonCollateral: BigInt(index + 1),
    micronotCollateral: BigInt(index + 2),
  }));
  const log = createTransferOutOfArgonFinalizedBlockLog({
    gatewayAddress,
    txHash: repeatHex('32', 32),
    transactionIndex: 0,
    logIndex: 0,
    nonce: 1n,
    amount: 675n,
    mintingCollateral,
  });
  expect(hexToBytes(log.data)).toHaveLength(2688);

  const { locators, proofsByBlockNumber, captureBlock } = await createGatewayProofHarness({
    gatewayAddress,
    locators: [{ blockNumber: 10n, logs: [log] }],
  });
  const receipt = createLegacyReceipt({
    txHash: log.transactionHash,
    transactionIndex: 0,
    logs: [{ ...log, topics: log.topics as [Hex, ...Hex[]] }],
  });
  const block = await createExecutionBlock({
    number: 10n,
    stateRoot: captureBlock.stateRoot,
    receipts: [receipt],
    blockLogs: [log],
  });
  const finalizedExecutionHeader = { blockHash: block.hash, blockNumber: 10n };
  const argonClient = createArgonGatewayClient({
    argonFinalizedExecutionHeaders: [finalizedExecutionHeader],
  });
  const executionClient = createExecutionClient({
    blocks: [block],
    receipts: [receipt],
    logsByBlockNumber: { '10': [log] },
    proofsByBlockNumber,
  });
  const options = { gatewayAddress, executionClient, locators, finalizedExecutionHeader };
  const storage = await captureAndBuildStorageProofs(argonClient, options);
  const receipts = await buildGatewayActivityReceiptProofPayloads(argonClient, options);

  for (const result of [storage, receipts]) {
    expect(result.deferredLocators).toEqual([]);
    expect(result.payloads).toHaveLength(1);
    expect(result.payloads[0].gatewayActivityNonceRange).toEqual({ start: 1n, end: 1n });
    expect(result.payloads[0].activities).toHaveLength(1);
    expect(result.payloads[0].activities[0]).toMatchObject({
      kind: 'TransferOutOfArgonFinalized',
      amount: 675n,
      mintingCollateral,
    });
  }

  const storageProof = storage.payloads[0].proof;
  const receiptBlock = receipts.payloads[0].proof.blocks[0];
  expect(storageProof.activityLogs).toHaveLength(1);
  expect(receiptBlock.receiptLogs).toHaveLength(1);
  for (const provedLog of [storageProof.activityLogs[0], receiptBlock.receiptLogs[0].eventLog]) {
    expect(hexToBytes(provedLog.data)).toHaveLength(2688);
    expect(decodeEthereumGatewayActivityLog(provedLog)).toMatchObject({ mintingCollateral });
    const activity = {
      ...log,
      txHash: log.transactionHash,
      ...decodeEthereumGatewayActivityLog(provedLog),
    };
    expect(
      appendEthereumGatewayActivityRoot(
        repeatHex('00', 32),
        hashEthereumGatewayActivity({ chainId: 1n, gatewayAddress }, activity),
      ),
    ).toBe(locators[0].activityRoot);
  }

  const receiptRef = receiptBlock.receiptProof.receipts[0];
  const recoveredReceipt = await verifyMPTWithMerkleProof(
    await createMPT(),
    hexToBytes(block.receiptsRoot),
    encodeReceiptTrieKey(receiptRef.transactionIndex),
    receiptRef.nodeIndexes.map(index => hexToBytes(receiptBlock.receiptProof.nodes[index])),
  );
  const encodedReceipt = encodeEthereumReceiptForProof(receipt);
  expect(bytesToHex(recoveredReceipt!)).toBe(bytesToHex(encodedReceipt));
  expect(encodedReceipt).toHaveLength(3018);
  const largestNodeBytes = Math.max(
    ...receiptBlock.receiptProof.nodes.map(node => hexToBytes(node).length),
  );
  expect(largestNodeBytes).toBe(3027);
  expect(largestNodeBytes).toBeGreaterThan(2048);
  expect(largestNodeBytes).toBeLessThanOrEqual(4096);
});

it('keeps ready receipt payloads when later cached locators need header backfill', async () => {
  const gatewayAddress = repeatHex('70', 20);
  const blocks: JSONRPCBlock[] = [];
  const receipts: EthereumReceipt[] = [];
  const locators: MissingGatewayActivityLocator[] = [];
  const logsByBlockNumber: Record<string, TestGatewayBlockLog[]> = {};
  let previousLocatorHash = repeatHex('00', 32);

  for (const [index, blockNumber] of [100n, 300n].entries()) {
    const log = createTransferToArgonStartedBlockLog({
      gatewayAddress,
      txHash: toHex(index + 1, { size: 32 }),
      transactionIndex: 0,
      logIndex: 0,
      nonce: BigInt(index + 1),
      argonAccountId: repeatHex('45', 32),
    });
    const receipt = createLegacyReceipt({
      txHash: log.transactionHash,
      transactionIndex: 0,
      logs: [{ ...log, topics: log.topics as [Hex, ...Hex[]] }],
    });
    blocks.push(
      await createExecutionBlock({ number: blockNumber, receipts: [receipt], blockLogs: [log] }),
    );
    receipts.push(receipt);
    logsByBlockNumber[blockNumber.toString()] = [log];

    const locator = createGatewayLocator(
      gatewayAddress,
      1n,
      blockNumber,
      [log],
      previousLocatorHash,
    );
    locators.push({ ...locator, locatorIndex: BigInt(index + 1) });
    previousLocatorHash = hashMintingGatewayActivityBlockLocator(locator);
  }

  const argonFinalizedExecutionHeaders = [
    { blockHash: blocks[0].hash, blockNumber: 100n },
    { blockHash: repeatHex('ff', 32), blockNumber: 1000n },
  ];
  const argonClient = createArgonGatewayClient({ argonFinalizedExecutionHeaders });
  const executionClient = createExecutionClient({ blocks, receipts, logsByBlockNumber });
  const options = { gatewayAddress, executionClient, locators };

  const result = await buildGatewayActivityReceiptProofPayloads(argonClient, options);
  expect(result.payloads).toHaveLength(1);
  expect(result.payloads[0].gatewayActivityNonceRange).toEqual({ start: 1n, end: 1n });
  expect(result.payloads[0].executionBlockNumberRange).toEqual({ start: 100n, end: 100n });
  expect(result.payloads[0].proof.executionBlockProof.anchorBlockHash).toBe(blocks[0].hash);
  expect(result.payloads[0].proof.blocks[0].receiptLogs).toHaveLength(1);
  expect(result.deferredLocators).toEqual([locators[1]]);

  const resumedClient = createArgonGatewayClient({
    argonFinalizedExecutionHeaders,
    previousGatewayActivityNonce: 1n,
  });
  await expect(buildGatewayActivityReceiptProofPayloads(resumedClient, options)).resolves.toEqual({
    payloads: [],
    deferredLocators: [locators[1]],
    recaptureLocators: [],
  });

  const rpcFailure = new Error('Execution RPC unavailable');
  vi.spyOn(executionClient, 'request').mockRejectedValueOnce(rpcFailure);
  await expect(buildGatewayActivityReceiptProofPayloads(resumedClient, options)).rejects.toBe(
    rpcFailure,
  );
});

it.each([
  { activitiesPerProof: 1, proofsPerTx: 10, ranges: [[1n, 3n]], blockCounts: [3] },
  {
    activitiesPerProof: 1,
    proofsPerTx: 2,
    ranges: [
      [1n, 2n],
      [3n, 3n],
    ],
    blockCounts: [2, 1],
  },
  {
    activitiesPerProof: 2,
    proofsPerTx: 1,
    ranges: [
      [1n, 2n],
      [3n, 3n],
    ],
    blockCounts: [1, 1],
  },
])(
  'chunks and resumes receipt proofs with $activitiesPerProof activities/proof and $proofsPerTx proofs/tx',
  async ({ activitiesPerProof, proofsPerTx, ranges, blockCounts }) => {
    const gatewayAddress = repeatHex('71', 20);
    const logs = [1n, 2n, 3n].map((nonce, logIndex) =>
      createTransferToArgonStartedBlockLog({
        gatewayAddress,
        txHash: repeatHex('31', 32),
        transactionIndex: 0,
        logIndex,
        nonce,
        argonAccountId: toHex(logIndex + 1, { size: 32 }),
      }),
    );
    const receipt = createLegacyReceipt({
      txHash: logs[0].transactionHash,
      transactionIndex: 0,
      logs: logs.map(log => ({ ...log, topics: log.topics as [Hex, ...Hex[]] })),
    });
    const block = await createExecutionBlock({ number: 10n, receipts: [receipt], blockLogs: logs });
    const locator = {
      ...createGatewayLocator(gatewayAddress, 1n, 10n, logs, repeatHex('00', 32)),
      locatorIndex: 1n,
    };
    const clientOptions = {
      storageProofs: false,
      consts: createGatewayProofConsts({
        maxActivitiesPerReceiptProof: activitiesPerProof,
        maxReceiptProofsPerExtrinsic: proofsPerTx,
      }),
      argonFinalizedExecutionHeaders: [{ blockHash: block.hash, blockNumber: 10n }],
    };
    const executionClient = createExecutionClient({
      blocks: [block],
      receipts: [receipt],
      logsByBlockNumber: { '10': logs },
    });
    const options = { gatewayAddress, executionClient, locators: [locator] };

    const { payloads, deferredLocators } = await buildGatewayActivityReceiptProofPayloads(
      createArgonGatewayClient(clientOptions),
      options,
    );
    expect(
      payloads.map(payload => [
        payload.gatewayActivityNonceRange.start,
        payload.gatewayActivityNonceRange.end,
      ]),
    ).toEqual(ranges);
    expect(payloads.map(payload => payload.proof.blocks.length)).toEqual(blockCounts);
    expect(payloads.map(payload => payload.previousGatewayActivityNonce)).toEqual(
      ranges.map(([start]) => start - 1n),
    );
    expect(
      payloads.flatMap(payload =>
        payload.activities.map(activity => activity.gatewayState.gatewayActivityNonce),
      ),
    ).toEqual([1n, 2n, 3n]);
    expect(deferredLocators).toEqual([]);

    const provedNonces: bigint[] = [];
    for (const payload of payloads) {
      expect(payload.proof.executionBlockProof.targetToAnchorHeaderChain).toEqual([]);
      expect(payload.proof.blocks.length).toBeLessThanOrEqual(proofsPerTx);
      for (const proofBlock of payload.proof.blocks) {
        expect(proofBlock.receiptLogs.length).toBeLessThanOrEqual(activitiesPerProof);
        provedNonces.push(
          ...proofBlock.receiptLogs.map(
            ({ eventLog }) =>
              decodeEthereumGatewayActivityLog(eventLog).gatewayState.gatewayActivityNonce,
          ),
        );
        const receiptRef = proofBlock.receiptProof.receipts[0];
        const recoveredReceipt = await verifyMPTWithMerkleProof(
          await createMPT(),
          hexToBytes(block.receiptsRoot),
          encodeReceiptTrieKey(receiptRef.transactionIndex),
          receiptRef.nodeIndexes.map(index => hexToBytes(proofBlock.receiptProof.nodes[index])),
        );
        expect(bytesToHex(recoveredReceipt!)).toBe(
          bytesToHex(encodeEthereumReceiptForProof(receipt)),
        );
      }
    }
    expect(provedNonces).toEqual([1n, 2n, 3n]);

    const resumed = await buildGatewayActivityReceiptProofPayloads(
      createArgonGatewayClient({
        ...clientOptions,
        previousGatewayActivityNonce: payloads[0].gatewayActivityNonceRange.end,
      }),
      options,
    );
    expect(resumed.payloads).toEqual(payloads.slice(1));
    expect(resumed.deferredLocators).toEqual([]);
  },
);

it('batches mixed receipt activities across blocks and defers only blocks beyond the finalized anchor', async () => {
  const gatewayAddress = repeatHex('72', 20);
  const blocks: JSONRPCBlock[] = [];
  const receipts: EthereumReceipt[] = [];
  const locators: MissingGatewayActivityLocator[] = [];
  const logsByBlockNumber: Record<string, TestGatewayBlockLog[]> = {};
  let previousLocatorHash = repeatHex('00', 32);
  for (const [index, blockNumber] of [10n, 11n, 12n].entries()) {
    const logArguments = {
      gatewayAddress,
      txHash: toHex(index + 1, { size: 32 }),
      transactionIndex: 0,
      logIndex: 0,
      nonce: BigInt(index + 1),
    };
    let log: TestGatewayBlockLog;
    if (index === 1) {
      log = createGlobalIssuanceCouncilRotatedBlockLog(logArguments);
    } else {
      log = createTransferToArgonStartedBlockLog({
        ...logArguments,
        argonAccountId: repeatHex('45', 32),
      });
    }
    const receipt = createLegacyReceipt({
      txHash: log.transactionHash,
      transactionIndex: 0,
      logs: [{ ...log, topics: log.topics as [Hex, ...Hex[]] }],
    });
    blocks.push(
      await createExecutionBlock({
        number: blockNumber,
        parentHash: blocks.at(-1)?.hash,
        receipts: [receipt],
        blockLogs: [log],
      }),
    );
    receipts.push(receipt);
    logsByBlockNumber[blockNumber.toString()] = [log];
    const locator = createGatewayLocator(
      gatewayAddress,
      1n,
      blockNumber,
      [log],
      previousLocatorHash,
    );
    locators.push({ ...locator, locatorIndex: BigInt(index + 1) });
    previousLocatorHash = hashMintingGatewayActivityBlockLocator(locator);
  }
  const executionClient = createExecutionClient({ blocks, receipts, logsByBlockNumber });
  const clientOptions = {
    storageProofs: false,
    consts: createGatewayProofConsts({ maxReceiptProofsPerExtrinsic: 2 }),
  };
  const options = { gatewayAddress, executionClient, locators };
  const allAnchors = blocks.map((block, index) => ({
    blockHash: block.hash,
    blockNumber: BigInt(index + 10),
  }));

  const ready = await buildGatewayActivityReceiptProofPayloads(
    createArgonGatewayClient({
      ...clientOptions,
      argonFinalizedExecutionHeaders: allAnchors.slice(0, 2),
    }),
    options,
  );
  expect(ready.payloads.map(payload => payload.gatewayActivityNonceRange)).toEqual([
    { start: 1n, end: 2n },
  ]);
  expect(ready.payloads[0].activities.map(activity => activity.kind)).toEqual([
    MintingGatewayEvents.TransferToArgonStarted.name,
    MintingGatewayEvents.GlobalIssuanceCouncilRotated.name,
  ]);
  expect(ready.deferredLocators).toEqual([locators[2]]);

  const complete = await buildGatewayActivityReceiptProofPayloads(
    createArgonGatewayClient({ ...clientOptions, argonFinalizedExecutionHeaders: allAnchors }),
    options,
  );
  expect(complete.payloads.map(payload => payload.gatewayActivityNonceRange)).toEqual([
    { start: 1n, end: 2n },
    { start: 3n, end: 3n },
  ]);
  expect(complete.payloads.map(payload => payload.proof.blocks.length)).toEqual([2, 1]);
  expect(complete.payloads[0].proof.executionBlockProof.targetToAnchorHeaderChain).toHaveLength(2);
  expect(complete.payloads[1].proof.executionBlockProof.targetToAnchorHeaderChain).toEqual([]);
  expect(complete.deferredLocators).toEqual([]);

  const resumed = await buildGatewayActivityReceiptProofPayloads(
    createArgonGatewayClient({
      ...clientOptions,
      previousGatewayActivityNonce: 2n,
      argonFinalizedExecutionHeaders: allAnchors,
    }),
    options,
  );
  expect(resumed.payloads).toEqual(complete.payloads.slice(1));
});

it('keeps a valid receipt prefix when later activity logs are on a different header chain', async () => {
  const gatewayAddress = repeatHex('73', 20);
  const logs = [1n, 2n].map((nonce, index) =>
    createTransferToArgonStartedBlockLog({
      gatewayAddress,
      txHash: toHex(index + 1, { size: 32 }),
      transactionIndex: 0,
      logIndex: 0,
      nonce,
      argonAccountId: repeatHex('45', 32),
    }),
  );
  const receipts = logs.map(log =>
    createLegacyReceipt({
      txHash: log.transactionHash,
      transactionIndex: 0,
      logs: [{ ...log, topics: log.topics as [Hex, ...Hex[]] }],
    }),
  );
  const firstBlock = await createExecutionBlock({
    number: 10n,
    receipts: [receipts[0]],
    blockLogs: [logs[0]],
  });
  const forkBlock = await createExecutionBlock({
    number: 11n,
    parentHash: firstBlock.hash,
    receipts: [receipts[1]],
    blockLogs: [logs[1]],
  });
  const canonicalBlock = await createExecutionBlock({ number: 11n, parentHash: firstBlock.hash });
  const anchor = await createExecutionBlock({ number: 12n, parentHash: canonicalBlock.hash });
  const firstLocator = {
    ...createGatewayLocator(gatewayAddress, 1n, 10n, [logs[0]], repeatHex('00', 32)),
    locatorIndex: 1n,
  };
  const forkLocator = {
    ...createGatewayLocator(
      gatewayAddress,
      1n,
      11n,
      [logs[1]],
      hashMintingGatewayActivityBlockLocator(firstLocator),
    ),
    locatorIndex: 2n,
  };
  const argonClient = createArgonGatewayClient({
    storageProofs: false,
    argonFinalizedExecutionHeaders: [{ blockHash: anchor.hash, blockNumber: 12n }],
  });
  const executionClient = createExecutionClient({
    blocks: [firstBlock, forkBlock, canonicalBlock, anchor],
    receipts,
    logsByBlockNumber: { '10': [logs[0]], '11': [logs[1]] },
  });
  const result = await buildGatewayActivityReceiptProofPayloads(argonClient, {
    gatewayAddress,
    executionClient,
    locators: [firstLocator, forkLocator],
  });

  expect(result.payloads.map(payload => payload.gatewayActivityNonceRange)).toEqual([
    { start: 1n, end: 1n },
  ]);
  expect(result.payloads[0].proof.blocks).toHaveLength(1);
  expect(result.payloads[0].proof.blocks[0].targetBlockNumber).toBe(10);
  expect(result.deferredLocators).toEqual([forkLocator]);

  const resumedClient = createArgonGatewayClient({
    storageProofs: false,
    previousGatewayActivityNonce: 1n,
    argonFinalizedExecutionHeaders: [{ blockHash: anchor.hash, blockNumber: 12n }],
  });
  await expect(
    buildGatewayActivityReceiptProofPayloads(resumedClient, {
      gatewayAddress,
      executionClient,
      locators: [forkLocator],
    }),
  ).rejects.toThrow('not on the shared execution header chain');
});

it.each([
  {
    maxActivitiesPerReceiptProof: 0,
    maxReceiptProofsPerExtrinsic: 10,
    invalidConstant: 'maxActivitiesPerReceiptProof',
  },
  {
    maxActivitiesPerReceiptProof: 16,
    maxReceiptProofsPerExtrinsic: 0,
    invalidConstant: 'maxReceiptProofsPerExtrinsic',
  },
])(
  'rejects invalid receipt bound $invalidConstant',
  async ({ maxActivitiesPerReceiptProof, maxReceiptProofsPerExtrinsic, invalidConstant }) => {
    const client = createArgonGatewayClient({
      storageProofs: false,
      consts: createGatewayProofConsts({
        maxActivitiesPerReceiptProof,
        maxReceiptProofsPerExtrinsic,
      }),
    });
    await expect(
      buildGatewayActivityReceiptProofPayloads(client, {
        gatewayAddress: repeatHex('70', 20),
        locators: [
          {
            locatorIndex: 1n,
            blockNumber: 10n,
            startGatewayActivityNonce: 1n,
            endGatewayActivityNonce: 1n,
            previousLocatorHash: repeatHex('00', 32),
            activityRoot: repeatHex('01', 32),
          },
        ],
      }),
    ).rejects.toThrow(`Gateway receipt proof requires ${invalidConstant} to be a positive integer`);
  },
);

it('requires captured locators when the default builder routes to storage proofs', async () => {
  const gatewayAddress = repeatHex('70', 20);
  const transferLog = createTransferToArgonStartedBlockLog({
    gatewayAddress,
    txHash: repeatHex('33', 32),
    transactionIndex: 0,
    logIndex: 0,
    nonce: 1n,
    argonAccountId: repeatHex('48', 32),
  });
  const { argonClient, executionClient, argonFinalizedExecutionHeader } =
    await createGatewayProofHarness({
      gatewayAddress,
      latestArgonFinalizedHeader: {
        blockHash: repeatHex('ab', 32),
        blockNumber: 12n,
      },
      locators: [{ blockNumber: 10n, logs: [transferLog] }],
    });
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
    'Gateway activity storage proofs require captured locators; call captureGatewayActivityStorageProofs first',
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
    await createGatewayProofHarness({
      gatewayAddress,
      locators: [{ blockNumber: 10n, logs: [transferLog] }],
      storageProofs: false,
    });

  await expect(
    captureAndBuildStorageProofs(argonClient, {
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
  const { argonClient, executionClient } = await createGatewayProofHarness({
    gatewayAddress,
    locators: [{ blockNumber: 10n, logs: [transferLog] }],
    receiptProofs: false,
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

it.each([true, false])(
  'prefers storage proofs when available (receipt support: %s)',
  async receiptProofs => {
    const gatewayAddress = repeatHex('73', 20);
    const log = createTransferToArgonStartedBlockLog({
      gatewayAddress,
      txHash: repeatHex('34', 32),
      transactionIndex: 0,
      logIndex: 0,
      nonce: 1n,
      argonAccountId: repeatHex('49', 32),
    });
    const { argonClient, executionClient, locators, argonFinalizedExecutionHeader } =
      await createGatewayProofHarness({
        gatewayAddress,
        receiptProofs,
        runtimeSpecVersion: 153,
        locators: [{ blockNumber: 10n, logs: [log] }],
      });

    const executionBlock = await executionClient.getBlock({
      blockNumber: argonFinalizedExecutionHeader.blockNumber,
    });
    const captures = await captureGatewayActivityStorageProofs({
      executionClient,
      gatewayAddress,
      executionBlock,
      locators,
    });
    const result = await buildGatewayActivityProof(argonClient, {
      executionClient,
      gatewayAddress,
      locators: captures,
    });
    expect(result.payloads).toHaveLength(1);
    expect(result.payloads[0].proof).toMatchObject({
      locatorIndex: 1n,
      activityLogs: [{ address: log.address, topics: log.topics, data: log.data }],
    });
  },
);

it('rejects the default builder when neither gateway proof call is available', async () => {
  const client = createArgonGatewayClient({ receiptProofs: false, storageProofs: false });
  await expect(
    buildGatewayActivityProof(client, {
      gatewayAddress: repeatHex('73', 20),
      executionClient: createExecutionClient({ blocks: [] }),
      locators: [],
    }),
  ).rejects.toThrow('Gateway activity proofs are not supported by this runtime');
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

it.each([v4Purchase, v4Refund])(
  'builds the runtime-verified proof for a native purchase receipt',
  async fixture => {
    const [status, cumulativeGasUsed, logsBloom, logs] = fromRlp(
      sliceHex(fixture.encodedReceipt as Hex, 1),
    ) as [Hex, Hex, Hex, [Hex, Hex[], Hex][]];
    expect(status).toBe('0x01');
    expect(sliceHex(fixture.encodedReceipt as Hex, 0, 1)).toBe('0x02');
    const txHash: Hex = `0x${'aa'.repeat(32)}`;
    const zeroHash: Hex = `0x${'00'.repeat(32)}`;
    const header = {
      number: toHex(10n),
      hash: zeroHash,
      parentHash: zeroHash,
      nonce: '0x0000000000000000',
      sha3Uncles: '0x1dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347',
      logsBloom,
      transactionsRoot: zeroHash,
      stateRoot: zeroHash,
      receiptsRoot: fixture.receiptsRoot as Hex,
      miner: `0x${'00'.repeat(20)}`,
      difficulty: '0x0',
      extraData: '0x',
      size: '0x1',
      gasLimit: toHex(30_000_000n),
      gasUsed: cumulativeGasUsed,
      timestamp: toHex(1n),
      transactions: [txHash],
      uncles: [],
    } satisfies JSONRPCBlock;
    header.hash = bytesToHex(createBlockHeaderFromRPC(header).hash());
    const receipt = {
      type: 'eip1559',
      status: 'success',
      cumulativeGasUsed: BigInt(cumulativeGasUsed),
      logsBloom,
      logs: logs.map(([address, topics, data], logIndex) => ({ address, topics, data, logIndex })),
      transactionHash: txHash,
      transactionIndex: fixture.transactionIndex,
      blockHash: header.hash,
    } as EthereumReceipt;
    expect(bytesToHex(encodeEthereumReceiptForProof(receipt))).toBe(fixture.encodedReceipt);
    const logIndex = receipt.logs.findIndex(log => log.topics[0] === fixture.eventLog.topics[0]);
    expect(logIndex).toBeGreaterThanOrEqual(0);
    const executionClient = {
      getTransactionReceipt: async () => receipt,
      getBlock: async () => ({ transactions: [txHash] }),
      request: async () => header,
    } as unknown as EthereumExecutionClient;
    const proof = await buildEthereumEventProof(
      { executionClient },
      { blockHash: header.hash, blockNumber: 10n },
      [[{ txHash, receipt, logIndexes: [logIndex] }]],
    );
    expect(proof.blocks[0].receiptProof.nodes).toEqual(fixture.nodes);
    expect(proof.blocks[0].receiptLogs[0].eventLog.data).toBe(fixture.eventLog.data);
    expect(proof.blocks[0].receiptProof.nodes.every(node => hexToBytes(node).length <= 4096)).toBe(
      true,
    );
  },
);

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
  const { argonClient, executionClient, locators, argonFinalizedExecutionHeader } =
    await createGatewayProofHarness({
      gatewayAddress,
      latestArgonFinalizedHeader,
      locators: [{ blockNumber: 10n, logs: [transferLog, councilRotationLog] }],
    });

  const { payloads } = await captureAndBuildStorageProofs(argonClient, {
    executionClient,
    finalizedExecutionHeader: argonFinalizedExecutionHeader,
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
  expect(payload?.proof.activityRootSeed).toBe(repeatHex('00', 32));
  expect(payload?.proof.activityLogs).toHaveLength(2);
  expect(payload?.proof.storageProof.executionBlockProof).toEqual({
    anchorBlockHash: argonFinalizedExecutionHeader.blockHash,
    targetToAnchorHeaderChain: [],
  });
  expect(payload?.proof.storageProof.slots).toHaveLength(2);
  const witness = payload.proof.storageProof;
  expect(new Set(witness.storageProof).size).toBe(witness.storageProof.length);
  expect(witness.slots[0].nodeIndexes[0]).toBe(witness.slots[1].nodeIndexes[0]);
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
  const { executionClient, argonFinalizedExecutionHeader } = await createGatewayProofHarness({
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
    await createGatewayProofHarness({
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

  const { payloads } = await captureAndBuildStorageProofs(argonClient, {
    executionClient,
    finalizedExecutionHeader: argonFinalizedExecutionHeader,
    gatewayAddress,
    locators: [locators[1]],
  });
  const payload = payloads[0];

  expect(payload?.previousGatewayActivityNonce).toBe(1n);
  expect(payload?.proof.locatorIndex).toBe(2n);
  expect(payload?.proof.activityRootSeed).toBe(hashMintingGatewayActivityBlockLocator(locators[0]));
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
  } = await createGatewayProofHarness({
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
    captureAndBuildStorageProofs(argonClient, {
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
    await createGatewayProofHarness({
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
  const { payloads } = await captureAndBuildStorageProofs(argonClient, {
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
    await createGatewayProofHarness({
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
  const { executionClient } = await createGatewayProofHarness({
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
  const { executionClient, locators, argonFinalizedExecutionHeader } =
    await createGatewayProofHarness({
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
  const { argonClient, executionClient, argonFinalizedExecutionHeader } =
    await createGatewayProofHarness({
      gatewayAddress,
      latestArgonFinalizedHeader: {
        blockHash: repeatHex('f9', 32),
        blockNumber: 12n,
      },
      locators: [
        { blockNumber: 10n, logs: [log10] },
        { blockNumber: 11n, logs: [log11] },
      ],
    });

  const locators = await discoverMissingGatewayActivityLocators({
    executionClient,
    finalizedExecutionBlockNumber: argonFinalizedExecutionHeader.blockNumber,
    gatewayAddress,
    minimumGatewayActivityNonce: 1n,
  });
  const { payloads } = await captureAndBuildStorageProofs(argonClient, {
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
  const { argonClient, executionClient, argonFinalizedExecutionHeader } =
    await createGatewayProofHarness({
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
    });

  const discovered = await discoverMissingGatewayActivityLocators({
    executionClient,
    finalizedExecutionBlockNumber: argonFinalizedExecutionHeader.blockNumber,
    gatewayAddress,
    minimumGatewayActivityNonce: 2n,
  });
  const nextLocator = discovered[0];
  const { payloads } = await captureAndBuildStorageProofs(argonClient, {
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

it('resumes inside a locator with the full witness and only the unconsumed payload suffix', async () => {
  const gatewayAddress = repeatHex('79', 20);
  const previousLog = createTransferToArgonStartedBlockLog({
    gatewayAddress,
    txHash: repeatHex('12', 32),
    transactionIndex: 0,
    logIndex: 0,
    nonce: 1n,
    argonAccountId: repeatHex('45', 32),
  });
  const logs = [2n, 3n].map((nonce, index) =>
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
    await createGatewayProofHarness({
      gatewayAddress,
      previousGatewayActivityNonce: 2n,
      locators: [
        { blockNumber: 9n, logs: [previousLog] },
        { blockNumber: 10n, logs },
      ],
    });

  const { payloads, deferredLocators } = await captureAndBuildStorageProofs(argonClient, {
    executionClient,
    finalizedExecutionHeader: argonFinalizedExecutionHeader,
    gatewayAddress,
    locators,
  });
  expect(deferredLocators).toEqual([]);
  expect(payloads).toHaveLength(1);
  expect(payloads[0].previousGatewayActivityNonce).toBe(2n);
  expect(payloads[0].gatewayActivityNonceRange).toEqual({ start: 3n, end: 3n });
  expect(
    payloads[0].activities.map(activity => activity.gatewayState.gatewayActivityNonce),
  ).toEqual([3n]);
  expect(payloads[0].proof).toMatchObject({
    locatorIndex: 2n,
    activityRootSeed: hashMintingGatewayActivityBlockLocator(locators[0]),
  });
  expect(payloads[0].proof.activityLogs).toEqual(
    logs.map(({ address, topics, data }) => ({ address, topics, data })),
  );

  // The consumed prefix is still part of the authenticated root.
  logs[0].data = `0x${logs[0].data.slice(2, -1)}1`;
  await expect(
    captureAndBuildStorageProofs(argonClient, {
      executionClient,
      finalizedExecutionHeader: argonFinalizedExecutionHeader,
      gatewayAddress,
      locators,
    }),
  ).rejects.toThrow('activity root did not match storage');
});

it('defers missing nonce gaps, skips replayed locators, and resumes when the gap is supplied', async () => {
  const gatewayAddress = repeatHex('79', 20);
  const locatorLogs = [1n, 2n, 3n].map(nonce => ({
    blockNumber: 9n + nonce,
    logs: [
      createTransferToArgonStartedBlockLog({
        gatewayAddress,
        txHash: toHex(nonce, { size: 32 }),
        transactionIndex: 0,
        logIndex: 0,
        nonce,
        argonAccountId: toHex(nonce, { size: 32 }),
      }),
    ],
  }));
  const { argonClient, executionClient, locators, argonFinalizedExecutionHeader } =
    await createGatewayProofHarness({
      gatewayAddress,
      locators: locatorLogs,
      previousGatewayActivityNonce: 1n,
    });
  const options = {
    executionClient,
    gatewayAddress,
    finalizedExecutionHeader: argonFinalizedExecutionHeader,
  };

  await expect(
    captureAndBuildStorageProofs(argonClient, {
      ...options,
      locators: [locators[2], locators[0]],
    }),
  ).resolves.toMatchObject({
    payloads: [],
    deferredLocators: [locators[2]],
    recaptureLocators: [],
  });

  const resumed = await captureAndBuildStorageProofs(argonClient, {
    ...options,
    locators: [locators[2], locators[1], locators[0], locators[1]],
  });
  expect(resumed.deferredLocators).toEqual([]);
  expect(resumed.payloads.map(payload => payload.previousGatewayActivityNonce)).toEqual([1n, 2n]);
  expect(
    resumed.payloads.flatMap(payload =>
      payload.activities.map(activity => activity.gatewayState.gatewayActivityNonce),
    ),
  ).toEqual([2n, 3n]);

  const replayClient = createArgonGatewayClient({ previousGatewayActivityNonce: 3n });
  await expect(
    captureAndBuildStorageProofs(replayClient, { ...options, locators }),
  ).resolves.toEqual({ payloads: [], deferredLocators: [], recaptureLocators: [] });
});

it('rejects missing activity inside a locator even if the missing nonce was already consumed', async () => {
  const gatewayAddress = repeatHex('79', 20);
  const logs = [1n, 3n].map((nonce, logIndex) =>
    createTransferToArgonStartedBlockLog({
      gatewayAddress,
      txHash: repeatHex('13', 32),
      transactionIndex: 0,
      logIndex,
      nonce,
      argonAccountId: toHex(nonce, { size: 32 }),
    }),
  );
  const { argonClient, executionClient, locators, argonFinalizedExecutionHeader } =
    await createGatewayProofHarness({
      gatewayAddress,
      previousGatewayActivityNonce: 2n,
      locators: [{ blockNumber: 10n, logs }],
    });

  await expect(
    captureAndBuildStorageProofs(argonClient, {
      executionClient,
      finalizedExecutionHeader: argonFinalizedExecutionHeader,
      gatewayAddress,
      locators,
    }),
  ).rejects.toThrow('gap in gateway activity coverage');
});

it('rejects a locator that exceeds maxActivitiesPerGatewayProof without splitting it', async () => {
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
    await createGatewayProofHarness({
      gatewayAddress,
      consts: createGatewayProofConsts({ maxActivitiesPerGatewayProof: 2 }),
      previousGatewayActivityNonce: 2n,
      locators: [{ blockNumber: 10n, logs }],
    });

  await expect(
    captureAndBuildStorageProofs(argonClient, {
      executionClient,
      finalizedExecutionHeader: argonFinalizedExecutionHeader,
      gatewayAddress,
      locators: [locators[0]],
    }),
  ).rejects.toThrow('exceeds maxActivitiesPerGatewayProof');
});

it.each([0, 1])(
  'checks the runtime aggregate data budget including consumed logs (excess bytes: %s)',
  async excessBytes => {
    const gatewayAddress = repeatHex('7a', 20);
    const logs = [1n, 2n].map((nonce, logIndex) =>
      createTransferOutOfArgonFinalizedBlockLog({
        gatewayAddress,
        txHash: repeatHex('14', 32),
        transactionIndex: 0,
        logIndex,
        nonce,
        amount: 25n,
        mintingCollateral: Array.from({ length: 25 }, (_, index) => ({
          signingKey: getAddress(toHex(index + 1, { size: 20 })),
          microgonCollateral: 1n,
          micronotCollateral: 0n,
        })),
      }),
    );
    const dataBytes = logs.reduce((total, log) => total + hexToBytes(log.data).length, 0);
    const { argonClient, executionClient, locators, argonFinalizedExecutionHeader } =
      await createGatewayProofHarness({
        gatewayAddress,
        consts: createGatewayProofConsts({
          maxGatewayActivityLogDataBytes: dataBytes - excessBytes,
        }),
        previousGatewayActivityNonce: 1n,
        locators: [{ blockNumber: 10n, logs }],
      });
    const result = captureAndBuildStorageProofs(argonClient, {
      executionClient,
      finalizedExecutionHeader: argonFinalizedExecutionHeader,
      gatewayAddress,
      locators,
    });

    if (excessBytes > 0) {
      await expect(result).rejects.toThrow('exceeds maxGatewayActivityLogDataBytes');
    } else {
      const { payloads, deferredLocators } = await result;
      expect(deferredLocators).toEqual([]);
      expect(payloads).toHaveLength(1);
      expect(payloads[0].proof.activityLogs).toHaveLength(2);
      expect(
        payloads[0].activities.map(activity => activity.gatewayState.gatewayActivityNonce),
      ).toEqual([2n]);
    }
  },
);

it('rejects more than 3 MiB of activity data even below the activity-count cap', async () => {
  const gatewayAddress = repeatHex('7a', 20);
  const mintingCollateral = Array.from({ length: 25 }, (_, index) => ({
    signingKey: getAddress(toHex(index + 1, { size: 20 })),
    microgonCollateral: 1n,
    micronotCollateral: 0n,
  }));
  const logs = Array.from({ length: 1171 }, (_, index) =>
    createTransferOutOfArgonFinalizedBlockLog({
      gatewayAddress,
      txHash: repeatHex('14', 32),
      transactionIndex: 0,
      logIndex: index,
      nonce: BigInt(index + 1),
      amount: 25n,
      mintingCollateral,
    }),
  );
  expect(logs.length).toBeLessThan(
    gatewayProofConsts.crosschainTransfer.maxActivitiesPerGatewayProof.toNumber(),
  );
  expect(logs.reduce((total, log) => total + hexToBytes(log.data).length, 0)).toBeGreaterThan(
    3 * 1024 * 1024,
  );
  const { argonClient, executionClient, locators, argonFinalizedExecutionHeader } =
    await createGatewayProofHarness({ gatewayAddress, locators: [{ blockNumber: 10n, logs }] });

  await expect(
    captureAndBuildStorageProofs(argonClient, {
      executionClient,
      finalizedExecutionHeader: argonFinalizedExecutionHeader,
      gatewayAddress,
      locators,
    }),
  ).rejects.toThrow('exceeds maxGatewayActivityLogDataBytes');
});

it.each(['activities', 'bytes'])(
  'reports an oversized locator as an error, not deferred work (limit: %s)',
  async limit => {
    const gatewayAddress = repeatHex('7a', 20);
    const smallLocators = [1n, 4n].map(nonce => ({
      blockNumber: 9n + nonce,
      logs: [
        createTransferToArgonStartedBlockLog({
          gatewayAddress,
          txHash: toHex(nonce, { size: 32 }),
          transactionIndex: 0,
          logIndex: 0,
          nonce,
          argonAccountId: toHex(nonce, { size: 32 }),
        }),
      ],
    }));
    const largeLogs = [2n, 3n].map((nonce, logIndex) =>
      createTransferOutOfArgonFinalizedBlockLog({
        gatewayAddress,
        txHash: repeatHex('14', 32),
        transactionIndex: 0,
        logIndex,
        nonce,
        amount: 25n,
        mintingCollateral: Array.from({ length: 25 }, (_, index) => ({
          signingKey: getAddress(toHex(index + 1, { size: 20 })),
          microgonCollateral: 1n,
          micronotCollateral: 0n,
        })),
      }),
    );
    const consts = createGatewayProofConsts({
      maxActivitiesPerGatewayProof: limit === 'activities' ? 1 : 4096,
      maxGatewayActivityLogDataBytes: hexToBytes(largeLogs[0].data).length,
    });
    const { argonClient, executionClient, locators, argonFinalizedExecutionHeader } =
      await createGatewayProofHarness({
        gatewayAddress,
        consts,
        locators: [smallLocators[0], { blockNumber: 11n, logs: largeLogs }, smallLocators[1]],
      });
    const captures = await captureGatewayActivityStorageProofs({
      executionClient,
      executionBlock: await executionClient.getBlock({
        blockNumber: argonFinalizedExecutionHeader.blockNumber,
      }),
      gatewayAddress,
      locators,
    });
    const options = {
      executionClient,
      gatewayAddress,
      locators: captures,
    };

    let expectedError = 'exceeds maxGatewayActivityLogDataBytes';
    if (limit === 'activities') {
      expectedError = 'exceeds maxActivitiesPerGatewayProof';
    }
    await expect(buildGatewayActivityProof(argonClient, options)).rejects.toThrow(expectedError);

    // Advancing the cursor cannot make a permanently oversized locator fit.
    const updatedClient = createArgonGatewayClient({ previousGatewayActivityNonce: 1n, consts });
    await expect(
      buildGatewayActivityProof(updatedClient, { ...options, locators: captures.slice(1) }),
    ).rejects.toThrow(expectedError);
  },
);

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
    await createGatewayProofHarness({
      gatewayAddress,
      latestArgonFinalizedHeader: {
        blockHash: repeatHex('fc', 32),
        blockNumber: 9n,
      },
      locators: [{ blockNumber: 10n, logs: [log] }],
    });

  await expect(
    captureAndBuildStorageProofs(argonClient, {
      executionClient,
      finalizedExecutionHeader: argonFinalizedExecutionHeader,
      gatewayAddress,
      locators: [locators[0]],
    }),
  ).resolves.toMatchObject({
    payloads: [],
    deferredLocators: [locators[0]],
    recaptureLocators: [],
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
    await createGatewayProofHarness({
      gatewayAddress,
      consts: createGatewayProofConsts({ maxActivitiesPerGatewayProof: 0 }),
      locators: [{ blockNumber: 10n, logs: [log] }],
    });

  await expect(() =>
    captureAndBuildStorageProofs(argonClient, {
      executionClient,
      finalizedExecutionHeader: argonFinalizedExecutionHeader,
      gatewayAddress,
      locators: [locators[0]],
    }),
  ).rejects.toThrow('Gateway proof requires maxActivitiesPerGatewayProof to be a positive integer');
});

it('captures before anchoring and relays cached witnesses with one shared shortest ancestry path', async () => {
  const gatewayAddress = repeatHex('70', 20);
  const logs = [1n, 2n, 3n].map(nonce =>
    createTransferToArgonStartedBlockLog({
      gatewayAddress,
      txHash: toHex(nonce === 3n ? 2n : nonce, { size: 32 }),
      transactionIndex: 0,
      logIndex: nonce === 3n ? 1 : 0,
      nonce,
      argonAccountId: toHex(nonce, { size: 32 }),
    }),
  );
  const { executionClient, captureBlock, locators } = await createGatewayProofHarness({
    gatewayAddress,
    locators: [
      { blockNumber: 9n, logs: [logs[0]] },
      { blockNumber: 10n, logs: logs.slice(1) },
    ],
  });
  const captureOrder: string[] = [];
  const getProof = executionClient.getProof;
  const proofRequests = vi.spyOn(executionClient, 'getProof').mockImplementation(async options => {
    captureOrder.push('proof');
    return getProof(options);
  });
  const getLogs = executionClient.getLogs;
  vi.spyOn(executionClient, 'getLogs').mockImplementation(async options => {
    captureOrder.push('logs');
    return getLogs(options);
  });
  const captures = await captureGatewayActivityStorageProofs({
    executionClient,
    executionBlock: await executionClient.getBlock({ blockNumber: 10n }),
    gatewayAddress,
    locators,
  });
  expect(captureOrder).toEqual(['proof', 'logs', 'logs']);
  expect(proofRequests).toHaveBeenCalledTimes(1);
  expect(proofRequests.mock.calls[0][0].storageKeys).toHaveLength(4);
  const cached = structuredClone(captures);

  const child = await createExecutionBlock({ number: 11n, parentHash: captureBlock.hash });
  const anchor = await createExecutionBlock({ number: 12n, parentHash: child.hash });
  // The captured header is cached; relay only needs its descendants from the RPC.
  const relayRpc = createExecutionClient({ blocks: [child, anchor] });
  const requests = vi.spyOn(relayRpc, 'request');
  vi.spyOn(relayRpc, 'getProof').mockRejectedValue(new Error('historical state is pruned'));
  vi.spyOn(relayRpc, 'getLogs').mockRejectedValue(new Error('relay must use cached logs'));
  vi.spyOn(relayRpc, 'readContract').mockRejectedValue(new Error('relay must use cached locators'));
  const waiting = await buildGatewayActivityStorageProofs(createArgonGatewayClient(), {
    executionClient: relayRpc,
    gatewayAddress,
    locators: cached,
  });
  expect(waiting.payloads).toEqual([]);
  expect(waiting.deferredLocators).toEqual(cached);
  expect(waiting.recaptureLocators).toEqual([]);
  expect(requests).not.toHaveBeenCalled();
  const defaultWaiting = await buildGatewayActivityProof(createArgonGatewayClient(), {
    executionClient: relayRpc,
    gatewayAddress,
    locators: waiting.deferredLocators,
  });
  expect(defaultWaiting.deferredLocators).toEqual(cached);

  const readyClient = createArgonGatewayClient({
    argonFinalizedExecutionHeaders: [
      { blockHash: anchor.hash, blockNumber: 12n },
      { blockHash: repeatHex('ff', 32), blockNumber: 100n },
    ],
  });
  const result = await buildGatewayActivityStorageProofs(readyClient, {
    executionClient: relayRpc,
    gatewayAddress,
    locators: defaultWaiting.deferredLocators,
  });
  expect(result.deferredLocators).toEqual([]);
  expect(result.recaptureLocators).toEqual([]);
  expect(result.payloads.map(payload => payload.previousGatewayActivityNonce)).toEqual([0n, 1n]);
  for (const payload of result.payloads) {
    expect(payload.proof).toMatchObject({
      storageProof: {
        accountProof: captures[0].proof.storageProof.accountProof,
        executionBlockProof: {
          anchorBlockHash: anchor.hash,
          targetToAnchorHeaderChain: [captureBlock, child].map(block => ({
            rlp: bytesToHex(createBlockHeaderFromRPC(block).serialize()),
          })),
        },
      },
    });
  }
  expect(requests).toHaveBeenCalledTimes(2);
  expect(requests).toHaveBeenNthCalledWith(1, {
    method: 'eth_getBlockByHash',
    params: [anchor.hash, false],
  });
  expect(requests).toHaveBeenNthCalledWith(2, {
    method: 'eth_getBlockByHash',
    params: [child.hash, false],
  });
  expect(relayRpc.getProof).not.toHaveBeenCalled();
  expect(relayRpc.getLogs).not.toHaveBeenCalled();
  expect(relayRpc.readContract).not.toHaveBeenCalled();
  expect(cached).toEqual(captures);

  // Receipt syncing may consume part of a locator while the app is holding its capture.
  const advancedClient = createArgonGatewayClient({
    previousGatewayActivityNonce: 2n,
    argonFinalizedExecutionHeaders: [{ blockHash: anchor.hash, blockNumber: 12n }],
  });
  const resumed = await buildGatewayActivityProof(advancedClient, {
    executionClient: relayRpc,
    gatewayAddress,
    locators: [...cached].reverse(),
  });
  expect(resumed.payloads).toHaveLength(1);
  expect(resumed.payloads[0].previousGatewayActivityNonce).toBe(2n);
  expect(
    resumed.payloads[0].activities.map(activity => activity.gatewayState.gatewayActivityNonce),
  ).toEqual([3n]);
  expect(resumed.payloads[0].proof).toMatchObject({ activityLogs: cached[1].proof.activityLogs });
  expect(relayRpc.getProof).not.toHaveBeenCalled();
});

it.each([0, 64, 65])(
  'bounds captured ancestry before fetching headers (distance: %s)',
  async distance => {
    const gatewayAddress = repeatHex('70', 20);
    const log = createTransferToArgonStartedBlockLog({
      gatewayAddress,
      txHash: repeatHex('11', 32),
      transactionIndex: 0,
      logIndex: 0,
      nonce: 1n,
      argonAccountId: repeatHex('22', 32),
    });
    const { executionClient, captureBlock, locators } = await createGatewayProofHarness({
      gatewayAddress,
      locators: [{ blockNumber: 10n, logs: [log] }],
    });
    const captures = await captureGatewayActivityStorageProofs({
      executionClient,
      executionBlock: await executionClient.getBlock({ blockNumber: 10n }),
      gatewayAddress,
      locators,
    });
    const blocks = [captureBlock];
    for (let index = 1; index <= distance; index++) {
      blocks.push(
        await createExecutionBlock({
          number: 10n + BigInt(index),
          parentHash: blocks.at(-1)!.hash,
        }),
      );
    }
    const anchor = blocks.at(-1)!;
    const client = createArgonGatewayClient({
      argonFinalizedExecutionHeaders: [
        { blockHash: anchor.hash, blockNumber: 10n + BigInt(distance) },
      ],
    });
    const relayRpc = createExecutionClient({ blocks });
    const requests = vi.spyOn(relayRpc, 'request');
    const result = await buildGatewayActivityStorageProofs(client, {
      executionClient: relayRpc,
      gatewayAddress,
      locators: captures,
    });
    expect(result.deferredLocators).toEqual([]);
    if (distance > 64) {
      expect(result.payloads).toEqual([]);
      expect(result.recaptureLocators).toEqual(captures);
      expect(requests).not.toHaveBeenCalled();
    } else {
      expect(result.recaptureLocators).toEqual([]);
      expect(
        result.payloads[0].proof.storageProof.executionBlockProof.targetToAnchorHeaderChain,
      ).toHaveLength(distance);
      if (distance === 0) expect(requests).not.toHaveBeenCalled();
    }
  },
);

it('reports an orphaned capture for recapture rather than deferring it forever', async () => {
  const gatewayAddress = repeatHex('70', 20);
  const log = createTransferToArgonStartedBlockLog({
    gatewayAddress,
    txHash: repeatHex('11', 32),
    transactionIndex: 0,
    logIndex: 0,
    nonce: 1n,
    argonAccountId: repeatHex('22', 32),
  });
  const { executionClient, captureBlock, locators } = await createGatewayProofHarness({
    gatewayAddress,
    locators: [{ blockNumber: 10n, logs: [log] }],
  });
  const captures = await captureGatewayActivityStorageProofs({
    executionClient,
    executionBlock: await executionClient.getBlock({ blockNumber: 10n }),
    gatewayAddress,
    locators,
  });
  const canonical = await createExecutionBlock({
    number: 10n,
    timestamp: 999n,
    stateRoot: captureBlock.stateRoot,
  });
  const anchor = await createExecutionBlock({ number: 11n, parentHash: canonical.hash });
  const client = createArgonGatewayClient({
    argonFinalizedExecutionHeaders: [{ blockHash: anchor.hash, blockNumber: 11n }],
  });
  const result = await buildGatewayActivityStorageProofs(client, {
    executionClient: createExecutionClient({ blocks: [canonical, anchor] }),
    gatewayAddress,
    locators: captures,
  });
  expect(result).toEqual({ payloads: [], deferredLocators: [], recaptureLocators: captures });
});

it.each(['account', 'slot', 'block'])(
  'rejects inconsistent capture witnesses before caching (%s)',
  async mismatch => {
    const gatewayAddress = repeatHex('70', 20);
    const log = createTransferToArgonStartedBlockLog({
      gatewayAddress,
      txHash: repeatHex('11', 32),
      transactionIndex: 0,
      logIndex: 0,
      nonce: 1n,
      argonAccountId: repeatHex('22', 32),
    });
    const { executionClient, captureBlock, locators, proofsByBlockNumber } =
      await createGatewayProofHarness({
        gatewayAddress,
        locators: [{ blockNumber: 10n, logs: [log] }],
      });
    let rpc = executionClient;
    let block = captureBlock;
    if (mismatch === 'account') proofsByBlockNumber['10'].accountProof[0] = '0xc0';
    if (mismatch === 'slot') proofsByBlockNumber['10'].storageProof[0].proof[0] = '0xc0';
    if (mismatch === 'block') {
      block = await createExecutionBlock({ number: 10n, stateRoot: repeatHex('33', 32) });
      rpc = createExecutionClient({ blocks: [block], proofsByBlockNumber });
    }
    const getLogs = vi.spyOn(rpc, 'getLogs');
    await expect(
      captureGatewayActivityStorageProofs({
        executionClient: rpc,
        executionBlock: await rpc.getBlock({ blockHash: block.hash }),
        gatewayAddress,
        locators,
      }),
    ).rejects.toThrow();
    expect(getLogs).not.toHaveBeenCalled();
  },
);

it.each(['padded', 'compact'])('accepts %s RPC slot keys with leading zeroes', async encoding => {
  const gatewayAddress = repeatHex('70', 20);
  const { argonClient, executionClient, locators, argonFinalizedExecutionHeader } =
    await createGatewayProofHarness({
      gatewayAddress,
      previousGatewayActivityNonce: 5n,
      locators: Array.from({ length: 6 }, (_, index) => ({
        blockNumber: 10n + BigInt(index),
        logs: [
          createTransferToArgonStartedBlockLog({
            gatewayAddress,
            txHash: toHex(index + 1, { size: 32 }),
            transactionIndex: 0,
            logIndex: 0,
            nonce: BigInt(index + 1),
            argonAccountId: repeatHex('22', 32),
          }),
        ],
      })),
    });
  const getProof = executionClient.getProof;
  vi.spyOn(executionClient, 'getProof').mockImplementation(async options => {
    const witness = await getProof(options);
    expect(witness.storageProof[0].key).toMatch(/^0x0/);
    return {
      ...witness,
      storageProof: witness.storageProof.map(slot => ({
        ...slot,
        key: encoding === 'compact' ? toHex(BigInt(slot.key)) : slot.key,
      })),
    };
  });

  const { payloads, deferredLocators, recaptureLocators } = await captureAndBuildStorageProofs(
    argonClient,
    {
      executionClient,
      gatewayAddress,
      finalizedExecutionHeader: argonFinalizedExecutionHeader,
      locators: [locators[5]],
    },
  );
  expect(deferredLocators).toEqual([]);
  expect(recaptureLocators).toEqual([]);
  expect(payloads).toHaveLength(1);
  expect(payloads[0].gatewayActivityNonceRange).toEqual({ start: 6n, end: 6n });
  expect(payloads[0].proof.storageProof.slots).toHaveLength(2);
  expect(payloads[0].proof.storageProof.slots[1].value).toBe(locators[5].activityRoot);
});

it('caches only ordered proof paths, excluding duplicate and unrelated RPC nodes', async () => {
  const gatewayAddress = repeatHex('70', 20);
  const log = createTransferToArgonStartedBlockLog({
    gatewayAddress,
    txHash: repeatHex('11', 32),
    transactionIndex: 0,
    logIndex: 0,
    nonce: 1n,
    argonAccountId: repeatHex('22', 32),
  });
  const { executionClient, locators, proofsByBlockNumber } = await createGatewayProofHarness({
    gatewayAddress,
    locators: [{ blockNumber: 10n, logs: [log] }],
  });
  const witness = proofsByBlockNumber['10'];
  const accountNodes = [...witness.accountProof];
  const storagePaths = witness.storageProof.map(slot => [...slot.proof]);
  witness.storageProof.reverse();
  witness.storageProof.push({ ...witness.storageProof[0], value: 0n });
  witness.accountProof.push(...witness.accountProof, '0xc0');
  for (const slot of witness.storageProof) slot.proof.push(...slot.proof, '0xc0');
  // Bypass the fixture adapter's slot ordering and duplicate filtering.
  const getProof = executionClient.getProof;
  vi.spyOn(executionClient, 'getProof').mockImplementation(async options => ({
    ...(await getProof(options)),
    storageProof: witness.storageProof,
  }));
  const [capture] = await captureGatewayActivityStorageProofs({
    executionClient,
    executionBlock: await executionClient.getBlock({ blockNumber: 10n }),
    gatewayAddress,
    locators,
  });
  expect(capture.proof.storageProof.accountProof).toEqual(accountNodes);
  for (const [index, slot] of capture.proof.storageProof.slots.entries()) {
    expect(slot.nodeIndexes.map(node => capture.proof.storageProof.storageProof[node])).toEqual(
      storagePaths[index],
    );
  }
});

type TestGatewayBlockLog =
  | ReturnType<typeof createTransferToArgonStartedBlockLog>
  | ReturnType<typeof createGlobalIssuanceCouncilRotatedBlockLog>;

// Exercise the normal app flow in existing storage scenarios; capture ownership stays outside builders.
async function captureAndBuildStorageProofs(
  client: Parameters<typeof buildGatewayActivityStorageProofs>[0],
  options: Parameters<typeof buildGatewayActivityReceiptProofPayloads>[1] & {
    finalizedExecutionHeader: { blockHash: Hex; blockNumber: bigint };
  },
) {
  if (
    !supportsGatewayActivityStorageProofs(client) ||
    client.consts.crosschainTransfer.maxActivitiesPerGatewayProof.toNumber() < 1
  ) {
    return buildGatewayActivityStorageProofs(client, { ...options, locators: [] });
  }
  const captureNumber = options.locators.reduce(
    (latest, locator) => (locator.blockNumber > latest ? locator.blockNumber : latest),
    options.finalizedExecutionHeader.blockNumber,
  );
  const executionClient = options.executionClient!;
  const executionBlock = await executionClient.getBlock({ blockNumber: captureNumber });
  const captures = await captureGatewayActivityStorageProofs({ ...options, executionBlock });
  return buildGatewayActivityStorageProofs(client, { ...options, locators: captures });
}

async function createGatewayProofHarness(args: {
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
  receiptProofs?: boolean;
  storageProofs?: boolean;
}) {
  const chainId = args.chainId ?? 1n;
  const mappingSlot = args.mappingSlot ?? 11n;
  const previousGatewayActivityNonce = args.previousGatewayActivityNonce ?? 0n;
  const requestedAnchor = args.latestArgonFinalizedHeader ?? {
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
  const storageTrie = await createMPT({ useKeyHashing: true });
  const slotValues = locatorFixtures.flatMap(({ locator }) => {
    const [range, root] = gatewayActivityLocatorSlots(locator.locatorIndex, mappingSlot);
    return [
      { key: range, value: BigInt(gatewayActivityLocatorRangeSlotValue(locator)) },
      { key: root, value: BigInt(locator.activityRoot) },
    ];
  });
  for (const { key, value } of slotValues) {
    await storageTrie.put(hexToBytes(key), hexToBytes(toRlp(toHex(value))));
  }
  const accountTrie = await createMPT({ useKeyHashing: true });
  await accountTrie.put(
    hexToBytes(args.gatewayAddress),
    hexToBytes(toRlp(['0x', '0x', bytesToHex(storageTrie.root()), repeatHex('00', 32)])),
  );
  const captureNumber = args.locators.reduce(
    (latest, locator) => (locator.blockNumber > latest ? locator.blockNumber : latest),
    requestedAnchor.blockNumber,
  );
  const captureBlock = await createExecutionBlock({
    number: captureNumber,
    stateRoot: bytesToHex(accountTrie.root()),
    blockLogs: locatorFixtures
      .filter(({ locator }) => locator.blockNumber === captureNumber)
      .flatMap(({ logs }) => logs),
  });
  const latestArgonFinalizedHeader = {
    blockNumber: requestedAnchor.blockNumber,
    blockHash:
      captureNumber === requestedAnchor.blockNumber ? captureBlock.hash : requestedAnchor.blockHash,
  };
  const proofsByBlockNumber = {
    [captureNumber.toString()]: {
      accountProof: (await createMerkleProof(accountTrie, hexToBytes(args.gatewayAddress))).map(
        node => bytesToHex(node),
      ),
      storageProof: await Promise.all(
        slotValues.map(async slot => ({
          ...slot,
          proof: (await createMerkleProof(storageTrie, hexToBytes(slot.key))).map(node =>
            bytesToHex(node),
          ),
        })),
      ),
    },
  };

  return {
    argonClient: createArgonGatewayClient({
      previousGatewayActivityNonce,
      argonFinalizedExecutionHeaders: [latestArgonFinalizedHeader],
      consts: args.consts,
      runtimeSpecVersion: args.runtimeSpecVersion,
      receiptProofs: args.receiptProofs,
      storageProofs: args.storageProofs,
    }),
    argonFinalizedExecutionHeader: latestArgonFinalizedHeader,
    captureBlock,
    previousGatewayActivityNonce,
    executionClient: createExecutionClient({
      blocks: [captureBlock],
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
