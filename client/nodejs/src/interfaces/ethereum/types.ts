// Auto-generated via `yarn polkadot-types-from-defs`, do not edit
/* eslint-disable */

import type { Bytes, Compact, Enum, Struct, Vec, u64 } from '@polkadot/types-codec';
import type { H160, H256 } from '@polkadot/types/interfaces/runtime';

/** @name ArgonPrimitivesEthereumEthereumExecutionBlockProof */
export interface ArgonPrimitivesEthereumEthereumExecutionBlockProof extends Struct {
  readonly anchorBlockHash: H256;
  readonly targetToAnchorHeaderChain: Vec<ArgonPrimitivesEthereumEthereumExecutionHeader>;
}

/** @name ArgonPrimitivesEthereumEthereumExecutionHeader */
export interface ArgonPrimitivesEthereumEthereumExecutionHeader extends Struct {
  readonly rlp: Bytes;
}

/** @name ArgonPrimitivesEthereumEthereumLog */
export interface ArgonPrimitivesEthereumEthereumLog extends Struct {
  readonly address: H160;
  readonly topics: Vec<H256>;
  readonly data: Bytes;
}

/** @name ArgonPrimitivesEthereumEthereumProof */
export interface ArgonPrimitivesEthereumEthereumProof extends Struct {
  readonly executionBlockProof: ArgonPrimitivesEthereumEthereumExecutionBlockProof;
  readonly receiptProof: ArgonPrimitivesEthereumEthereumReceiptProof;
}

/** @name ArgonPrimitivesEthereumEthereumReceiptProof */
export interface ArgonPrimitivesEthereumEthereumReceiptProof extends Struct {
  readonly transactionIndex: Compact<u64>;
  readonly nodes: Vec<Bytes>;
}

/** @name ArgonPrimitivesEthereumEthereumVerifyError */
export interface ArgonPrimitivesEthereumEthereumVerifyError extends Enum {
  readonly isVerifierUnavailable: boolean;
  readonly isAnchorNotFound: boolean;
  readonly isInvalidHeader: boolean;
  readonly isInvalidHeaderChain: boolean;
  readonly isLogNotFound: boolean;
  readonly isInvalidProof: boolean;
  readonly type: 'VerifierUnavailable' | 'AnchorNotFound' | 'InvalidHeader' | 'InvalidHeaderChain' | 'LogNotFound' | 'InvalidProof';
}

export type PHANTOM_ETHEREUM = 'ethereum';
