export const ethereum = {
  runtime: {
    EthereumApis: [
      {
        methods: {
          verify_event_log: {
            description: 'Preflight verify an Ethereum event log proof.',
            params: [
              {
                name: 'eventLog',
                type: 'ArgonPrimitivesEthereumEthereumLog',
              },
              {
                name: 'proof',
                type: 'ArgonPrimitivesEthereumEthereumProof',
              },
            ],
            type: 'Result<Null, ArgonPrimitivesEthereumEthereumVerifyError>',
          },
        },
        version: 1,
      },
    ],
  },
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
      receiptProof: 'ArgonPrimitivesEthereumEthereumReceiptProof',
    },
    ArgonPrimitivesEthereumEthereumReceiptProof: {
      transactionIndex: 'Compact<u64>',
      nodes: 'Vec<Bytes>',
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
