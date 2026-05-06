import { afterEach, expect, it } from 'vitest';
import { ssz } from '@lodestar/types';
import {
  type ArgonClient,
  getEthereumBeaconSyncBootstrapTx,
  getEthereumBeaconSyncState,
  getNextEthereumBeaconSyncTxs,
} from '../index';

const originalFetch = globalThis.fetch;

afterEach(() => {
  globalThis.fetch = originalFetch;
});

it('reads the verifier maintenance window from chain state', async () => {
  const state = await getEthereumBeaconSyncState(createMockClient());

  expect(state.isBootstrapped).toBe(true);
  if (!state.isBootstrapped) throw new Error('expected bootstrapped verifier state');

  expect(state.hasNextSyncCommittee).toBe(false);
  expect(state.latestFinalizedSlot).toBe(800n);
  expect(state.nextRecommendedFinalizedSlot).toBe(832n);
});

it('returns nothing when the verifier is not bootstrapped', async () => {
  globalThis.fetch = createFetch({});

  const txs = await getNextEthereumBeaconSyncTxs(
    createMockClient({ isBootstrapped: false }),
    'https://beacon.example',
  );

  expect(txs).toEqual([]);
});

it('returns nothing when the verifier state is current and the anchor is already retained', async () => {
  const anchorFixture = await createAnchorFixture(
    '0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa',
    800,
  );
  globalThis.fetch = createFetch({
    'https://beacon.example/eth/v1/beacon/light_client/finality_update': {
      data: createLightClientUpdate(800),
    },
    'https://beacon.example/eth/v1/config/spec': {
      data: createBeaconSpec(),
    },
    'https://beacon.example/eth/v1/beacon/headers/0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa':
      anchorFixture.headerResponse,
    'https://beacon.example/eth/v2/beacon/blocks/0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa':
      anchorFixture.blockResponse,
  });

  const txs = await getNextEthereumBeaconSyncTxs(
    createMockClient({ hasNextSyncCommittee: true, hasExecutionAnchor: true }),
    'https://beacon.example',
  );

  expect(txs).toEqual([]);
});

it('builds submit and anchor txs once the free header interval is reached', async () => {
  const anchorFixture = await createAnchorFixture(
    '0xbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb',
    832,
  );
  globalThis.fetch = createFetch({
    'https://beacon.example/eth/v1/beacon/light_client/finality_update': {
      data: createLightClientUpdate(832),
    },
    'https://beacon.example/eth/v1/beacon/headers/832': {
      data: {
        root: '0xbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb',
        canonical: true,
        header: {
          message: createBeaconHeader(832),
          signature: '0xsig',
        },
      },
    },
    'https://beacon.example/eth/v1/config/spec': {
      data: createBeaconSpec(),
    },
    'https://beacon.example/eth/v1/beacon/headers/0xbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb':
      anchorFixture.headerResponse,
    'https://beacon.example/eth/v2/beacon/blocks/0xbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb':
      anchorFixture.blockResponse,
  });

  const txs = await getNextEthereumBeaconSyncTxs(
    createMockClient({ hasNextSyncCommittee: true }),
    'https://beacon.example',
  );

  expect(txs).toEqual([
    {
      method: 'submit',
      update: expect.objectContaining({
        finalizedHeader: expect.objectContaining({ slot: '832' }),
      }),
    },
    {
      method: 'importExecutionHeaderAnchor',
      executionProof: expect.objectContaining({
        header: expect.objectContaining({ slot: '832' }),
      }),
    },
  ]);
});

