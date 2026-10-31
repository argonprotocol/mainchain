import { Hardfork } from '@ethereumjs/common';
import { expect, it, vi } from 'vitest';
import { createBlockHeaderFromRPC } from '@ethereumjs/block';
import { bytesToHex, fromRlp, hexToBytes, keccak256, toHex, toRlp, type Hex } from 'viem';
import { buildExecutionHeaderProof, normalizeBeaconStateBranch } from '../EthereumBeaconApi';
import type {
  EthereumLightClientExecutionPayloadHeader,
  EthereumLightClientHeader,
} from '../EthereumBeaconTypes';
import type { EthereumExecutionClient } from '../EthereumExecution';
import { decodeExecutionHeaderRlp, loadExecutionHeader } from '../EthereumProof';
import {
  createExecutionBlock,
  createExecutionClient,
  createExecutionBlockFromRlp,
  gloasExecutionProof,
  repeatHex,
} from './ethereumProofTestUtils';

import historicalProofs from '../../../../pallets/ethereum_verifier/tests/fixtures/historical-execution-hash-proofs.json';

const zero = repeatHex('00', 32);
const spec = {
  SLOTS_PER_EPOCH: '8',
  CAPELLA_FORK_EPOCH: '1',
  DENEB_FORK_EPOCH: '2',
  GLOAS_FORK_EPOCH: '3',
};

it('fetches the exact Gloas parent execution hash and preserves the shared Rust fixture bytes', async () => {
  const block = createExecutionBlockFromRlp(
    gloasExecutionProof.execution_header_rlp as Hex,
    Hardfork.Amsterdam,
  );
  const rlp = hexToBytes(gloasExecutionProof.execution_header_rlp as Hex);
  const request = vi.fn(async () => block);
  const header: EthereumLightClientHeader = {
    beacon: {
      ...gloasExecutionProof.header,
      slot: String(gloasExecutionProof.header.slot),
      proposer_index: String(gloasExecutionProof.header.proposer_index),
    },
    execution_block_hash: block.hash,
    execution_branch: gloasExecutionProof.execution_branch,
  };
  const proof = await buildExecutionHeaderProof(
    header,
    { executionClient: { request } as unknown as EthereumExecutionClient },
    spec,
  );

  expect(request).toHaveBeenCalledExactlyOnceWith({
    method: 'eth_getBlockByHash',
    params: [block.hash, false],
  });
  expect(rlp).toHaveLength(660);
  expect(block.slotNumber).toBe(toHex(115967n));
  expect(proof).toEqual({
    executionHeader: { Rlp: rlp },
    executionBranch: gloasExecutionProof.execution_branch,
  });
  const retained = decodeExecutionHeaderRlp(rlp);
  expect(retained.number).toBe(113202n);
  expect(bytesToHex(retained.stateRoot)).toBe(block.stateRoot);
  expect(bytesToHex(retained.hash())).toBe(block.hash);
  expect(retained.serialize()).toEqual(rlp);
  expect(retained.toJSON()).toMatchObject({ slotNumber: block.slotNumber });
});

it('rejects altered, truncated and unknown execution fields instead of discarding them', async () => {
  const block = createExecutionBlockFromRlp(
    gloasExecutionProof.execution_header_rlp as Hex,
    Hardfork.Amsterdam,
  );
  const fields = fromRlp(gloasExecutionProof.execution_header_rlp as Hex, 'hex') as Hex[];
  for (const [rpc, error] of [
    [{ ...block, receiptsRoot: zero }, 'hash mismatch'],
    [{ ...block, slotNumber: undefined }, 'Unsupported execution header field count'],
    [{ ...block, hash: keccak256(toRlp([...fields, '0x01'])) }, 'hash mismatch'],
  ] as const) {
    const executionClient = { request: async () => rpc } as unknown as EthereumExecutionClient;
    await expect(loadExecutionHeader(executionClient, rpc.hash)).rejects.toThrow(error);
    await expect(loadExecutionHeader(executionClient, BigInt(block.number))).rejects.toThrow(error);
  }
});

it('preserves the public EthereumJS header fields and methods for pre-Gloas callers', async () => {
  const block = await createExecutionBlock({ number: 10n });
  const header = await loadExecutionHeader(createExecutionClient({ blocks: [block] }), block.hash);
  expect(bytesToHex(header.stateRoot)).toBe(block.stateRoot);
  expect(header.toJSON()).toMatchObject({ number: '0xa', stateRoot: block.stateRoot });
  expect(header.serialize()).toEqual(createBlockHeaderFromRPC(block).serialize());
  expect(bytesToHex(header.hash())).toBe(block.hash);
});

