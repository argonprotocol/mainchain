// Auto-generated via `yarn polkadot-types-from-defs`, do not edit
/* eslint-disable */

// import type lookup before we augment - in some environments
// this is required to allow for ambient/previous definitions
import '@polkadot/types/types/registry';

import type {
  ArgonNotaryAuditAccountHistoryLookupError,
  ArgonNotaryAuditErrorVerifyError,
  ArgonPrimitivesAccountAccountType,
  ArgonPrimitivesBalanceChangeAccountOrigin,
  ArgonPrimitivesBalanceChangeMerkleProof,
  ArgonPrimitivesBitcoinBitcoinBlock,
  ArgonPrimitivesBitcoinBitcoinCosignScriptPubkey,
  ArgonPrimitivesBitcoinBitcoinNetwork,
  ArgonPrimitivesBitcoinBitcoinRejectedReason,
  ArgonPrimitivesBitcoinBitcoinXPub,
  ArgonPrimitivesBitcoinCompressedBitcoinPubkey,
  ArgonPrimitivesBitcoinH256Le,
  ArgonPrimitivesBitcoinNetworkKind,
  ArgonPrimitivesBitcoinOpaqueBitcoinXpub,
  ArgonPrimitivesBitcoinUtxoRef,
  ArgonPrimitivesBitcoinUtxoValue,
  ArgonPrimitivesBlockSealAppPublic,
  ArgonPrimitivesBlockSealBlockPayout,
  ArgonPrimitivesBlockSealBlockRewardType,
  ArgonPrimitivesBlockSealMiningBidStats,
  ArgonPrimitivesBlockSealMiningRegistration,
  ArgonPrimitivesBlockSealMiningSlotConfig,
  ArgonPrimitivesBlockVoteBlockVoteT,
  ArgonPrimitivesDigestsBlockVoteDigest,
  ArgonPrimitivesDigestsDigestset,
  ArgonPrimitivesDigestsFrameInfo,
  ArgonPrimitivesDigestsNotebookDigest,
  ArgonPrimitivesDigestsParentVotingKeyDigest,
  ArgonPrimitivesDomainSemver,
  ArgonPrimitivesDomainVersionHost,
  ArgonPrimitivesDomainZoneRecord,
  ArgonPrimitivesForkPower,
  ArgonPrimitivesInherentsBitcoinUtxoFunding,
  ArgonPrimitivesInherentsBitcoinUtxoSync,
  ArgonPrimitivesInherentsBlockSealInherent,
  ArgonPrimitivesNotaryNotaryMeta,
  ArgonPrimitivesNotaryNotaryNotebookKeyDetails,
  ArgonPrimitivesNotaryNotaryNotebookVoteDigestDetails,
  ArgonPrimitivesNotaryNotaryRecord,
  ArgonPrimitivesNotebookChainTransfer,
  ArgonPrimitivesNotebookNotebookAuditResult,
  ArgonPrimitivesNotebookNotebookHeader,
  ArgonPrimitivesNotebookSignedNotebookHeader,
  ArgonPrimitivesProvidersBlockSealerInfo,
  ArgonPrimitivesTickTicker,
  ArgonPrimitivesVault,
  ArgonPrimitivesVaultVaultError,
  ArgonPrimitivesVaultVaultTerms,
  ArgonRuntimeOriginCaller,
  ArgonRuntimeProxyType,
  ArgonRuntimeRuntime,
  ArgonRuntimeRuntimeFreezeReason,
  ArgonRuntimeRuntimeHoldReason,
  ArgonRuntimeSessionKeys,
  FinalityGrandpaEquivocationPrecommit,
  FinalityGrandpaEquivocationPrevote,
  FinalityGrandpaPrecommit,
  FinalityGrandpaPrevote,
  FrameMetadataHashExtensionCheckMetadataHash,
  FrameMetadataHashExtensionMode,
  FrameSupportDispatchDispatchClass,
  FrameSupportDispatchPays,
  FrameSupportDispatchPerDispatchClassU32,
  FrameSupportDispatchPerDispatchClassWeight,
  FrameSupportDispatchPerDispatchClassWeightsPerClass,
  FrameSupportDispatchRawOrigin,
  FrameSupportPalletId,
  FrameSupportTokensMiscBalanceStatus,
  FrameSupportTokensMiscIdAmountRuntimeFreezeReason,
  FrameSupportTokensMiscIdAmountRuntimeHoldReason,
  FrameSystemAccountInfo,
  FrameSystemCall,
  FrameSystemCodeUpgradeAuthorization,
  FrameSystemDispatchEventInfo,
  FrameSystemError,
  FrameSystemEvent,
  FrameSystemEventRecord,
  FrameSystemExtensionsAuthorizeCall,
  FrameSystemExtensionsCheckGenesis,
  FrameSystemExtensionsCheckNonZeroSender,
  FrameSystemExtensionsCheckNonce,
  FrameSystemExtensionsCheckSpecVersion,
  FrameSystemExtensionsCheckTxVersion,
  FrameSystemExtensionsCheckWeight,
  FrameSystemExtensionsWeightReclaim,
  FrameSystemLastRuntimeUpgradeInfo,
  FrameSystemLimitsBlockLength,
  FrameSystemLimitsBlockWeights,
  FrameSystemLimitsWeightsPerClass,
  FrameSystemPhase,
  IsmpConsensusStateCommitment,
  IsmpConsensusStateMachineHeight,
  IsmpConsensusStateMachineId,
  IsmpEventsRequestResponseHandled,
  IsmpEventsTimeoutHandled,
  IsmpGrandpaAddStateMachine,
  IsmpGrandpaCall,
  IsmpGrandpaEvent,
  IsmpHostStateMachine,
  IsmpMessagingConsensusMessage,
  IsmpMessagingCreateConsensusState,
  IsmpMessagingFraudProofMessage,
  IsmpMessagingMessage,
  IsmpMessagingProof,
  IsmpMessagingRequestMessage,
  IsmpMessagingResponseMessage,
  IsmpMessagingStateCommitmentHeight,
  IsmpMessagingTimeoutMessage,
  IsmpRouterGetRequest,
  IsmpRouterGetResponse,
  IsmpRouterPostRequest,
  IsmpRouterPostResponse,
  IsmpRouterRequest,
  IsmpRouterRequestResponse,
  IsmpRouterResponse,
  IsmpRouterStorageValue,
  PalletBalancesAccountData,
  PalletBalancesAdjustmentDirection,
  PalletBalancesBalanceLock,
  PalletBalancesCall,
  PalletBalancesError,
  PalletBalancesEvent,
  PalletBalancesReasons,
  PalletBalancesReserveData,
  PalletBitcoinLocksCall,
  PalletBitcoinLocksError,
  PalletBitcoinLocksEvent,
  PalletBitcoinLocksFeeCoupon,
  PalletBitcoinLocksFeeCouponProof,
  PalletBitcoinLocksHoldReason,
  PalletBitcoinLocksLockOptions,
  PalletBitcoinLocksLockReleaseRequest,
  PalletBitcoinLocksLockedBitcoin,
  PalletBitcoinLocksOrphanedUtxo,
  PalletBitcoinLocksOrphanedUtxoCosignRequest,
  PalletBitcoinUtxosCall,
  PalletBitcoinUtxosError,
  PalletBitcoinUtxosEvent,
  PalletBlockRewardsCall,
  PalletBlockRewardsError,
  PalletBlockRewardsEvent,
  PalletBlockRewardsFreezeReason,
  PalletBlockRewardsHoldReason,
  PalletBlockSealCall,
  PalletBlockSealError,
  PalletBlockSealSpecCall,
  PalletBlockSealSpecError,
  PalletBlockSealSpecEvent,
  PalletChainTransferCall,
  PalletChainTransferError,
  PalletChainTransferEvent,
  PalletChainTransferQueuedTransferOut,
  PalletDigestsError,
  PalletDigestsEvent,
  PalletDomainsCall,
  PalletDomainsDomainRegistration,
  PalletDomainsError,
  PalletDomainsEvent,
  PalletFeeControlError,
  PalletFeeControlEvent,
  PalletGrandpaCall,
  PalletGrandpaError,
  PalletGrandpaEvent,
  PalletGrandpaStoredPendingChange,
  PalletGrandpaStoredState,
  PalletHyperbridgeError,
  PalletHyperbridgeEvent,
  PalletHyperbridgeSubstrateHostParams,
  PalletHyperbridgeVersionedHostParams,
  PalletIsmpCall,
  PalletIsmpError,
  PalletIsmpErrorsHandlingError,
  PalletIsmpEvent,
  PalletIsmpUtilsFundMessageParams,
  PalletIsmpUtilsMessageCommitment,
  PalletIsmpUtilsUpdateConsensusState,
  PalletMiningSlotCall,
  PalletMiningSlotError,
  PalletMiningSlotEvent,
  PalletMiningSlotHoldReason,
  PalletMiningSlotMinerNonceScoring,
  PalletMintCall,
  PalletMintError,
  PalletMintEvent,
  PalletMintMintAction,
  PalletMintMintType,
  PalletMultisigCall,
  PalletMultisigError,
  PalletMultisigEvent,
  PalletMultisigMultisig,
  PalletMultisigTimepoint,
  PalletNotariesCall,
  PalletNotariesError,
  PalletNotariesEvent,
  PalletNotebookCall,
  PalletNotebookError,
  PalletNotebookEvent,
  PalletPriceIndexCall,
  PalletPriceIndexCpiMeasurementBucket,
  PalletPriceIndexError,
  PalletPriceIndexEvent,
  PalletPriceIndexPriceIndex,
  PalletProxyAnnouncement,
  PalletProxyCall,
  PalletProxyDepositKind,
  PalletProxyError,
  PalletProxyEvent,
  PalletProxyProxyDefinition,
  PalletSudoCall,
  PalletSudoError,
  PalletSudoEvent,
  PalletTicksCall,
  PalletTicksError,
  PalletTimestampCall,
  PalletTokenGatewayAssetRegistration,
  PalletTokenGatewayCall,
  PalletTokenGatewayError,
  PalletTokenGatewayEvent,
  PalletTokenGatewayPrecisionUpdate,
  PalletTokenGatewayTeleportParams,
  PalletTransactionPaymentChargeTransactionPayment,
  PalletTransactionPaymentEvent,
  PalletTransactionPaymentReleases,
  PalletTreasuryCall,
  PalletTreasuryError,
  PalletTreasuryEvent,
  PalletTreasuryFunderState,
  PalletTreasuryHoldReason,
  PalletTreasuryTreasuryCapital,
  PalletTreasuryTreasuryPool,
  PalletTxPauseCall,
  PalletTxPauseError,
  PalletTxPauseEvent,
  PalletUtilityCall,
  PalletUtilityError,
  PalletUtilityEvent,
  PalletVaultsCall,
  PalletVaultsError,
  PalletVaultsEvent,
  PalletVaultsHoldReason,
  PalletVaultsVaultConfig,
  PalletVaultsVaultFrameRevenue,
  SpArithmeticArithmeticError,
  SpConsensusGrandpaAppPublic,
  SpConsensusGrandpaAppSignature,
  SpConsensusGrandpaEquivocation,
  SpConsensusGrandpaEquivocationProof,
  SpCoreVoid,
  SpRuntimeDigest,
  SpRuntimeDigestDigestItem,
  SpRuntimeDispatchError,
  SpRuntimeModuleError,
  SpRuntimeMultiSignature,
  SpRuntimeProvingTrieTrieError,
  SpRuntimeTokenError,
  SpRuntimeTransactionalError,
  SpVersionRuntimeVersion,
  SpWeightsRuntimeDbWeight,
  SpWeightsWeightV2Weight,
  TokenGatewayPrimitivesGatewayAssetRegistration,
  TokenGatewayPrimitivesGatewayAssetUpdate,
} from '@polkadot/types/lookup';

