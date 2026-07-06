import { u8aEq } from '@polkadot/util';
import { describe, expect, it, vi } from 'vitest';
import { TxResult } from '../TxResult';
import { TxSubmissionErrorCode } from '../utils';

describe('TxResult', () => {
  it('rejects pending waits when the node drops the transaction', async () => {
    const result = createTxResult();

    result.onSubscriptionResult({
      events: [],
      isFinalized: false,
      status: {
        isBroadcast: false,
        isDropped: true,
        isInBlock: false,
        isInvalid: false,
        isUsurped: false,
      },
      txIndex: undefined,
    } as any);

    await expect(result.waitForInFirstBlock).rejects.toMatchObject({
      errorCode: TxSubmissionErrorCode.Dropped,
    });
    await expect(result.waitForFinalizedBlock).rejects.toMatchObject({
      errorCode: TxSubmissionErrorCode.Dropped,
    });
  });

  it('includes the replacement hash when the transaction is usurped', async () => {
    const result = createTxResult();

    result.onSubscriptionResult({
      events: [],
      isFinalized: false,
      status: {
        isBroadcast: false,
        asUsurped: {
          toHex: () => '0xreplacement',
        },
        isDropped: false,
        isInBlock: false,
        isInvalid: false,
        isUsurped: true,
      },
      txIndex: undefined,
    } as any);

    await expect(result.waitForInFirstBlock).rejects.toMatchObject({
      errorCode: TxSubmissionErrorCode.Usurped,
      message: expect.stringContaining('0xreplacement'),
    });
    await expect(result.waitForFinalizedBlock).rejects.toMatchObject({
      errorCode: TxSubmissionErrorCode.Usurped,
      message: expect.stringContaining('0xreplacement'),
    });
  });

  it('waits to publish in-block until it can resolve a real finalized block', async () => {
    const getHeader = vi
      .fn()
      .mockRejectedValueOnce(new Error('Unable to retrieve header and parent from supplied hash'))
      .mockResolvedValue({
        number: {
          toNumber: () => 145,
        },
      });
    const result = createTxResult({
      rpc: {
        chain: {
          getHeader,
        },
      },
    } as any);

    let inBlockHash: Uint8Array | undefined;
    void result.waitForInFirstBlock.then(hash => {
      inBlockHash = hash;
    });

    result.onSubscriptionResult({
      events: [],
      isFinalized: false,
      status: {
        isBroadcast: false,
        isDropped: false,
        isInBlock: true,
        asInBlock: Uint8Array.from([1, 2, 3]),
        isInvalid: false,
        isUsurped: false,
      },
      txIndex: 4,
    } as any);

    await Promise.resolve();

    expect(getHeader).toHaveBeenCalledTimes(1);
    expect(inBlockHash).toBeUndefined();
    expect(result.blockHash).toBeUndefined();
    expect(result.blockNumber).toBeUndefined();

    result.onSubscriptionResult({
      events: [],
      isFinalized: true,
      status: {
        isBroadcast: false,
        isDropped: false,
        isFinalized: true,
        asFinalized: Uint8Array.from([4, 5, 6]),
        isInBlock: false,
        isInvalid: false,
        isUsurped: false,
      },
      txIndex: 4,
    } as any);

    await expect(result.waitForInFirstBlock).resolves.toEqual(Uint8Array.from([4, 5, 6]));
    await expect(result.waitForFinalizedBlock).resolves.toEqual(Uint8Array.from([4, 5, 6]));
    expect(result.blockHash).toEqual(Uint8Array.from([4, 5, 6]));
    expect(result.blockNumber).toBe(145);
    expect(result.extrinsicIndex).toBe(4);
  });

  it('ignores a stale in-block lookup after finalized wins', async () => {
    const inBlockHash = Uint8Array.from([1, 2, 3]);
    const finalizedHash = Uint8Array.from([4, 5, 6]);
    let resolveInBlockHeader!: (value: unknown) => void;
    let resolveFinalizedHeader!: (value: unknown) => void;
    const getHeader = vi.fn().mockImplementation((hash: Uint8Array) => {
      if (u8aEq(hash, inBlockHash)) {
        return new Promise(resolve => {
          resolveInBlockHeader = resolve;
        });
      }

      if (u8aEq(hash, finalizedHash)) {
        return new Promise(resolve => {
          resolveFinalizedHeader = resolve;
        });
      }

      throw new Error('Unexpected hash');
    });
    const result = createTxResult({
      rpc: {
        chain: {
          getHeader,
        },
      },
    } as any);

    result.onSubscriptionResult({
      events: [],
      isFinalized: false,
      status: {
        isBroadcast: false,
        isDropped: false,
        isInBlock: true,
        asInBlock: inBlockHash,
        isInvalid: false,
        isUsurped: false,
      },
      txIndex: 4,
    } as any);

    await Promise.resolve();

    result.onSubscriptionResult({
      events: [],
      isFinalized: true,
      status: {
        isBroadcast: false,
        isDropped: false,
        isFinalized: true,
        asFinalized: finalizedHash,
        isInBlock: false,
        isInvalid: false,
        isUsurped: false,
      },
      txIndex: 4,
    } as any);

    await Promise.resolve();

    resolveFinalizedHeader({
      number: {
        toNumber: () => 145,
      },
    });

    await expect(result.waitForInFirstBlock).resolves.toEqual(finalizedHash);
    await expect(result.waitForFinalizedBlock).resolves.toEqual(finalizedHash);
    expect(result.blockHash).toEqual(finalizedHash);
    expect(result.blockNumber).toBe(145);

    resolveInBlockHeader({
      number: {
        toNumber: () => 144,
      },
    });

    await Promise.resolve();
    await Promise.resolve();

    expect(result.blockHash).toEqual(finalizedHash);
    expect(result.blockNumber).toBe(145);
  });
});

function createTxResult(client: Partial<ConstructorParameters<typeof TxResult>[0]> = {}): TxResult {
  return new TxResult(client as any, {
    signedHash: '0xtx',
    method: {},
    accountAddress: '5Submitter',
    submittedTime: new Date('2026-06-23T00:00:00.000Z'),
    submittedAtBlockNumber: 123,
    nonce: 7,
  });
}
