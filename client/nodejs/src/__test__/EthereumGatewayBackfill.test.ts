import { afterEach, expect, it } from 'vitest';
import type { ArgonClient, IArgonQueryable } from '../index';
import {
  buildGatewayExecutionHeaderBackfill,
  buildGatewayExecutionHeaderBackfills,
} from '../EthereumGatewayBackfill';
import { MintingGatewayEvents, mintingGatewayAbi } from '../EvmContracts';
import {
  createExecutionBlock,
  createExecutionClient,
  createLegacyReceipt,
  repeatHex,
} from './ethereumProofTestUtils';
import { encodeAbiParameters, encodeEventTopics, type Hex } from 'viem';

const originalFetch = globalThis.fetch;

afterEach(() => {
  globalThis.fetch = originalFetch;
});

it('builds a historical checkpoint backfill payload for the first uncovered gateway proof block', async () => {
  const gatewayAddress: Hex = repeatHex('57', 20);
  const txHash: Hex = repeatHex('11', 32);
  const log = createTransferToArgonStartedBlockLog({
    gatewayAddress,
    txHash,
    transactionIndex: 0,
    logIndex: 0,
    blockHash: repeatHex('00', 32),
    blockNumber: 10n,
    nonce: 1n,
    argonAccountId: repeatHex('61', 32),
  });
  const receipt = createLegacyReceipt({
    txHash,
    transactionIndex: 0,
    logs: [{ address: gatewayAddress, topics: log.topics as [Hex, ...Hex[]], data: log.data }],
  });
  const block10 = await createExecutionBlock({
    number: 10n,
    receipts: [receipt],
    blockLogs: [log],
  });

  globalThis.fetch = createFetch({
    'https://beacon.example/eth/v1/beacon/genesis': {
      data: {
        genesis_time: '0',
        genesis_fork_version: '0x00000000',
        genesis_validators_root: repeatHex('01', 32),
      },
    },
    'https://beacon.example/eth/v1/config/spec': {
      data: {
        SLOTS_PER_EPOCH: '32',
        SECONDS_PER_SLOT: '1',
      },
    },
    'https://beacon.example/eth/v1/beacon/headers?slot=96': createHeaderAtSlotResponse(
      96,
      repeatHex('96', 32),
    ),
    'https://beacon.example/eth/v1/beacon/light_client/bootstrap/0x9696969696969696969696969696969696969696969696969696969696969696':
      createBootstrapResponse(96, 30n, repeatHex('96', 32), ['0xbranch96']),
    'https://beacon.example/eth/v1/beacon/headers?slot=64': createHeaderAtSlotResponse(
      64,
      repeatHex('64', 32),
    ),
    'https://beacon.example/eth/v1/beacon/light_client/bootstrap/0x6464646464646464646464646464646464646464646464646464646464646464':
      createBootstrapResponse(64, 12n, repeatHex('64', 32), ['0xbranch64']),
    'https://beacon.example/eth/v1/beacon/headers?slot=32': { data: [] },
    'https://beacon.example/eth/v1/beacon/headers?slot=0': createHeaderAtSlotResponse(
      0,
      repeatHex('00', 32),
    ),
    'https://beacon.example/eth/v1/beacon/light_client/bootstrap/0x0000000000000000000000000000000000000000000000000000000000000000':
      createBootstrapResponse(0, 4n, repeatHex('00', 32), ['0xbranch0']),
  });

  const payload = await buildGatewayExecutionHeaderBackfill(
    createBackfillClient({
      latestFinalizedSlot: 96n,
      firstRetainedAnchorAtOrAfterBlock: null,
    }),
    {
      beaconApiUrl: 'https://beacon.example',
      executionClient: createExecutionClient({
        blocks: [block10],
        receipts: [receipt],
        logsByBlockNumber: { '10': [log] },
        locators: [
          { blockNumber: 10n, startGatewayActivityNonce: 1n, endGatewayActivityNonce: 1n },
        ],
      }),
      gatewayAddress,
    },
  );

  expect(payload).toEqual(
    expect.objectContaining({
      targetExecutionBlockNumber: 10n,
      expectedBeaconRoot: repeatHex('64', 32),
      executionBlockNumber: 12n,
      executionBlockHash: repeatHex('64', 32),
      executionHeaderProof: expect.objectContaining({
        executionBranch: ['0xbranch64'],
      }),
      header: expect.objectContaining({
        slot: '64',
      }),
    }),
  );
});