it('prefers a light client period update when the next sync committee is missing', async () => {
  const anchorFixture = await createAnchorFixture(
    '0xcccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc',
    801,
  );
  globalThis.fetch = createFetch({
    'https://beacon.example/eth/v1/beacon/light_client/finality_update': {
      data: createLightClientUpdate(1600),
    },
    'https://beacon.example/eth/v1/beacon/light_client/updates?count=1&start_period=0': {
      data: [
        createLightClientUpdate(801, {
          next_sync_committee: {
            pubkeys: ['0x4'],
            aggregate_pubkey: '0x5',
          },
          next_sync_committee_branch: ['0x6'],
        }),
      ],
    },
    'https://beacon.example/eth/v1/beacon/headers/801': {
      data: {
        root: '0xcccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc',
        canonical: true,
        header: {
          message: createBeaconHeader(801),
          signature: '0xsig',
        },
      },
    },
    'https://beacon.example/eth/v1/config/spec': {
      data: createBeaconSpec(),
    },
    'https://beacon.example/eth/v1/beacon/headers/0xcccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc':
      anchorFixture.headerResponse,
    'https://beacon.example/eth/v2/beacon/blocks/0xcccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc':
      anchorFixture.blockResponse,
  });

  const txs = await getNextEthereumBeaconSyncTxs(
    createMockClient({ hasNextSyncCommittee: false }),
    'https://beacon.example',
  );

  expect(txs[0]).toEqual({
    method: 'submit',
    update: expect.objectContaining({
      finalizedHeader: expect.objectContaining({ slot: '801' }),
      nextSyncCommitteeUpdate: expect.anything(),
    }),
  });
});

it('still submits a same-slot update when it fills a missing next sync committee', async () => {
  const anchorFixture = await createAnchorFixture(
    '0xdddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd',
    800,
  );
  globalThis.fetch = createFetch({
    'https://beacon.example/eth/v1/beacon/light_client/finality_update': {
      data: createLightClientUpdate(800, {
        next_sync_committee: {
          pubkeys: ['0x4'],
          aggregate_pubkey: '0x5',
        },
        next_sync_committee_branch: ['0x6'],
      }),
    },
    'https://beacon.example/eth/v1/beacon/headers/800': {
      data: {
        root: '0xdddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd',
        canonical: true,
        header: {
          message: createBeaconHeader(800),
          signature: '0xsig',
        },
      },
    },
    'https://beacon.example/eth/v1/config/spec': {
      data: createBeaconSpec(),
    },
    'https://beacon.example/eth/v1/beacon/headers/0xdddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd':
      anchorFixture.headerResponse,
    'https://beacon.example/eth/v2/beacon/blocks/0xdddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd':
      anchorFixture.blockResponse,
  });

  const txs = await getNextEthereumBeaconSyncTxs(
    createMockClient({ hasNextSyncCommittee: false, latestSyncCommitteeUpdatePeriod: 0 }),
    'https://beacon.example',
  );

  expect(txs[0]).toEqual({
    method: 'submit',
    update: expect.objectContaining({
      finalizedHeader: expect.objectContaining({ slot: '800' }),
      nextSyncCommitteeUpdate: expect.anything(),
    }),
  });
});

it('returns only an anchor import when the header state is current but the anchor is missing', async () => {
  const anchorFixture = await createAnchorFixture(
    '0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa',
    800,
  );
  globalThis.fetch = createFetch({
    'https://beacon.example/eth/v1/beacon/light_client/finality_update': {
      data: createLightClientUpdate(800),
    },
    'https://beacon.example/eth/v1/config/spec': {
      data: createBeaconSpec(),
    },
    'https://beacon.example/eth/v1/beacon/headers/0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa':
      anchorFixture.headerResponse,
    'https://beacon.example/eth/v2/beacon/blocks/0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa':
      anchorFixture.blockResponse,
  });

  const txs = await getNextEthereumBeaconSyncTxs(
    createMockClient({ hasNextSyncCommittee: true, hasExecutionAnchor: false }),
    'https://beacon.example',
  );

  expect(txs).toEqual([
    {
      method: 'importExecutionHeaderAnchor',
      executionProof: expect.objectContaining({
        header: expect.objectContaining({ slot: '800' }),
      }),
    },
  ]);
});

