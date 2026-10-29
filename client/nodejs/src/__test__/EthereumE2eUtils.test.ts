import { afterEach, expect, it, vi } from 'vitest';
import { waitForExecutionReceipt } from '@argonprotocol/testing';
import type { RpcTransactionReceipt } from 'viem';
import { repeatHex } from './ethereumProofTestUtils';

afterEach(() => vi.useRealTimers());

it('waits through indexing and pending receipts, then returns a typed execution receipt', async () => {
  vi.useFakeTimers();
  const receipt: RpcTransactionReceipt = {
    blockHash: repeatHex('11', 32),
    blockNumber: '0xa',
    contractAddress: null,
    cumulativeGasUsed: '0x5208',
    effectiveGasPrice: '0x1',
    from: repeatHex('22', 20),
    gasUsed: '0x5208',
    logs: [],
    logsBloom: repeatHex('00', 256),
    status: '0x1',
    to: repeatHex('33', 20),
    transactionHash: repeatHex('44', 32),
    transactionIndex: '0x0',
    type: '0x2',
  };
  const ethereum = {
    callExecution: vi
      .fn()
      .mockRejectedValueOnce(new Error('transaction indexing is in progress'))
      .mockResolvedValueOnce(null)
      .mockResolvedValueOnce(receipt),
  };
  const pending = waitForExecutionReceipt(ethereum, receipt.transactionHash);
  await vi.runAllTimersAsync();
  const result = await pending;
  expect(result.status).toBe('success');
  expect(result.blockNumber).toBe(10n);
  expect(result.transactionIndex).toBe(0);
  expect(result.gasUsed).toBe(21_000n);
  expect(result.type).toBe('eip1559');
});

it('does not hide permanent execution RPC errors as indexing delays', async () => {
  const ethereum = { callExecution: vi.fn().mockRejectedValue(new Error('Method not found')) };
  await expect(waitForExecutionReceipt(ethereum, repeatHex('44', 32))).rejects.toThrow(
    'Method not found',
  );
});
