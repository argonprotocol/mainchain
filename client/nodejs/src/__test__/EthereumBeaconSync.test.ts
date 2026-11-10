import { Hardfork } from '@ethereumjs/common';
import { afterEach, expect, it, vi } from 'vitest';
import {
  type ArgonClient,
  getEthereumBeaconSyncBootstrapTx,
  getEthereumBeaconSyncState,
  getNextEthereumBeaconSyncTxs,
} from '../index';
import { hexToBytes, type Hex } from 'viem';
import {
  createExecutionClient,
  createExecutionBlockFromRlp,
  gloasExecutionProof,
} from './ethereumProofTestUtils';

const originalFetch = globalThis.fetch;

afterEach(() => {
  globalThis.fetch = originalFetch;
  vi.useRealTimers();
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

it('rejects a beacon endpoint whose preset does not match the chain config', async () => {
  globalThis.fetch = createFetch({
    'http://minimal-beacon.invalid/eth/v1/config/spec': {
      data: createBeaconSpec({ slotsPerHistoricalRoot: '64' }),
    },
  });

  await expect(
    getNextEthereumBeaconSyncTxs(
      createMockClient({ beaconPreset: 'mainnet' }),
      'http://minimal-beacon.invalid',
    ),
  ).rejects.toThrow('Beacon preset mismatch: chain expects mainnet, but endpoint reports minimal');
});

it('caches the expected beacon preset per client across sync attempts', async () => {
  let beaconPresetQueries = 0;
  const client = createMockClient({
    beaconPreset: 'mainnet',
    onBeaconPresetQuery: () => {
      beaconPresetQueries += 1;
    },
  });

  globalThis.fetch = createFetch({
    'http://minimal-beacon.invalid/eth/v1/config/spec': {
      data: createBeaconSpec({ slotsPerHistoricalRoot: '64' }),
    },
  });

  await expect(
    getNextEthereumBeaconSyncTxs(client, 'http://minimal-beacon.invalid'),
  ).rejects.toThrow('Beacon preset mismatch: chain expects mainnet, but endpoint reports minimal');
  await expect(
    getNextEthereumBeaconSyncTxs(client, 'http://minimal-beacon.invalid'),
  ).rejects.toThrow('Beacon preset mismatch: chain expects mainnet, but endpoint reports minimal');

  expect(beaconPresetQueries).toBe(1);
});

it('returns nothing when the verifier state is current and the anchor is already retained', async () => {
  globalThis.fetch = createFetch({
    'https://beacon.example/eth/v1/beacon/light_client/finality_update': {
      data: createLightClientUpdate(800),
    },
    'https://beacon.example/eth/v1/config/spec': {
      data: createBeaconSpec(),
    },
  });

  const txs = await getNextEthereumBeaconSyncTxs(
    createMockClient({ hasNextSyncCommittee: true, hasExecutionAnchor: true }),
    'https://beacon.example',
  );

  expect(txs).toEqual([]);
});

it('builds a submit tx with an execution header proof once the free header interval is reached', async () => {
  globalThis.fetch = createFetch({
    'https://beacon.example/eth/v1/beacon/light_client/finality_update': {
      data: createLightClientUpdate(832, {
        finalized_header: {
          beacon: createBeaconHeader(832),
          execution: createExecutionHeader(832),
          execution_branch: ['0xbranch832'],
        },
      }),
    },
    'https://beacon.example/eth/v1/config/spec': {
      data: createBeaconSpec(),
    },
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
        executionHeaderProof: expect.objectContaining({
          executionBranch: ['0xbranch832'],
        }),
      }),
    },
  ]);
});