it('requires an execution source only for hash-only responses', async () => {
  const { payload, header } = historicalHeader(false);
  await expect(
    buildExecutionHeaderProof({
      beacon: header.beacon,
      execution: payload,
      execution_branch: header.execution_branch.slice(-4),
    }),
  ).resolves.toMatchObject({ executionHeader: { Capella: { blockHash: payload.block_hash } } });
  await expect(buildExecutionHeaderProof(header, undefined, spec)).rejects.toThrow(
    'execution source',
  );
});

it.each([false, true])(
  'normalizes historical hash-only headers by slot (Deneb=%s)',
  async isDeneb => {
    const { payload, header, block, outer, rlp } = historicalHeader(isDeneb);
    const executionClient = createExecutionClient({ blocks: [block] });
    const proof = await buildExecutionHeaderProof(header, { executionClient }, spec);
    const oldProof = await buildExecutionHeaderProof(
      { beacon: header.beacon, execution: payload, execution_branch: outer },
      undefined,
      spec,
    );
    expect(Object.keys(oldProof.executionHeader)).toEqual([isDeneb ? 'Deneb' : 'Capella']);
    expect(oldProof.executionBranch).toEqual(outer);
    expect(proof).toEqual({
      executionHeader: { Rlp: rlp },
      executionBranch: header.execution_branch.slice(isDeneb ? 2 : 3),
    });
    const badPadding = [...header.execution_branch];
    badPadding[0] = repeatHex('ff', 32);
    await expect(
      buildExecutionHeaderProof(
        { ...header, execution_branch: badPadding },
        { executionClient },
        spec,
      ),
    ).rejects.toThrow('padding');
    const wrongStateRoot = { ...block, stateRoot: repeatHex('ff', 32) };
    await expect(
      buildExecutionHeaderProof(
        header,
        { executionClient: createExecutionClient({ blocks: [wrongStateRoot] }) },
        spec,
      ),
    ).rejects.toThrow('hash mismatch');
  },
);

it('uses the Capella slot layout even when an upgraded full payload includes Deneb fields', async () => {
  const { payload, header } = historicalHeader(false);
  const proof = await buildExecutionHeaderProof(
    {
      beacon: header.beacon,
      execution: { ...payload, blob_gas_used: '0', excess_blob_gas: '0' },
      execution_branch: [],
    },
    undefined,
    spec,
  );
  expect(Object.keys(proof.executionHeader)).toEqual(['Capella']);
});

it.each(['0', '7'])('rejects hash-only execution anchors before Capella (slot %s)', async slot => {
  const { header, block } = historicalHeader(false);
  await expect(
    buildExecutionHeaderProof(
      { ...header, beacon: { ...header.beacon, slot } },
      { executionClient: createExecutionClient({ blocks: [block] }) },
      spec,
    ),
  ).rejects.toThrow(slot === '0' ? 'Empty light-client' : 'Capella or later');
});

it('removes only zero state-proof padding using the attested slot fork', () => {
  const schedule = { ...spec, ELECTRA_FORK_EPOCH: '2' };
  const committee = Array<string>(6).fill(repeatHex('01', 32));
  expect(
    normalizeBeaconStateBranch([...Array(5).fill(zero), ...committee], '16', schedule, 'committee'),
  ).toEqual(committee);
  const finality = [...committee, repeatHex('02', 32)];
  expect(normalizeBeaconStateBranch([zero, zero, ...finality], '16', schedule, 'finality')).toEqual(
    finality,
  );
  expect(() =>
    normalizeBeaconStateBranch([repeatHex('ff', 32), ...committee], '16', schedule, 'committee'),
  ).toThrow('padding');
  expect(normalizeBeaconStateBranch(committee, '24', schedule, 'committee')).toBe(committee);
});

function historicalHeader(isDeneb: boolean) {
  const fixture = historicalProofs[isDeneb ? 1 : 0];
  const rlp = hexToBytes(fixture.execution_header_rlp as Hex);
  const block = createExecutionBlockFromRlp(
    fixture.execution_header_rlp as Hex,
    isDeneb ? Hardfork.Cancun : Hardfork.Shanghai,
  );
  const payload: EthereumLightClientExecutionPayloadHeader = fixture.payload_header;
  const header: EthereumLightClientHeader = {
    beacon: {
      ...fixture.header,
      slot: String(fixture.header.slot),
      proposer_index: String(fixture.header.proposer_index),
    },
    execution_block_hash: block.hash,
    execution_branch: [
      ...Array<string>(11 - fixture.execution_branch.length).fill(zero),
      ...fixture.execution_branch,
    ],
  };
  return { block, payload, header, outer: fixture.payload_branch, rlp };
}