it('builds a force-checkpoint tx from beacon bootstrap data without block-roots witnesses', async () => {
  globalThis.fetch = createFetch({
    'https://beacon.example/eth/v1/beacon/genesis': {
      data: {
        genesis_fork_version: '0x00000000',
        genesis_validators_root: '0x11',
      },
    },
    'https://beacon.example/eth/v1/config/spec': {
      data: {
        ALTAIR_FORK_VERSION: '0x01000000',
        ALTAIR_FORK_EPOCH: '0',
        BELLATRIX_FORK_VERSION: '0x02000000',
        BELLATRIX_FORK_EPOCH: '1',
        CAPELLA_FORK_VERSION: '0x03000000',
        CAPELLA_FORK_EPOCH: '2',
        DENEB_FORK_VERSION: '0x04000000',
        DENEB_FORK_EPOCH: '3',
        ELECTRA_FORK_VERSION: '0x05000000',
        ELECTRA_FORK_EPOCH: '4',
        FULU_FORK_VERSION: '0x06000000',
        FULU_FORK_EPOCH: '5',
      },
    },
    'https://beacon.example/eth/v1/beacon/headers/finalized': {
      data: {
        root: '0xabc',
        canonical: true,
        header: {
          message: createBeaconHeader(832),
          signature: '0xsig',
        },
      },
    },
    'https://beacon.example/eth/v1/beacon/light_client/bootstrap/0xabc': {
      data: {
        header: {
          beacon: createBeaconHeader(832),
          execution: createExecutionHeader(832),
          execution_branch: [],
        },
        current_sync_committee: {
          pubkeys: ['0x1'],
          aggregate_pubkey: '0x2',
        },
        current_sync_committee_branch: ['0x3'],
      },
    },
  });

  const tx = await getEthereumBeaconSyncBootstrapTx(createMockClient(), 'https://beacon.example');

  expect(tx).toEqual({
    method: 'forceCheckpoint',
    checkpoint: expect.objectContaining({
      header: expect.objectContaining({ slot: '832' }),
      currentSyncCommittee: expect.objectContaining({ aggregatePubkey: '0x2' }),
      validatorsRoot: '0x11',
    }),
    forks: expect.objectContaining({
      deneb: { version: '0x04000000', epoch: 3n },
    }),
  });
});

function createMockClient(args?: {
  isBootstrapped?: boolean;
  latestFinalizedSlot?: number;
  latestSyncCommitteeUpdatePeriod?: number;
  headerInterval?: number;
  hasNextSyncCommittee?: boolean;
  hasExecutionAnchor?: boolean;
}) {
  const defaultLatestFinalizedBlockRoot =
    '0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa';
  const {
    isBootstrapped = true,
    latestFinalizedSlot = 800,
    latestSyncCommitteeUpdatePeriod = 0,
    headerInterval = 32,
    hasNextSyncCommittee = false,
    hasExecutionAnchor = false,
  } = args ?? {};

  const client = {
    consts: {
      ethereumVerifier: {
        freeHeadersInterval: { toBigInt: () => BigInt(headerInterval) },
      },
    },
    query: {
      ethereumVerifier: {
        latestFinalizedBlockRoot: async () => ({
          toHex: () => defaultLatestFinalizedBlockRoot,
        }),
        finalizedBeaconState: async () =>
          isBootstrapped
            ? {
                isNone: false,
                unwrap: () => ({
                  slot: { toBigInt: () => BigInt(latestFinalizedSlot) },
                }),
              }
            : {
                isNone: true,
                unwrap: () => {
                  throw new Error('missing finalized state');
                },
              },
        latestSyncCommitteeUpdatePeriod: async () => ({
          toBigInt: () => BigInt(latestSyncCommitteeUpdatePeriod),
        }),
        nextSyncCommittee: {
          size: async () => ({
            isZero: () => !hasNextSyncCommittee,
          }),
        },
        executionHeaderAnchors: async () =>
          hasExecutionAnchor
            ? {
                isNone: false,
                unwrap: () => ({
                  blockNumber: { toBigInt: () => 832n },
                }),
              }
            : {
                isNone: true,
                unwrap: () => {
                  throw new Error('missing anchor');
                },
              },
      },
    },
    tx: {
      ethereumVerifier: {
        submit: (update: unknown) => ({ method: 'submit', update }),
        importExecutionHeaderAnchor: (executionProof: unknown) => ({
          method: 'importExecutionHeaderAnchor',
          executionProof,
        }),
        forceCheckpoint: (checkpoint: unknown, forks: unknown) => ({
          method: 'forceCheckpoint',
          checkpoint,
          forks,
        }),
      },
    },
  };

  return client as unknown as ArgonClient;
}

