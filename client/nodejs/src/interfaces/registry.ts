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
  ArgonPrimitivesBlockSealRewardDestination,
  ArgonPrimitivesBlockVoteBlockVoteT,
  ArgonPrimitivesDigestsBlockVoteDigest,
  ArgonPrimitivesDigestsDigestset,
  ArgonPrimitivesDigestsNotebookDigest,
  ArgonPrimitivesDigestsParentVotingKeyDigest,
  ArgonPrimitivesDomainSemver,
  ArgonPrimitivesDomainVersionHost,
  ArgonPrimitivesDomainZoneRecord,
  ArgonPrimitivesForkPower,
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
  ArgonPrimitivesVaultFundType,
  ArgonPrimitivesVaultObligation,
  ArgonPrimitivesVaultObligationError,
  ArgonPrimitivesVaultObligationExpiration,
  ArgonPrimitivesVaultVaultArgons,
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
  FrameSupportDispatchDispatchInfo,
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
  FrameSystemError,
  FrameSystemEvent,
  FrameSystemEventRecord,
  FrameSystemExtensionsCheckGenesis,
  FrameSystemExtensionsCheckNonZeroSender,
  FrameSystemExtensionsCheckNonce,
  FrameSystemExtensionsCheckSpecVersion,
  FrameSystemExtensionsCheckTxVersion,
  FrameSystemExtensionsCheckWeight,
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
  PalletBitcoinLocksHoldReason,
  PalletBitcoinLocksLockReleaseRequest,
  PalletBitcoinLocksLockedBitcoin,
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
  PalletPriceIndexError,
  PalletPriceIndexEvent,
  PalletPriceIndexPriceIndex,
  PalletProxyAnnouncement,
  PalletProxyCall,
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
  PalletTxPauseCall,
  PalletTxPauseError,
  PalletTxPauseEvent,
  PalletUtilityCall,
  PalletUtilityError,
  PalletUtilityEvent,
  PalletVaultsBidPoolEntrant,
  PalletVaultsCall,
  PalletVaultsError,
  PalletVaultsEvent,
  PalletVaultsHoldReason,
  PalletVaultsVaultConfig,
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
    ArgonPrimitivesBlockSealRewardDestination: ArgonPrimitivesBlockSealRewardDestination;
    ArgonPrimitivesBlockVoteBlockVoteT: ArgonPrimitivesBlockVoteBlockVoteT;
    ArgonPrimitivesDigestsBlockVoteDigest: ArgonPrimitivesDigestsBlockVoteDigest;
    ArgonPrimitivesDigestsDigestset: ArgonPrimitivesDigestsDigestset;
    ArgonPrimitivesDigestsNotebookDigest: ArgonPrimitivesDigestsNotebookDigest;
    ArgonPrimitivesDigestsParentVotingKeyDigest: ArgonPrimitivesDigestsParentVotingKeyDigest;
    ArgonPrimitivesDomainSemver: ArgonPrimitivesDomainSemver;
    ArgonPrimitivesDomainVersionHost: ArgonPrimitivesDomainVersionHost;
    ArgonPrimitivesDomainZoneRecord: ArgonPrimitivesDomainZoneRecord;
    ArgonPrimitivesForkPower: ArgonPrimitivesForkPower;
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
    ArgonPrimitivesVaultFundType: ArgonPrimitivesVaultFundType;
    ArgonPrimitivesVaultObligation: ArgonPrimitivesVaultObligation;
    ArgonPrimitivesVaultObligationError: ArgonPrimitivesVaultObligationError;
    ArgonPrimitivesVaultObligationExpiration: ArgonPrimitivesVaultObligationExpiration;
    ArgonPrimitivesVaultVaultArgons: ArgonPrimitivesVaultVaultArgons;
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
    FrameSupportDispatchDispatchInfo: FrameSupportDispatchDispatchInfo;
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
    FrameSystemError: FrameSystemError;
    FrameSystemEvent: FrameSystemEvent;
    FrameSystemEventRecord: FrameSystemEventRecord;
    FrameSystemExtensionsCheckGenesis: FrameSystemExtensionsCheckGenesis;
    FrameSystemExtensionsCheckNonZeroSender: FrameSystemExtensionsCheckNonZeroSender;
    FrameSystemExtensionsCheckNonce: FrameSystemExtensionsCheckNonce;
    FrameSystemExtensionsCheckSpecVersion: FrameSystemExtensionsCheckSpecVersion;
    FrameSystemExtensionsCheckTxVersion: FrameSystemExtensionsCheckTxVersion;
    FrameSystemExtensionsCheckWeight: FrameSystemExtensionsCheckWeight;
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
    PalletBitcoinLocksHoldReason: PalletBitcoinLocksHoldReason;
    PalletBitcoinLocksLockReleaseRequest: PalletBitcoinLocksLockReleaseRequest;
    PalletBitcoinLocksLockedBitcoin: PalletBitcoinLocksLockedBitcoin;
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
    PalletPriceIndexError: PalletPriceIndexError;
    PalletPriceIndexEvent: PalletPriceIndexEvent;
    PalletPriceIndexPriceIndex: PalletPriceIndexPriceIndex;
    PalletProxyAnnouncement: PalletProxyAnnouncement;
    PalletProxyCall: PalletProxyCall;
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
    PalletTxPauseCall: PalletTxPauseCall;
    PalletTxPauseError: PalletTxPauseError;
    PalletTxPauseEvent: PalletTxPauseEvent;
    PalletUtilityCall: PalletUtilityCall;
    PalletUtilityError: PalletUtilityError;
    PalletUtilityEvent: PalletUtilityEvent;
    PalletVaultsBidPoolEntrant: PalletVaultsBidPoolEntrant;
    PalletVaultsCall: PalletVaultsCall;
    PalletVaultsError: PalletVaultsError;
    PalletVaultsEvent: PalletVaultsEvent;
    PalletVaultsHoldReason: PalletVaultsHoldReason;
    PalletVaultsVaultConfig: PalletVaultsVaultConfig;
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
    SpRuntimeTokenError: SpRuntimeTokenError;
    SpRuntimeTransactionalError: SpRuntimeTransactionalError;
    SpVersionRuntimeVersion: SpVersionRuntimeVersion;
    SpWeightsRuntimeDbWeight: SpWeightsRuntimeDbWeight;
    SpWeightsWeightV2Weight: SpWeightsWeightV2Weight;
    TokenGatewayPrimitivesGatewayAssetRegistration: TokenGatewayPrimitivesGatewayAssetRegistration;
    TokenGatewayPrimitivesGatewayAssetUpdate: TokenGatewayPrimitivesGatewayAssetUpdate;
  } // InterfaceTypes
} // declare module