it('refreshes a published Gloas activation and builds its hash-only sync update', async () => {
  const block = createExecutionBlockFromRlp(
    gloasExecutionProof.execution_header_rlp as Hex,
    Hardfork.Amsterdam,
  );
  const slot = gloasExecutionProof.header.slot;
  globalThis.fetch = createFetch({
    'https://gloas-sync.example/eth/v1/beacon/light_client/finality_update': {
      data: createLightClientUpdate(slot),
    },
    'https://gloas-sync.example/eth/v1/config/spec': {
      data: { ...createBeaconSpec(), GLOAS_FORK_EPOCH: '18446744073709551615' },
    },
  });
  const client = createMockClient({
    hasNextSyncCommittee: true,
    latestFinalizedSlot: slot - 32,
    latestSyncCommitteeUpdatePeriod: Math.floor(slot / 8192),
  });
  await expect(
    getNextEthereumBeaconSyncTxs(client, 'https://gloas-sync.example'),
  ).resolves.toHaveLength(1);

  globalThis.fetch = createFetch({
    'https://gloas-sync.example/eth/v1/beacon/light_client/finality_update': {
      version: 'gloas',
      data: createLightClientUpdate(slot, {
        finalized_header: {
          beacon: { ...gloasExecutionProof.header, slot: String(slot) },
          execution_block_hash: block.hash,
          execution_branch: gloasExecutionProof.execution_branch,
        },
      }),
    },
    'https://gloas-sync.example/eth/v1/config/spec': {
      data: { ...createBeaconSpec(), GLOAS_FORK_EPOCH: '1' },
    },
  });
  await expect(getNextEthereumBeaconSyncTxs(client, 'https://gloas-sync.example')).rejects.toThrow(
    'execution source',
  );
  const txs = await getNextEthereumBeaconSyncTxs(client, 'https://gloas-sync.example', {
    executionClient: createExecutionClient({ blocks: [block] }),
  });
  expect(txs).toEqual([
    expect.objectContaining({
      method: 'submit',
      update: expect.objectContaining({
        executionHeaderProof: {
          executionHeader: { Rlp: hexToBytes(gloasExecutionProof.execution_header_rlp as Hex) },
          executionBranch: gloasExecutionProof.execution_branch,
        },
      }),
    }),
  ]);
});

it('supports minimal beacon updates with execution header proofs embedded in submit', async () => {
  globalThis.fetch = createFetch({
    'https://minimal-beacon.example/eth/v1/beacon/light_client/finality_update': {
      data: createLightClientUpdate(32, {
        sync_aggregate: {
          sync_committee_bits: '0xffffffff',
          sync_committee_signature: '0x02',
        },
        finalized_header: {
          beacon: createBeaconHeader(32),
          execution: createExecutionHeader(32),
          execution_branch: ['0xfulu32'],
        },
      }),
    },
    'https://minimal-beacon.example/eth/v1/config/spec': {
      data: createBeaconSpec({
        slotsPerHistoricalRoot: '64',
        slotsPerEpoch: '8',
        epochsPerSyncCommitteePeriod: '8',
      }),
    },
  });

  const txs = await getNextEthereumBeaconSyncTxs(
    createMockClient({
      beaconPreset: 'minimal',
      hasNextSyncCommittee: true,
      latestFinalizedSlot: 0,
    }),
    'https://minimal-beacon.example',
  );

  expect(txs).toEqual([
    {
      method: 'submit',
      update: expect.objectContaining({
        finalizedHeader: expect.objectContaining({ slot: '32' }),
        executionHeaderProof: expect.objectContaining({
          executionBranch: ['0xfulu32'],
        }),
      }),
    },
  ]);
});

it('uses a bridgeable finality update when the next sync committee is missing and finality already carries it', async () => {
  globalThis.fetch = createFetch({
    'https://beacon.example/eth/v1/beacon/light_client/finality_update': {
      data: createLightClientUpdate(1600, {
        next_sync_committee: {
          pubkeys: ['0x7'],
          aggregate_pubkey: '0x8',
        },
        next_sync_committee_branch: ['0x9'],
      }),
    },
    'https://beacon.example/eth/v1/beacon/light_client/bootstrap/0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa':
      createBootstrapResponse(800, ['0xstored800']),
    'https://beacon.example/eth/v1/config/spec': {
      data: createBeaconSpec(),
    },
  });

  const txs = await getNextEthereumBeaconSyncTxs(
    createMockClient({ hasNextSyncCommittee: false }),
    'https://beacon.example',
  );

  expect(txs[0]).toEqual({
    method: 'submit',
    update: expect.objectContaining({
      finalizedHeader: expect.objectContaining({ slot: '1600' }),
      nextSyncCommitteeUpdate: expect.objectContaining({
        nextSyncCommittee: expect.anything(),
        nextSyncCommitteeBranch: ['0x9'],
      }),
      executionHeaderProof: expect.objectContaining({
        executionBranch: [],
      }),
    }),
  });
});

