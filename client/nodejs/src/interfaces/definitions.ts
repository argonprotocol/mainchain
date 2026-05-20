export const ethereum = {
  types: {
    ArgonPrimitivesEthereumEthereumExecutionHeader: {
      rlp: 'Bytes',
    },
    ArgonPrimitivesEthereumEthereumExecutionBlockProof: {
      anchorBlockHash: 'H256',
      targetToAnchorHeaderChain: 'Vec<ArgonPrimitivesEthereumEthereumExecutionHeader>',
    },
    ArgonPrimitivesEthereumEthereumLog: {
      address: 'H160',
      topics: 'Vec<H256>',
      data: 'Bytes',
    },
    ArgonPrimitivesEthereumEthereumProof: {
      executionBlockProof: 'ArgonPrimitivesEthereumEthereumExecutionBlockProof',
      receiptProof: 'ArgonPrimitivesEthereumEthereumCombinedReceiptProof',
    },
    ArgonPrimitivesEthereumEthereumCombinedReceiptProof: {
      nodes: 'Vec<Bytes>',
      receipts: 'Vec<ArgonPrimitivesEthereumEthereumReceiptProofReceipt>',
    },
    ArgonPrimitivesEthereumEthereumReceiptProofReceipt: {
      transactionIndex: 'Compact<u64>',
      nodeIndexes: 'Vec<u16>',
    },
    ArgonPrimitivesEthereumEthereumReceiptLog: {
      transactionIndex: 'Compact<u64>',
      eventLog: 'ArgonPrimitivesEthereumEthereumLog',
    },
    ArgonPrimitivesEthereumEthereumVerifyError: {
      _enum: [
        'VerifierUnavailable',
        'AnchorNotFound',
        'InvalidHeader',
        'InvalidHeaderChain',
        'LogNotFound',
        'InvalidProof',
      ],
    },
  },
};
