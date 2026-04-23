import { TxSubmitter } from '../TxSubmitter';
import { expect, it, vi } from 'vitest';

it('supports external signers via address+signer', async () => {
  const address = '5G9v3eN9y1xS7G8fFqL3J4d4G2fG1m3wZ7kH9n1Y2x3p4q5r';
  const signer = { signPayload: vi.fn() } as any;
  const txProgressCallback = vi.fn();

  const signedTx = {
    hash: { toHex: () => '0xdeadbeef' },
    method: { toHuman: () => ({}) },
    nonce: { toNumber: () => 7 },
    send: vi.fn().mockResolvedValue(undefined),
  } as any;

  const tx = {
    paymentInfo: vi.fn().mockResolvedValue({
      partialFee: { toBigInt: () => 12n },
    }),
    signAsync: vi.fn().mockResolvedValue(signedTx),
  } as any;

  const client = {
    rpc: {
      system: {
        accountNextIndex: vi.fn().mockResolvedValue(99),
      },
      chain: {
        getHeader: vi.fn().mockResolvedValue({ number: { toNumber: () => 123 } }),
      },
    },
  } as any;

  const submitter = new TxSubmitter(client, tx, { address, signer });

  await expect(submitter.feeEstimate(0n)).resolves.toBe(12n);
  expect(tx.paymentInfo).toHaveBeenCalledWith(address, { tip: 0n });

  const result = await submitter.submit({
    useLatestNonce: true,
    disableAutomaticTxTracking: true,
    txProgressCallback,
  });
  expect(client.rpc.system.accountNextIndex).toHaveBeenCalledWith(address);
  expect(tx.signAsync).toHaveBeenCalledWith(
    address,
    expect.objectContaining({
      nonce: 99,
      signer,
    }),
  );
  expect(result.extrinsic.accountAddress).toBe(address);
  expect(result.txProgressCallback).toBe(txProgressCallback);
  expect(signedTx.send).toHaveBeenCalledTimes(1);

  client.rpc.system.accountNextIndex.mockClear();
  tx.signAsync.mockClear();

  await submitter.sign({ useLatestNonce: true, nonce: 0 });
  expect(client.rpc.system.accountNextIndex).not.toHaveBeenCalled();
  expect(tx.signAsync).toHaveBeenCalledWith(
    address,
    expect.objectContaining({
      nonce: 0,
      signer,
    }),
  );
});
