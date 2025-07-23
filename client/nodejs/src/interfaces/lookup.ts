// Auto-generated via `yarn polkadot-types-from-defs`, do not edit
/* eslint-disable */

/* eslint-disable sort-keys */

export default {
  /**
   * Lookup3: frame_system::AccountInfo<Nonce, pallet_balances::types::AccountData<Balance>>
   **/
  FrameSystemAccountInfo: {
    nonce: 'u32',
    consumers: 'u32',
    providers: 'u32',
    sufficients: 'u32',
    data: 'PalletBalancesAccountData',
  },
  /**
   * Lookup5: pallet_balances::types::AccountData<Balance>
   **/
  PalletBalancesAccountData: {
    free: 'u128',
    reserved: 'u128',
    frozen: 'u128',
    flags: 'u128',
  },
  /**
   * Lookup9: frame_support::dispatch::PerDispatchClass<sp_weights::weight_v2::Weight>
   **/
  FrameSupportDispatchPerDispatchClassWeight: {
    normal: 'SpWeightsWeightV2Weight',
    operational: 'SpWeightsWeightV2Weight',
    mandatory: 'SpWeightsWeightV2Weight',
  },
  /**
   * Lookup10: sp_weights::weight_v2::Weight
   **/
  SpWeightsWeightV2Weight: {
    refTime: 'Compact<u64>',
    proofSize: 'Compact<u64>',
  },
  /**
   * Lookup15: sp_runtime::generic::digest::Digest
   **/
  SpRuntimeDigest: {
    logs: 'Vec<SpRuntimeDigestDigestItem>',
  },
  /**
   * Lookup17: sp_runtime::generic::digest::DigestItem
   **/
  SpRuntimeDigestDigestItem: {
    _enum: {
      Other: 'Bytes',
      __Unused1: 'Null',
      __Unused2: 'Null',
      __Unused3: 'Null',
      Consensus: '([u8;4],Bytes)',
      Seal: '([u8;4],Bytes)',
      PreRuntime: '([u8;4],Bytes)',
      __Unused7: 'Null',
      RuntimeEnvironmentUpdated: 'Null',
    },
  },
  /**
   * Lookup20: frame_system::EventRecord<argon_runtime::RuntimeEvent, primitive_types::H256>
   **/
  FrameSystemEventRecord: {
    phase: 'FrameSystemPhase',
    event: 'Event',
    topics: 'Vec<H256>',
  },
  /**
   * Lookup22: frame_system::pallet::Event<T>
   **/
  FrameSystemEvent: {
    _enum: {
      ExtrinsicSuccess: {
        dispatchInfo: 'FrameSystemDispatchEventInfo',
      },
      ExtrinsicFailed: {
        dispatchError: 'SpRuntimeDispatchError',
        dispatchInfo: 'FrameSystemDispatchEventInfo',
      },
      CodeUpdated: 'Null',
      NewAccount: {
        account: 'AccountId32',
      },
      KilledAccount: {
        account: 'AccountId32',
      },
      Remarked: {
        _alias: {
          hash_: 'hash',
        },
        sender: 'AccountId32',
        hash_: 'H256',
      },
      UpgradeAuthorized: {
        codeHash: 'H256',
        checkVersion: 'bool',
      },
      RejectedInvalidAuthorizedUpgrade: {
        codeHash: 'H256',
        error: 'SpRuntimeDispatchError',
      },
    },
  },
  /**
   * Lookup23: frame_system::DispatchEventInfo
   **/
  FrameSystemDispatchEventInfo: {
    weight: 'SpWeightsWeightV2Weight',
    class: 'FrameSupportDispatchDispatchClass',
    paysFee: 'FrameSupportDispatchPays',
  },
  /**
   * Lookup24: frame_support::dispatch::DispatchClass
   **/
  FrameSupportDispatchDispatchClass: {
    _enum: ['Normal', 'Operational', 'Mandatory'],
  },
  /**
   * Lookup25: frame_support::dispatch::Pays
   **/
  FrameSupportDispatchPays: {
    _enum: ['Yes', 'No'],
  },
  /**
   * Lookup26: sp_runtime::DispatchError
   **/
  SpRuntimeDispatchError: {
    _enum: {
      Other: 'Null',
      CannotLookup: 'Null',
      BadOrigin: 'Null',
      Module: 'SpRuntimeModuleError',
      ConsumerRemaining: 'Null',
      NoProviders: 'Null',
      TooManyConsumers: 'Null',
      Token: 'SpRuntimeTokenError',
      Arithmetic: 'SpArithmeticArithmeticError',
      Transactional: 'SpRuntimeTransactionalError',
      Exhausted: 'Null',
      Corruption: 'Null',
      Unavailable: 'Null',
      RootNotAllowed: 'Null',
      Trie: 'SpRuntimeProvingTrieTrieError',
    },
  },
  /**
   * Lookup27: sp_runtime::ModuleError
   **/
  SpRuntimeModuleError: {
    index: 'u8',
    error: '[u8;4]',
  },
  /**
   * Lookup28: sp_runtime::TokenError
   **/
  SpRuntimeTokenError: {
    _enum: [
      'FundsUnavailable',
      'OnlyProvider',
      'BelowMinimum',
      'CannotCreate',
      'UnknownAsset',
      'Frozen',
      'Unsupported',
      'CannotCreateHold',
      'NotExpendable',
      'Blocked',
    ],
  },
  /**
   * Lookup29: sp_arithmetic::ArithmeticError
   **/
  SpArithmeticArithmeticError: {
    _enum: ['Underflow', 'Overflow', 'DivisionByZero'],
  },
  /**
   * Lookup30: sp_runtime::TransactionalError
   **/
  SpRuntimeTransactionalError: {
    _enum: ['LimitReached', 'NoLayer'],
  },
  /**
   * Lookup31: sp_runtime::proving_trie::TrieError
   **/
  SpRuntimeProvingTrieTrieError: {
    _enum: [
      'InvalidStateRoot',
      'IncompleteDatabase',
      'ValueAtIncompleteKey',
      'DecoderError',
      'InvalidHash',
      'DuplicateKey',
      'ExtraneousNode',
      'ExtraneousValue',
      'ExtraneousHashReference',
      'InvalidChildReference',
      'ValueMismatch',
      'IncompleteProof',
      'RootMismatch',
      'DecodeError',
    ],
  },
  /**
   * Lookup32: pallet_digests::pallet::Event<T>
   **/
  PalletDigestsEvent: 'Null',
  /**
   * Lookup33: pallet_multisig::pallet::Event<T>
   **/
  PalletMultisigEvent: {
    _enum: {
      NewMultisig: {
        approving: 'AccountId32',
        multisig: 'AccountId32',
        callHash: '[u8;32]',
      },
      MultisigApproval: {
        approving: 'AccountId32',
        timepoint: 'PalletMultisigTimepoint',
        multisig: 'AccountId32',
        callHash: '[u8;32]',
      },
      MultisigExecuted: {
        approving: 'AccountId32',
        timepoint: 'PalletMultisigTimepoint',
        multisig: 'AccountId32',
        callHash: '[u8;32]',
        result: 'Result<Null, SpRuntimeDispatchError>',
      },
      MultisigCancelled: {
        cancelling: 'AccountId32',
        timepoint: 'PalletMultisigTimepoint',
        multisig: 'AccountId32',
        callHash: '[u8;32]',
      },
      DepositPoked: {
        who: 'AccountId32',
        callHash: '[u8;32]',
        oldDeposit: 'u128',
        newDeposit: 'u128',
      },
    },
  },
  /**
   * Lookup34: pallet_multisig::Timepoint<BlockNumber>
   **/
  PalletMultisigTimepoint: {
    height: 'u32',
    index: 'u32',
  },
  /**
   * Lookup37: pallet_proxy::pallet::Event<T>
   **/
  PalletProxyEvent: {
    _enum: {
      ProxyExecuted: {
        result: 'Result<Null, SpRuntimeDispatchError>',
      },
      PureCreated: {
        pure: 'AccountId32',
        who: 'AccountId32',
        proxyType: 'ArgonRuntimeProxyType',
        disambiguationIndex: 'u16',
      },
      Announced: {
        real: 'AccountId32',
        proxy: 'AccountId32',
        callHash: 'H256',
      },
      ProxyAdded: {
        delegator: 'AccountId32',
        delegatee: 'AccountId32',
        proxyType: 'ArgonRuntimeProxyType',
        delay: 'u32',
      },
      ProxyRemoved: {
        delegator: 'AccountId32',
        delegatee: 'AccountId32',
        proxyType: 'ArgonRuntimeProxyType',
        delay: 'u32',
      },
      DepositPoked: {
        who: 'AccountId32',
        kind: 'PalletProxyDepositKind',
        oldDeposit: 'u128',
        newDeposit: 'u128',
      },
    },
  },
  /**
   * Lookup38: argon_runtime::ProxyType
   **/
  ArgonRuntimeProxyType: {
    _enum: ['Any', 'NonTransfer', 'PriceIndex', 'MiningBid', 'BitcoinCosign', 'VaultAdmin'],
  },
  /**
   * Lookup40: pallet_proxy::DepositKind
   **/
  PalletProxyDepositKind: {
    _enum: ['Proxies', 'Announcements'],
  },
  /**
   * Lookup41: pallet_mining_slot::pallet::Event<T>
   **/
  PalletMiningSlotEvent: {
    _enum: {
      NewMiners: {
        newMiners: 'Vec<ArgonPrimitivesBlockSealMiningRegistration>',
        releasedMiners: 'u32',
        frameId: 'u64',
      },
      SlotBidderAdded: {
        accountId: 'AccountId32',
        bidAmount: 'u128',
        index: 'u32',
      },
      SlotBidderDropped: {
        accountId: 'AccountId32',
        preservedArgonotHold: 'bool',
      },
      ReleaseMinerSeatError: {
        accountId: 'AccountId32',
        error: 'SpRuntimeDispatchError',
      },
      MiningConfigurationUpdated: {
        ticksBeforeBidEndForVrfClose: 'u64',
        ticksBetweenSlots: 'u64',
        slotBiddingStartAfterTicks: 'u64',
      },
      MiningBidsClosed: {
        frameId: 'u64',
      },
      ReleaseBidError: {
        accountId: 'AccountId32',
        error: 'SpRuntimeDispatchError',
      },
    },
  },
  /**
   * Lookup43: argon_primitives::block_seal::MiningRegistration<sp_core::crypto::AccountId32, Balance, argon_runtime::SessionKeys>
   **/
  ArgonPrimitivesBlockSealMiningRegistration: {
    accountId: 'AccountId32',
    externalFundingAccount: 'Option<AccountId32>',
    bid: 'Compact<u128>',
    argonots: 'Compact<u128>',
    authorityKeys: 'ArgonRuntimeSessionKeys',
    startingFrameId: 'Compact<u64>',
    bidAtTick: 'Compact<u64>',
  },
  /**
   * Lookup44: argon_runtime::SessionKeys
   **/
  ArgonRuntimeSessionKeys: {
    grandpa: 'SpConsensusGrandpaAppPublic',
    blockSealAuthority: 'ArgonPrimitivesBlockSealAppPublic',
  },
  /**
   * Lookup45: sp_consensus_grandpa::app::Public
   **/
  SpConsensusGrandpaAppPublic: '[u8;32]',
  /**
   * Lookup46: argon_primitives::block_seal::app::Public
   **/
  ArgonPrimitivesBlockSealAppPublic: '[u8;32]',
  /**
   * Lookup50: pallet_bitcoin_utxos::pallet::Event<T>
   **/
  PalletBitcoinUtxosEvent: {
    _enum: {
      UtxoVerified: {
        utxoId: 'u64',
      },
      UtxoRejected: {
        utxoId: 'u64',
        rejectedReason: 'ArgonPrimitivesBitcoinBitcoinRejectedReason',
      },
      UtxoSpent: {
        utxoId: 'u64',
        blockHeight: 'u64',
      },
      UtxoUnwatched: {
        utxoId: 'u64',
      },
      UtxoSpentError: {
        utxoId: 'u64',
        error: 'SpRuntimeDispatchError',
      },
      UtxoVerifiedError: {
        utxoId: 'u64',
        error: 'SpRuntimeDispatchError',
      },
      UtxoRejectedError: {
        utxoId: 'u64',
        error: 'SpRuntimeDispatchError',
      },
      UtxoExpiredError: {
        utxoRef: 'ArgonPrimitivesBitcoinUtxoRef',
        error: 'SpRuntimeDispatchError',
      },
    },
  },
  /**
   * Lookup51: argon_primitives::bitcoin::BitcoinRejectedReason
   **/
  ArgonPrimitivesBitcoinBitcoinRejectedReason: {
    _enum: ['SatoshisMismatch', 'Spent', 'LookupExpired', 'DuplicateUtxo'],
  },
  /**
   * Lookup52: argon_primitives::bitcoin::UtxoRef
   **/
  ArgonPrimitivesBitcoinUtxoRef: {
    txid: 'ArgonPrimitivesBitcoinH256Le',
    outputIndex: 'Compact<u32>',
  },
  /**
   * Lookup53: argon_primitives::bitcoin::H256Le
   **/
  ArgonPrimitivesBitcoinH256Le: '[u8;32]',
  /**
   * Lookup55: pallet_vaults::pallet::Event<T>
   **/
  PalletVaultsEvent: {
    _enum: {
      VaultCreated: {
        vaultId: 'u32',
        securitization: 'u128',
        securitizationRatio: 'u128',
        operatorAccountId: 'AccountId32',
        openedTick: 'u64',
      },
      VaultModified: {
        vaultId: 'u32',
        securitization: 'u128',
        securitizationRatio: 'u128',
      },
      VaultTermsChangeScheduled: {
        vaultId: 'u32',
        changeTick: 'u64',
      },
      VaultTermsChanged: {
        vaultId: 'u32',
      },
      VaultClosed: {
        vaultId: 'u32',
        securitizationRemaining: 'u128',
        securitizationReleased: 'u128',
      },
      VaultBitcoinXpubChange: {
        vaultId: 'u32',
      },
      FundsLocked: {
        vaultId: 'u32',
        locker: 'AccountId32',
        amount: 'u128',
        isRatchet: 'bool',
        feeRevenue: 'u128',
      },
      FundLockCanceled: {
        vaultId: 'u32',
        amount: 'u128',
      },
      FundsScheduledForRelease: {
        vaultId: 'u32',
        amount: 'u128',
        releaseHeight: 'u64',
      },
      LostBitcoinCompensated: {
        vaultId: 'u32',
        beneficiary: 'AccountId32',
        toBeneficiary: 'u128',
        burned: 'u128',
      },
      FundsReleased: {
        vaultId: 'u32',
        amount: 'u128',
      },
      FundsReleasedError: {
        vaultId: 'u32',
        error: 'SpRuntimeDispatchError',
      },
    },
  },
  /**
   * Lookup57: pallet_bitcoin_locks::pallet::Event<T>
   **/
  PalletBitcoinLocksEvent: {
    _enum: {
      BitcoinLockCreated: {
        utxoId: 'u64',
        vaultId: 'u32',
        lockPrice: 'u128',
        accountId: 'AccountId32',
        securityFee: 'u128',
      },
      BitcoinLockRatcheted: {
        utxoId: 'u64',
        vaultId: 'u32',
        originalLockPrice: 'u128',
        newLockPrice: 'u128',
        amountBurned: 'u128',
        accountId: 'AccountId32',
      },
      BitcoinLockBurned: {
        utxoId: 'u64',
        vaultId: 'u32',
        wasUtxoSpent: 'bool',
      },
      BitcoinUtxoCosignRequested: {
        utxoId: 'u64',
        vaultId: 'u32',
      },
      BitcoinUtxoCosigned: {
        utxoId: 'u64',
        vaultId: 'u32',
        signature: 'Bytes',
      },
      BitcoinCosignPastDue: {
        utxoId: 'u64',
        vaultId: 'u32',
        compensationAmount: 'u128',
        compensatedAccountId: 'AccountId32',
      },
      CosignOverdueError: {
        utxoId: 'u64',
        error: 'SpRuntimeDispatchError',
      },
      LockExpirationError: {
        utxoId: 'u64',
        error: 'SpRuntimeDispatchError',
      },
    },
  },
  /**
   * Lookup60: pallet_notaries::pallet::Event<T>
   **/
  PalletNotariesEvent: {
    _enum: {
      NotaryProposed: {
        operatorAccount: 'AccountId32',
        meta: 'ArgonPrimitivesNotaryNotaryMeta',
        expires: 'u32',
      },
      NotaryActivated: {
        notary: 'ArgonPrimitivesNotaryNotaryRecord',
      },
      NotaryMetaUpdateQueued: {
        notaryId: 'u32',
        meta: 'ArgonPrimitivesNotaryNotaryMeta',
        effectiveTick: 'u64',
      },
      NotaryMetaUpdated: {
        notaryId: 'u32',
        meta: 'ArgonPrimitivesNotaryNotaryMeta',
      },
      NotaryMetaUpdateError: {
        notaryId: 'u32',
        error: 'SpRuntimeDispatchError',
        meta: 'ArgonPrimitivesNotaryNotaryMeta',
      },
    },
  },
  /**
   * Lookup61: argon_primitives::notary::NotaryMeta<MaxHosts>
   **/
  ArgonPrimitivesNotaryNotaryMeta: {
    name: 'Bytes',
    public: '[u8;32]',
    hosts: 'Vec<Bytes>',
  },
  /**
   * Lookup68: argon_primitives::notary::NotaryRecord<sp_core::crypto::AccountId32, BlockNumber, MaxHosts>
   **/
  ArgonPrimitivesNotaryNotaryRecord: {
    notaryId: 'Compact<u32>',
    operatorAccountId: 'AccountId32',
    activatedBlock: 'Compact<u32>',
    metaUpdatedBlock: 'Compact<u32>',
    metaUpdatedTick: 'Compact<u64>',
    meta: 'ArgonPrimitivesNotaryNotaryMeta',
  },
  /**
   * Lookup69: pallet_notebook::pallet::Event<T>
   **/
  PalletNotebookEvent: {
    _enum: {
      NotebookSubmitted: {
        notaryId: 'u32',
        notebookNumber: 'u32',
      },
      NotebookAuditFailure: {
        notaryId: 'u32',
        notebookNumber: 'u32',
        notebookHash: 'H256',
        firstFailureReason: 'ArgonNotaryAuditErrorVerifyError',
      },
      NotebookReadyForReprocess: {
        notaryId: 'u32',
        notebookNumber: 'u32',
      },
    },
  },
  /**
   * Lookup70: argon_notary_audit::error::VerifyError
   **/
  ArgonNotaryAuditErrorVerifyError: {
    _enum: {
      MissingAccountOrigin: {
        accountId: 'AccountId32',
        accountType: 'ArgonPrimitivesAccountAccountType',
      },
      HistoryLookupError: {
        source: 'ArgonNotaryAuditAccountHistoryLookupError',
      },
      InvalidAccountChangelist: 'Null',
      InvalidChainTransfersList: 'Null',
      InvalidBalanceChangeRoot: 'Null',
      InvalidHeaderTaxRecorded: 'Null',
      InvalidPreviousNonce: 'Null',
      InvalidPreviousBalance: 'Null',
      InvalidPreviousAccountOrigin: 'Null',
      InvalidPreviousBalanceChangeNotebook: 'Null',
      InvalidBalanceChange: 'Null',
      InvalidBalanceChangeSignature: {
        changeIndex: 'u16',
      },
      InvalidNoteRecipients: 'Null',
      BalanceChangeError: {
        changeIndex: 'u16',
        noteIndex: 'u16',
        message: 'Bytes',
      },
      InvalidNetBalanceChangeset: 'Null',
      InsufficientBalance: {
        balance: 'u128',
        amount: 'u128',
        noteIndex: 'u16',
        changeIndex: 'u16',
      },
      ExceededMaxBalance: {
        balance: 'u128',
        amount: 'u128',
        noteIndex: 'u16',
        changeIndex: 'u16',
      },
      BalanceChangeMismatch: {
        changeIndex: 'u16',
        providedBalance: 'u128',
        calculatedBalance: 'i128',
      },
      BalanceChangeNotNetZero: {
        sent: 'u128',
        claimed: 'u128',
      },
      InvalidDomainLeaseAllocation: 'Null',
      TaxBalanceChangeNotNetZero: {
        sent: 'u128',
        claimed: 'u128',
      },
      MissingBalanceProof: 'Null',
      InvalidPreviousBalanceProof: 'Null',
      InvalidNotebookHash: 'Null',
      InvalidNotebookHeaderHash: 'Null',
      DuplicateChainTransfer: 'Null',
      DuplicatedAccountOriginUid: 'Null',
      InvalidNotarySignature: 'Null',
      InvalidSecretProvided: 'Null',
      NotebookTooOld: 'Null',
      CatchupNotebooksMissing: 'Null',
      DecodeError: 'Null',
      AccountChannelHoldDoesntExist: 'Null',
      AccountAlreadyHasChannelHold: 'Null',
      ChannelHoldNotReadyForClaim: {
        currentTick: 'u64',
        claimTick: 'u64',
      },
      AccountLocked: 'Null',
      MissingChannelHoldNote: 'Null',
      InvalidChannelHoldNote: 'Null',
      InvalidChannelHoldClaimers: 'Null',
      ChannelHoldNoteBelowMinimum: 'Null',
      InvalidTaxNoteAccount: 'Null',
      InvalidTaxOperation: 'Null',
      InsufficientTaxIncluded: {
        taxSent: 'u128',
        taxOwed: 'u128',
        accountId: 'AccountId32',
      },
      InsufficientBlockVoteTax: 'Null',
      IneligibleTaxVoter: 'Null',
      BlockVoteInvalidSignature: 'Null',
      InvalidBlockVoteAllocation: 'Null',
      InvalidBlockVoteRoot: 'Null',
      InvalidBlockVotesCount: 'Null',
      InvalidBlockVotingPower: 'Null',
      InvalidBlockVoteList: 'Null',
      InvalidComputeProof: 'Null',
      InvalidBlockVoteSource: 'Null',
      InsufficientBlockVoteMinimum: 'Null',
      InvalidBlockVoteTick: {
        tick: 'u64',
        notebookTick: 'u64',
      },
      InvalidDefaultBlockVote: 'Null',
      InvalidDefaultBlockVoteAuthor: {
        author: 'AccountId32',
        expected: 'AccountId32',
      },
      NoDefaultBlockVote: 'Null',
    },
  },
  /**
   * Lookup71: argon_primitives::account::AccountType
   **/
  ArgonPrimitivesAccountAccountType: {
    _enum: ['Tax', 'Deposit'],
  },
  /**
   * Lookup72: argon_notary_audit::AccountHistoryLookupError
   **/
  ArgonNotaryAuditAccountHistoryLookupError: {
    _enum: [
      'RootNotFound',
      'LastChangeNotFound',
      'InvalidTransferToLocalchain',
      'BlockSpecificationNotFound',
    ],
  },
  /**
   * Lookup76: pallet_chain_transfer::pallet::Event<T>
   **/
  PalletChainTransferEvent: {
    _enum: {
      TransferToLocalchain: {
        accountId: 'AccountId32',
        amount: 'u128',
        transferId: 'u32',
        notaryId: 'u32',
        expirationTick: 'u64',
      },
      TransferToLocalchainExpired: {
        accountId: 'AccountId32',
        transferId: 'u32',
        notaryId: 'u32',
      },
      TransferFromLocalchain: {
        accountId: 'AccountId32',
        amount: 'u128',
        notaryId: 'u32',
      },
      TransferFromLocalchainError: {
        accountId: 'AccountId32',
        amount: 'u128',
        notaryId: 'u32',
        notebookNumber: 'u32',
        error: 'SpRuntimeDispatchError',
      },
      TransferToLocalchainRefundError: {
        accountId: 'AccountId32',
        transferId: 'u32',
        notaryId: 'u32',
        notebookNumber: 'u32',
        error: 'SpRuntimeDispatchError',
      },
      PossibleInvalidLocalchainTransferAllowed: {
        transferId: 'u32',
        notaryId: 'u32',
        notebookNumber: 'u32',
      },
      TaxationError: {
        notaryId: 'u32',
        notebookNumber: 'u32',
        tax: 'u128',
        error: 'SpRuntimeDispatchError',
      },
    },
  },
  /**
   * Lookup77: pallet_block_seal_spec::pallet::Event<T>
   **/
  PalletBlockSealSpecEvent: {
    _enum: {
      VoteMinimumAdjusted: {
        expectedBlockVotes: 'u128',
        actualBlockVotes: 'u128',
        startVoteMinimum: 'u128',
        newVoteMinimum: 'u128',
      },
      ComputeDifficultyAdjusted: {
        expectedBlockTime: 'u64',
        actualBlockTime: 'u64',
        startDifficulty: 'u128',
        newDifficulty: 'u128',
      },
    },
  },
  /**
   * Lookup78: pallet_domains::pallet::Event<T>
   **/
  PalletDomainsEvent: {
    _enum: {
      ZoneRecordUpdated: {
        domainHash: 'H256',
        zoneRecord: 'ArgonPrimitivesDomainZoneRecord',
      },
      DomainRegistered: {
        domainHash: 'H256',
        registration: 'PalletDomainsDomainRegistration',
      },
      DomainRenewed: {
        domainHash: 'H256',
      },
      DomainExpired: {
        domainHash: 'H256',
      },
      DomainRegistrationCanceled: {
        domainHash: 'H256',
        registration: 'PalletDomainsDomainRegistration',
      },
      DomainRegistrationError: {
        domainHash: 'H256',
        accountId: 'AccountId32',
        error: 'SpRuntimeDispatchError',
      },
    },
  },
  /**
   * Lookup79: argon_primitives::domain::ZoneRecord<sp_core::crypto::AccountId32>
   **/
  ArgonPrimitivesDomainZoneRecord: {
    paymentAccount: 'AccountId32',
    notaryId: 'u32',
    versions: 'BTreeMap<ArgonPrimitivesDomainSemver, ArgonPrimitivesDomainVersionHost>',
  },
  /**
   * Lookup81: argon_primitives::domain::Semver
   **/
  ArgonPrimitivesDomainSemver: {
    major: 'u32',
    minor: 'u32',
    patch: 'u32',
  },
  /**
   * Lookup82: argon_primitives::domain::VersionHost
   **/
  ArgonPrimitivesDomainVersionHost: {
    datastoreId: 'Bytes',
    host: 'Bytes',
  },
  /**
   * Lookup87: pallet_domains::DomainRegistration<sp_core::crypto::AccountId32>
   **/
  PalletDomainsDomainRegistration: {
    accountId: 'AccountId32',
    registeredAtTick: 'u64',
  },
  /**
   * Lookup88: pallet_price_index::pallet::Event<T>
   **/
  PalletPriceIndexEvent: {
    _enum: {
      NewIndex: 'Null',
      OperatorChanged: {
        operatorId: 'AccountId32',
      },
    },
  },
  /**
   * Lookup89: pallet_grandpa::pallet::Event
   **/
  PalletGrandpaEvent: {
    _enum: {
      NewAuthorities: {
        authoritySet: 'Vec<(SpConsensusGrandpaAppPublic,u64)>',
      },
      Paused: 'Null',
      Resumed: 'Null',
    },
  },
  /**
   * Lookup92: pallet_block_rewards::pallet::Event<T>
   **/
  PalletBlockRewardsEvent: {
    _enum: {
      RewardCreated: {
        rewards: 'Vec<ArgonPrimitivesBlockSealBlockPayout>',
      },
      RewardCreateError: {
        accountId: 'AccountId32',
        argons: 'Option<u128>',
        ownership: 'Option<u128>',
        error: 'SpRuntimeDispatchError',
      },
    },
  },
  /**
   * Lookup94: argon_primitives::block_seal::BlockPayout<sp_core::crypto::AccountId32, Balance>
   **/
  ArgonPrimitivesBlockSealBlockPayout: {
    accountId: 'AccountId32',
    ownership: 'Compact<u128>',
    argons: 'Compact<u128>',
    rewardType: 'ArgonPrimitivesBlockSealBlockRewardType',
    blockSealAuthority: 'Option<ArgonPrimitivesBlockSealAppPublic>',
  },
  /**
   * Lookup95: argon_primitives::block_seal::BlockRewardType
   **/
  ArgonPrimitivesBlockSealBlockRewardType: {
    _enum: ['Miner', 'Voter', 'ProfitShare'],
  },
  /**
   * Lookup98: pallet_mint::pallet::Event<T>
   **/
  PalletMintEvent: {
    _enum: {
      BitcoinMint: {
        accountId: 'AccountId32',
        utxoId: 'Option<u64>',
        amount: 'u128',
      },
      MiningMint: {
        amount: 'U256',
        perMiner: 'u128',
        argonCpi: 'i128',
        liquidity: 'u128',
      },
      MintError: {
        mintType: 'PalletMintMintType',
        accountId: 'AccountId32',
        utxoId: 'Option<u64>',
        amount: 'u128',
        error: 'SpRuntimeDispatchError',
      },
    },
  },
  /**
   * Lookup103: pallet_mint::pallet::MintType
   **/
  PalletMintMintType: {
    _enum: ['Bitcoin', 'Mining'],
  },
  /**
   * Lookup104: pallet_balances::pallet::Event<T, I>
   **/
  PalletBalancesEvent: {
    _enum: {
      Endowed: {
        account: 'AccountId32',
        freeBalance: 'u128',
      },
      DustLost: {
        account: 'AccountId32',
        amount: 'u128',
      },
      Transfer: {
        from: 'AccountId32',
        to: 'AccountId32',
        amount: 'u128',
      },
      BalanceSet: {
        who: 'AccountId32',
        free: 'u128',
      },
      Reserved: {
        who: 'AccountId32',
        amount: 'u128',
      },
      Unreserved: {
        who: 'AccountId32',
        amount: 'u128',
      },
      ReserveRepatriated: {
        from: 'AccountId32',
        to: 'AccountId32',
        amount: 'u128',
        destinationStatus: 'FrameSupportTokensMiscBalanceStatus',
      },
      Deposit: {
        who: 'AccountId32',
        amount: 'u128',
      },
      Withdraw: {
        who: 'AccountId32',
        amount: 'u128',
      },
      Slashed: {
        who: 'AccountId32',
        amount: 'u128',
      },
      Minted: {
        who: 'AccountId32',
        amount: 'u128',
      },
      Burned: {
        who: 'AccountId32',
        amount: 'u128',
      },
      Suspended: {
        who: 'AccountId32',
        amount: 'u128',
      },
      Restored: {
        who: 'AccountId32',
        amount: 'u128',
      },
      Upgraded: {
        who: 'AccountId32',
      },
      Issued: {
        amount: 'u128',
      },
      Rescinded: {
        amount: 'u128',
      },
      Locked: {
        who: 'AccountId32',
        amount: 'u128',
      },
      Unlocked: {
        who: 'AccountId32',
        amount: 'u128',
      },
      Frozen: {
        who: 'AccountId32',
        amount: 'u128',
      },
      Thawed: {
        who: 'AccountId32',
        amount: 'u128',
      },
      TotalIssuanceForced: {
        _alias: {
          new_: 'new',
        },
        old: 'u128',
        new_: 'u128',
      },
    },
  },
  /**
   * Lookup105: frame_support::traits::tokens::misc::BalanceStatus
   **/
  FrameSupportTokensMiscBalanceStatus: {
    _enum: ['Free', 'Reserved'],
  },
  /**
   * Lookup107: pallet_tx_pause::pallet::Event<T>
   **/
  PalletTxPauseEvent: {
    _enum: {
      CallPaused: {
        fullName: '(Bytes,Bytes)',
      },
      CallUnpaused: {
        fullName: '(Bytes,Bytes)',
      },
    },
  },
  /**
   * Lookup110: pallet_transaction_payment::pallet::Event<T>
   **/
  PalletTransactionPaymentEvent: {
    _enum: {
      TransactionFeePaid: {
        who: 'AccountId32',
        actualFee: 'u128',
        tip: 'u128',
      },
    },
  },
  /**
   * Lookup111: pallet_utility::pallet::Event
   **/
  PalletUtilityEvent: {
    _enum: {
      BatchInterrupted: {
        index: 'u32',
        error: 'SpRuntimeDispatchError',
      },
      BatchCompleted: 'Null',
      BatchCompletedWithErrors: 'Null',
      ItemCompleted: 'Null',
      ItemFailed: {
        error: 'SpRuntimeDispatchError',
      },
      DispatchedAs: {
        result: 'Result<Null, SpRuntimeDispatchError>',
      },
      IfElseMainSuccess: 'Null',
      IfElseFallbackCalled: {
        mainError: 'SpRuntimeDispatchError',
      },
    },
  },
  /**
   * Lookup112: pallet_sudo::pallet::Event<T>
   **/
  PalletSudoEvent: {
    _enum: {
      Sudid: {
        sudoResult: 'Result<Null, SpRuntimeDispatchError>',
      },
      KeyChanged: {
        _alias: {
          new_: 'new',
        },
        old: 'Option<AccountId32>',
        new_: 'AccountId32',
      },
      KeyRemoved: 'Null',
      SudoAsDone: {
        sudoResult: 'Result<Null, SpRuntimeDispatchError>',
      },
    },
  },
  /**
   * Lookup113: pallet_ismp::pallet::Event<T>
   **/
  PalletIsmpEvent: {
    _enum: {
      StateMachineUpdated: {
        stateMachineId: 'IsmpConsensusStateMachineId',
        latestHeight: 'u64',
      },
      StateCommitmentVetoed: {
        height: 'IsmpConsensusStateMachineHeight',
        fisherman: 'Bytes',
      },
      ConsensusClientCreated: {
        consensusClientId: '[u8;4]',
      },
      ConsensusClientFrozen: {
        consensusClientId: '[u8;4]',
      },
      Response: {
        destChain: 'IsmpHostStateMachine',
        sourceChain: 'IsmpHostStateMachine',
        requestNonce: 'u64',
        commitment: 'H256',
        reqCommitment: 'H256',
      },
      Request: {
        destChain: 'IsmpHostStateMachine',
        sourceChain: 'IsmpHostStateMachine',
        requestNonce: 'u64',
        commitment: 'H256',
      },
      Errors: {
        errors: 'Vec<PalletIsmpErrorsHandlingError>',
      },
      PostRequestHandled: 'IsmpEventsRequestResponseHandled',
      PostResponseHandled: 'IsmpEventsRequestResponseHandled',
      GetRequestHandled: 'IsmpEventsRequestResponseHandled',
      PostRequestTimeoutHandled: 'IsmpEventsTimeoutHandled',
      PostResponseTimeoutHandled: 'IsmpEventsTimeoutHandled',
      GetRequestTimeoutHandled: 'IsmpEventsTimeoutHandled',
    },
  },
  /**
   * Lookup114: ismp::consensus::StateMachineId
   **/
  IsmpConsensusStateMachineId: {
    stateId: 'IsmpHostStateMachine',
    consensusStateId: '[u8;4]',
  },
  /**
   * Lookup115: ismp::host::StateMachine
   **/
  IsmpHostStateMachine: {
    _enum: {
      Evm: 'u32',
      Polkadot: 'u32',
      Kusama: 'u32',
      Substrate: '[u8;4]',
      Tendermint: '[u8;4]',
      Relay: {
        relay: '[u8;4]',
        paraId: 'u32',
      },
    },
  },
  /**
   * Lookup116: ismp::consensus::StateMachineHeight
   **/
  IsmpConsensusStateMachineHeight: {
    id: 'IsmpConsensusStateMachineId',
    height: 'u64',
  },
  /**
   * Lookup119: pallet_ismp::errors::HandlingError
   **/
  PalletIsmpErrorsHandlingError: {
    message: 'Bytes',
  },
  /**
   * Lookup121: ismp::events::RequestResponseHandled
   **/
  IsmpEventsRequestResponseHandled: {
    commitment: 'H256',
    relayer: 'Bytes',
  },
  /**
   * Lookup122: ismp::events::TimeoutHandled
   **/
  IsmpEventsTimeoutHandled: {
    commitment: 'H256',
    source: 'IsmpHostStateMachine',
    dest: 'IsmpHostStateMachine',
  },
  /**
   * Lookup123: ismp_grandpa::pallet::Event<T>
   **/
  IsmpGrandpaEvent: {
    _enum: {
      StateMachineAdded: {
        stateMachines: 'Vec<IsmpHostStateMachine>',
      },
      StateMachineRemoved: {
        stateMachines: 'Vec<IsmpHostStateMachine>',
      },
    },
  },
  /**
   * Lookup125: pallet_hyperbridge::pallet::Event<T>
   **/
  PalletHyperbridgeEvent: {
    _enum: {
      HostParamsUpdated: {
        _alias: {
          new_: 'new',
        },
        old: 'PalletHyperbridgeVersionedHostParams',
        new_: 'PalletHyperbridgeVersionedHostParams',
      },
      RelayerFeeWithdrawn: {
        amount: 'u128',
        account: 'AccountId32',
      },
      ProtocolRevenueWithdrawn: {
        amount: 'u128',
        account: 'AccountId32',
      },
    },
  },
  /**
   * Lookup126: pallet_hyperbridge::VersionedHostParams<Balance>
   **/
  PalletHyperbridgeVersionedHostParams: {
    _enum: {
      V1: 'PalletHyperbridgeSubstrateHostParams',
    },
  },
  /**
   * Lookup127: pallet_hyperbridge::SubstrateHostParams<B>
   **/
  PalletHyperbridgeSubstrateHostParams: {
    defaultPerByteFee: 'u128',
    perByteFees: 'BTreeMap<IsmpHostStateMachine, u128>',
    assetRegistrationFee: 'u128',
  },
  /**
   * Lookup131: pallet_token_gateway::pallet::Event<T>
   **/
  PalletTokenGatewayEvent: {
    _enum: {
      AssetTeleported: {
        from: 'AccountId32',
        to: 'H256',
        amount: 'u128',
        dest: 'IsmpHostStateMachine',
        commitment: 'H256',
      },
      AssetReceived: {
        beneficiary: 'AccountId32',
        amount: 'u128',
        source: 'IsmpHostStateMachine',
      },
      AssetRefunded: {
        beneficiary: 'AccountId32',
        amount: 'u128',
        source: 'IsmpHostStateMachine',
      },
      ERC6160AssetRegistrationDispatched: {
        commitment: 'H256',
      },
    },
  },
  /**
   * Lookup132: pallet_liquidity_pools::pallet::Event<T>
   **/
  PalletLiquidityPoolsEvent: {
    _enum: {
      CouldNotDistributeBidPool: {
        accountId: 'AccountId32',
        frameId: 'u64',
        vaultId: 'u32',
        amount: 'u128',
        dispatchError: 'SpRuntimeDispatchError',
        isForVault: 'bool',
      },
      CouldNotBurnBidPool: {
        frameId: 'u64',
        amount: 'u128',
        dispatchError: 'SpRuntimeDispatchError',
      },
      BidPoolDistributed: {
        frameId: 'u64',
        bidPoolDistributed: 'u128',
        bidPoolBurned: 'u128',
        bidPoolShares: 'u32',
      },
      NextBidPoolCapitalLocked: {
        frameId: 'u64',
        totalActivatedCapital: 'u128',
        participatingVaults: 'u32',
      },
      ErrorRefundingLiquidityPoolCapital: {
        frameId: 'u64',
        vaultId: 'u32',
        amount: 'u128',
        accountId: 'AccountId32',
        dispatchError: 'SpRuntimeDispatchError',
      },
      RefundedLiquidityPoolCapital: {
        frameId: 'u64',
        vaultId: 'u32',
        amount: 'u128',
        accountId: 'AccountId32',
      },
      VaultOperatorPrebond: {
        vaultId: 'u32',
        accountId: 'AccountId32',
        amount: 'u128',
      },
    },
  },
  /**
   * Lookup133: pallet_skip_feeless_payment::pallet::Event<T>
   **/
  PalletSkipFeelessPaymentEvent: {
    _enum: {
      FeeSkipped: {
        origin: 'ArgonRuntimeOriginCaller',
      },
    },
  },
  /**
   * Lookup134: argon_runtime::OriginCaller
   **/
  ArgonRuntimeOriginCaller: {
    _enum: {
      system: 'FrameSupportDispatchRawOrigin',
    },
  },
  /**
   * Lookup135: frame_support::dispatch::RawOrigin<sp_core::crypto::AccountId32>
   **/
  FrameSupportDispatchRawOrigin: {
    _enum: {
      Root: 'Null',
      Signed: 'AccountId32',
      None: 'Null',
    },
  },
  /**
   * Lookup136: frame_system::Phase
   **/
  FrameSystemPhase: {
    _enum: {
      ApplyExtrinsic: 'u32',
      Finalization: 'Null',
      Initialization: 'Null',
    },
  },
  /**
   * Lookup140: frame_system::LastRuntimeUpgradeInfo
   **/
  FrameSystemLastRuntimeUpgradeInfo: {
    specVersion: 'Compact<u32>',
    specName: 'Text',
  },
  /**
   * Lookup143: frame_system::CodeUpgradeAuthorization<T>
   **/
  FrameSystemCodeUpgradeAuthorization: {
    codeHash: 'H256',
    checkVersion: 'bool',
  },
  /**
   * Lookup144: frame_system::pallet::Call<T>
   **/
  FrameSystemCall: {
    _enum: {
      remark: {
        remark: 'Bytes',
      },
      set_heap_pages: {
        pages: 'u64',
      },
      set_code: {
        code: 'Bytes',
      },
      set_code_without_checks: {
        code: 'Bytes',
      },
      set_storage: {
        items: 'Vec<(Bytes,Bytes)>',
      },
      kill_storage: {
        _alias: {
          keys_: 'keys',
        },
        keys_: 'Vec<Bytes>',
      },
      kill_prefix: {
        prefix: 'Bytes',
        subkeys: 'u32',
      },
      remark_with_event: {
        remark: 'Bytes',
      },
      __Unused8: 'Null',
      authorize_upgrade: {
        codeHash: 'H256',
      },
      authorize_upgrade_without_checks: {
        codeHash: 'H256',
      },
      apply_authorized_upgrade: {
        code: 'Bytes',
      },
    },
  },
  /**
   * Lookup148: frame_system::limits::BlockWeights
   **/
  FrameSystemLimitsBlockWeights: {
    baseBlock: 'SpWeightsWeightV2Weight',
    maxBlock: 'SpWeightsWeightV2Weight',
    perClass: 'FrameSupportDispatchPerDispatchClassWeightsPerClass',
  },
  /**
   * Lookup149: frame_support::dispatch::PerDispatchClass<frame_system::limits::WeightsPerClass>
   **/
  FrameSupportDispatchPerDispatchClassWeightsPerClass: {
    normal: 'FrameSystemLimitsWeightsPerClass',
    operational: 'FrameSystemLimitsWeightsPerClass',
    mandatory: 'FrameSystemLimitsWeightsPerClass',
  },
  /**
   * Lookup150: frame_system::limits::WeightsPerClass
   **/
  FrameSystemLimitsWeightsPerClass: {
    baseExtrinsic: 'SpWeightsWeightV2Weight',
    maxExtrinsic: 'Option<SpWeightsWeightV2Weight>',
    maxTotal: 'Option<SpWeightsWeightV2Weight>',
    reserved: 'Option<SpWeightsWeightV2Weight>',
  },
  /**
   * Lookup152: frame_system::limits::BlockLength
   **/
  FrameSystemLimitsBlockLength: {
    max: 'FrameSupportDispatchPerDispatchClassU32',
  },
  /**
   * Lookup153: frame_support::dispatch::PerDispatchClass<T>
   **/
  FrameSupportDispatchPerDispatchClassU32: {
    normal: 'u32',
    operational: 'u32',
    mandatory: 'u32',
  },
  /**
   * Lookup154: sp_weights::RuntimeDbWeight
   **/
  SpWeightsRuntimeDbWeight: {
    read: 'u64',
    write: 'u64',
  },
  /**
   * Lookup155: sp_version::RuntimeVersion
   **/
  SpVersionRuntimeVersion: {
    specName: 'Text',
    implName: 'Text',
    authoringVersion: 'u32',
    specVersion: 'u32',
    implVersion: 'u32',
    apis: 'Vec<([u8;8],u32)>',
    transactionVersion: 'u32',
    systemVersion: 'u8',
  },
  /**
   * Lookup160: frame_system::pallet::Error<T>
   **/
  FrameSystemError: {
    _enum: [
      'InvalidSpecName',
      'SpecVersionNeedsToIncrease',
      'FailedToExtractRuntimeVersion',
      'NonDefaultComposite',
      'NonZeroRefCount',
      'CallFiltered',
      'MultiBlockMigrationsOngoing',
      'NothingAuthorized',
      'Unauthorized',
    ],
  },
  /**
   * Lookup161: argon_primitives::digests::Digestset<argon_notary_audit::error::VerifyError, sp_core::crypto::AccountId32>
   **/
  ArgonPrimitivesDigestsDigestset: {
    author: 'AccountId32',
    blockVote: 'ArgonPrimitivesDigestsBlockVoteDigest',
    votingKey: 'Option<ArgonPrimitivesDigestsParentVotingKeyDigest>',
    forkPower: 'Option<ArgonPrimitivesForkPower>',
    tick: 'u64',
    notebooks: 'ArgonPrimitivesDigestsNotebookDigest',
  },
  /**
   * Lookup162: argon_primitives::digests::BlockVoteDigest
   **/
  ArgonPrimitivesDigestsBlockVoteDigest: {
    votingPower: 'Compact<u128>',
    votesCount: 'Compact<u32>',
  },
  /**
   * Lookup164: argon_primitives::digests::ParentVotingKeyDigest
   **/
  ArgonPrimitivesDigestsParentVotingKeyDigest: {
    parentVotingKey: 'Option<H256>',
  },
  /**
   * Lookup167: argon_primitives::fork_power::ForkPower
   **/
  ArgonPrimitivesForkPower: {
    isLatestVote: 'bool',
    notebooks: 'Compact<u64>',
    votingPower: 'U256',
    sealStrength: 'U256',
    totalComputeDifficulty: 'U256',
    voteCreatedBlocks: 'Compact<u128>',
    minerVoteXorDistance: 'Option<U256>',
  },
  /**
   * Lookup170: argon_primitives::digests::NotebookDigest<argon_notary_audit::error::VerifyError>
   **/
  ArgonPrimitivesDigestsNotebookDigest: {
    notebooks: 'Vec<ArgonPrimitivesNotebookNotebookAuditResult>',
  },
  /**
   * Lookup172: argon_primitives::notebook::NotebookAuditResult<argon_notary_audit::error::VerifyError>
   **/
  ArgonPrimitivesNotebookNotebookAuditResult: {
    notaryId: 'Compact<u32>',
    notebookNumber: 'Compact<u32>',
    tick: 'Compact<u64>',
    auditFirstFailure: 'Option<ArgonNotaryAuditErrorVerifyError>',
  },
  /**
   * Lookup175: pallet_digests::pallet::Error<T>
   **/
  PalletDigestsError: {
    _enum: [
      'DuplicateBlockVoteDigest',
      'DuplicateAuthorDigest',
      'DuplicateTickDigest',
      'DuplicateParentVotingKeyDigest',
      'DuplicateNotebookDigest',
      'DuplicateForkPowerDigest',
      'MissingBlockVoteDigest',
      'MissingAuthorDigest',
      'MissingTickDigest',
      'MissingParentVotingKeyDigest',
      'MissingNotebookDigest',
      'CouldNotDecodeDigest',
    ],
  },
  /**
   * Lookup176: pallet_timestamp::pallet::Call<T>
   **/
  PalletTimestampCall: {
    _enum: {
      set: {
        now: 'Compact<u64>',
      },
    },
  },
  /**
   * Lookup178: pallet_multisig::Multisig<BlockNumber, Balance, sp_core::crypto::AccountId32, MaxApprovals>
   **/
  PalletMultisigMultisig: {
    when: 'PalletMultisigTimepoint',
    deposit: 'u128',
    depositor: 'AccountId32',
    approvals: 'Vec<AccountId32>',
  },
  /**
   * Lookup181: pallet_multisig::pallet::Call<T>
   **/
  PalletMultisigCall: {
    _enum: {
      as_multi_threshold_1: {
        otherSignatories: 'Vec<AccountId32>',
        call: 'Call',
      },
      as_multi: {
        threshold: 'u16',
        otherSignatories: 'Vec<AccountId32>',
        maybeTimepoint: 'Option<PalletMultisigTimepoint>',
        call: 'Call',
        maxWeight: 'SpWeightsWeightV2Weight',
      },
      approve_as_multi: {
        threshold: 'u16',
        otherSignatories: 'Vec<AccountId32>',
        maybeTimepoint: 'Option<PalletMultisigTimepoint>',
        callHash: '[u8;32]',
        maxWeight: 'SpWeightsWeightV2Weight',
      },
      cancel_as_multi: {
        threshold: 'u16',
        otherSignatories: 'Vec<AccountId32>',
        timepoint: 'PalletMultisigTimepoint',
        callHash: '[u8;32]',
      },
      poke_deposit: {
        threshold: 'u16',
        otherSignatories: 'Vec<AccountId32>',
        callHash: '[u8;32]',
      },
    },
  },
  /**
   * Lookup183: pallet_proxy::pallet::Call<T>
   **/
  PalletProxyCall: {
    _enum: {
      proxy: {
        real: 'MultiAddress',
        forceProxyType: 'Option<ArgonRuntimeProxyType>',
        call: 'Call',
      },
      add_proxy: {
        delegate: 'MultiAddress',
        proxyType: 'ArgonRuntimeProxyType',
        delay: 'u32',
      },
      remove_proxy: {
        delegate: 'MultiAddress',
        proxyType: 'ArgonRuntimeProxyType',
        delay: 'u32',
      },
      remove_proxies: 'Null',
      create_pure: {
        proxyType: 'ArgonRuntimeProxyType',
        delay: 'u32',
        index: 'u16',
      },
      kill_pure: {
        spawner: 'MultiAddress',
        proxyType: 'ArgonRuntimeProxyType',
        index: 'u16',
        height: 'Compact<u32>',
        extIndex: 'Compact<u32>',
      },
      announce: {
        real: 'MultiAddress',
        callHash: 'H256',
      },
      remove_announcement: {
        real: 'MultiAddress',
        callHash: 'H256',
      },
      reject_announcement: {
        delegate: 'MultiAddress',
        callHash: 'H256',
      },
      proxy_announced: {
        delegate: 'MultiAddress',
        real: 'MultiAddress',
        forceProxyType: 'Option<ArgonRuntimeProxyType>',
        call: 'Call',
      },
      poke_deposit: 'Null',
    },
  },
  /**
   * Lookup188: pallet_ticks::pallet::Call<T>
   **/
  PalletTicksCall: 'Null',
  /**
   * Lookup189: pallet_mining_slot::pallet::Call<T>
   **/
  PalletMiningSlotCall: {
    _enum: {
      bid: {
        _alias: {
          keys_: 'keys',
        },
        bid: 'u128',
        keys_: 'ArgonRuntimeSessionKeys',
        miningAccountId: 'Option<AccountId32>',
      },
      configure_mining_slot_delay: {
        miningSlotDelay: 'Option<u64>',
        ticksBeforeBidEndForVrfClose: 'Option<u64>',
      },
    },
  },
  /**
   * Lookup190: pallet_bitcoin_utxos::pallet::Call<T>
   **/
  PalletBitcoinUtxosCall: {
    _enum: {
      sync: {
        utxoSync: 'ArgonPrimitivesInherentsBitcoinUtxoSync',
      },
      set_confirmed_block: {
        bitcoinHeight: 'u64',
        bitcoinBlockHash: 'ArgonPrimitivesBitcoinH256Le',
      },
      set_operator: {
        accountId: 'AccountId32',
      },
    },
  },
  /**
   * Lookup191: argon_primitives::inherents::BitcoinUtxoSync
   **/
  ArgonPrimitivesInherentsBitcoinUtxoSync: {
    spent: 'BTreeMap<u64, u64>',
    verified: 'BTreeMap<u64, ArgonPrimitivesBitcoinUtxoRef>',
    invalid: 'BTreeMap<u64, ArgonPrimitivesBitcoinBitcoinRejectedReason>',
    syncToBlock: 'ArgonPrimitivesBitcoinBitcoinBlock',
  },
  /**
   * Lookup201: argon_primitives::bitcoin::BitcoinBlock
   **/
  ArgonPrimitivesBitcoinBitcoinBlock: {
    blockHeight: 'Compact<u64>',
    blockHash: 'ArgonPrimitivesBitcoinH256Le',
  },
  /**
   * Lookup202: pallet_vaults::pallet::Call<T>
   **/
  PalletVaultsCall: {
    _enum: {
      create: {
        vaultConfig: 'PalletVaultsVaultConfig',
      },
      modify_funding: {
        vaultId: 'u32',
        securitization: 'u128',
        securitizationRatio: 'u128',
      },
      modify_terms: {
        vaultId: 'u32',
        terms: 'ArgonPrimitivesVaultVaultTerms',
      },
      close: {
        vaultId: 'u32',
      },
      replace_bitcoin_xpub: {
        vaultId: 'u32',
        bitcoinXpub: 'ArgonPrimitivesBitcoinOpaqueBitcoinXpub',
      },
    },
  },
  /**
   * Lookup203: pallet_vaults::pallet::VaultConfig<Balance>
   **/
  PalletVaultsVaultConfig: {
    terms: 'ArgonPrimitivesVaultVaultTerms',
    securitization: 'Compact<u128>',
    bitcoinXpubkey: 'ArgonPrimitivesBitcoinOpaqueBitcoinXpub',
    securitizationRatio: 'Compact<u128>',
  },
  /**
   * Lookup204: argon_primitives::vault::VaultTerms<Balance>
   **/
  ArgonPrimitivesVaultVaultTerms: {
    bitcoinAnnualPercentRate: 'Compact<u128>',
    bitcoinBaseFee: 'Compact<u128>',
    liquidityPoolProfitSharing: 'Compact<Permill>',
  },
  /**
   * Lookup208: argon_primitives::bitcoin::OpaqueBitcoinXpub
   **/
  ArgonPrimitivesBitcoinOpaqueBitcoinXpub: '[u8;78]',
  /**
   * Lookup210: pallet_bitcoin_locks::pallet::Call<T>
   **/
  PalletBitcoinLocksCall: {
    _enum: {
      initialize: {
        vaultId: 'u32',
        satoshis: 'Compact<u64>',
        bitcoinPubkey: 'ArgonPrimitivesBitcoinCompressedBitcoinPubkey',
      },
      request_release: {
        utxoId: 'u64',
        toScriptPubkey: 'Bytes',
        bitcoinNetworkFee: 'u64',
      },
      cosign_release: {
        utxoId: 'u64',
        signature: 'Bytes',
      },
      ratchet: {
        utxoId: 'u64',
      },
      admin_modify_minimum_locked_sats: {
        satoshis: 'u64',
      },
    },
  },
  /**
   * Lookup211: argon_primitives::bitcoin::CompressedBitcoinPubkey
   **/
  ArgonPrimitivesBitcoinCompressedBitcoinPubkey: '[u8;33]',
  /**
   * Lookup215: pallet_notaries::pallet::Call<T>
   **/
  PalletNotariesCall: {
    _enum: {
      propose: {
        meta: 'ArgonPrimitivesNotaryNotaryMeta',
      },
      activate: {
        operatorAccount: 'AccountId32',
      },
      update: {
        notaryId: 'Compact<u32>',
        meta: 'ArgonPrimitivesNotaryNotaryMeta',
        effectiveTick: 'Compact<u64>',
      },
    },
  },
  /**
   * Lookup216: pallet_notebook::pallet::Call<T>
   **/
  PalletNotebookCall: {
    _enum: {
      submit: {
        notebooks: 'Vec<ArgonPrimitivesNotebookSignedNotebookHeader>',
      },
      unlock: {
        notaryId: 'u32',
      },
    },
  },
  /**
   * Lookup218: argon_primitives::notebook::SignedNotebookHeader
   **/
  ArgonPrimitivesNotebookSignedNotebookHeader: {
    header: 'ArgonPrimitivesNotebookNotebookHeader',
    signature: '[u8;64]',
  },
  /**
   * Lookup219: argon_primitives::notebook::NotebookHeader
   **/
  ArgonPrimitivesNotebookNotebookHeader: {
    version: 'Compact<u16>',
    notebookNumber: 'Compact<u32>',
    tick: 'Compact<u64>',
    tax: 'Compact<u128>',
    notaryId: 'Compact<u32>',
    chainTransfers: 'Vec<ArgonPrimitivesNotebookChainTransfer>',
    changedAccountsRoot: 'H256',
    changedAccountOrigins: 'Vec<ArgonPrimitivesBalanceChangeAccountOrigin>',
    blockVotesRoot: 'H256',
    blockVotesCount: 'Compact<u32>',
    blocksWithVotes: 'Vec<H256>',
    blockVotingPower: 'Compact<u128>',
    secretHash: 'H256',
    parentSecret: 'Option<H256>',
    domains: 'Vec<(H256,AccountId32)>',
  },
  /**
   * Lookup222: argon_primitives::notebook::ChainTransfer
   **/
  ArgonPrimitivesNotebookChainTransfer: {
    _enum: {
      ToMainchain: {
        accountId: 'AccountId32',
        amount: 'Compact<u128>',
      },
      ToLocalchain: {
        transferId: 'Compact<u32>',
      },
    },
  },
  /**
   * Lookup225: argon_primitives::balance_change::AccountOrigin
   **/
  ArgonPrimitivesBalanceChangeAccountOrigin: {
    notebookNumber: 'Compact<u32>',
    accountUid: 'Compact<u32>',
  },
  /**
   * Lookup232: pallet_chain_transfer::pallet::Call<T>
   **/
  PalletChainTransferCall: {
    _enum: {
      send_to_localchain: {
        amount: 'Compact<u128>',
        notaryId: 'u32',
      },
    },
  },
  /**
   * Lookup233: pallet_block_seal_spec::pallet::Call<T>
   **/
  PalletBlockSealSpecCall: {
    _enum: {
      configure: {
        voteMinimum: 'Option<u128>',
        computeDifficulty: 'Option<u128>',
      },
    },
  },
  /**
   * Lookup234: pallet_domains::pallet::Call<T>
   **/
  PalletDomainsCall: {
    _enum: {
      set_zone_record: {
        domainHash: 'H256',
        zoneRecord: 'ArgonPrimitivesDomainZoneRecord',
      },
    },
  },
  /**
   * Lookup235: pallet_price_index::pallet::Call<T>
   **/
  PalletPriceIndexCall: {
    _enum: {
      submit: {
        index: 'PalletPriceIndexPriceIndex',
      },
      set_operator: {
        accountId: 'AccountId32',
      },
    },
  },
  /**
   * Lookup236: pallet_price_index::PriceIndex
   **/
  PalletPriceIndexPriceIndex: {
    btcUsdPrice: 'Compact<u128>',
    argonotUsdPrice: 'u128',
    argonUsdPrice: 'Compact<u128>',
    argonUsdTargetPrice: 'u128',
    argonTimeWeightedAverageLiquidity: 'u128',
    tick: 'Compact<u64>',
  },
  /**
   * Lookup237: pallet_grandpa::pallet::Call<T>
   **/
  PalletGrandpaCall: {
    _enum: {
      report_equivocation: {
        equivocationProof: 'SpConsensusGrandpaEquivocationProof',
        keyOwnerProof: 'SpCoreVoid',
      },
      report_equivocation_unsigned: {
        equivocationProof: 'SpConsensusGrandpaEquivocationProof',
        keyOwnerProof: 'SpCoreVoid',
      },
      note_stalled: {
        delay: 'u32',
        bestFinalizedBlockNumber: 'u32',
      },
    },
  },
  /**
   * Lookup238: sp_consensus_grandpa::EquivocationProof<primitive_types::H256, N>
   **/
  SpConsensusGrandpaEquivocationProof: {
    setId: 'u64',
    equivocation: 'SpConsensusGrandpaEquivocation',
  },
  /**
   * Lookup239: sp_consensus_grandpa::Equivocation<primitive_types::H256, N>
   **/
  SpConsensusGrandpaEquivocation: {
    _enum: {
      Prevote: 'FinalityGrandpaEquivocationPrevote',
      Precommit: 'FinalityGrandpaEquivocationPrecommit',
    },
  },
  /**
   * Lookup240: finality_grandpa::Equivocation<sp_consensus_grandpa::app::Public, finality_grandpa::Prevote<primitive_types::H256, N>, sp_consensus_grandpa::app::Signature>
   **/
  FinalityGrandpaEquivocationPrevote: {
    roundNumber: 'u64',
    identity: 'SpConsensusGrandpaAppPublic',
    first: '(FinalityGrandpaPrevote,SpConsensusGrandpaAppSignature)',
    second: '(FinalityGrandpaPrevote,SpConsensusGrandpaAppSignature)',
  },
  /**
   * Lookup241: finality_grandpa::Prevote<primitive_types::H256, N>
   **/
  FinalityGrandpaPrevote: {
    targetHash: 'H256',
    targetNumber: 'u32',
  },
  /**
   * Lookup242: sp_consensus_grandpa::app::Signature
   **/
  SpConsensusGrandpaAppSignature: '[u8;64]',
  /**
   * Lookup244: finality_grandpa::Equivocation<sp_consensus_grandpa::app::Public, finality_grandpa::Precommit<primitive_types::H256, N>, sp_consensus_grandpa::app::Signature>
   **/
  FinalityGrandpaEquivocationPrecommit: {
    roundNumber: 'u64',
    identity: 'SpConsensusGrandpaAppPublic',
    first: '(FinalityGrandpaPrecommit,SpConsensusGrandpaAppSignature)',
    second: '(FinalityGrandpaPrecommit,SpConsensusGrandpaAppSignature)',
  },
  /**
   * Lookup245: finality_grandpa::Precommit<primitive_types::H256, N>
   **/
  FinalityGrandpaPrecommit: {
    targetHash: 'H256',
    targetNumber: 'u32',
  },
  /**
   * Lookup247: sp_core::Void
   **/
  SpCoreVoid: 'Null',
  /**
   * Lookup248: pallet_block_seal::pallet::Call<T>
   **/
  PalletBlockSealCall: {
    _enum: {
      apply: {
        seal: 'ArgonPrimitivesInherentsBlockSealInherent',
      },
    },
  },
  /**
   * Lookup249: argon_primitives::inherents::BlockSealInherent
   **/
  ArgonPrimitivesInherentsBlockSealInherent: {
    _enum: {
      Vote: {
        sealStrength: 'U256',
        notaryId: 'Compact<u32>',
        sourceNotebookNumber: 'Compact<u32>',
        sourceNotebookProof: 'ArgonPrimitivesBalanceChangeMerkleProof',
        blockVote: 'ArgonPrimitivesBlockVoteBlockVoteT',
        xorDistance: 'Option<U256>',
      },
      Compute: 'Null',
    },
  },
  /**
   * Lookup250: argon_primitives::balance_change::MerkleProof
   **/
  ArgonPrimitivesBalanceChangeMerkleProof: {
    proof: 'Vec<H256>',
    numberOfLeaves: 'Compact<u32>',
    leafIndex: 'Compact<u32>',
  },
  /**
   * Lookup252: argon_primitives::block_vote::BlockVoteT<primitive_types::H256>
   **/
  ArgonPrimitivesBlockVoteBlockVoteT: {
    accountId: 'AccountId32',
    blockHash: 'H256',
    index: 'Compact<u32>',
    power: 'Compact<u128>',
    signature: 'SpRuntimeMultiSignature',
    blockRewardsAccountId: 'AccountId32',
    tick: 'Compact<u64>',
  },
  /**
   * Lookup253: sp_runtime::MultiSignature
   **/
  SpRuntimeMultiSignature: {
    _enum: {
      Ed25519: '[u8;64]',
      Sr25519: '[u8;64]',
      Ecdsa: '[u8;65]',
    },
  },
  /**
   * Lookup255: pallet_block_rewards::pallet::Call<T>
   **/
  PalletBlockRewardsCall: {
    _enum: {
      set_block_rewards_paused: {
        paused: 'bool',
      },
    },
  },
  /**
   * Lookup256: pallet_mint::pallet::Call<T>
   **/
  PalletMintCall: 'Null',
  /**
   * Lookup257: pallet_balances::pallet::Call<T, I>
   **/
  PalletBalancesCall: {
    _enum: {
      transfer_allow_death: {
        dest: 'MultiAddress',
        value: 'Compact<u128>',
      },
      __Unused1: 'Null',
      force_transfer: {
        source: 'MultiAddress',
        dest: 'MultiAddress',
        value: 'Compact<u128>',
      },
      transfer_keep_alive: {
        dest: 'MultiAddress',
        value: 'Compact<u128>',
      },
      transfer_all: {
        dest: 'MultiAddress',
        keepAlive: 'bool',
      },
      force_unreserve: {
        who: 'MultiAddress',
        amount: 'u128',
      },
      upgrade_accounts: {
        who: 'Vec<AccountId32>',
      },
      __Unused7: 'Null',
      force_set_balance: {
        who: 'MultiAddress',
        newFree: 'Compact<u128>',
      },
      force_adjust_total_issuance: {
        direction: 'PalletBalancesAdjustmentDirection',
        delta: 'Compact<u128>',
      },
      burn: {
        value: 'Compact<u128>',
        keepAlive: 'bool',
      },
    },
  },
  /**
   * Lookup258: pallet_balances::types::AdjustmentDirection
   **/
  PalletBalancesAdjustmentDirection: {
    _enum: ['Increase', 'Decrease'],
  },
  /**
   * Lookup260: pallet_tx_pause::pallet::Call<T>
   **/
  PalletTxPauseCall: {
    _enum: {
      pause: {
        fullName: '(Bytes,Bytes)',
      },
      unpause: {
        ident: '(Bytes,Bytes)',
      },
    },
  },
  /**
   * Lookup261: pallet_utility::pallet::Call<T>
   **/
  PalletUtilityCall: {
    _enum: {
      batch: {
        calls: 'Vec<Call>',
      },
      as_derivative: {
        index: 'u16',
        call: 'Call',
      },
      batch_all: {
        calls: 'Vec<Call>',
      },
      dispatch_as: {
        asOrigin: 'ArgonRuntimeOriginCaller',
        call: 'Call',
      },
      force_batch: {
        calls: 'Vec<Call>',
      },
      with_weight: {
        call: 'Call',
        weight: 'SpWeightsWeightV2Weight',
      },
      if_else: {
        main: 'Call',
        fallback: 'Call',
      },
      dispatch_as_fallible: {
        asOrigin: 'ArgonRuntimeOriginCaller',
        call: 'Call',
      },
    },
  },
  /**
   * Lookup263: pallet_sudo::pallet::Call<T>
   **/
  PalletSudoCall: {
    _enum: {
      sudo: {
        call: 'Call',
      },
      sudo_unchecked_weight: {
        call: 'Call',
        weight: 'SpWeightsWeightV2Weight',
      },
      set_key: {
        _alias: {
          new_: 'new',
        },
        new_: 'MultiAddress',
      },
      sudo_as: {
        who: 'MultiAddress',
        call: 'Call',
      },
      remove_key: 'Null',
    },
  },
  /**
   * Lookup264: pallet_ismp::pallet::Call<T>
   **/
  PalletIsmpCall: {
    _enum: {
      handle_unsigned: {
        messages: 'Vec<IsmpMessagingMessage>',
      },
      __Unused1: 'Null',
      create_consensus_client: {
        message: 'IsmpMessagingCreateConsensusState',
      },
      update_consensus_state: {
        message: 'PalletIsmpUtilsUpdateConsensusState',
      },
      fund_message: {
        message: 'PalletIsmpUtilsFundMessageParams',
      },
    },
  },
  /**
   * Lookup266: ismp::messaging::Message
   **/
  IsmpMessagingMessage: {
    _enum: {
      Consensus: 'IsmpMessagingConsensusMessage',
      FraudProof: 'IsmpMessagingFraudProofMessage',
      Request: 'IsmpMessagingRequestMessage',
      Response: 'IsmpMessagingResponseMessage',
      Timeout: 'IsmpMessagingTimeoutMessage',
    },
  },
  /**
   * Lookup267: ismp::messaging::ConsensusMessage
   **/
  IsmpMessagingConsensusMessage: {
    consensusProof: 'Bytes',
    consensusStateId: '[u8;4]',
    signer: 'Bytes',
  },
  /**
   * Lookup268: ismp::messaging::FraudProofMessage
   **/
  IsmpMessagingFraudProofMessage: {
    proof1: 'Bytes',
    proof2: 'Bytes',
    consensusStateId: '[u8;4]',
    signer: 'Bytes',
  },
  /**
   * Lookup269: ismp::messaging::RequestMessage
   **/
  IsmpMessagingRequestMessage: {
    requests: 'Vec<IsmpRouterPostRequest>',
    proof: 'IsmpMessagingProof',
    signer: 'Bytes',
  },
  /**
   * Lookup271: ismp::router::PostRequest
   **/
  IsmpRouterPostRequest: {
    source: 'IsmpHostStateMachine',
    dest: 'IsmpHostStateMachine',
    nonce: 'u64',
    from: 'Bytes',
    to: 'Bytes',
    timeoutTimestamp: 'u64',
    body: 'Bytes',
  },
  /**
   * Lookup272: ismp::messaging::Proof
   **/
  IsmpMessagingProof: {
    height: 'IsmpConsensusStateMachineHeight',
    proof: 'Bytes',
  },
  /**
   * Lookup273: ismp::messaging::ResponseMessage
   **/
  IsmpMessagingResponseMessage: {
    datagram: 'IsmpRouterRequestResponse',
    proof: 'IsmpMessagingProof',
    signer: 'Bytes',
  },
  /**
   * Lookup274: ismp::router::RequestResponse
   **/
  IsmpRouterRequestResponse: {
    _enum: {
      Request: 'Vec<IsmpRouterRequest>',
      Response: 'Vec<IsmpRouterResponse>',
    },
  },
  /**
   * Lookup276: ismp::router::Request
   **/
  IsmpRouterRequest: {
    _enum: {
      Post: 'IsmpRouterPostRequest',
      Get: 'IsmpRouterGetRequest',
    },
  },
  /**
   * Lookup277: ismp::router::GetRequest
   **/
  IsmpRouterGetRequest: {
    _alias: {
      keys_: 'keys',
    },
    source: 'IsmpHostStateMachine',
    dest: 'IsmpHostStateMachine',
    nonce: 'u64',
    from: 'Bytes',
    keys_: 'Vec<Bytes>',
    height: 'u64',
    context: 'Bytes',
    timeoutTimestamp: 'u64',
  },
  /**
   * Lookup279: ismp::router::Response
   **/
  IsmpRouterResponse: {
    _enum: {
      Post: 'IsmpRouterPostResponse',
      Get: 'IsmpRouterGetResponse',
    },
  },
  /**
   * Lookup280: ismp::router::PostResponse
   **/
  IsmpRouterPostResponse: {
    post: 'IsmpRouterPostRequest',
    response: 'Bytes',
    timeoutTimestamp: 'u64',
  },
  /**
   * Lookup281: ismp::router::GetResponse
   **/
  IsmpRouterGetResponse: {
    get: 'IsmpRouterGetRequest',
    values: 'Vec<IsmpRouterStorageValue>',
  },
  /**
   * Lookup283: ismp::router::StorageValue
   **/
  IsmpRouterStorageValue: {
    key: 'Bytes',
    value: 'Option<Bytes>',
  },
  /**
   * Lookup285: ismp::messaging::TimeoutMessage
   **/
  IsmpMessagingTimeoutMessage: {
    _enum: {
      Post: {
        requests: 'Vec<IsmpRouterRequest>',
        timeoutProof: 'IsmpMessagingProof',
      },
      PostResponse: {
        responses: 'Vec<IsmpRouterPostResponse>',
        timeoutProof: 'IsmpMessagingProof',
      },
      Get: {
        requests: 'Vec<IsmpRouterRequest>',
      },
    },
  },
  /**
   * Lookup287: ismp::messaging::CreateConsensusState
   **/
  IsmpMessagingCreateConsensusState: {
    consensusState: 'Bytes',
    consensusClientId: '[u8;4]',
    consensusStateId: '[u8;4]',
    unbondingPeriod: 'u64',
    challengePeriods: 'BTreeMap<IsmpHostStateMachine, u64>',
    stateMachineCommitments:
      'Vec<(IsmpConsensusStateMachineId,IsmpMessagingStateCommitmentHeight)>',
  },
  /**
   * Lookup293: ismp::messaging::StateCommitmentHeight
   **/
  IsmpMessagingStateCommitmentHeight: {
    commitment: 'IsmpConsensusStateCommitment',
    height: 'u64',
  },
  /**
   * Lookup294: ismp::consensus::StateCommitment
   **/
  IsmpConsensusStateCommitment: {
    timestamp: 'u64',
    overlayRoot: 'Option<H256>',
    stateRoot: 'H256',
  },
  /**
   * Lookup295: pallet_ismp::utils::UpdateConsensusState
   **/
  PalletIsmpUtilsUpdateConsensusState: {
    consensusStateId: '[u8;4]',
    unbondingPeriod: 'Option<u64>',
    challengePeriods: 'BTreeMap<IsmpHostStateMachine, u64>',
  },
  /**
   * Lookup296: pallet_ismp::utils::FundMessageParams<Balance>
   **/
  PalletIsmpUtilsFundMessageParams: {
    commitment: 'PalletIsmpUtilsMessageCommitment',
    amount: 'u128',
  },
  /**
   * Lookup297: pallet_ismp::utils::MessageCommitment
   **/
  PalletIsmpUtilsMessageCommitment: {
    _enum: {
      Request: 'H256',
      Response: 'H256',
    },
  },
  /**
   * Lookup298: ismp_grandpa::pallet::Call<T>
   **/
  IsmpGrandpaCall: {
    _enum: {
      add_state_machines: {
        newStateMachines: 'Vec<IsmpGrandpaAddStateMachine>',
      },
      remove_state_machines: {
        stateMachines: 'Vec<IsmpHostStateMachine>',
      },
    },
  },
  /**
   * Lookup300: ismp_grandpa::AddStateMachine
   **/
  IsmpGrandpaAddStateMachine: {
    stateMachine: 'IsmpHostStateMachine',
    slotDuration: 'u64',
  },
  /**
   * Lookup301: pallet_token_gateway::pallet::Call<T>
   **/
  PalletTokenGatewayCall: {
    _enum: {
      teleport: {
        params: 'PalletTokenGatewayTeleportParams',
      },
      set_token_gateway_addresses: {
        addresses: 'BTreeMap<IsmpHostStateMachine, Bytes>',
      },
      create_erc6160_asset: {
        asset: 'PalletTokenGatewayAssetRegistration',
      },
      update_erc6160_asset: {
        asset: 'TokenGatewayPrimitivesGatewayAssetUpdate',
      },
      update_asset_precision: {
        update: 'PalletTokenGatewayPrecisionUpdate',
      },
    },
  },
  /**
   * Lookup302: pallet_token_gateway::types::TeleportParams<AssetId, Balance>
   **/
  PalletTokenGatewayTeleportParams: {
    assetId: 'u32',
    destination: 'IsmpHostStateMachine',
    recepient: 'H256',
    amount: 'u128',
    timeout: 'u64',
    tokenGateway: 'Bytes',
    relayerFee: 'u128',
    callData: 'Option<Bytes>',
    redeem: 'bool',
  },
  /**
   * Lookup306: pallet_token_gateway::types::AssetRegistration<AssetId>
   **/
  PalletTokenGatewayAssetRegistration: {
    localId: 'u32',
    reg: 'TokenGatewayPrimitivesGatewayAssetRegistration',
    native: 'bool',
    precision: 'BTreeMap<IsmpHostStateMachine, u8>',
  },
  /**
   * Lookup307: token_gateway_primitives::GatewayAssetRegistration
   **/
  TokenGatewayPrimitivesGatewayAssetRegistration: {
    name: 'Bytes',
    symbol: 'Bytes',
    chains: 'Vec<IsmpHostStateMachine>',
    minimumBalance: 'Option<u128>',
  },
  /**
   * Lookup312: token_gateway_primitives::GatewayAssetUpdate
   **/
  TokenGatewayPrimitivesGatewayAssetUpdate: {
    assetId: 'H256',
    addChains: 'Vec<IsmpHostStateMachine>',
    removeChains: 'Vec<IsmpHostStateMachine>',
    newAdmins: 'Vec<(IsmpHostStateMachine,H160)>',
  },
  /**
   * Lookup318: pallet_token_gateway::types::PrecisionUpdate<AssetId>
   **/
  PalletTokenGatewayPrecisionUpdate: {
    assetId: 'u32',
    precisions: 'BTreeMap<IsmpHostStateMachine, u8>',
  },
  /**
   * Lookup319: pallet_liquidity_pools::pallet::Call<T>
   **/
  PalletLiquidityPoolsCall: {
    _enum: {
      bond_argons: {
        vaultId: 'u32',
        amount: 'u128',
      },
      __Unused1: 'Null',
      unbond_argons: {
        vaultId: 'u32',
        frameId: 'u64',
      },
      vault_operator_prebond: {
        vaultId: 'u32',
        amount: 'u128',
        maxAmountPerFrame: 'u128',
      },
    },
  },
  /**
   * Lookup321: pallet_multisig::pallet::Error<T>
   **/
  PalletMultisigError: {
    _enum: [
      'MinimumThreshold',
      'AlreadyApproved',
      'NoApprovalsNeeded',
      'TooFewSignatories',
      'TooManySignatories',
      'SignatoriesOutOfOrder',
      'SenderInSignatories',
      'NotFound',
      'NotOwner',
      'NoTimepoint',
      'WrongTimepoint',
      'UnexpectedTimepoint',
      'MaxWeightTooLow',
      'AlreadyStored',
    ],
  },
  /**
   * Lookup324: pallet_proxy::ProxyDefinition<sp_core::crypto::AccountId32, argon_runtime::ProxyType, BlockNumber>
   **/
  PalletProxyProxyDefinition: {
    delegate: 'AccountId32',
    proxyType: 'ArgonRuntimeProxyType',
    delay: 'u32',
  },
  /**
   * Lookup328: pallet_proxy::Announcement<sp_core::crypto::AccountId32, primitive_types::H256, BlockNumber>
   **/
  PalletProxyAnnouncement: {
    real: 'AccountId32',
    callHash: 'H256',
    height: 'u32',
  },
  /**
   * Lookup330: pallet_proxy::pallet::Error<T>
   **/
  PalletProxyError: {
    _enum: [
      'TooMany',
      'NotFound',
      'NotProxy',
      'Unproxyable',
      'Duplicate',
      'NoPermission',
      'Unannounced',
      'NoSelfProxy',
    ],
  },
  /**
   * Lookup331: argon_primitives::tick::Ticker
   **/
  ArgonPrimitivesTickTicker: {
    tickDurationMillis: 'Compact<u64>',
    channelHoldExpirationTicks: 'Compact<u64>',
  },
  /**
   * Lookup333: pallet_ticks::pallet::Error<T>
   **/
  PalletTicksError: 'Null',
  /**
   * Lookup346: argon_primitives::block_seal::MiningBidStats
   **/
  ArgonPrimitivesBlockSealMiningBidStats: {
    bidsCount: 'u32',
    bidAmountMin: 'u128',
    bidAmountMax: 'u128',
    bidAmountSum: 'u128',
  },
  /**
   * Lookup350: argon_primitives::block_seal::MiningSlotConfig
   **/
  ArgonPrimitivesBlockSealMiningSlotConfig: {
    ticksBeforeBidEndForVrfClose: 'Compact<u64>',
    ticksBetweenSlots: 'Compact<u64>',
    slotBiddingStartAfterTicks: 'Compact<u64>',
  },
  /**
   * Lookup354: pallet_mining_slot::pallet::Error<T>
   **/
  PalletMiningSlotError: {
    _enum: [
      'SlotNotTakingBids',
      'TooManyBlockRegistrants',
      'InsufficientOwnershipTokens',
      'BidTooLow',
      'CannotRegisterOverlappingSessions',
      'InsufficientFunds',
      'BidCannotBeReduced',
      'InvalidBidAmount',
      'UnrecoverableHold',
    ],
  },
  /**
   * Lookup355: argon_primitives::bitcoin::UtxoValue
   **/
  ArgonPrimitivesBitcoinUtxoValue: {
    utxoId: 'u64',
    scriptPubkey: 'ArgonPrimitivesBitcoinBitcoinCosignScriptPubkey',
    satoshis: 'Compact<u64>',
    submittedAtHeight: 'Compact<u64>',
    watchForSpentUntilHeight: 'Compact<u64>',
  },
  /**
   * Lookup356: argon_primitives::bitcoin::BitcoinCosignScriptPubkey
   **/
  ArgonPrimitivesBitcoinBitcoinCosignScriptPubkey: {
    _enum: {
      P2WSH: {
        wscriptHash: 'H256',
      },
    },
  },
  /**
   * Lookup361: argon_primitives::bitcoin::BitcoinNetwork
   **/
  ArgonPrimitivesBitcoinBitcoinNetwork: {
    _enum: ['Bitcoin', 'Testnet', 'Signet', 'Regtest'],
  },
  /**
   * Lookup364: pallet_bitcoin_utxos::pallet::Error<T>
   **/
  PalletBitcoinUtxosError: {
    _enum: [
      'NoPermissions',
      'NoBitcoinConfirmedBlock',
      'InsufficientBitcoinAmount',
      'NoBitcoinPricesAvailable',
      'ScriptPubkeyConflict',
      'UtxoNotLocked',
      'RedemptionsUnavailable',
      'InvalidBitcoinSyncHeight',
      'BitcoinHeightNotConfirmed',
      'MaxUtxosExceeded',
      'InvalidBitcoinScript',
      'DuplicateUtxoId',
    ],
  },
  /**
   * Lookup365: argon_primitives::vault::Vault<sp_core::crypto::AccountId32, Balance>
   **/
  ArgonPrimitivesVault: {
    operatorAccountId: 'AccountId32',
    securitization: 'Compact<u128>',
    argonsLocked: 'Compact<u128>',
    argonsPendingActivation: 'Compact<u128>',
    argonsScheduledForRelease: 'BTreeMap<u64, u128>',
    securitizationRatio: 'Compact<u128>',
    isClosed: 'bool',
    terms: 'ArgonPrimitivesVaultVaultTerms',
    pendingTerms: 'Option<(u64,ArgonPrimitivesVaultVaultTerms)>',
    openedTick: 'Compact<u64>',
  },
  /**
   * Lookup373: argon_primitives::bitcoin::BitcoinXPub
   **/
  ArgonPrimitivesBitcoinBitcoinXPub: {
    publicKey: 'ArgonPrimitivesBitcoinCompressedBitcoinPubkey',
    depth: 'Compact<u8>',
    parentFingerprint: '[u8;4]',
    childNumber: 'Compact<u32>',
    chainCode: '[u8;32]',
    network: 'ArgonPrimitivesBitcoinNetworkKind',
  },
  /**
   * Lookup375: argon_primitives::bitcoin::NetworkKind
   **/
  ArgonPrimitivesBitcoinNetworkKind: {
    _enum: ['Main', 'Test'],
  },
  /**
   * Lookup380: pallet_vaults::pallet::VaultFrameFeeRevenue<T>
   **/
  PalletVaultsVaultFrameFeeRevenue: {
    frameId: 'Compact<u64>',
    feeRevenue: 'Compact<u128>',
    bitcoinLocksCreated: 'Compact<u32>',
    bitcoinLocksMarketValue: 'Compact<u128>',
    bitcoinLocksTotalSatoshis: 'Compact<u64>',
    satoshisReleased: 'Compact<u64>',
  },
  /**
   * Lookup382: pallet_vaults::pallet::Error<T>
   **/
  PalletVaultsError: {
    _enum: [
      'NoMoreVaultIds',
      'InsufficientFunds',
      'InsufficientVaultFunds',
      'AccountBelowMinimumBalance',
      'VaultClosed',
      'InvalidVaultAmount',
      'VaultReductionBelowSecuritization',
      'InvalidSecuritization',
      'ReusedVaultBitcoinXpub',
      'InvalidBitcoinScript',
      'InvalidXpubkey',
      'WrongXpubNetwork',
      'UnsafeXpubkey',
      'UnableToDeriveVaultXpubChild',
      'BitcoinConversionFailed',
      'NoPermissions',
      'HoldUnexpectedlyModified',
      'UnrecoverableHold',
      'VaultNotFound',
      'VaultNotYetActive',
      'NoVaultBitcoinPubkeysAvailable',
      'TermsModificationOverflow',
      'TermsChangeAlreadyScheduled',
      'InternalError',
      'UnableToGenerateVaultBitcoinPubkey',
      'FundingChangeAlreadyScheduled',
    ],
  },
  /**
   * Lookup383: pallet_bitcoin_locks::pallet::LockedBitcoin<T>
   **/
  PalletBitcoinLocksLockedBitcoin: {
    vaultId: 'Compact<u32>',
    lockPrice: 'u128',
    ownerAccount: 'AccountId32',
    satoshis: 'Compact<u64>',
    vaultPubkey: 'ArgonPrimitivesBitcoinCompressedBitcoinPubkey',
    vaultClaimPubkey: 'ArgonPrimitivesBitcoinCompressedBitcoinPubkey',
    vaultXpubSources: '([u8;4],u32,u32)',
    ownerPubkey: 'ArgonPrimitivesBitcoinCompressedBitcoinPubkey',
    vaultClaimHeight: 'Compact<u64>',
    openClaimHeight: 'Compact<u64>',
    createdAtHeight: 'Compact<u64>',
    utxoScriptPubkey: 'ArgonPrimitivesBitcoinBitcoinCosignScriptPubkey',
    isVerified: 'bool',
    isRejectedNeedsRelease: 'bool',
    fundHoldExtensions: 'BTreeMap<u64, u128>',
  },
  /**
   * Lookup386: pallet_bitcoin_locks::pallet::LockReleaseRequest<Balance>
   **/
  PalletBitcoinLocksLockReleaseRequest: {
    utxoId: 'Compact<u64>',
    vaultId: 'Compact<u32>',
    bitcoinNetworkFee: 'Compact<u64>',
    cosignDueBlock: 'Compact<u64>',
    toScriptPubkey: 'Bytes',
    redemptionPrice: 'Compact<u128>',
  },
  /**
   * Lookup393: pallet_bitcoin_locks::pallet::Error<T>
   **/
  PalletBitcoinLocksError: {
    _enum: {
      InsufficientFunds: 'Null',
      InsufficientVaultFunds: 'Null',
      AccountWouldGoBelowMinimumBalance: 'Null',
      VaultClosed: 'Null',
      InvalidVaultAmount: 'Null',
      RedemptionNotLocked: 'Null',
      BitcoinReleaseInitiationDeadlinePassed: 'Null',
      BitcoinFeeTooHigh: 'Null',
      BitcoinUtxoNotFound: 'Null',
      BitcoinUnableToBeDecodedForRelease: 'Null',
      BitcoinSignatureUnableToBeDecoded: 'Null',
      BitcoinPubkeyUnableToBeDecoded: 'Null',
      BitcoinInvalidCosignature: 'Null',
      InsufficientSatoshisLocked: 'Null',
      NoBitcoinPricesAvailable: 'Null',
      InvalidBitcoinScript: 'Null',
      NoPermissions: 'Null',
      HoldUnexpectedlyModified: 'Null',
      UnrecoverableHold: 'Null',
      VaultNotFound: 'Null',
      GenericVaultError: 'ArgonPrimitivesVaultVaultError',
      LockNotFound: 'Null',
      NoVaultBitcoinPubkeysAvailable: 'Null',
      UnableToGenerateVaultBitcoinPubkey: 'Null',
      VaultNotYetActive: 'Null',
      ExpirationAtBlockOverflow: 'Null',
      NoRatchetingAvailable: 'Null',
      LockInProcessOfRelease: 'Null',
      UnverifiedLock: 'Null',
      OverflowError: 'Null',
    },
  },
  /**
   * Lookup394: argon_primitives::vault::VaultError
   **/
  ArgonPrimitivesVaultVaultError: {
    _enum: [
      'VaultClosed',
      'AccountWouldBeBelowMinimum',
      'InsufficientFunds',
      'InsufficientVaultFunds',
      'HoldUnexpectedlyModified',
      'UnrecoverableHold',
      'VaultNotFound',
      'NoVaultBitcoinPubkeysAvailable',
      'UnableToGenerateVaultBitcoinPubkey',
      'InvalidBitcoinScript',
      'InternalError',
      'VaultNotYetActive',
    ],
  },
  /**
   * Lookup406: pallet_notaries::pallet::Error<T>
   **/
  PalletNotariesError: {
    _enum: [
      'ProposalNotFound',
      'MaxNotariesExceeded',
      'MaxProposalsPerBlockExceeded',
      'NotAnActiveNotary',
      'InvalidNotaryOperator',
      'NoMoreNotaryIds',
      'EffectiveTickTooSoon',
      'TooManyKeys',
      'InvalidNotary',
    ],
  },
  /**
   * Lookup410: argon_primitives::notary::NotaryNotebookKeyDetails
   **/
  ArgonPrimitivesNotaryNotaryNotebookKeyDetails: {
    notebookNumber: 'Compact<u32>',
    tick: 'Compact<u64>',
    blockVotesRoot: 'H256',
    secretHash: 'H256',
    parentSecret: 'Option<H256>',
  },
  /**
   * Lookup413: pallet_notebook::pallet::Error<T>
   **/
  PalletNotebookError: {
    _enum: [
      'DuplicateNotebookNumber',
      'MissingNotebookNumber',
      'NotebookTickAlreadyUsed',
      'InvalidNotebookSignature',
      'InvalidSecretProvided',
      'CouldNotDecodeNotebook',
      'DuplicateNotebookDigest',
      'MissingNotebookDigest',
      'InvalidNotebookDigest',
      'MultipleNotebookInherentsProvided',
      'InternalError',
      'NotebookSubmittedForLockedNotary',
      'InvalidReprocessNotebook',
      'InvalidNotaryOperator',
      'InvalidNotebookSubmissionTick',
    ],
  },
  /**
   * Lookup414: pallet_chain_transfer::QueuedTransferOut<sp_core::crypto::AccountId32, Balance>
   **/
  PalletChainTransferQueuedTransferOut: {
    accountId: 'AccountId32',
    amount: 'u128',
    expirationTick: 'u64',
    notaryId: 'u32',
  },
  /**
   * Lookup420: frame_support::PalletId
   **/
  FrameSupportPalletId: '[u8;8]',
  /**
   * Lookup421: pallet_chain_transfer::pallet::Error<T>
   **/
  PalletChainTransferError: {
    _enum: [
      'MaxBlockTransfersExceeded',
      'InsufficientFunds',
      'InsufficientNotarizedFunds',
      'InvalidOrDuplicatedLocalchainTransfer',
      'NotebookIncludesExpiredLocalchainTransfer',
      'InvalidNotaryUsedForTransfer',
    ],
  },
  /**
   * Lookup425: argon_primitives::notary::NotaryNotebookVoteDigestDetails
   **/
  ArgonPrimitivesNotaryNotaryNotebookVoteDigestDetails: {
    notaryId: 'Compact<u32>',
    notebookNumber: 'Compact<u32>',
    tick: 'Compact<u64>',
    blockVotesCount: 'Compact<u32>',
    blockVotingPower: 'Compact<u128>',
  },
  /**
   * Lookup430: pallet_block_seal_spec::pallet::Error<T>
   **/
  PalletBlockSealSpecError: {
    _enum: ['MaxNotebooksAtTickExceeded'],
  },
  /**
   * Lookup432: pallet_domains::pallet::Error<T>
   **/
  PalletDomainsError: {
    _enum: [
      'DomainNotRegistered',
      'NotDomainOwner',
      'FailedToAddToAddressHistory',
      'FailedToAddExpiringDomain',
      'AccountDecodingError',
    ],
  },
  /**
   * Lookup433: pallet_price_index::pallet::Error<T>
   **/
  PalletPriceIndexError: {
    _enum: [
      'NotAuthorizedOperator',
      'MissingValue',
      'PricesTooOld',
      'MaxPriceChangePerTickExceeded',
    ],
  },
  /**
   * Lookup434: pallet_grandpa::StoredState<N>
   **/
  PalletGrandpaStoredState: {
    _enum: {
      Live: 'Null',
      PendingPause: {
        scheduledAt: 'u32',
        delay: 'u32',
      },
      Paused: 'Null',
      PendingResume: {
        scheduledAt: 'u32',
        delay: 'u32',
      },
    },
  },
  /**
   * Lookup435: pallet_grandpa::StoredPendingChange<N, Limit>
   **/
  PalletGrandpaStoredPendingChange: {
    scheduledAt: 'u32',
    delay: 'u32',
    nextAuthorities: 'Vec<(SpConsensusGrandpaAppPublic,u64)>',
    forced: 'Option<u32>',
  },
  /**
   * Lookup438: pallet_grandpa::pallet::Error<T>
   **/
  PalletGrandpaError: {
    _enum: [
      'PauseFailed',
      'ResumeFailed',
      'ChangePending',
      'TooSoon',
      'InvalidKeyOwnershipProof',
      'InvalidEquivocationProof',
      'DuplicateOffenceReport',
    ],
  },
  /**
   * Lookup439: argon_primitives::providers::BlockSealerInfo<sp_core::crypto::AccountId32>
   **/
  ArgonPrimitivesProvidersBlockSealerInfo: {
    blockAuthorAccountId: 'AccountId32',
    blockVoteRewardsAccount: 'Option<AccountId32>',
    blockSealAuthority: 'Option<ArgonPrimitivesBlockSealAppPublic>',
  },
  /**
   * Lookup442: pallet_block_seal::pallet::Error<T>
   **/
  PalletBlockSealError: {
    _enum: [
      'InvalidVoteSealStrength',
      'InvalidSubmitter',
      'UnableToDecodeVoteAccount',
      'UnregisteredBlockAuthor',
      'InvalidBlockVoteProof',
      'NoGrandparentVoteMinimum',
      'DuplicateBlockSealProvided',
      'InsufficientVotingPower',
      'ParentVotingKeyNotFound',
      'InvalidVoteGrandparentHash',
      'IneligibleNotebookUsed',
      'NoEligibleVotingRoot',
      'CouldNotDecodeVote',
      'MaxNotebooksAtTickExceeded',
      'NoClosestMinerFoundForVote',
      'BlockVoteInvalidSignature',
      'InvalidForkPowerParent',
      'BlockSealDecodeError',
      'InvalidComputeBlockTick',
      'InvalidMinerXorDistance',
    ],
  },
  /**
   * Lookup446: pallet_block_rewards::pallet::Error<T>
   **/
  PalletBlockRewardsError: 'Null',
  /**
   * Lookup452: pallet_mint::pallet::MintAction<B>
   **/
  PalletMintMintAction: {
    argonBurned: 'u128',
    argonMinted: 'u128',
    bitcoinMinted: 'u128',
  },
  /**
   * Lookup453: pallet_mint::pallet::Error<T>
   **/
  PalletMintError: {
    _enum: ['TooManyPendingMints'],
  },
  /**
   * Lookup455: pallet_balances::types::BalanceLock<Balance>
   **/
  PalletBalancesBalanceLock: {
    id: '[u8;8]',
    amount: 'u128',
    reasons: 'PalletBalancesReasons',
  },
  /**
   * Lookup456: pallet_balances::types::Reasons
   **/
  PalletBalancesReasons: {
    _enum: ['Fee', 'Misc', 'All'],
  },
  /**
   * Lookup459: pallet_balances::types::ReserveData<ReserveIdentifier, Balance>
   **/
  PalletBalancesReserveData: {
    id: '[u8;8]',
    amount: 'u128',
  },
  /**
   * Lookup462: frame_support::traits::tokens::misc::IdAmount<argon_runtime::RuntimeHoldReason, Balance>
   **/
  FrameSupportTokensMiscIdAmountRuntimeHoldReason: {
    id: 'ArgonRuntimeRuntimeHoldReason',
    amount: 'u128',
  },
  /**
   * Lookup463: argon_runtime::RuntimeHoldReason
   **/
  ArgonRuntimeRuntimeHoldReason: {
    _enum: {
      __Unused0: 'Null',
      __Unused1: 'Null',
      __Unused2: 'Null',
      __Unused3: 'Null',
      __Unused4: 'Null',
      __Unused5: 'Null',
      MiningSlot: 'PalletMiningSlotHoldReason',
      __Unused7: 'Null',
      Vaults: 'PalletVaultsHoldReason',
      BitcoinLocks: 'PalletBitcoinLocksHoldReason',
      __Unused10: 'Null',
      __Unused11: 'Null',
      __Unused12: 'Null',
      __Unused13: 'Null',
      __Unused14: 'Null',
      __Unused15: 'Null',
      __Unused16: 'Null',
      __Unused17: 'Null',
      __Unused18: 'Null',
      BlockRewards: 'PalletBlockRewardsHoldReason',
      __Unused20: 'Null',
      __Unused21: 'Null',
      __Unused22: 'Null',
      __Unused23: 'Null',
      __Unused24: 'Null',
      __Unused25: 'Null',
      __Unused26: 'Null',
      __Unused27: 'Null',
      __Unused28: 'Null',
      __Unused29: 'Null',
      __Unused30: 'Null',
      LiquidityPools: 'PalletLiquidityPoolsHoldReason',
    },
  },
  /**
   * Lookup464: pallet_mining_slot::pallet::HoldReason
   **/
  PalletMiningSlotHoldReason: {
    _enum: ['RegisterAsMiner'],
  },
  /**
   * Lookup465: pallet_vaults::pallet::HoldReason
   **/
  PalletVaultsHoldReason: {
    _enum: ['EnterVault', 'ObligationFee'],
  },
  /**
   * Lookup466: pallet_bitcoin_locks::pallet::HoldReason
   **/
  PalletBitcoinLocksHoldReason: {
    _enum: ['ReleaseBitcoinLock'],
  },
  /**
   * Lookup467: pallet_block_rewards::pallet::HoldReason
   **/
  PalletBlockRewardsHoldReason: {
    _enum: ['MaturationPeriod'],
  },
  /**
   * Lookup468: pallet_liquidity_pools::pallet::HoldReason
   **/
  PalletLiquidityPoolsHoldReason: {
    _enum: ['ContributedToLiquidityPool'],
  },
  /**
   * Lookup471: frame_support::traits::tokens::misc::IdAmount<argon_runtime::RuntimeFreezeReason, Balance>
   **/
  FrameSupportTokensMiscIdAmountRuntimeFreezeReason: {
    id: 'ArgonRuntimeRuntimeFreezeReason',
    amount: 'u128',
  },
  /**
   * Lookup472: argon_runtime::RuntimeFreezeReason
   **/
  ArgonRuntimeRuntimeFreezeReason: {
    _enum: {
      __Unused0: 'Null',
      __Unused1: 'Null',
      __Unused2: 'Null',
      __Unused3: 'Null',
      __Unused4: 'Null',
      __Unused5: 'Null',
      __Unused6: 'Null',
      __Unused7: 'Null',
      __Unused8: 'Null',
      __Unused9: 'Null',
      __Unused10: 'Null',
      __Unused11: 'Null',
      __Unused12: 'Null',
      __Unused13: 'Null',
      __Unused14: 'Null',
      __Unused15: 'Null',
      __Unused16: 'Null',
      __Unused17: 'Null',
      __Unused18: 'Null',
      BlockRewards: 'PalletBlockRewardsFreezeReason',
    },
  },
  /**
   * Lookup473: pallet_block_rewards::pallet::FreezeReason
   **/
  PalletBlockRewardsFreezeReason: {
    _enum: ['MaturationPeriod'],
  },
  /**
   * Lookup475: pallet_balances::pallet::Error<T, I>
   **/
  PalletBalancesError: {
    _enum: [
      'VestingBalance',
      'LiquidityRestrictions',
      'InsufficientBalance',
      'ExistentialDeposit',
      'Expendability',
      'ExistingVestingSchedule',
      'DeadAccount',
      'TooManyReserves',
      'TooManyHolds',
      'TooManyFreezes',
      'IssuanceDeactivated',
      'DeltaZero',
    ],
  },
  /**
   * Lookup477: pallet_tx_pause::pallet::Error<T>
   **/
  PalletTxPauseError: {
    _enum: ['IsPaused', 'IsUnpaused', 'Unpausable', 'NotFound'],
  },
  /**
   * Lookup478: pallet_transaction_payment::Releases
   **/
  PalletTransactionPaymentReleases: {
    _enum: ['V1Ancient', 'V2'],
  },
  /**
   * Lookup479: pallet_utility::pallet::Error<T>
   **/
  PalletUtilityError: {
    _enum: ['TooManyCalls'],
  },
  /**
   * Lookup480: pallet_sudo::pallet::Error<T>
   **/
  PalletSudoError: {
    _enum: ['RequireSudo'],
  },
  /**
   * Lookup481: pallet_ismp::pallet::Error<T>
   **/
  PalletIsmpError: {
    _enum: [
      'InvalidMessage',
      'MessageNotFound',
      'ConsensusClientCreationFailed',
      'UnbondingPeriodUpdateFailed',
      'ChallengePeriodUpdateFailed',
    ],
  },
  /**
   * Lookup482: pallet_hyperbridge::pallet::Error<T>
   **/
  PalletHyperbridgeError: 'Null',
  /**
   * Lookup484: pallet_token_gateway::pallet::Error<T>
   **/
  PalletTokenGatewayError: {
    _enum: [
      'UnregisteredAsset',
      'AssetTeleportError',
      'CoprocessorNotConfigured',
      'DispatchError',
      'AssetCreationError',
      'AssetDecimalsNotFound',
      'NotInitialized',
      'UnknownAsset',
      'NotAssetOwner',
    ],
  },
  /**
   * Lookup486: pallet_liquidity_pools::pallet::LiquidityPool<T>
   **/
  PalletLiquidityPoolsLiquidityPool: {
    contributorBalances: 'Vec<(AccountId32,u128)>',
    doNotRenew: 'Vec<AccountId32>',
    isRolledOver: 'bool',
    distributedProfits: 'Option<u128>',
    vaultSharingPercent: 'Compact<Permill>',
  },
  /**
   * Lookup495: pallet_liquidity_pools::pallet::LiquidityPoolCapital<T>
   **/
  PalletLiquidityPoolsLiquidityPoolCapital: {
    vaultId: 'Compact<u32>',
    activatedCapital: 'Compact<u128>',
    frameId: 'Compact<u64>',
  },
  /**
   * Lookup497: pallet_liquidity_pools::pallet::PrebondedArgons<T>
   **/
  PalletLiquidityPoolsPrebondedArgons: {
    vaultId: 'Compact<u32>',
    accountId: 'AccountId32',
    amountUnbonded: 'Compact<u128>',
    startingFrameId: 'Compact<u64>',
    bondedByStartOffset: 'Vec<u128>',
    maxAmountPerFrame: 'Compact<u128>',
  },
  /**
   * Lookup498: pallet_liquidity_pools::pallet::Error<T>
   **/
  PalletLiquidityPoolsError: {
    _enum: [
      'ContributionTooLow',
      'VaultNotAcceptingMiningBonds',
      'BelowMinimum',
      'NotAFundContributor',
      'InternalError',
      'CouldNotFindLiquidityPool',
      'MaxContributorsExceeded',
      'ActivatedSecuritizationExceeded',
      'MaxVaultsExceeded',
      'AlreadyRenewed',
      'NotAVaultOperator',
      'MaxAmountBelowMinimum',
    ],
  },
  /**
   * Lookup501: frame_system::extensions::check_non_zero_sender::CheckNonZeroSender<T>
   **/
  FrameSystemExtensionsCheckNonZeroSender: 'Null',
  /**
   * Lookup502: frame_system::extensions::check_spec_version::CheckSpecVersion<T>
   **/
  FrameSystemExtensionsCheckSpecVersion: 'Null',
  /**
   * Lookup503: frame_system::extensions::check_tx_version::CheckTxVersion<T>
   **/
  FrameSystemExtensionsCheckTxVersion: 'Null',
  /**
   * Lookup504: frame_system::extensions::check_genesis::CheckGenesis<T>
   **/
  FrameSystemExtensionsCheckGenesis: 'Null',
  /**
   * Lookup507: frame_system::extensions::check_nonce::CheckNonce<T>
   **/
  FrameSystemExtensionsCheckNonce: 'Compact<u32>',
  /**
   * Lookup508: frame_system::extensions::check_weight::CheckWeight<T>
   **/
  FrameSystemExtensionsCheckWeight: 'Null',
  /**
   * Lookup509: pallet_transaction_payment::ChargeTransactionPayment<T>
   **/
  PalletTransactionPaymentChargeTransactionPayment: 'Compact<u128>',
  /**
   * Lookup510: frame_metadata_hash_extension::CheckMetadataHash<T>
   **/
  FrameMetadataHashExtensionCheckMetadataHash: {
    mode: 'FrameMetadataHashExtensionMode',
  },
  /**
   * Lookup511: frame_metadata_hash_extension::Mode
   **/
  FrameMetadataHashExtensionMode: {
    _enum: ['Disabled', 'Enabled'],
  },
  /**
   * Lookup512: frame_system::extensions::weight_reclaim::WeightReclaim<T>
   **/
  FrameSystemExtensionsWeightReclaim: 'Null',
  /**
   * Lookup514: argon_runtime::Runtime
   **/
  ArgonRuntimeRuntime: 'Null',
};