it('fetches the current-period update when the next sync committee is missing and finality does not carry it', async () => {
  globalThis.fetch = createFetch({
    'https://beacon.example/eth/v1/beacon/light_client/finality_update': {
      data: createLightClientUpdate(1600),
    },
    'https://beacon.example/eth/v1/beacon/light_client/updates?count=1&start_period=0': [
      createVersionedLightClientUpdate(800, {
        finalized_header: {
          beacon: createBeaconHeader(800),
          execution: createExecutionHeader(800),
          execution_branch: ['0xperiod0-execution'],
        },
        next_sync_committee: {
          pubkeys: ['0x4'],
          aggregate_pubkey: '0x5',
        },
        next_sync_committee_branch: ['0x6'],
      }),
    ],
    'https://beacon.example/eth/v1/beacon/light_client/bootstrap/0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa':
      createBootstrapResponse(800, ['0xstored800']),
    'https://beacon.example/eth/v1/config/spec': {
      data: createBeaconSpec(),
    },
  });

  const txs = await getNextEthereumBeaconSyncTxs(
    createMockClient({ hasNextSyncCommittee: false }),
    'https://beacon.example',
  );

  expect(txs[0]).toEqual({
    method: 'submit',
    update: expect.objectContaining({
      finalizedHeader: expect.objectContaining({ slot: '800' }),
      nextSyncCommitteeUpdate: expect.objectContaining({
        nextSyncCommittee: expect.anything(),
        nextSyncCommitteeBranch: ['0x6'],
      }),
      executionHeaderProof: expect.objectContaining({
        executionBranch: ['0xperiod0-execution'],
      }),
    }),
  });
});

it('steps to the next sync-committee period even when finality already carries committee data', async () => {
  const beaconApiUrl = 'https://minimal-beacon-multi-period.example';

  globalThis.fetch = createFetch({
    [`${beaconApiUrl}/eth/v1/beacon/light_client/finality_update`]: {
      data: createLightClientUpdate(7960, {
        attested_header: {
          beacon: createBeaconHeader(7983),
          execution: createExecutionHeader(7983),
          execution_branch: ['0xfinality7983'],
        },
        signature_slot: '7984',
        next_sync_committee: {
          pubkeys: ['0xfinality-period124'],
          aggregate_pubkey: '0xfinality-period124-agg',
        },
        next_sync_committee_branch: ['0xfinality-period124-branch'],
      }),
    },
    [`${beaconApiUrl}/eth/v1/beacon/light_client/updates?count=1&start_period=122`]: [
      createVersionedLightClientUpdate(7808, {
        attested_header: {
          beacon: createBeaconHeader(7824),
          execution: createExecutionHeader(7824),
          execution_branch: ['0xperiod122-attested'],
        },
        signature_slot: '7825',
        next_sync_committee: {
          pubkeys: ['0xperiod122'],
          aggregate_pubkey: '0xperiod122-agg',
        },
        next_sync_committee_branch: ['0xperiod122-branch'],
        finalized_header: {
          beacon: createBeaconHeader(7808),
          execution: createExecutionHeader(7808),
          execution_branch: ['0xperiod122-finalized'],
        },
      }),
    ],
    [`${beaconApiUrl}/eth/v1/config/spec`]: {
      data: createBeaconSpec({
        slotsPerEpoch: '8',
        epochsPerSyncCommitteePeriod: '8',
        slotsPerHistoricalRoot: '64',
      }),
    },
  });

  const txs = await getNextEthereumBeaconSyncTxs(
    createMockClient({
      beaconPreset: 'minimal',
      hasNextSyncCommittee: true,
      latestFinalizedSlot: 7776,
      latestSyncCommitteeUpdatePeriod: 121,
      headerInterval: 32,
    }),
    beaconApiUrl,
  );

  expect(txs).toEqual([
    {
      method: 'submit',
      update: expect.objectContaining({
        finalizedHeader: expect.objectContaining({ slot: '7808' }),
        signatureSlot: '7825',
        nextSyncCommitteeUpdate: expect.objectContaining({
          nextSyncCommittee: expect.anything(),
          nextSyncCommitteeBranch: ['0xperiod122-branch'],
        }),
        executionHeaderProof: expect.objectContaining({
          executionBranch: ['0xperiod122-finalized'],
        }),
      }),
    },
  ]);
});