declare module '@polkadot/types/types/registry' {
  interface InterfaceTypes {
    ArgonNotaryAuditAccountHistoryLookupError: ArgonNotaryAuditAccountHistoryLookupError;
    ArgonNotaryAuditErrorVerifyError: ArgonNotaryAuditErrorVerifyError;
    ArgonPrimitivesAccountAccountType: ArgonPrimitivesAccountAccountType;
    ArgonPrimitivesBalanceChangeAccountOrigin: ArgonPrimitivesBalanceChangeAccountOrigin;
    ArgonPrimitivesBalanceChangeMerkleProof: ArgonPrimitivesBalanceChangeMerkleProof;
    ArgonPrimitivesBitcoinBitcoinBlock: ArgonPrimitivesBitcoinBitcoinBlock;
    ArgonPrimitivesBitcoinBitcoinCosignScriptPubkey: ArgonPrimitivesBitcoinBitcoinCosignScriptPubkey;
    ArgonPrimitivesBitcoinBitcoinNetwork: ArgonPrimitivesBitcoinBitcoinNetwork;
    ArgonPrimitivesBitcoinBitcoinRejectedReason: ArgonPrimitivesBitcoinBitcoinRejectedReason;
    ArgonPrimitivesBitcoinBitcoinXPub: ArgonPrimitivesBitcoinBitcoinXPub;
    ArgonPrimitivesBitcoinCompressedBitcoinPubkey: ArgonPrimitivesBitcoinCompressedBitcoinPubkey;
    ArgonPrimitivesBitcoinH256Le: ArgonPrimitivesBitcoinH256Le;
    ArgonPrimitivesBitcoinNetworkKind: ArgonPrimitivesBitcoinNetworkKind;
    ArgonPrimitivesBitcoinOpaqueBitcoinXpub: ArgonPrimitivesBitcoinOpaqueBitcoinXpub;
    ArgonPrimitivesBitcoinUtxoRef: ArgonPrimitivesBitcoinUtxoRef;
    ArgonPrimitivesBitcoinUtxoValue: ArgonPrimitivesBitcoinUtxoValue;
    ArgonPrimitivesBlockSealAppPublic: ArgonPrimitivesBlockSealAppPublic;
    ArgonPrimitivesBlockSealBlockPayout: ArgonPrimitivesBlockSealBlockPayout;
    ArgonPrimitivesBlockSealBlockRewardType: ArgonPrimitivesBlockSealBlockRewardType;
    ArgonPrimitivesBlockSealMiningBidStats: ArgonPrimitivesBlockSealMiningBidStats;
    ArgonPrimitivesBlockSealMiningRegistration: ArgonPrimitivesBlockSealMiningRegistration;
    ArgonPrimitivesBlockSealMiningSlotConfig: ArgonPrimitivesBlockSealMiningSlotConfig;
    ArgonPrimitivesBlockVoteBlockVoteT: ArgonPrimitivesBlockVoteBlockVoteT;
    ArgonPrimitivesDigestsBlockVoteDigest: ArgonPrimitivesDigestsBlockVoteDigest;
    ArgonPrimitivesDigestsDigestset: ArgonPrimitivesDigestsDigestset;
    ArgonPrimitivesDigestsFrameInfo: ArgonPrimitivesDigestsFrameInfo;
    ArgonPrimitivesDigestsNotebookDigest: ArgonPrimitivesDigestsNotebookDigest;
    ArgonPrimitivesDigestsParentVotingKeyDigest: ArgonPrimitivesDigestsParentVotingKeyDigest;
    ArgonPrimitivesDomainSemver: ArgonPrimitivesDomainSemver;
    ArgonPrimitivesDomainVersionHost: ArgonPrimitivesDomainVersionHost;
    ArgonPrimitivesDomainZoneRecord: ArgonPrimitivesDomainZoneRecord;
    ArgonPrimitivesForkPower: ArgonPrimitivesForkPower;
    ArgonPrimitivesInherentsBitcoinUtxoFunding: ArgonPrimitivesInherentsBitcoinUtxoFunding;
    ArgonPrimitivesInherentsBitcoinUtxoSync: ArgonPrimitivesInherentsBitcoinUtxoSync;
    ArgonPrimitivesInherentsBlockSealInherent: ArgonPrimitivesInherentsBlockSealInherent;
    ArgonPrimitivesNotaryNotaryMeta: ArgonPrimitivesNotaryNotaryMeta;
    ArgonPrimitivesNotaryNotaryNotebookKeyDetails: ArgonPrimitivesNotaryNotaryNotebookKeyDetails;
    ArgonPrimitivesNotaryNotaryNotebookVoteDigestDetails: ArgonPrimitivesNotaryNotaryNotebookVoteDigestDetails;
    ArgonPrimitivesNotaryNotaryRecord: ArgonPrimitivesNotaryNotaryRecord;
    ArgonPrimitivesNotebookChainTransfer: ArgonPrimitivesNotebookChainTransfer;
    ArgonPrimitivesNotebookNotebookAuditResult: ArgonPrimitivesNotebookNotebookAuditResult;
    ArgonPrimitivesNotebookNotebookHeader: ArgonPrimitivesNotebookNotebookHeader;
    ArgonPrimitivesNotebookSignedNotebookHeader: ArgonPrimitivesNotebookSignedNotebookHeader;
    ArgonPrimitivesProvidersBlockSealerInfo: ArgonPrimitivesProvidersBlockSealerInfo;
    ArgonPrimitivesTickTicker: ArgonPrimitivesTickTicker;
    ArgonPrimitivesVault: ArgonPrimitivesVault;
    ArgonPrimitivesVaultVaultError: ArgonPrimitivesVaultVaultError;
    ArgonPrimitivesVaultVaultTerms: ArgonPrimitivesVaultVaultTerms;
    ArgonRuntimeOriginCaller: ArgonRuntimeOriginCaller;
    ArgonRuntimeProxyType: ArgonRuntimeProxyType;
    ArgonRuntimeRuntime: ArgonRuntimeRuntime;
    ArgonRuntimeRuntimeFreezeReason: ArgonRuntimeRuntimeFreezeReason;
    ArgonRuntimeRuntimeHoldReason: ArgonRuntimeRuntimeHoldReason;
    ArgonRuntimeSessionKeys: ArgonRuntimeSessionKeys;
    FinalityGrandpaEquivocationPrecommit: FinalityGrandpaEquivocationPrecommit;
    FinalityGrandpaEquivocationPrevote: FinalityGrandpaEquivocationPrevote;
    FinalityGrandpaPrecommit: FinalityGrandpaPrecommit;
    FinalityGrandpaPrevote: FinalityGrandpaPrevote;
    FrameMetadataHashExtensionCheckMetadataHash: FrameMetadataHashExtensionCheckMetadataHash;
    FrameMetadataHashExtensionMode: FrameMetadataHashExtensionMode;
    FrameSupportDispatchDispatchClass: FrameSupportDispatchDispatchClass;
    FrameSupportDispatchPays: FrameSupportDispatchPays;
    FrameSupportDispatchPerDispatchClassU32: FrameSupportDispatchPerDispatchClassU32;
    FrameSupportDispatchPerDispatchClassWeight: FrameSupportDispatchPerDispatchClassWeight;
    FrameSupportDispatchPerDispatchClassWeightsPerClass: FrameSupportDispatchPerDispatchClassWeightsPerClass;
    FrameSupportDispatchRawOrigin: FrameSupportDispatchRawOrigin;
    FrameSupportPalletId: FrameSupportPalletId;
    FrameSupportTokensMiscBalanceStatus: FrameSupportTokensMiscBalanceStatus;
    FrameSupportTokensMiscIdAmountRuntimeFreezeReason: FrameSupportTokensMiscIdAmountRuntimeFreezeReason;
    FrameSupportTokensMiscIdAmountRuntimeHoldReason: FrameSupportTokensMiscIdAmountRuntimeHoldReason;
    FrameSystemAccountInfo: FrameSystemAccountInfo;
    FrameSystemCall: FrameSystemCall;
    FrameSystemCodeUpgradeAuthorization: FrameSystemCodeUpgradeAuthorization;
    FrameSystemDispatchEventInfo: FrameSystemDispatchEventInfo;
    FrameSystemError: FrameSystemError;
    FrameSystemEvent: FrameSystemEvent;
    FrameSystemEventRecord: FrameSystemEventRecord;
    FrameSystemExtensionsAuthorizeCall: FrameSystemExtensionsAuthorizeCall;
    FrameSystemExtensionsCheckGenesis: FrameSystemExtensionsCheckGenesis;
    FrameSystemExtensionsCheckNonZeroSender: FrameSystemExtensionsCheckNonZeroSender;
    FrameSystemExtensionsCheckNonce: FrameSystemExtensionsCheckNonce;
    FrameSystemExtensionsCheckSpecVersion: FrameSystemExtensionsCheckSpecVersion;
    FrameSystemExtensionsCheckTxVersion: FrameSystemExtensionsCheckTxVersion;
    FrameSystemExtensionsCheckWeight: FrameSystemExtensionsCheckWeight;
    FrameSystemExtensionsWeightReclaim: FrameSystemExtensionsWeightReclaim;
    FrameSystemLastRuntimeUpgradeInfo: FrameSystemLastRuntimeUpgradeInfo;
    FrameSystemLimitsBlockLength: FrameSystemLimitsBlockLength;
    FrameSystemLimitsBlockWeights: FrameSystemLimitsBlockWeights;
    FrameSystemLimitsWeightsPerClass: FrameSystemLimitsWeightsPerClass;
    FrameSystemPhase: FrameSystemPhase;
    IsmpConsensusStateCommitment: IsmpConsensusStateCommitment;
    IsmpConsensusStateMachineHeight: IsmpConsensusStateMachineHeight;
    IsmpConsensusStateMachineId: IsmpConsensusStateMachineId;
    IsmpEventsRequestResponseHandled: IsmpEventsRequestResponseHandled;
    IsmpEventsTimeoutHandled: IsmpEventsTimeoutHandled;
    IsmpGrandpaAddStateMachine: IsmpGrandpaAddStateMachine;
    IsmpGrandpaCall: IsmpGrandpaCall;
    IsmpGrandpaEvent: IsmpGrandpaEvent;
    IsmpHostStateMachine: IsmpHostStateMachine;
    IsmpMessagingConsensusMessage: IsmpMessagingConsensusMessage;
    IsmpMessagingCreateConsensusState: IsmpMessagingCreateConsensusState;
    IsmpMessagingFraudProofMessage: IsmpMessagingFraudProofMessage;
    IsmpMessagingMessage: IsmpMessagingMessage;
    IsmpMessagingProof: IsmpMessagingProof;
    IsmpMessagingRequestMessage: IsmpMessagingRequestMessage;
    IsmpMessagingResponseMessage: IsmpMessagingResponseMessage;
    IsmpMessagingStateCommitmentHeight: IsmpMessagingStateCommitmentHeight;
    IsmpMessagingTimeoutMessage: IsmpMessagingTimeoutMessage;
    IsmpRouterGetRequest: IsmpRouterGetRequest;
    IsmpRouterGetResponse: IsmpRouterGetResponse;
    IsmpRouterPostRequest: IsmpRouterPostRequest;
    IsmpRouterPostResponse: IsmpRouterPostResponse;
    IsmpRouterRequest: IsmpRouterRequest;
    IsmpRouterRequestResponse: IsmpRouterRequestResponse;
    IsmpRouterResponse: IsmpRouterResponse;
    IsmpRouterStorageValue: IsmpRouterStorageValue;
    PalletBalancesAccountData: PalletBalancesAccountData;
    PalletBalancesAdjustmentDirection: PalletBalancesAdjustmentDirection;
    PalletBalancesBalanceLock: PalletBalancesBalanceLock;
    PalletBalancesCall: PalletBalancesCall;
    PalletBalancesError: PalletBalancesError;
    PalletBalancesEvent: PalletBalancesEvent;
    PalletBalancesReasons: PalletBalancesReasons;
    PalletBalancesReserveData: PalletBalancesReserveData;
    PalletBitcoinLocksCall: PalletBitcoinLocksCall;
    PalletBitcoinLocksError: PalletBitcoinLocksError;
    PalletBitcoinLocksEvent: PalletBitcoinLocksEvent;
    PalletBitcoinLocksFeeCoupon: PalletBitcoinLocksFeeCoupon;
    PalletBitcoinLocksFeeCouponProof: PalletBitcoinLocksFeeCouponProof;
    PalletBitcoinLocksHoldReason: PalletBitcoinLocksHoldReason;
    PalletBitcoinLocksLockOptions: PalletBitcoinLocksLockOptions;
    PalletBitcoinLocksLockReleaseRequest: PalletBitcoinLocksLockReleaseRequest;
    PalletBitcoinLocksLockedBitcoin: PalletBitcoinLocksLockedBitcoin;
    PalletBitcoinLocksOrphanedUtxo: PalletBitcoinLocksOrphanedUtxo;
    PalletBitcoinLocksOrphanedUtxoCosignRequest: PalletBitcoinLocksOrphanedUtxoCosignRequest;
    PalletBitcoinUtxosCall: PalletBitcoinUtxosCall;
    PalletBitcoinUtxosError: PalletBitcoinUtxosError;
    PalletBitcoinUtxosEvent: PalletBitcoinUtxosEvent;
    PalletBlockRewardsCall: PalletBlockRewardsCall;
    PalletBlockRewardsError: PalletBlockRewardsError;
    PalletBlockRewardsEvent: PalletBlockRewardsEvent;
    PalletBlockRewardsFreezeReason: PalletBlockRewardsFreezeReason;
    PalletBlockRewardsHoldReason: PalletBlockRewardsHoldReason;
    PalletBlockSealCall: PalletBlockSealCall;
    PalletBlockSealError: PalletBlockSealError;
    PalletBlockSealSpecCall: PalletBlockSealSpecCall;
    PalletBlockSealSpecError: PalletBlockSealSpecError;
    PalletBlockSealSpecEvent: PalletBlockSealSpecEvent;
    PalletChainTransferCall: PalletChainTransferCall;
    PalletChainTransferError: PalletChainTransferError;
    PalletChainTransferEvent: PalletChainTransferEvent;
    PalletChainTransferQueuedTransferOut: PalletChainTransferQueuedTransferOut;
    PalletDigestsError: PalletDigestsError;
    PalletDigestsEvent: PalletDigestsEvent;
    PalletDomainsCall: PalletDomainsCall;
    PalletDomainsDomainRegistration: PalletDomainsDomainRegistration;
    PalletDomainsError: PalletDomainsError;
    PalletDomainsEvent: PalletDomainsEvent;
    PalletFeeControlError: PalletFeeControlError;
    PalletFeeControlEvent: PalletFeeControlEvent;
    PalletGrandpaCall: PalletGrandpaCall;
    PalletGrandpaError: PalletGrandpaError;
    PalletGrandpaEvent: PalletGrandpaEvent;
    PalletGrandpaStoredPendingChange: PalletGrandpaStoredPendingChange;
    PalletGrandpaStoredState: PalletGrandpaStoredState;
    PalletHyperbridgeError: PalletHyperbridgeError;
    PalletHyperbridgeEvent: PalletHyperbridgeEvent;
    PalletHyperbridgeSubstrateHostParams: PalletHyperbridgeSubstrateHostParams;
    PalletHyperbridgeVersionedHostParams: PalletHyperbridgeVersionedHostParams;
    PalletIsmpCall: PalletIsmpCall;
    PalletIsmpError: PalletIsmpError;
    PalletIsmpErrorsHandlingError: PalletIsmpErrorsHandlingError;
    PalletIsmpEvent: PalletIsmpEvent;
    PalletIsmpUtilsFundMessageParams: PalletIsmpUtilsFundMessageParams;
    PalletIsmpUtilsMessageCommitment: PalletIsmpUtilsMessageCommitment;
    PalletIsmpUtilsUpdateConsensusState: PalletIsmpUtilsUpdateConsensusState;
    PalletMiningSlotCall: PalletMiningSlotCall;
    PalletMiningSlotError: PalletMiningSlotError;
    PalletMiningSlotEvent: PalletMiningSlotEvent;
    PalletMiningSlotHoldReason: PalletMiningSlotHoldReason;
    PalletMiningSlotMinerNonceScoring: PalletMiningSlotMinerNonceScoring;
    PalletMintCall: PalletMintCall;
    PalletMintError: PalletMintError;
    PalletMintEvent: PalletMintEvent;
    PalletMintMintAction: PalletMintMintAction;
    PalletMintMintType: PalletMintMintType;
    PalletMultisigCall: PalletMultisigCall;
    PalletMultisigError: PalletMultisigError;
    PalletMultisigEvent: PalletMultisigEvent;
    PalletMultisigMultisig: PalletMultisigMultisig;
    PalletMultisigTimepoint: PalletMultisigTimepoint;
    PalletNotariesCall: PalletNotariesCall;
    PalletNotariesError: PalletNotariesError;
    PalletNotariesEvent: PalletNotariesEvent;
    PalletNotebookCall: PalletNotebookCall;
    PalletNotebookError: PalletNotebookError;
    PalletNotebookEvent: PalletNotebookEvent;
    PalletPriceIndexCall: PalletPriceIndexCall;
    PalletPriceIndexCpiMeasurementBucket: PalletPriceIndexCpiMeasurementBucket;
    PalletPriceIndexError: PalletPriceIndexError;
    PalletPriceIndexEvent: PalletPriceIndexEvent;
    PalletPriceIndexPriceIndex: PalletPriceIndexPriceIndex;
    PalletProxyAnnouncement: PalletProxyAnnouncement;
    PalletProxyCall: PalletProxyCall;
    PalletProxyDepositKind: PalletProxyDepositKind;
    PalletProxyError: PalletProxyError;
    PalletProxyEvent: PalletProxyEvent;
    PalletProxyProxyDefinition: PalletProxyProxyDefinition;
    PalletSudoCall: PalletSudoCall;
    PalletSudoError: PalletSudoError;
    PalletSudoEvent: PalletSudoEvent;
    PalletTicksCall: PalletTicksCall;
    PalletTicksError: PalletTicksError;
    PalletTimestampCall: PalletTimestampCall;
    PalletTokenGatewayAssetRegistration: PalletTokenGatewayAssetRegistration;
    PalletTokenGatewayCall: PalletTokenGatewayCall;
    PalletTokenGatewayError: PalletTokenGatewayError;
    PalletTokenGatewayEvent: PalletTokenGatewayEvent;
    PalletTokenGatewayPrecisionUpdate: PalletTokenGatewayPrecisionUpdate;
    PalletTokenGatewayTeleportParams: PalletTokenGatewayTeleportParams;
    PalletTransactionPaymentChargeTransactionPayment: PalletTransactionPaymentChargeTransactionPayment;
    PalletTransactionPaymentEvent: PalletTransactionPaymentEvent;
    PalletTransactionPaymentReleases: PalletTransactionPaymentReleases;
    PalletTreasuryCall: PalletTreasuryCall;
    PalletTreasuryError: PalletTreasuryError;
    PalletTreasuryEvent: PalletTreasuryEvent;
    PalletTreasuryFunderState: PalletTreasuryFunderState;
    PalletTreasuryHoldReason: PalletTreasuryHoldReason;
    PalletTreasuryTreasuryCapital: PalletTreasuryTreasuryCapital;
    PalletTreasuryTreasuryPool: PalletTreasuryTreasuryPool;
    PalletTxPauseCall: PalletTxPauseCall;
    PalletTxPauseError: PalletTxPauseError;
    PalletTxPauseEvent: PalletTxPauseEvent;
    PalletUtilityCall: PalletUtilityCall;
    PalletUtilityError: PalletUtilityError;
    PalletUtilityEvent: PalletUtilityEvent;
    PalletVaultsCall: PalletVaultsCall;
    PalletVaultsError: PalletVaultsError;
    PalletVaultsEvent: PalletVaultsEvent;
    PalletVaultsHoldReason: PalletVaultsHoldReason;
    PalletVaultsVaultConfig: PalletVaultsVaultConfig;
    PalletVaultsVaultFrameRevenue: PalletVaultsVaultFrameRevenue;
    SpArithmeticArithmeticError: SpArithmeticArithmeticError;
    SpConsensusGrandpaAppPublic: SpConsensusGrandpaAppPublic;
    SpConsensusGrandpaAppSignature: SpConsensusGrandpaAppSignature;
    SpConsensusGrandpaEquivocation: SpConsensusGrandpaEquivocation;
    SpConsensusGrandpaEquivocationProof: SpConsensusGrandpaEquivocationProof;
    SpCoreVoid: SpCoreVoid;
    SpRuntimeDigest: SpRuntimeDigest;
    SpRuntimeDigestDigestItem: SpRuntimeDigestDigestItem;
    SpRuntimeDispatchError: SpRuntimeDispatchError;
    SpRuntimeModuleError: SpRuntimeModuleError;
    SpRuntimeMultiSignature: SpRuntimeMultiSignature;
    SpRuntimeProvingTrieTrieError: SpRuntimeProvingTrieTrieError;
    SpRuntimeTokenError: SpRuntimeTokenError;
    SpRuntimeTransactionalError: SpRuntimeTransactionalError;
    SpVersionRuntimeVersion: SpVersionRuntimeVersion;
    SpWeightsRuntimeDbWeight: SpWeightsRuntimeDbWeight;
    SpWeightsWeightV2Weight: SpWeightsWeightV2Weight;
    TokenGatewayPrimitivesGatewayAssetRegistration: TokenGatewayPrimitivesGatewayAssetRegistration;
    TokenGatewayPrimitivesGatewayAssetUpdate: TokenGatewayPrimitivesGatewayAssetUpdate;
  } // InterfaceTypes
} // declare module