it('returns null when a retained execution anchor already covers the missing gateway proof block', async () => {
  const gatewayAddress: Hex = repeatHex('58', 20);
  const txHash: Hex = repeatHex('12', 32);
  const log = createTransferToArgonStartedBlockLog({
    gatewayAddress,
    txHash,
    transactionIndex: 0,
    logIndex: 0,
    blockHash: repeatHex('00', 32),
    blockNumber: 10n,
    nonce: 1n,
    argonAccountId: repeatHex('62', 32),
  });
  const receipt = createLegacyReceipt({
    txHash,
    transactionIndex: 0,
    logs: [{ address: gatewayAddress, topics: log.topics as [Hex, ...Hex[]], data: log.data }],
  });
  const block10 = await createExecutionBlock({
    number: 10n,
    receipts: [receipt],
    blockLogs: [log],
  });

  globalThis.fetch = createFetch({});

  await expect(
    buildGatewayExecutionHeaderBackfill(
      createBackfillClient({
        latestFinalizedSlot: 96n,
        firstRetainedAnchorAtOrAfterBlock: {
          blockHash: repeatHex('77', 32),
          blockNumber: 12n,
        },
      }),
      {
        beaconApiUrl: 'https://beacon.example',
        executionClient: createExecutionClient({
          blocks: [block10],
          receipts: [receipt],
          logsByBlockNumber: { '10': [log] },
          locators: [
            { blockNumber: 10n, startGatewayActivityNonce: 1n, endGatewayActivityNonce: 1n },
          ],
        }),
        gatewayAddress,
      },
    ),
  ).resolves.toBeNull();
});

it('lists only the missing backfills for uncovered gateway locator blocks', async () => {
  const gatewayAddress: Hex = repeatHex('59', 20);
  const txHash10: Hex = repeatHex('13', 32);
  const txHash40: Hex = repeatHex('14', 32);
  const txHash120: Hex = repeatHex('15', 32);
  const log10 = createTransferToArgonStartedBlockLog({
    gatewayAddress,
    txHash: txHash10,
    transactionIndex: 0,
    logIndex: 0,
    blockHash: repeatHex('10', 32),
    blockNumber: 10n,
    nonce: 1n,
    argonAccountId: repeatHex('63', 32),
  });
  const log40 = createTransferToArgonStartedBlockLog({
    gatewayAddress,
    txHash: txHash40,
    transactionIndex: 0,
    logIndex: 0,
    blockHash: repeatHex('40', 32),
    blockNumber: 40n,
    nonce: 2n,
    argonAccountId: repeatHex('64', 32),
  });
  const log120 = createTransferToArgonStartedBlockLog({
    gatewayAddress,
    txHash: txHash120,
    transactionIndex: 0,
    logIndex: 0,
    blockHash: repeatHex('12', 32),
    blockNumber: 120n,
    nonce: 3n,
    argonAccountId: repeatHex('65', 32),
  });
  const receipt10 = createLegacyReceipt({
    txHash: txHash10,
    transactionIndex: 0,
    logs: [{ address: gatewayAddress, topics: log10.topics as [Hex, ...Hex[]], data: log10.data }],
  });
  const receipt40 = createLegacyReceipt({
    txHash: txHash40,
    transactionIndex: 0,
    logs: [{ address: gatewayAddress, topics: log40.topics as [Hex, ...Hex[]], data: log40.data }],
  });
  const receipt120 = createLegacyReceipt({
    txHash: txHash120,
    transactionIndex: 0,
    logs: [
      { address: gatewayAddress, topics: log120.topics as [Hex, ...Hex[]], data: log120.data },
    ],
  });
  const block10 = await createExecutionBlock({
    number: 10n,
    receipts: [receipt10],
    blockLogs: [log10],
  });
  const block40 = await createExecutionBlock({
    number: 40n,
    receipts: [receipt40],
    blockLogs: [log40],
  });
  const block120 = await createExecutionBlock({
    number: 120n,
    receipts: [receipt120],
    blockLogs: [log120],
  });

  globalThis.fetch = createFetch({
    'https://beacon.example/eth/v1/beacon/genesis': {
      data: {
        genesis_time: '0',
        genesis_fork_version: '0x00000000',
        genesis_validators_root: repeatHex('01', 32),
      },
    },
    'https://beacon.example/eth/v1/config/spec': {
      data: {
        SLOTS_PER_EPOCH: '32',
        SECONDS_PER_SLOT: '1',
      },
    },
    'https://beacon.example/eth/v1/beacon/headers?slot=160': createHeaderAtSlotResponse(
      160,
      repeatHex('a0', 32),
    ),
    'https://beacon.example/eth/v1/beacon/light_client/bootstrap/0xa0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0':
      createBootstrapResponse(160, 150n, repeatHex('a0', 32), ['0xbrancha0']),
    'https://beacon.example/eth/v1/beacon/headers?slot=128': createHeaderAtSlotResponse(
      128,
      repeatHex('80', 32),
    ),
    'https://beacon.example/eth/v1/beacon/light_client/bootstrap/0x8080808080808080808080808080808080808080808080808080808080808080':
      createBootstrapResponse(128, 130n, repeatHex('80', 32), ['0xbranch80']),
    'https://beacon.example/eth/v1/beacon/headers?slot=96': createHeaderAtSlotResponse(
      96,
      repeatHex('60', 32),
    ),
    'https://beacon.example/eth/v1/beacon/light_client/bootstrap/0x6060606060606060606060606060606060606060606060606060606060606060':
      createBootstrapResponse(96, 90n, repeatHex('60', 32), ['0xbranch60']),
    'https://beacon.example/eth/v1/beacon/headers?slot=64': createHeaderAtSlotResponse(
      64,
      repeatHex('40', 32),
    ),
    'https://beacon.example/eth/v1/beacon/light_client/bootstrap/0x4040404040404040404040404040404040404040404040404040404040404040':
      createBootstrapResponse(64, 55n, repeatHex('40', 32), ['0xbranch40']),
    'https://beacon.example/eth/v1/beacon/headers?slot=32': createHeaderAtSlotResponse(
      32,
      repeatHex('20', 32),
    ),
    'https://beacon.example/eth/v1/beacon/light_client/bootstrap/0x2020202020202020202020202020202020202020202020202020202020202020':
      createBootstrapResponse(32, 20n, repeatHex('20', 32), ['0xbranch20']),
    'https://beacon.example/eth/v1/beacon/headers?slot=0': createHeaderAtSlotResponse(
      0,
      repeatHex('00', 32),
    ),
    'https://beacon.example/eth/v1/beacon/light_client/bootstrap/0x0000000000000000000000000000000000000000000000000000000000000000':
      createBootstrapResponse(0, 4n, repeatHex('00', 32), ['0xbranch0']),
  });

  const payloads = await buildGatewayExecutionHeaderBackfills(
    createBackfillClient({
      latestFinalizedSlot: 160n,
      firstRetainedAnchorAtOrAfterBlock: {
        blockHash: repeatHex('60', 32),
        blockNumber: 90n,
      },
    }),
    {
      beaconApiUrl: 'https://beacon.example',
      executionClient: createExecutionClient({
        blocks: [block10, block40, block120],
        receipts: [receipt10, receipt40, receipt120],
        logsByBlockNumber: {
          '10': [log10],
          '40': [log40],
          '120': [log120],
        },
        locators: [
          { blockNumber: 10n, startGatewayActivityNonce: 1n, endGatewayActivityNonce: 1n },
          { blockNumber: 40n, startGatewayActivityNonce: 2n, endGatewayActivityNonce: 2n },
          { blockNumber: 120n, startGatewayActivityNonce: 3n, endGatewayActivityNonce: 3n },
        ],
      }),
      gatewayAddress,
    },
  );

  expect(payloads).toEqual([
    expect.objectContaining({
      targetExecutionBlockNumber: 10n,
      executionBlockNumber: 20n,
      expectedBeaconRoot: repeatHex('20', 32),
    }),
    expect.objectContaining({
      targetExecutionBlockNumber: 120n,
      executionBlockNumber: 130n,
      expectedBeaconRoot: repeatHex('80', 32),
    }),
  ]);
});