it('submits the current-period sync committee update before the next free header window', async () => {
  const beaconApiUrl = 'https://minimal-beacon-next-committee.example';

  globalThis.fetch = createFetch({
    [`${beaconApiUrl}/eth/v1/beacon/light_client/finality_update`]: {
      data: createLightClientUpdate(96, {
        attested_header: {
          beacon: createBeaconHeader(97),
          execution: createExecutionHeader(97),
          execution_branch: ['0xfinality97'],
        },
        signature_slot: '98',
      }),
    },
    [`${beaconApiUrl}/eth/v1/beacon/light_client/updates?count=1&start_period=1`]: [
      createVersionedLightClientUpdate(64, {
        attested_header: {
          beacon: createBeaconHeader(65),
          execution: createExecutionHeader(65),
          execution_branch: ['0xperiod1-attested'],
        },
        signature_slot: '66',
        next_sync_committee: {
          pubkeys: ['0xperiod1'],
          aggregate_pubkey: '0xperiod1-agg',
        },
        next_sync_committee_branch: ['0xperiod1-branch'],
        finalized_header: {
          beacon: createBeaconHeader(64),
          execution: createExecutionHeader(64),
          execution_branch: ['0xperiod1-finalized'],
        },
      }),
    ],
    [`${beaconApiUrl}/eth/v1/beacon/light_client/bootstrap/0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa`]:
      createBootstrapResponse(48, ['0xstored48']),
    [`${beaconApiUrl}/eth/v1/config/spec`]: {
      data: createBeaconSpec({
        slotsPerEpoch: '8',
        epochsPerSyncCommitteePeriod: '8',
        slotsPerHistoricalRoot: '64',
      }),
    },
  });

  const txs = await getNextEthereumBeaconSyncTxs(
    createMockClient({
      beaconPreset: 'minimal',
      hasNextSyncCommittee: true,
      hasExecutionAnchor: true,
      latestFinalizedSlot: 48,
      latestSyncCommitteeUpdatePeriod: 0,
      headerInterval: 32,
    }),
    beaconApiUrl,
  );

  expect(txs).toEqual([
    {
      method: 'submit',
      update: expect.objectContaining({
        finalizedHeader: expect.objectContaining({ slot: '64' }),
        signatureSlot: '66',
        nextSyncCommitteeUpdate: expect.objectContaining({
          nextSyncCommittee: expect.anything(),
          nextSyncCommitteeBranch: ['0xperiod1-branch'],
        }),
      }),
    },
  ]);
});

it('uses a finality update beyond the historical window when its committee is authenticated', async () => {
  const beaconApiUrl = 'https://minimal-beacon-finality-gap.example';
  globalThis.fetch = createFetch({
    [`${beaconApiUrl}/eth/v1/config/spec`]: {
      data: createBeaconSpec({
        slotsPerEpoch: '8',
        epochsPerSyncCommitteePeriod: '8',
        slotsPerHistoricalRoot: '64',
      }),
    },
    [`${beaconApiUrl}/eth/v1/beacon/light_client/finality_update`]: {
      data: createLightClientUpdate(120, {
        next_sync_committee: { pubkeys: ['0x1'], aggregate_pubkey: '0x2' },
        next_sync_committee_branch: ['0xperiod1'],
      }),
    },
  });

  const txs = await getNextEthereumBeaconSyncTxs(
    createMockClient({
      beaconPreset: 'minimal',
      hasNextSyncCommittee: true,
      latestFinalizedSlot: 40,
    }),
    beaconApiUrl,
  );

  expect(txs).toEqual([
    {
      method: 'submit',
      update: expect.objectContaining({
        finalizedHeader: expect.objectContaining({ slot: '120' }),
        signatureSlot: '122',
        nextSyncCommitteeUpdate: expect.objectContaining({
          nextSyncCommitteeBranch: ['0xperiod1'],
        }),
      }),
    },
  ]);
});

