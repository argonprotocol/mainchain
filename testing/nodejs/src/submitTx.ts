import {
  type ArgonClient,
  checkForExtrinsicSuccess,
  type GenericEvent,
  type KeyringPair,
  type SubmittableExtrinsic,
} from '@argonprotocol/mainchain';

export type TestTxResult = {
  blockHash: string;
  events: GenericEvent[];
  finalFee?: bigint;
};

/** Sign a test transaction and resolve once it is included successfully in a block. */
export function submitTx(
  client: ArgonClient,
  tx: SubmittableExtrinsic,
  signer: KeyringPair,
): Promise<TestTxResult> {
  return new Promise((resolve, reject) => {
    let unsubscribe: (() => void) | undefined;
    let completed = false;

    const finish = (callback: () => void) => {
      completed = true;
      unsubscribe?.();
      callback();
    };

    tx.signAndSend(signer, result => {
      if (completed) return;

      if (result.status.isDropped || result.status.isInvalid || result.status.isUsurped) {
        finish(() => reject(new Error(`Transaction ${result.status.type.toLowerCase()}`)));
        return;
      }

      if (!result.status.isInBlock) return;

      void checkForExtrinsicSuccess(result.events, client).then(
        () => {
          const feePaid = result.events.find(({ event }) =>
            client.events.transactionPayment.TransactionFeePaid.is(event),
          );

          finish(() =>
            resolve({
              blockHash: result.status.asInBlock.toHex(),
              events: result.events.map(({ event }) => event),
              finalFee:
                feePaid && client.events.transactionPayment.TransactionFeePaid.is(feePaid.event)
                  ? feePaid.event.data.actualFee.toBigInt()
                  : undefined,
            }),
          );
        },
        error => finish(() => reject(error)),
      );
    })
      .then(stop => {
        unsubscribe = stop;
        if (completed) unsubscribe();
      })
      .catch(error => finish(() => reject(error)));
  });
}