function createFetch(responses: Record<string, unknown>) {
  return async (input: string | URL | Request) => {
    const url =
      typeof input === 'string' ? input : input instanceof URL ? input.toString() : input.url;
    const body = responses[url];

    if (!body) {
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

function createLightClientUpdate(slot: number, extra: Record<string, unknown> = {}) {
  return {
    attested_header: {
      beacon: createBeaconHeader(slot + 1),
      execution: createExecutionHeader(slot + 1),
      execution_branch: [],
    },
    sync_aggregate: {
      sync_committee_bits: `0x${'ff'.repeat(64)}`,
      sync_committee_signature: '0x02',
    },
    signature_slot: `${slot + 2}`,
    finalized_header: {
      beacon: createBeaconHeader(slot),
      execution: createExecutionHeader(slot),
      execution_branch: [],
    },
    finality_branch: ['0xfinality'],
    ...extra,
  };
}

function createBeaconSpec() {
  return {
    SLOTS_PER_EPOCH: '32',
    EPOCHS_PER_SYNC_COMMITTEE_PERIOD: '256',
    SLOTS_PER_HISTORICAL_ROOT: '8192',
  };
}

function createBeaconHeader(slot: number) {
  return {
    slot: `${slot}`,
    proposer_index: '1',
    parent_root: `0xparent${slot}`,
    state_root: `0xstate${slot}`,
    body_root: `0xbody${slot}`,
  };
}

async function createAnchorFixture(blockRoot: string, slot: number) {
  const BeaconBlockBody = ssz.capella.BeaconBlockBody;
  const bodyJson = {
    randao_reveal: `0x${'11'.repeat(96)}`,
    eth1_data: {
      deposit_root: `0x${'22'.repeat(32)}`,
      deposit_count: '0',
      block_hash: `0x${'33'.repeat(32)}`,
    },
    graffiti: `0x${'44'.repeat(32)}`,
    proposer_slashings: [],
    attester_slashings: [],
    attestations: [],
    deposits: [],
    voluntary_exits: [],
    sync_aggregate: {
      sync_committee_bits: `0x${'00'.repeat(64)}`,
      sync_committee_signature: `0x${'55'.repeat(96)}`,
    },
    execution_payload: {
      parent_hash: `0x${'66'.repeat(32)}`,
      fee_recipient: `0x${'77'.repeat(20)}`,
      state_root: `0x${'88'.repeat(32)}`,
      receipts_root: `0x${'99'.repeat(32)}`,
      logs_bloom: `0x${'00'.repeat(256)}`,
      prev_randao: `0x${'aa'.repeat(32)}`,
      block_number: `${slot}`,
      gas_limit: '30000000',
      gas_used: '21000',
      timestamp: `${slot}`,
      extra_data: '0x',
      base_fee_per_gas: '7',
      block_hash: `0x${'bb'.repeat(32)}`,
      transactions: [],
      withdrawals: [],
    },
    bls_to_execution_changes: [],
  };
  const body = BeaconBlockBody.fromJson(bodyJson);
  const bodyRoot = `0x${Buffer.from(BeaconBlockBody.hashTreeRoot(body)).toString('hex')}`;

  return {
    headerResponse: {
      data: {
        root: blockRoot,
        canonical: true,
        header: {
          message: {
            slot: `${slot}`,
            proposer_index: '1',
            parent_root: `0x${'cc'.repeat(32)}`,
            state_root: `0x${'dd'.repeat(32)}`,
            body_root: bodyRoot,
          },
          signature: `0x${'ee'.repeat(96)}`,
        },
      },
    },
    blockResponse: {
      version: 'capella',
      data: {
        message: {
          slot: `${slot}`,
          proposer_index: '1',
          parent_root: `0x${'cc'.repeat(32)}`,
          state_root: `0x${'dd'.repeat(32)}`,
          body: bodyJson,
        },
      },
    },
  };
}

function createExecutionHeader(slot: number) {
  return {
    parent_hash: `0xparenthash${slot}`,
    fee_recipient: `0xfee${slot}`,
    state_root: `0xexecutionstate${slot}`,
    receipts_root: `0xreceipts${slot}`,
    logs_bloom: '0x00',
    prev_randao: `0xrandao${slot}`,
    block_number: `${slot}`,
    gas_limit: '1',
    gas_used: '1',
    timestamp: `${slot}`,
    extra_data: '0x',
    base_fee_per_gas: '1',
    block_hash: `0xblock${slot}`,
    transactions_root: `0xtxs${slot}`,
    withdrawals_root: `0xwithdrawals${slot}`,
    blob_gas_used: '0',
    excess_blob_gas: '0',
  };
}