it.each([
  {
    preset: 'minimal' as const,
    storedSlot: 1792,
    updateSlot: 1864,
    headSlot: 5120,
    nextPeriod: 29,
    spec: createBeaconSpec({
      slotsPerEpoch: '8',
      epochsPerSyncCommitteePeriod: '8',
      slotsPerHistoricalRoot: '64',
    }),
  },
  {
    preset: 'mainnet' as const,
    storedSlot: 800,
    updateSlot: 9000,
    headSlot: 36000,
    nextPeriod: 1,
    spec: createBeaconSpec(),
  },
])(
  'advances a $preset beacon past the historical window using its authenticated next period',
  async ({ preset, storedSlot, updateSlot, headSlot, nextPeriod, spec }) => {
    const beaconApiUrl = `https://${preset}-beacon-retained-gap.example`;
    globalThis.fetch = createFetch({
      [`${beaconApiUrl}/eth/v1/config/spec`]: { data: spec },
      [`${beaconApiUrl}/eth/v1/beacon/light_client/finality_update`]: {
        data: createLightClientUpdate(headSlot),
      },
      [`${beaconApiUrl}/eth/v1/beacon/light_client/updates?count=1&start_period=${nextPeriod}`]: [
        createVersionedLightClientUpdate(updateSlot, {
          next_sync_committee: { pubkeys: ['0x1'], aggregate_pubkey: '0x2' },
          next_sync_committee_branch: ['0xnext-period'],
        }),
      ],
    });

    const txs = await getNextEthereumBeaconSyncTxs(
      createMockClient({
        beaconPreset: preset,
        hasNextSyncCommittee: true,
        latestFinalizedSlot: storedSlot,
        latestSyncCommitteeUpdatePeriod: nextPeriod - 1,
      }),
      beaconApiUrl,
    );

    expect(txs).toEqual([
      {
        method: 'submit',
        update: expect.objectContaining({
          finalizedHeader: expect.objectContaining({ slot: String(updateSlot) }),
          signatureSlot: String(updateSlot + 2),
          nextSyncCommitteeUpdate: expect.objectContaining({
            nextSyncCommitteeBranch: ['0xnext-period'],
          }),
        }),
      },
    ]);
  },
);

it('uses retained committee state when the last committee-update period predates a checkpoint reset', async () => {
  const beaconApiUrl = 'https://minimal-beacon-reset.example';
  globalThis.fetch = createFetch({
    [`${beaconApiUrl}/eth/v1/config/spec`]: {
      data: createBeaconSpec({
        slotsPerEpoch: '8',
        epochsPerSyncCommitteePeriod: '8',
        slotsPerHistoricalRoot: '64',
      }),
    },
    [`${beaconApiUrl}/eth/v1/beacon/light_client/finality_update`]: {
      data: createLightClientUpdate(5120),
    },
    [`${beaconApiUrl}/eth/v1/beacon/light_client/updates?count=1&start_period=29`]: [
      createVersionedLightClientUpdate(1864, {
        next_sync_committee: { pubkeys: ['0x1'], aggregate_pubkey: '0x2' },
        next_sync_committee_branch: ['0xperiod29'],
      }),
    ],
  });

  const txs = await getNextEthereumBeaconSyncTxs(
    createMockClient({
      beaconPreset: 'minimal',
      hasNextSyncCommittee: true,
      latestFinalizedSlot: 1792,
      latestSyncCommitteeUpdatePeriod: 100,
    }),
    beaconApiUrl,
  );

  expect(txs[0]).toMatchObject({
    method: 'submit',
    update: {
      finalizedHeader: { slot: '1864' },
      nextSyncCommitteeUpdate: { nextSyncCommitteeBranch: ['0xperiod29'] },
    },
  });
});

it('fetches an authenticated period update when finality is signed by an unknown committee', async () => {
  const beaconApiUrl = 'https://minimal-beacon-signature-boundary.example';
  globalThis.fetch = createFetch({
    [`${beaconApiUrl}/eth/v1/config/spec`]: {
      data: createBeaconSpec({
        slotsPerEpoch: '8',
        epochsPerSyncCommitteePeriod: '8',
        slotsPerHistoricalRoot: '64',
      }),
    },
    [`${beaconApiUrl}/eth/v1/beacon/light_client/finality_update`]: {
      data: createLightClientUpdate(96, {
        attested_header: {
          beacon: createBeaconHeader(127),
          execution: createExecutionHeader(127),
          execution_branch: [],
        },
        signature_slot: '128',
        next_sync_committee: { pubkeys: ['0x1'], aggregate_pubkey: '0x2' },
        next_sync_committee_branch: ['0xunknown-signature-period'],
      }),
    },
    [`${beaconApiUrl}/eth/v1/beacon/light_client/updates?count=1&start_period=1`]: [
      createVersionedLightClientUpdate(96, {
        next_sync_committee: { pubkeys: ['0x3'], aggregate_pubkey: '0x4' },
        next_sync_committee_branch: ['0xknown-signature-period'],
      }),
    ],
  });

  const txs = await getNextEthereumBeaconSyncTxs(
    createMockClient({
      beaconPreset: 'minimal',
      hasNextSyncCommittee: true,
      latestFinalizedSlot: 40,
    }),
    beaconApiUrl,
  );

  expect(txs[0]).toMatchObject({
    method: 'submit',
    update: {
      signatureSlot: '98',
      nextSyncCommitteeUpdate: { nextSyncCommitteeBranch: ['0xknown-signature-period'] },
    },
  });
});

