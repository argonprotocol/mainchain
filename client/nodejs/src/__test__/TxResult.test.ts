import { describe, expect, it } from 'vitest';
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
});

function createTxResult(): TxResult {
  return new TxResult({} as any, {
    signedHash: '0xtx',
    method: {},
    accountAddress: '5Submitter',
    submittedTime: new Date('2026-06-23T00:00:00.000Z'),
    submittedAtBlockNumber: 123,
    nonce: 7,
  });
}
