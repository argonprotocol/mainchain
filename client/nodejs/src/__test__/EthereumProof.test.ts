import { expect, it } from 'vitest';
import { createMPT, verifyMPTWithMerkleProof } from '@ethereumjs/mpt';
import { bytesToHex, hexToBytes, type Hex } from 'viem';
import { encodeEthereumReceiptForProof, encodeReceiptTrieKey } from '../EthereumProof';
import productionInboundReceipt from './fixtures/productionInboundReceipt.json';

type EncodedReceiptInput = Parameters<typeof encodeEthereumReceiptForProof>[0];

it('encodes a production inbound receipt exactly as retained proof expects', async () => {
  const receipt = {
    ...productionInboundReceipt.receipt,
    cumulativeGasUsed: BigInt(productionInboundReceipt.receipt.cumulativeGasUsed),
  } as EncodedReceiptInput;

  expect(bytesToHex(encodeReceiptTrieKey(productionInboundReceipt.transactionIndex))).toBe(
    productionInboundReceipt.encodedTrieKey,
  );
  expect(bytesToHex(encodeEthereumReceiptForProof(receipt))).toBe(
    productionInboundReceipt.encodedReceipt,
  );

  const recoveredReceipt = await verifyMPTWithMerkleProof(
    await createMPT(),
    hexToBytes(productionInboundReceipt.receiptsRoot as Hex),
    encodeReceiptTrieKey(productionInboundReceipt.transactionIndex),
    productionInboundReceipt.receiptProofNodes.map(hex => hexToBytes(hex as Hex)),
  );

  expect(bytesToHex(recoveredReceipt!)).toBe(productionInboundReceipt.encodedReceipt);
});