it('reports unavailable committee recovery when the retained update also skips a trusted period', async () => {
  const beaconApiUrl = 'https://minimal-beacon-unknown-committee.example';
  globalThis.fetch = createFetch({
    [`${beaconApiUrl}/eth/v1/config/spec`]: {
      data: createBeaconSpec({
        slotsPerEpoch: '8',
        epochsPerSyncCommitteePeriod: '8',
        slotsPerHistoricalRoot: '64',
      }),
    },
    [`${beaconApiUrl}/eth/v1/beacon/light_client/finality_update`]: {
      data: createLightClientUpdate(5120),
    },
    [`${beaconApiUrl}/eth/v1/beacon/light_client/updates?count=1&start_period=29`]: [
      createVersionedLightClientUpdate(1864, {
        attested_header: {
          beacon: createBeaconHeader(1919),
          execution: createExecutionHeader(1919),
          execution_branch: [],
        },
        signature_slot: '1920',
        next_sync_committee: { pubkeys: ['0x1'], aggregate_pubkey: '0x2' },
        next_sync_committee_branch: ['0xperiod29'],
      }),
    ],
  });

  await expect(
    getNextEthereumBeaconSyncTxs(
      createMockClient({
        beaconPreset: 'minimal',
        hasNextSyncCommittee: true,
        latestFinalizedSlot: 1792,
        latestSyncCommitteeUpdatePeriod: 28,
      }),
      beaconApiUrl,
    ),
  ).rejects.toThrow(
    /period 28.*period 29.*unauthenticated period 30.*authenticated intermediate committee update/,
  );
});

it('authenticates the missing next committee before advancing across a large gap', async () => {
  const beaconApiUrl = 'https://minimal-beacon-missing-committee.example';
  globalThis.fetch = createFetch({
    [`${beaconApiUrl}/eth/v1/config/spec`]: {
      data: createBeaconSpec({
        slotsPerEpoch: '8',
        epochsPerSyncCommitteePeriod: '8',
        slotsPerHistoricalRoot: '64',
      }),
    },
    [`${beaconApiUrl}/eth/v1/beacon/light_client/finality_update`]: {
      data: createLightClientUpdate(1864, {
        next_sync_committee: { pubkeys: ['0x1'], aggregate_pubkey: '0x2' },
        next_sync_committee_branch: ['0xperiod29'],
      }),
    },
    [`${beaconApiUrl}/eth/v1/beacon/light_client/updates?count=1&start_period=28`]: [
      createVersionedLightClientUpdate(1792, {
        attested_header: {
          beacon: createBeaconHeader(1808),
          execution: createExecutionHeader(1808),
          execution_branch: [],
        },
        signature_slot: '1809',
        next_sync_committee: { pubkeys: ['0x3'], aggregate_pubkey: '0x4' },
        next_sync_committee_branch: ['0xperiod28'],
      }),
    ],
  });

  const txs = await getNextEthereumBeaconSyncTxs(
    createMockClient({
      beaconPreset: 'minimal',
      hasNextSyncCommittee: false,
      latestFinalizedSlot: 1792,
      latestSyncCommitteeUpdatePeriod: 28,
    }),
    beaconApiUrl,
  );

  expect(txs[0]).toMatchObject({
    method: 'submit',
    update: {
      finalizedHeader: { slot: '1792' },
      signatureSlot: '1809',
      nextSyncCommitteeUpdate: { nextSyncCommitteeBranch: ['0xperiod28'] },
    },
  });
});

it('waits for supermajority on an authenticated retained update', async () => {
  vi.useFakeTimers();
  const beaconApiUrl = 'https://minimal-beacon-quorum-pending.example';
  globalThis.fetch = createFetch({
    [`${beaconApiUrl}/eth/v1/config/spec`]: {
      data: createBeaconSpec({
        slotsPerEpoch: '8',
        epochsPerSyncCommitteePeriod: '8',
        slotsPerHistoricalRoot: '64',
      }),
    },
    [`${beaconApiUrl}/eth/v1/beacon/light_client/finality_update`]: {
      data: createLightClientUpdate(5120),
    },
    [`${beaconApiUrl}/eth/v1/beacon/light_client/updates?count=1&start_period=29`]: [
      createVersionedLightClientUpdate(1864, {
        next_sync_committee: { pubkeys: ['0x1'], aggregate_pubkey: '0x2' },
        next_sync_committee_branch: ['0xperiod29'],
        sync_aggregate: { sync_committee_bits: '0x01000000', sync_committee_signature: '0x02' },
      }),
    ],
  });

  const sync = getNextEthereumBeaconSyncTxs(
    createMockClient({
      beaconPreset: 'minimal',
      hasNextSyncCommittee: true,
      latestFinalizedSlot: 1792,
      latestSyncCommitteeUpdatePeriod: 28,
    }),
    beaconApiUrl,
  );
  await Promise.all([expect(sync).resolves.toEqual([]), vi.runAllTimersAsync()]);
});

