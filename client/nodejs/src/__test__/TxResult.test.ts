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

  it('keeps watching after an in-block result without a transaction index', async () => {
    const reIncludedHash = Uint8Array.from([4, 5, 6]);
    const getHeader = vi.fn().mockResolvedValue({
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

    result.onSubscriptionResult({
      events: [],
      isFinalized: false,
      status: {
        isBroadcast: false,
        isDropped: false,
        isInBlock: true,
        asInBlock: Uint8Array.from([1, 2, 3]),
        isInvalid: false,
        isRetracted: false,
        isUsurped: false,
      },
      txIndex: undefined,
    } as any);

    expect(result.submissionError).toBeUndefined();
    expect(getHeader).not.toHaveBeenCalled();

    result.onSubscriptionResult({
      events: [],
      isFinalized: false,
      status: {
        isBroadcast: false,
        isDropped: false,
        isInBlock: true,
        asInBlock: reIncludedHash,
        isInvalid: false,
        isRetracted: false,
        isUsurped: false,
      },
      txIndex: 4,
    } as any);

    await expect(result.waitForInFirstBlock).resolves.toEqual(reIncludedHash);

    result.onSubscriptionResult({
      events: [],
      isFinalized: true,
      status: {
        isBroadcast: false,
        isDropped: false,
        isFinalized: true,
        asFinalized: reIncludedHash,
        isInBlock: false,
        isInvalid: false,
        isRetracted: false,
        isUsurped: false,
      },
      txIndex: 4,
    } as any);

    await expect(result.waitForFinalizedBlock).resolves.toEqual(reIncludedHash);
    expect(result.submissionError).toBeUndefined();
    expect(result.isFinalized).toBe(true);
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

  it('ignores an in-block lookup after the transaction is retracted', async () => {
    const inBlockHash = Uint8Array.from([1, 2, 3]);
    let resolveInBlockHeader!: (value: unknown) => void;
    const getHeader = vi.fn().mockImplementation(
      () =>
        new Promise(resolve => {
          resolveInBlockHeader = resolve;
        }),
    );
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
        isRetracted: false,
        isUsurped: false,
      },
      txIndex: 4,
    } as any);

    await Promise.resolve();

    result.onSubscriptionResult({
      events: [],
      isFinalized: false,
      status: {
        isBroadcast: false,
        isDropped: false,
        isInBlock: false,
        isInvalid: false,
        isRetracted: true,
        asRetracted: inBlockHash,
        isUsurped: false,
      },
      txIndex: undefined,
    } as any);

    resolveInBlockHeader({
      number: {
        toNumber: () => 144,
      },
    });

    await Promise.resolve();
    await Promise.resolve();

    expect(result.blockHash).toBeUndefined();
    expect(result.blockNumber).toBeUndefined();
  });

  it('clears a published block inclusion when the transaction is retracted', async () => {
    const inBlockHash = Uint8Array.from([1, 2, 3]);
    const result = createTxResult({
      rpc: {
        chain: {
          getHeader: vi.fn().mockResolvedValue({
            number: {
              toNumber: () => 144,
            },
          }),
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
        isRetracted: false,
        isUsurped: false,
      },
      txIndex: 4,
    } as any);

    await expect(result.waitForInFirstBlock).resolves.toEqual(inBlockHash);
    expect(result.blockNumber).toBe(144);
    expect(result.extrinsicIndex).toBe(4);
    expect(result.txProgress).toBe(99);

    result.onSubscriptionResult({
      events: [],
      isFinalized: false,
      status: {
        isBroadcast: false,
        isDropped: false,
        isInBlock: false,
        isInvalid: false,
        isRetracted: true,
        asRetracted: inBlockHash,
        isUsurped: false,
      },
      txIndex: undefined,
    } as any);

    expect(result.blockHash).toBeUndefined();
    expect(result.blockNumber).toBeUndefined();
    expect(result.extrinsicIndex).toBeUndefined();
    expect(result.txProgress).toBe(10);
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