function createBackfillClient(args: {
  latestFinalizedSlot: bigint;
  firstRetainedAnchorAtOrAfterBlock: {
    blockHash: Hex;
    blockNumber: bigint;
  } | null;
}) {
  const latestFinalizedBlockRoot = repeatHex('aa', 32);

  return {
    query: {
      crosschainTransfer: {
        gatewayStateBySourceChain: async () => ({
          isSome: false,
        }),
      },
      ethereumVerifier: {
        latestFinalizedBlockRoot: async () => ({
          toHex: () => latestFinalizedBlockRoot,
        }),
        finalizedBeaconState: async () => ({
          isNone: false,
          unwrap: () => ({
            slot: { toBigInt: () => args.latestFinalizedSlot },
          }),
        }),
        executionHeaderAnchorsByBlockNumber: Object.assign(
          async (scanKey?: Hex) => {
            const requestedBlockNumber = scanKey ? BigInt(scanKey) : undefined;
            if (
              args.firstRetainedAnchorAtOrAfterBlock &&
              requestedBlockNumber === args.firstRetainedAnchorAtOrAfterBlock.blockNumber
            ) {
              return {
                isSome: true,
                unwrap: () => ({
                  blockHash: {
                    toHex: () => args.firstRetainedAnchorAtOrAfterBlock!.blockHash,
                  },
                  blockNumber: {
                    toBigInt: () => args.firstRetainedAnchorAtOrAfterBlock!.blockNumber,
                  },
                }),
              };
            }
            return { isSome: false };
          },
          {
            key: (scanKey: Hex) => `storage:${scanKey.toLowerCase()}`,
            entriesPaged: async ({ startKey }: { startKey?: string } = {}) => {
              if (!args.firstRetainedAnchorAtOrAfterBlock) {
                return [];
              }

              const startBlockNumber = startKey ? BigInt(startKey.split(':').at(-1)!) : 0n;
              if (args.firstRetainedAnchorAtOrAfterBlock.blockNumber < startBlockNumber) {
                return [];
              }

              return [
                [
                  'storage:covered',
                  {
                    isSome: true,
                    unwrap: () => ({
                      blockHash: {
                        toHex: () => args.firstRetainedAnchorAtOrAfterBlock!.blockHash,
                      },
                      blockNumber: {
                        toBigInt: () => args.firstRetainedAnchorAtOrAfterBlock!.blockNumber,
                      },
                    }),
                  },
                ] as const,
              ];
            },
          },
        ),
      },
    },
    consts: {
      crosschainTransfer: {
        maxProofExecutionHeaderDepth: {
          toNumber: () => 64,
        },
      },
    },
  } as unknown as IArgonQueryable & Pick<ArgonClient, 'consts'>;
}