it('retries an unavailable authenticated period update when finality has an unknown signer', async () => {
  const beaconApiUrl = 'https://minimal-beacon-period-retry.example';
  let periodUpdateRequests = 0;
  const responses = createFetch({
    [`${beaconApiUrl}/eth/v1/config/spec`]: {
      data: createBeaconSpec({
        slotsPerEpoch: '8',
        epochsPerSyncCommitteePeriod: '8',
        slotsPerHistoricalRoot: '64',
      }),
    },
    [`${beaconApiUrl}/eth/v1/beacon/light_client/finality_update`]: {
      data: createLightClientUpdate(96, {
        attested_header: {
          beacon: createBeaconHeader(127),
          execution: createExecutionHeader(127),
          execution_branch: [],
        },
        signature_slot: '128',
        next_sync_committee: { pubkeys: ['0x1'], aggregate_pubkey: '0x2' },
        next_sync_committee_branch: ['0xperiod1'],
      }),
    },
    [`${beaconApiUrl}/eth/v1/beacon/light_client/updates?count=1&start_period=1`]: [
      createVersionedLightClientUpdate(96, {
        next_sync_committee: { pubkeys: ['0x3'], aggregate_pubkey: '0x4' },
        next_sync_committee_branch: ['0xretained-period1'],
      }),
    ],
  });
  globalThis.fetch = async input => {
    if (String(input).includes('/light_client/updates?')) {
      periodUpdateRequests += 1;
      if (periodUpdateRequests === 1) return { ok: true, json: async () => [] } as Response;
    }
    return responses(input);
  };

  const txs = await getNextEthereumBeaconSyncTxs(
    createMockClient({
      beaconPreset: 'minimal',
      hasNextSyncCommittee: true,
      latestFinalizedSlot: 40,
    }),
    beaconApiUrl,
  );

  expect(periodUpdateRequests).toBe(2);
  expect(txs[0]).toMatchObject({
    method: 'submit',
    update: {
      signatureSlot: '98',
      nextSyncCommitteeUpdate: { nextSyncCommitteeBranch: ['0xretained-period1'] },
    },
  });
});

it('returns nothing when the required current-period committee update is unavailable', async () => {
  globalThis.fetch = createFetch({
    'https://beacon.example/eth/v1/beacon/light_client/finality_update': {
      data: createLightClientUpdate(1600),
    },
    'https://beacon.example/eth/v1/beacon/light_client/updates?count=1&start_period=0': [{} as any],
    'https://beacon.example/eth/v1/config/spec': {
      data: createBeaconSpec(),
    },
  });

  const txs = await getNextEthereumBeaconSyncTxs(
    createMockClient({ hasNextSyncCommittee: false }),
    'https://beacon.example',
  );

  expect(txs).toEqual([]);
});

it('returns nothing when the header state is current but no new update is needed', async () => {
  globalThis.fetch = createFetch({
    'https://beacon.example/eth/v1/beacon/light_client/finality_update': {
      data: createLightClientUpdate(800, {
        finalized_header: {
          beacon: createBeaconHeader(800),
          execution: createExecutionHeader(800),
          execution_branch: ['0xfinality800'],
        },
      }),
    },
    'https://beacon.example/eth/v1/beacon/light_client/bootstrap/0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa':
      createBootstrapResponse(800, ['0xstored800']),
    'https://beacon.example/eth/v1/config/spec': {
      data: createBeaconSpec(),
    },
  });

  const txs = await getNextEthereumBeaconSyncTxs(
    createMockClient({ hasNextSyncCommittee: true, hasExecutionAnchor: false }),
    'https://beacon.example',
  );

  expect(txs).toEqual([]);
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
        ...createBeaconSpec(),
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
      executionHeaderProof: expect.objectContaining({
        executionBranch: [],
      }),
    }),
    forks: expect.objectContaining({
      deneb: { version: '0x04000000', epoch: 3n },
      gloas: { version: '0x00000000', epoch: 18446744073709551615n },
    }),
  });
});

