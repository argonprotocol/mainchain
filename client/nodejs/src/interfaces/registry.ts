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
  ArgonPrimitivesEthereumEthereumBeaconPreset,
  ArgonPrimitivesEthereumEthereumCombinedReceiptProof,
  ArgonPrimitivesEthereumEthereumExecutionBlockProof,
  ArgonPrimitivesEthereumEthereumExecutionHeader,
  ArgonPrimitivesEthereumEthereumLog,
  ArgonPrimitivesEthereumEthereumReceiptLog,
  ArgonPrimitivesEthereumEthereumReceiptLogProofBatch,
  ArgonPrimitivesEthereumEthereumReceiptLogProofBlock,
  ArgonPrimitivesEthereumEthereumReceiptProofReceipt,
  ArgonPrimitivesForkPower,
  ArgonPrimitivesInherentsBitcoinUtxoFunding,
  ArgonPrimitivesInherentsBitcoinUtxoSpend,
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
  ArgonPrimitivesProvidersOperationalRewardKind,
  ArgonPrimitivesTickTicker,
  ArgonPrimitivesVault,
  ArgonPrimitivesVaultTreasuryBonusApprovalProof,
  ArgonPrimitivesVaultVaultArgonotCommitment,
  ArgonPrimitivesVaultVaultError,
  ArgonPrimitivesVaultVaultTerms,
  ArgonRuntimeGrandpaKeyOwnerProof,
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
  FrameSupportStorageNoDrop,
  FrameSupportTokensFungibleImbalance,
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
  PalletBalancesAccountData,
  PalletBalancesAdjustmentDirection,
  PalletBalancesBalanceLock,
  PalletBalancesCall,
  PalletBalancesError,
  PalletBalancesEvent,
  PalletBalancesReasons,
  PalletBalancesReserveData,
  PalletBalancesUnexpectedKind,
  PalletBitcoinLocksCall,
  PalletBitcoinLocksError,
  PalletBitcoinLocksEvent,
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
  PalletCrosschainTransferAssetKind,
  PalletCrosschainTransferCall,
  PalletCrosschainTransferChainConfig,
  PalletCrosschainTransferCouncilApprovalQueueEntry,
  PalletCrosschainTransferCouncilApprovalTargetId,
  PalletCrosschainTransferError,
  PalletCrosschainTransferEvent,
  PalletCrosschainTransferGatewayState,
  PalletCrosschainTransferGatewaySyncPause,
  PalletCrosschainTransferGatewaySyncPauseReason,
  PalletCrosschainTransferGlobalIssuanceCouncil,
  PalletCrosschainTransferGlobalIssuanceCouncilMember,
  PalletCrosschainTransferHoldReason,
  PalletCrosschainTransferMintingAuthority,
  PalletCrosschainTransferMintingAuthorityActivationRepaymentPricing,
  PalletCrosschainTransferMintingAuthorityState,
  PalletCrosschainTransferSourceChain,
  PalletCrosschainTransferSourceChainCirculation,
  PalletCrosschainTransferTransferOutMintingAuthorityTransferReservation,
  PalletCrosschainTransferTransferOutPendingCollateralizationRequest,
  PalletCrosschainTransferTransferOutTransferOutOfArgon,
  PalletCrosschainTransferTransferOutTransferOutState,
  PalletCrosschainTransferTransferToArgonActivity,
  PalletDigestsError,
  PalletDigestsEvent,
  PalletDomainsCall,
  PalletDomainsDomainRegistration,
  PalletDomainsError,
  PalletDomainsEvent,
  PalletEthereumVerifierBasicOperatingMode,
  PalletEthereumVerifierCall,
  PalletEthereumVerifierCheckpointUpdate,
  PalletEthereumVerifierError,
  PalletEthereumVerifierEvent,
  PalletEthereumVerifierExecutionHeaderAnchor,
  PalletEthereumVerifierExecutionHeaderProof,
  PalletEthereumVerifierFinalizedBeaconHeaderState,
  PalletEthereumVerifierFork,
  PalletEthereumVerifierForkVersions,
  PalletEthereumVerifierNextSyncCommitteeUpdate,
  PalletEthereumVerifierSyncAggregate,
  PalletEthereumVerifierSyncCommittee,
  PalletEthereumVerifierSyncCommitteePrepared,
  PalletEthereumVerifierUpdate,
  PalletFeeControlError,
  PalletFeeControlEvent,
  PalletGrandpaCall,
  PalletGrandpaError,
  PalletGrandpaEvent,
  PalletGrandpaStoredPendingChange,
  PalletGrandpaStoredState,
  PalletLocalchainTransferCall,
  PalletLocalchainTransferError,
  PalletLocalchainTransferEvent,
  PalletLocalchainTransferQueuedTransferOut,
  PalletMiningSlotCall,
  PalletMiningSlotError,
  PalletMiningSlotEvent,
  PalletMiningSlotHoldReason,
  PalletMiningSlotMinerNonceScoring,
  PalletMintCall,
  PalletMintError,
  PalletMintEvent,
  PalletMintMintAction,
  PalletMintMintQueueCursor,
  PalletMintMintType,
  PalletMintPendingMintUtxo,
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
  PalletOperationalAccountsAccountOwnershipProof,
  PalletOperationalAccountsCall,
  PalletOperationalAccountsError,
  PalletOperationalAccountsEvent,
  PalletOperationalAccountsOpaqueEncryptionPubkey,
  PalletOperationalAccountsOperationalAccount,
  PalletOperationalAccountsOperationalProgressPatch,
  PalletOperationalAccountsReferralProof,
  PalletOperationalAccountsRegistration,
  PalletOperationalAccountsRegistrationV1,
  PalletOperationalAccountsRewardsConfig,
  PalletPriceIndexArgonotAverageFrameAccumulator,
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
  PalletTransactionPaymentChargeTransactionPayment,
  PalletTransactionPaymentEvent,
  PalletTransactionPaymentReleases,
  PalletTreasuryBondLot,
  PalletTreasuryBondLotAllocation,
  PalletTreasuryBondLotSummary,
  PalletTreasuryBondProgram,
  PalletTreasuryBondProgramId,
  PalletTreasuryBondReleaseReason,
  PalletTreasuryCall,
  PalletTreasuryError,
  PalletTreasuryEvent,
  PalletTreasuryFrameArgonotBondParticipants,
  PalletTreasuryFrameVaultCapital,
  PalletTreasuryHoldReason,
  PalletTreasuryVaultCapital,
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
  PalletVaultsRecentCapacityDrop,
  PalletVaultsVaultConfig,
  PalletVaultsVaultFrameRevenue,
  SnowbridgeAmclBls381Big,
  SnowbridgeAmclBls381Ecp,
  SnowbridgeAmclBls381Fp,
  SnowbridgeBeaconPrimitivesBeaconHeader,
  SnowbridgeBeaconPrimitivesDenebExecutionPayloadHeader,
  SnowbridgeBeaconPrimitivesExecutionPayloadHeader,
  SnowbridgeBeaconPrimitivesPublicKey,
  SnowbridgeBeaconPrimitivesSignature,
  SnowbridgeBeaconPrimitivesVersionedExecutionPayloadHeader,
  SnowbridgeMilagroBlsKeysPublicKey,
  SpArithmeticArithmeticError,
  SpConsensusGrandpaAppPublic,
  SpConsensusGrandpaAppSignature,
  SpConsensusGrandpaEquivocation,
  SpConsensusGrandpaEquivocationProof,
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
    ArgonPrimitivesEthereumEthereumBeaconPreset: ArgonPrimitivesEthereumEthereumBeaconPreset;
    ArgonPrimitivesEthereumEthereumCombinedReceiptProof: ArgonPrimitivesEthereumEthereumCombinedReceiptProof;
    ArgonPrimitivesEthereumEthereumExecutionBlockProof: ArgonPrimitivesEthereumEthereumExecutionBlockProof;
    ArgonPrimitivesEthereumEthereumExecutionHeader: ArgonPrimitivesEthereumEthereumExecutionHeader;
    ArgonPrimitivesEthereumEthereumLog: ArgonPrimitivesEthereumEthereumLog;
    ArgonPrimitivesEthereumEthereumReceiptLog: ArgonPrimitivesEthereumEthereumReceiptLog;
    ArgonPrimitivesEthereumEthereumReceiptLogProofBatch: ArgonPrimitivesEthereumEthereumReceiptLogProofBatch;
    ArgonPrimitivesEthereumEthereumReceiptLogProofBlock: ArgonPrimitivesEthereumEthereumReceiptLogProofBlock;
    ArgonPrimitivesEthereumEthereumReceiptProofReceipt: ArgonPrimitivesEthereumEthereumReceiptProofReceipt;
    ArgonPrimitivesForkPower: ArgonPrimitivesForkPower;
    ArgonPrimitivesInherentsBitcoinUtxoFunding: ArgonPrimitivesInherentsBitcoinUtxoFunding;
    ArgonPrimitivesInherentsBitcoinUtxoSpend: ArgonPrimitivesInherentsBitcoinUtxoSpend;
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
    ArgonPrimitivesProvidersOperationalRewardKind: ArgonPrimitivesProvidersOperationalRewardKind;
    ArgonPrimitivesTickTicker: ArgonPrimitivesTickTicker;
    ArgonPrimitivesVault: ArgonPrimitivesVault;
    ArgonPrimitivesVaultTreasuryBonusApprovalProof: ArgonPrimitivesVaultTreasuryBonusApprovalProof;
    ArgonPrimitivesVaultVaultArgonotCommitment: ArgonPrimitivesVaultVaultArgonotCommitment;
    ArgonPrimitivesVaultVaultError: ArgonPrimitivesVaultVaultError;
    ArgonPrimitivesVaultVaultTerms: ArgonPrimitivesVaultVaultTerms;
    ArgonRuntimeGrandpaKeyOwnerProof: ArgonRuntimeGrandpaKeyOwnerProof;
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
    FrameSupportStorageNoDrop: FrameSupportStorageNoDrop;
    FrameSupportTokensFungibleImbalance: FrameSupportTokensFungibleImbalance;
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
    PalletBalancesAccountData: PalletBalancesAccountData;
    PalletBalancesAdjustmentDirection: PalletBalancesAdjustmentDirection;
    PalletBalancesBalanceLock: PalletBalancesBalanceLock;
    PalletBalancesCall: PalletBalancesCall;
    PalletBalancesError: PalletBalancesError;
    PalletBalancesEvent: PalletBalancesEvent;
    PalletBalancesReasons: PalletBalancesReasons;
    PalletBalancesReserveData: PalletBalancesReserveData;
    PalletBalancesUnexpectedKind: PalletBalancesUnexpectedKind;
    PalletBitcoinLocksCall: PalletBitcoinLocksCall;
    PalletBitcoinLocksError: PalletBitcoinLocksError;
    PalletBitcoinLocksEvent: PalletBitcoinLocksEvent;
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
    PalletCrosschainTransferAssetKind: PalletCrosschainTransferAssetKind;
    PalletCrosschainTransferCall: PalletCrosschainTransferCall;
    PalletCrosschainTransferChainConfig: PalletCrosschainTransferChainConfig;
    PalletCrosschainTransferCouncilApprovalQueueEntry: PalletCrosschainTransferCouncilApprovalQueueEntry;
    PalletCrosschainTransferCouncilApprovalTargetId: PalletCrosschainTransferCouncilApprovalTargetId;
    PalletCrosschainTransferError: PalletCrosschainTransferError;
    PalletCrosschainTransferEvent: PalletCrosschainTransferEvent;
    PalletCrosschainTransferGatewayState: PalletCrosschainTransferGatewayState;
    PalletCrosschainTransferGatewaySyncPause: PalletCrosschainTransferGatewaySyncPause;
    PalletCrosschainTransferGatewaySyncPauseReason: PalletCrosschainTransferGatewaySyncPauseReason;
    PalletCrosschainTransferGlobalIssuanceCouncil: PalletCrosschainTransferGlobalIssuanceCouncil;
    PalletCrosschainTransferGlobalIssuanceCouncilMember: PalletCrosschainTransferGlobalIssuanceCouncilMember;
    PalletCrosschainTransferHoldReason: PalletCrosschainTransferHoldReason;
    PalletCrosschainTransferMintingAuthority: PalletCrosschainTransferMintingAuthority;
    PalletCrosschainTransferMintingAuthorityActivationRepaymentPricing: PalletCrosschainTransferMintingAuthorityActivationRepaymentPricing;
    PalletCrosschainTransferMintingAuthorityState: PalletCrosschainTransferMintingAuthorityState;
    PalletCrosschainTransferSourceChain: PalletCrosschainTransferSourceChain;
    PalletCrosschainTransferSourceChainCirculation: PalletCrosschainTransferSourceChainCirculation;
    PalletCrosschainTransferTransferOutMintingAuthorityTransferReservation: PalletCrosschainTransferTransferOutMintingAuthorityTransferReservation;
    PalletCrosschainTransferTransferOutPendingCollateralizationRequest: PalletCrosschainTransferTransferOutPendingCollateralizationRequest;
    PalletCrosschainTransferTransferOutTransferOutOfArgon: PalletCrosschainTransferTransferOutTransferOutOfArgon;
    PalletCrosschainTransferTransferOutTransferOutState: PalletCrosschainTransferTransferOutTransferOutState;
    PalletCrosschainTransferTransferToArgonActivity: PalletCrosschainTransferTransferToArgonActivity;
    PalletDigestsError: PalletDigestsError;
    PalletDigestsEvent: PalletDigestsEvent;
    PalletDomainsCall: PalletDomainsCall;
    PalletDomainsDomainRegistration: PalletDomainsDomainRegistration;
    PalletDomainsError: PalletDomainsError;
    PalletDomainsEvent: PalletDomainsEvent;
    PalletEthereumVerifierBasicOperatingMode: PalletEthereumVerifierBasicOperatingMode;
    PalletEthereumVerifierCall: PalletEthereumVerifierCall;
    PalletEthereumVerifierCheckpointUpdate: PalletEthereumVerifierCheckpointUpdate;
    PalletEthereumVerifierError: PalletEthereumVerifierError;
    PalletEthereumVerifierEvent: PalletEthereumVerifierEvent;
    PalletEthereumVerifierExecutionHeaderAnchor: PalletEthereumVerifierExecutionHeaderAnchor;
    PalletEthereumVerifierExecutionHeaderProof: PalletEthereumVerifierExecutionHeaderProof;
    PalletEthereumVerifierFinalizedBeaconHeaderState: PalletEthereumVerifierFinalizedBeaconHeaderState;
    PalletEthereumVerifierFork: PalletEthereumVerifierFork;
    PalletEthereumVerifierForkVersions: PalletEthereumVerifierForkVersions;
    PalletEthereumVerifierNextSyncCommitteeUpdate: PalletEthereumVerifierNextSyncCommitteeUpdate;
    PalletEthereumVerifierSyncAggregate: PalletEthereumVerifierSyncAggregate;
    PalletEthereumVerifierSyncCommittee: PalletEthereumVerifierSyncCommittee;
    PalletEthereumVerifierSyncCommitteePrepared: PalletEthereumVerifierSyncCommitteePrepared;
    PalletEthereumVerifierUpdate: PalletEthereumVerifierUpdate;
    PalletFeeControlError: PalletFeeControlError;
    PalletFeeControlEvent: PalletFeeControlEvent;
    PalletGrandpaCall: PalletGrandpaCall;
    PalletGrandpaError: PalletGrandpaError;
    PalletGrandpaEvent: PalletGrandpaEvent;
    PalletGrandpaStoredPendingChange: PalletGrandpaStoredPendingChange;
    PalletGrandpaStoredState: PalletGrandpaStoredState;
    PalletLocalchainTransferCall: PalletLocalchainTransferCall;
    PalletLocalchainTransferError: PalletLocalchainTransferError;
    PalletLocalchainTransferEvent: PalletLocalchainTransferEvent;
    PalletLocalchainTransferQueuedTransferOut: PalletLocalchainTransferQueuedTransferOut;
    PalletMiningSlotCall: PalletMiningSlotCall;
    PalletMiningSlotError: PalletMiningSlotError;
    PalletMiningSlotEvent: PalletMiningSlotEvent;
    PalletMiningSlotHoldReason: PalletMiningSlotHoldReason;
    PalletMiningSlotMinerNonceScoring: PalletMiningSlotMinerNonceScoring;
    PalletMintCall: PalletMintCall;
    PalletMintError: PalletMintError;
    PalletMintEvent: PalletMintEvent;
    PalletMintMintAction: PalletMintMintAction;
    PalletMintMintQueueCursor: PalletMintMintQueueCursor;
    PalletMintMintType: PalletMintMintType;
    PalletMintPendingMintUtxo: PalletMintPendingMintUtxo;
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
    PalletOperationalAccountsAccountOwnershipProof: PalletOperationalAccountsAccountOwnershipProof;
    PalletOperationalAccountsCall: PalletOperationalAccountsCall;
    PalletOperationalAccountsError: PalletOperationalAccountsError;
    PalletOperationalAccountsEvent: PalletOperationalAccountsEvent;
    PalletOperationalAccountsOpaqueEncryptionPubkey: PalletOperationalAccountsOpaqueEncryptionPubkey;
    PalletOperationalAccountsOperationalAccount: PalletOperationalAccountsOperationalAccount;
    PalletOperationalAccountsOperationalProgressPatch: PalletOperationalAccountsOperationalProgressPatch;
    PalletOperationalAccountsReferralProof: PalletOperationalAccountsReferralProof;
    PalletOperationalAccountsRegistration: PalletOperationalAccountsRegistration;
    PalletOperationalAccountsRegistrationV1: PalletOperationalAccountsRegistrationV1;
    PalletOperationalAccountsRewardsConfig: PalletOperationalAccountsRewardsConfig;
    PalletPriceIndexArgonotAverageFrameAccumulator: PalletPriceIndexArgonotAverageFrameAccumulator;
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
    PalletTransactionPaymentChargeTransactionPayment: PalletTransactionPaymentChargeTransactionPayment;
    PalletTransactionPaymentEvent: PalletTransactionPaymentEvent;
    PalletTransactionPaymentReleases: PalletTransactionPaymentReleases;
    PalletTreasuryBondLot: PalletTreasuryBondLot;
    PalletTreasuryBondLotAllocation: PalletTreasuryBondLotAllocation;
    PalletTreasuryBondLotSummary: PalletTreasuryBondLotSummary;
    PalletTreasuryBondProgram: PalletTreasuryBondProgram;
    PalletTreasuryBondProgramId: PalletTreasuryBondProgramId;
    PalletTreasuryBondReleaseReason: PalletTreasuryBondReleaseReason;
    PalletTreasuryCall: PalletTreasuryCall;
    PalletTreasuryError: PalletTreasuryError;
    PalletTreasuryEvent: PalletTreasuryEvent;
    PalletTreasuryFrameArgonotBondParticipants: PalletTreasuryFrameArgonotBondParticipants;
    PalletTreasuryFrameVaultCapital: PalletTreasuryFrameVaultCapital;
    PalletTreasuryHoldReason: PalletTreasuryHoldReason;
    PalletTreasuryVaultCapital: PalletTreasuryVaultCapital;
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
    PalletVaultsRecentCapacityDrop: PalletVaultsRecentCapacityDrop;
    PalletVaultsVaultConfig: PalletVaultsVaultConfig;
    PalletVaultsVaultFrameRevenue: PalletVaultsVaultFrameRevenue;
    SnowbridgeAmclBls381Big: SnowbridgeAmclBls381Big;
    SnowbridgeAmclBls381Ecp: SnowbridgeAmclBls381Ecp;
    SnowbridgeAmclBls381Fp: SnowbridgeAmclBls381Fp;
    SnowbridgeBeaconPrimitivesBeaconHeader: SnowbridgeBeaconPrimitivesBeaconHeader;
    SnowbridgeBeaconPrimitivesDenebExecutionPayloadHeader: SnowbridgeBeaconPrimitivesDenebExecutionPayloadHeader;
    SnowbridgeBeaconPrimitivesExecutionPayloadHeader: SnowbridgeBeaconPrimitivesExecutionPayloadHeader;
    SnowbridgeBeaconPrimitivesPublicKey: SnowbridgeBeaconPrimitivesPublicKey;
    SnowbridgeBeaconPrimitivesSignature: SnowbridgeBeaconPrimitivesSignature;
    SnowbridgeBeaconPrimitivesVersionedExecutionPayloadHeader: SnowbridgeBeaconPrimitivesVersionedExecutionPayloadHeader;
    SnowbridgeMilagroBlsKeysPublicKey: SnowbridgeMilagroBlsKeysPublicKey;
    SpArithmeticArithmeticError: SpArithmeticArithmeticError;
    SpConsensusGrandpaAppPublic: SpConsensusGrandpaAppPublic;
    SpConsensusGrandpaAppSignature: SpConsensusGrandpaAppSignature;
    SpConsensusGrandpaEquivocation: SpConsensusGrandpaEquivocation;
    SpConsensusGrandpaEquivocationProof: SpConsensusGrandpaEquivocationProof;
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
  } // InterfaceTypes
} // declare module