function createFetch(responses: Record<string, unknown>) {
  return async (input: string | URL | Request) => {
    const url =
      typeof input === 'string' ? input : input instanceof URL ? input.toString() : input.url;
    const body = responses[url];

    if (!body) {
      if (
        url.includes('/eth/v1/beacon/headers?slot=') ||
        url.includes('/eth/v1/beacon/light_client/bootstrap/')
      ) {
        return {
          ok: false,
          json: async () => ({}),
          status: 404,
          statusText: 'Not Found',
        } as Response;
      }

      throw new Error(`Unexpected fetch: ${url}`);
    }

    return {
      ok: true,
      json: async () => body,
      status: 200,
      statusText: 'OK',
    } as Response;
  };
}

function createHeaderAtSlotResponse(slot: number, root: Hex) {
  return {
    data: [
      {
        root,
        canonical: true,
        header: {
          message: createBeaconHeader(slot),
          signature: '0xsig',
        },
      },
    ],
  };
}

function createBootstrapResponse(
  slot: number,
  executionBlockNumber: bigint,
  blockHash: Hex,
  executionBranch: string[],
) {
  return {
    data: {
      header: {
        beacon: createBeaconHeader(slot),
        execution: createExecutionHeader(executionBlockNumber, blockHash),
        execution_branch: executionBranch,
      },
      current_sync_committee: {
        pubkeys: ['0x1'],
        aggregate_pubkey: '0x2',
      },
      current_sync_committee_branch: ['0x3'],
    },
  };
}

function createBeaconHeader(slot: number) {
  return {
    slot: `${slot}`,
    proposer_index: '1',
    parent_root: repeatHex('ab', 32),
    state_root: repeatHex('cd', 32),
    body_root: repeatHex('ef', 32),
  };
}

function createExecutionHeader(blockNumber: bigint, blockHash: Hex) {
  return {
    parent_hash: repeatHex('01', 32),
    fee_recipient: repeatHex('02', 20),
    state_root: repeatHex('03', 32),
    receipts_root: repeatHex('04', 32),
    logs_bloom: '0x00',
    prev_randao: repeatHex('05', 32),
    block_number: `${blockNumber}`,
    gas_limit: '30000000',
    gas_used: '21000',
    timestamp: `${blockNumber}`,
    extra_data: '0x',
    base_fee_per_gas: '7',
    block_hash: blockHash,
    transactions_root: repeatHex('06', 32),
    withdrawals_root: repeatHex('07', 32),
  };
}

function createTransferToArgonStartedBlockLog(args: {
  gatewayAddress: Hex;
  txHash: Hex;
  transactionIndex: number;
  logIndex: number;
  blockHash: Hex;
  blockNumber: bigint;
  nonce: bigint;
  argonAccountId: Hex;
}) {
  return {
    address: args.gatewayAddress,
    topics: encodeEventTopics({
      abi: mintingGatewayAbi,
      eventName: MintingGatewayEvents.TransferToArgonStarted.name,
      args: {
        from: repeatHex('11', 20),
        token: repeatHex('22', 20),
      },
    }) as Hex[],
    data: encodeAbiParameters(
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
    ),
    transactionHash: args.txHash,
    transactionIndex: args.transactionIndex,
    logIndex: args.logIndex,
    blockHash: args.blockHash,
    blockNumber: args.blockNumber,
  };
}