it('bootstraps Gloas with its configured fork schedule and exact execution header bytes', async () => {
  const block = createExecutionBlockFromRlp(
    gloasExecutionProof.execution_header_rlp as Hex,
    Hardfork.Amsterdam,
  );
  const slot = gloasExecutionProof.header.slot;
  const forkSpec: Record<string, string> = { ...createBeaconSpec() };
  const forkNames = ['ALTAIR', 'BELLATRIX', 'CAPELLA', 'DENEB', 'ELECTRA', 'FULU', 'GLOAS'];
  for (const [index, fork] of forkNames.entries()) {
    forkSpec[`${fork}_FORK_VERSION`] = `0x0${index + 1}000000`;
    forkSpec[`${fork}_FORK_EPOCH`] = String(index);
  }
  globalThis.fetch = createFetch({
    'https://gloas-bootstrap.example/eth/v1/config/spec': { data: forkSpec },
    'https://gloas-bootstrap.example/eth/v1/beacon/genesis': {
      data: { genesis_fork_version: '0x00000000', genesis_validators_root: '0x11' },
    },
    'https://gloas-bootstrap.example/eth/v1/beacon/headers/finalized': { data: { root: '0xabc' } },
    'https://gloas-bootstrap.example/eth/v1/beacon/light_client/bootstrap/0xabc': {
      data: {
        ...createBootstrapResponse(slot, []).data,
        header: {
          beacon: { ...gloasExecutionProof.header, slot: String(slot) },
          execution_block_hash: block.hash,
          execution_branch: gloasExecutionProof.execution_branch,
        },
      },
    },
  });
  const tx = await getEthereumBeaconSyncBootstrapTx(
    createMockClient(),
    'https://gloas-bootstrap.example',
    { executionClient: createExecutionClient({ blocks: [block] }) },
  );
  expect(tx).toMatchObject({
    method: 'forceCheckpoint',
    checkpoint: {
      executionHeaderProof: {
        executionHeader: { Rlp: hexToBytes(gloasExecutionProof.execution_header_rlp as Hex) },
        executionBranch: gloasExecutionProof.execution_branch,
      },
    },
    forks: { gloas: { version: '0x07000000', epoch: 6n } },
  });
});

it('uses mainnet as the default beacon preset value', async () => {
  await expect(createMockClient().query.ethereumVerifier.beaconPreset()).resolves.toMatchObject({
    isMainnet: true,
    isMinimal: false,
  });
});

function createMockClient(args?: {
  isBootstrapped?: boolean;
  latestFinalizedSlot?: number;
  latestSyncCommitteeUpdatePeriod?: number;
  headerInterval?: number;
  hasNextSyncCommittee?: boolean;
  hasExecutionAnchor?: boolean;
  beaconPreset?: 'mainnet' | 'minimal';
  onBeaconPresetQuery?: () => void;
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
    beaconPreset = 'mainnet',
    onBeaconPresetQuery,
  } = args ?? {};

  const client = {
    consts: {
      ethereumVerifier: {
        freeHeadersInterval: { toBigInt: () => BigInt(headerInterval) },
      },
    },
    query: {
      ethereumVerifier: {
        beaconPreset: async () => {
          onBeaconPresetQuery?.();

          return {
            isMainnet: beaconPreset === 'mainnet',
            isMinimal: beaconPreset === 'minimal',
            toString: () => beaconPreset,
          };
        },
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

function createVersionedLightClientUpdate(slot: number, extra: Record<string, unknown> = {}) {
  return {
    version: 'fulu',
    data: createLightClientUpdate(slot, extra),
  };
}

function createBootstrapResponse(slot: number, executionBranch: string[]) {
  return {
    data: {
      header: {
        beacon: createBeaconHeader(slot),
        execution: createExecutionHeader(slot),
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

function createBeaconSpec(args?: {
  slotsPerHistoricalRoot?: string;
  slotsPerEpoch?: string;
  epochsPerSyncCommitteePeriod?: string;
}) {
  const {
    slotsPerHistoricalRoot = '8192',
    slotsPerEpoch = '32',
    epochsPerSyncCommitteePeriod = '256',
  } = args ?? {};

  return {
    SLOTS_PER_EPOCH: slotsPerEpoch,
    EPOCHS_PER_SYNC_COMMITTEE_PERIOD: epochsPerSyncCommitteePeriod,
    SLOTS_PER_HISTORICAL_ROOT: slotsPerHistoricalRoot,
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
