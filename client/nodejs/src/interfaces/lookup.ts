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
    data: 'PalletBalancesAccountData'
  },
  /**
   * Lookup5: pallet_balances::types::AccountData<Balance>
   **/
  PalletBalancesAccountData: {
    free: 'u128',
    reserved: 'u128',
    frozen: 'u128',
    flags: 'u128'
  },
  /**
   * Lookup9: frame_support::dispatch::PerDispatchClass<sp_weights::weight_v2::Weight>
   **/
  FrameSupportDispatchPerDispatchClassWeight: {
    normal: 'SpWeightsWeightV2Weight',
    operational: 'SpWeightsWeightV2Weight',
    mandatory: 'SpWeightsWeightV2Weight'
  },
  /**
   * Lookup10: sp_weights::weight_v2::Weight
   **/
  SpWeightsWeightV2Weight: {
    refTime: 'Compact<u64>',
    proofSize: 'Compact<u64>'
  },
  /**
   * Lookup15: sp_runtime::generic::digest::Digest
   **/
  SpRuntimeDigest: {
    logs: 'Vec<SpRuntimeDigestDigestItem>'
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
      RuntimeEnvironmentUpdated: 'Null'
    }
  },
  /**
   * Lookup20: frame_system::EventRecord<ulx_node_runtime::RuntimeEvent, primitive_types::H256>
   **/
  FrameSystemEventRecord: {
    phase: 'FrameSystemPhase',
    event: 'Event',
    topics: 'Vec<H256>'
  },
  /**
   * Lookup22: frame_system::pallet::Event<T>
   **/
  FrameSystemEvent: {
    _enum: {
      ExtrinsicSuccess: {
        dispatchInfo: 'FrameSupportDispatchDispatchInfo',
      },
      ExtrinsicFailed: {
        dispatchError: 'SpRuntimeDispatchError',
        dispatchInfo: 'FrameSupportDispatchDispatchInfo',
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
        checkVersion: 'bool'
      }
    }
  },
  /**
   * Lookup23: frame_support::dispatch::DispatchInfo
   **/
  FrameSupportDispatchDispatchInfo: {
    weight: 'SpWeightsWeightV2Weight',
    class: 'FrameSupportDispatchDispatchClass',
    paysFee: 'FrameSupportDispatchPays'
  },
  /**
   * Lookup24: frame_support::dispatch::DispatchClass
   **/
  FrameSupportDispatchDispatchClass: {
    _enum: ['Normal', 'Operational', 'Mandatory']
  },
  /**
   * Lookup25: frame_support::dispatch::Pays
   **/
  FrameSupportDispatchPays: {
    _enum: ['Yes', 'No']
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
      RootNotAllowed: 'Null'
    }
  },
  /**
   * Lookup27: sp_runtime::ModuleError
   **/
  SpRuntimeModuleError: {
    index: 'u8',
    error: '[u8;4]'
  },
  /**
   * Lookup28: sp_runtime::TokenError
   **/
  SpRuntimeTokenError: {
    _enum: ['FundsUnavailable', 'OnlyProvider', 'BelowMinimum', 'CannotCreate', 'UnknownAsset', 'Frozen', 'Unsupported', 'CannotCreateHold', 'NotExpendable', 'Blocked']
  },
  /**
   * Lookup29: sp_arithmetic::ArithmeticError
   **/
  SpArithmeticArithmeticError: {
    _enum: ['Underflow', 'Overflow', 'DivisionByZero']
  },
  /**
   * Lookup30: sp_runtime::TransactionalError
   **/
  SpRuntimeTransactionalError: {
    _enum: ['LimitReached', 'NoLayer']
  },
  /**
   * Lookup31: pallet_multisig::pallet::Event<T>
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
        callHash: '[u8;32]'
      }
    }
  },
  /**
   * Lookup32: pallet_multisig::Timepoint<BlockNumber>
   **/
  PalletMultisigTimepoint: {
    height: 'u32',
    index: 'u32'
  },
  /**
   * Lookup35: pallet_proxy::pallet::Event<T>
   **/
  PalletProxyEvent: {
    _enum: {
      ProxyExecuted: {
        result: 'Result<Null, SpRuntimeDispatchError>',
      },
      PureCreated: {
        pure: 'AccountId32',
        who: 'AccountId32',
        proxyType: 'UlxNodeRuntimeProxyType',
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
        proxyType: 'UlxNodeRuntimeProxyType',
        delay: 'u32',
      },
      ProxyRemoved: {
        delegator: 'AccountId32',
        delegatee: 'AccountId32',
        proxyType: 'UlxNodeRuntimeProxyType',
        delay: 'u32'
      }
    }
  },
  /**
   * Lookup36: ulx_node_runtime::ProxyType
   **/
  UlxNodeRuntimeProxyType: {
    _enum: ['Any', 'NonTransfer', 'PriceIndex']
  },
  /**
   * Lookup38: pallet_mining_slot::pallet::Event<T>
   **/
  PalletMiningSlotEvent: {
    _enum: {
      NewMiners: {
        startIndex: 'u32',
        newMiners: 'Vec<UlxPrimitivesBlockSealMiningRegistration>',
      },
      SlotBidderAdded: {
        accountId: 'AccountId32',
        bidAmount: 'u128',
        index: 'u32',
      },
      SlotBidderReplaced: {
        accountId: 'AccountId32',
        bondId: 'Option<u64>',
        keptOwnershipBond: 'bool',
      },
      UnbondedMiner: {
        accountId: 'AccountId32',
        bondId: 'Option<u64>',
        keptOwnershipBond: 'bool',
      },
      UnbondMinerError: {
        accountId: 'AccountId32',
        bondId: 'Option<u64>',
        error: 'SpRuntimeDispatchError'
      }
    }
  },
  /**
   * Lookup40: ulx_primitives::block_seal::MiningRegistration<sp_core::crypto::AccountId32, Balance>
   **/
  UlxPrimitivesBlockSealMiningRegistration: {
    accountId: 'AccountId32',
    rewardDestination: 'UlxPrimitivesBlockSealRewardDestination',
    bondId: 'Option<u64>',
    bondAmount: 'Compact<u128>',
    ownershipTokens: 'Compact<u128>',
    rewardSharing: 'Option<UlxPrimitivesBlockSealRewardSharing>'
  },
  /**
   * Lookup41: ulx_primitives::block_seal::RewardDestination<sp_core::crypto::AccountId32>
   **/
  UlxPrimitivesBlockSealRewardDestination: {
    _enum: {
      Owner: 'Null',
      Account: 'AccountId32'
    }
  },
  /**
   * Lookup45: ulx_primitives::block_seal::RewardSharing<sp_core::crypto::AccountId32>
   **/
  UlxPrimitivesBlockSealRewardSharing: {
    accountId: 'AccountId32',
    percentTake: 'Compact<Percent>'
  },
  /**
   * Lookup49: pallet_bitcoin_utxos::pallet::Event<T>
   **/
  PalletBitcoinUtxosEvent: {
    _enum: {
      UtxoVerified: {
        utxoId: 'u64',
      },
      UtxoRejected: {
        utxoId: 'u64',
        rejectedReason: 'UlxPrimitivesBitcoinBitcoinRejectedReason',
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
        utxoRef: 'UlxPrimitivesBitcoinUtxoRef',
        error: 'SpRuntimeDispatchError'
      }
    }
  },
  /**
   * Lookup50: ulx_primitives::bitcoin::BitcoinRejectedReason
   **/
  UlxPrimitivesBitcoinBitcoinRejectedReason: {
    _enum: ['SatoshisMismatch', 'Spent', 'LookupExpired', 'DuplicateUtxo']
  },
  /**
   * Lookup51: ulx_primitives::bitcoin::UtxoRef
   **/
  UlxPrimitivesBitcoinUtxoRef: {
    txid: 'UlxPrimitivesBitcoinH256Le',
    outputIndex: 'Compact<u32>'
  },
  /**
   * Lookup52: ulx_primitives::bitcoin::H256Le
   **/
  UlxPrimitivesBitcoinH256Le: '[u8;32]',
  /**
   * Lookup54: pallet_vaults::pallet::Event<T>
   **/
  PalletVaultsEvent: {
    _enum: {
      VaultCreated: {
        vaultId: 'u32',
        bitcoinArgons: 'u128',
        miningArgons: 'u128',
        securitizationPercent: 'u128',
        operatorAccountId: 'AccountId32',
      },
      VaultModified: {
        vaultId: 'u32',
        bitcoinArgons: 'u128',
        miningArgons: 'u128',
        securitizationPercent: 'u128',
      },
      VaultTermsChangeScheduled: {
        vaultId: 'u32',
        changeBlock: 'u32',
      },
      VaultTermsChanged: {
        vaultId: 'u32',
      },
      VaultClosed: {
        vaultId: 'u32',
        bitcoinAmountStillBonded: 'u128',
        miningAmountStillBonded: 'u128',
        securitizationStillBonded: 'u128'
      }
    }
  },
  /**
   * Lookup56: pallet_bond::pallet::Event<T>
   **/
  PalletBondEvent: {
    _enum: {
      BondCreated: {
        vaultId: 'u32',
        bondId: 'u64',
        bondType: 'UlxPrimitivesBondBondType',
        bondedAccountId: 'AccountId32',
        utxoId: 'Option<u64>',
        amount: 'u128',
        expiration: 'UlxPrimitivesBondBondExpiration',
      },
      BondCompleted: {
        vaultId: 'u32',
        bondId: 'u64',
      },
      BondCanceled: {
        vaultId: 'u32',
        bondId: 'u64',
        bondedAccountId: 'AccountId32',
        bondType: 'UlxPrimitivesBondBondType',
        returnedFee: 'u128',
      },
      BitcoinBondBurned: {
        vaultId: 'u32',
        bondId: 'u64',
        utxoId: 'u64',
        amountBurned: 'u128',
        amountHeld: 'u128',
        wasUtxoSpent: 'bool',
      },
      BitcoinUtxoCosignRequested: {
        bondId: 'u64',
        vaultId: 'u32',
        utxoId: 'u64',
      },
      BitcoinUtxoCosigned: {
        bondId: 'u64',
        vaultId: 'u32',
        utxoId: 'u64',
        pubkey: 'UlxPrimitivesBitcoinCompressedBitcoinPubkey',
        signature: 'Bytes',
      },
      BitcoinCosignPastDue: {
        bondId: 'u64',
        vaultId: 'u32',
        utxoId: 'u64',
        compensationAmount: 'u128',
        compensationStillOwed: 'u128',
        compensatedAccountId: 'AccountId32',
      },
      BondCompletionError: {
        bondId: 'u64',
        error: 'SpRuntimeDispatchError',
      },
      CosignOverdueError: {
        utxoId: 'u64',
        error: 'SpRuntimeDispatchError'
      }
    }
  },
  /**
   * Lookup57: ulx_primitives::bond::BondType
   **/
  UlxPrimitivesBondBondType: {
    _enum: ['Mining', 'Bitcoin']
  },
  /**
   * Lookup58: ulx_primitives::bond::BondExpiration<BlockNumber>
   **/
  UlxPrimitivesBondBondExpiration: {
    _enum: {
      UlixeeBlock: 'Compact<u32>',
      BitcoinBlock: 'Compact<u64>'
    }
  },
  /**
   * Lookup59: ulx_primitives::bitcoin::CompressedBitcoinPubkey
   **/
  UlxPrimitivesBitcoinCompressedBitcoinPubkey: '[u8;33]',
  /**
   * Lookup63: pallet_notaries::pallet::Event<T>
   **/
  PalletNotariesEvent: {
    _enum: {
      NotaryProposed: {
        operatorAccount: 'AccountId32',
        meta: 'UlxPrimitivesNotaryNotaryMeta',
        expires: 'u32',
      },
      NotaryActivated: {
        notary: 'UlxPrimitivesNotaryNotaryRecord',
      },
      NotaryMetaUpdateQueued: {
        notaryId: 'u32',
        meta: 'UlxPrimitivesNotaryNotaryMeta',
        effectiveTick: 'u32',
      },
      NotaryMetaUpdated: {
        notaryId: 'u32',
        meta: 'UlxPrimitivesNotaryNotaryMeta',
      },
      NotaryMetaUpdateError: {
        notaryId: 'u32',
        error: 'SpRuntimeDispatchError',
        meta: 'UlxPrimitivesNotaryNotaryMeta'
      }
    }
  },
  /**
   * Lookup64: ulx_primitives::notary::NotaryMeta<MaxHosts>
   **/
  UlxPrimitivesNotaryNotaryMeta: {
    name: 'Bytes',
    public: '[u8;32]',
    hosts: 'Vec<Bytes>'
  },
  /**
   * Lookup71: ulx_primitives::notary::NotaryRecord<sp_core::crypto::AccountId32, BlockNumber, MaxHosts>
   **/
  UlxPrimitivesNotaryNotaryRecord: {
    notaryId: 'Compact<u32>',
    operatorAccountId: 'AccountId32',
    activatedBlock: 'Compact<u32>',
    metaUpdatedBlock: 'Compact<u32>',
    metaUpdatedTick: 'Compact<u32>',
    meta: 'UlxPrimitivesNotaryNotaryMeta'
  },
  /**
   * Lookup72: pallet_notebook::pallet::Event<T>
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
        firstFailureReason: 'UlxNotaryAuditErrorVerifyError'
      }
    }
  },
  /**
   * Lookup73: ulx_notary_audit::error::VerifyError
   **/
  UlxNotaryAuditErrorVerifyError: {
    _enum: {
      MissingAccountOrigin: {
        accountId: 'AccountId32',
        accountType: 'UlxPrimitivesAccountAccountType',
      },
      HistoryLookupError: {
        source: 'UlxNotaryAuditAccountHistoryLookupError',
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
        message: 'Text',
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
      AccountEscrowHoldDoesntExist: 'Null',
      AccountAlreadyHasEscrowHold: 'Null',
      EscrowHoldNotReadyForClaim: {
        currentTick: 'u32',
        claimTick: 'u32',
      },
      AccountLocked: 'Null',
      MissingEscrowHoldNote: 'Null',
      InvalidEscrowHoldNote: 'Null',
      InvalidEscrowClaimers: 'Null',
      EscrowNoteBelowMinimum: 'Null',
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
      BlockVoteDataDomainMismatch: 'Null',
      BlockVoteEscrowReused: 'Null'
    }
  },
  /**
   * Lookup74: ulx_primitives::account::AccountType
   **/
  UlxPrimitivesAccountAccountType: {
    _enum: ['Tax', 'Deposit']
  },
  /**
   * Lookup75: ulx_notary_audit::AccountHistoryLookupError
   **/
  UlxNotaryAuditAccountHistoryLookupError: {
    _enum: ['RootNotFound', 'LastChangeNotFound', 'InvalidTransferToLocalchain', 'BlockSpecificationNotFound']
  },
  /**
   * Lookup78: pallet_chain_transfer::pallet::Event<T>
   **/
  PalletChainTransferEvent: {
    _enum: {
      TransferToLocalchain: {
        accountId: 'AccountId32',
        amount: 'u128',
        transferId: 'u32',
        notaryId: 'u32',
        expirationTick: 'u32',
      },
      TransferToLocalchainExpired: {
        accountId: 'AccountId32',
        transferId: 'u32',
        notaryId: 'u32',
      },
      TransferIn: {
        accountId: 'AccountId32',
        amount: 'u128',
        notaryId: 'u32',
      },
      TransferInError: {
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
      PossibleInvalidTransferAllowed: {
        transferId: 'u32',
        notaryId: 'u32',
        notebookNumber: 'u32',
      },
      TaxationError: {
        notaryId: 'u32',
        notebookNumber: 'u32',
        tax: 'u128',
        error: 'SpRuntimeDispatchError'
      }
    }
  },
  /**
   * Lookup79: pallet_block_seal_spec::pallet::Event<T>
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
        newDifficulty: 'u128'
      }
    }
  },
  /**
   * Lookup80: pallet_data_domain::pallet::Event<T>
   **/
  PalletDataDomainEvent: {
    _enum: {
      ZoneRecordUpdated: {
        domainHash: 'H256',
        zoneRecord: 'UlxPrimitivesDataDomainZoneRecord',
      },
      DataDomainRegistered: {
        domainHash: 'H256',
        registration: 'PalletDataDomainDataDomainRegistration',
      },
      DataDomainRenewed: {
        domainHash: 'H256',
      },
      DataDomainExpired: {
        domainHash: 'H256',
      },
      DataDomainRegistrationCanceled: {
        domainHash: 'H256',
        registration: 'PalletDataDomainDataDomainRegistration',
      },
      DataDomainRegistrationError: {
        domainHash: 'H256',
        accountId: 'AccountId32',
        error: 'SpRuntimeDispatchError'
      }
    }
  },
  /**
   * Lookup81: ulx_primitives::data_domain::ZoneRecord<sp_core::crypto::AccountId32>
   **/
  UlxPrimitivesDataDomainZoneRecord: {
    paymentAccount: 'AccountId32',
    notaryId: 'u32',
    versions: 'BTreeMap<UlxPrimitivesDataDomainSemver, UlxPrimitivesDataDomainVersionHost>'
  },
  /**
   * Lookup83: ulx_primitives::data_domain::Semver
   **/
  UlxPrimitivesDataDomainSemver: {
    major: 'u32',
    minor: 'u32',
    patch: 'u32'
  },
  /**
   * Lookup84: ulx_primitives::data_domain::VersionHost
   **/
  UlxPrimitivesDataDomainVersionHost: {
    datastoreId: 'Bytes',
    host: 'Bytes'
  },
  /**
   * Lookup88: pallet_data_domain::DataDomainRegistration<sp_core::crypto::AccountId32>
   **/
  PalletDataDomainDataDomainRegistration: {
    accountId: 'AccountId32',
    registeredAtTick: 'u32'
  },
  /**
   * Lookup89: pallet_price_index::pallet::Event<T>
   **/
  PalletPriceIndexEvent: {
    _enum: {
      NewIndex: 'Null',
      OperatorChanged: {
        operatorId: 'AccountId32'
      }
    }
  },
  /**
   * Lookup90: pallet_session::pallet::Event
   **/
  PalletSessionEvent: {
    _enum: {
      NewSession: {
        sessionIndex: 'u32'
      }
    }
  },
  /**
   * Lookup91: pallet_block_rewards::pallet::Event<T>
   **/
  PalletBlockRewardsEvent: {
    _enum: {
      RewardCreated: {
        maturationBlock: 'u32',
        rewards: 'Vec<UlxPrimitivesBlockSealBlockPayout>',
      },
      RewardUnlocked: {
        rewards: 'Vec<UlxPrimitivesBlockSealBlockPayout>',
      },
      RewardUnlockError: {
        accountId: 'AccountId32',
        argons: 'Option<u128>',
        ulixees: 'Option<u128>',
        error: 'SpRuntimeDispatchError',
      },
      RewardCreateError: {
        accountId: 'AccountId32',
        argons: 'Option<u128>',
        ulixees: 'Option<u128>',
        error: 'SpRuntimeDispatchError'
      }
    }
  },
  /**
   * Lookup93: ulx_primitives::block_seal::BlockPayout<sp_core::crypto::AccountId32, Balance>
   **/
  UlxPrimitivesBlockSealBlockPayout: {
    accountId: 'AccountId32',
    ulixees: 'Compact<u128>',
    argons: 'Compact<u128>'
  },
  /**
   * Lookup95: pallet_grandpa::pallet::Event
   **/
  PalletGrandpaEvent: {
    _enum: {
      NewAuthorities: {
        authoritySet: 'Vec<(SpConsensusGrandpaAppPublic,u64)>',
      },
      Paused: 'Null',
      Resumed: 'Null'
    }
  },
  /**
   * Lookup98: sp_consensus_grandpa::app::Public
   **/
  SpConsensusGrandpaAppPublic: '[u8;32]',
  /**
   * Lookup99: pallet_offences::pallet::Event
   **/
  PalletOffencesEvent: {
    _enum: {
      Offence: {
        kind: '[u8;16]',
        timeslot: 'Bytes'
      }
    }
  },
  /**
   * Lookup101: pallet_mint::pallet::Event<T>
   **/
  PalletMintEvent: {
    _enum: {
      ArgonsMinted: {
        mintType: 'PalletMintMintType',
        accountId: 'AccountId32',
        utxoId: 'Option<u64>',
        amount: 'u128',
      },
      MintError: {
        mintType: 'PalletMintMintType',
        accountId: 'AccountId32',
        utxoId: 'Option<u64>',
        amount: 'u128',
        error: 'SpRuntimeDispatchError'
      }
    }
  },
  /**
   * Lookup102: pallet_mint::pallet::MintType
   **/
  PalletMintMintType: {
    _enum: ['Bitcoin', 'Ulixee']
  },
  /**
   * Lookup103: pallet_balances::pallet::Event<T, I>
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
        new_: 'u128'
      }
    }
  },
  /**
   * Lookup104: frame_support::traits::tokens::misc::BalanceStatus
   **/
  FrameSupportTokensMiscBalanceStatus: {
    _enum: ['Free', 'Reserved']
  },
  /**
   * Lookup106: pallet_tx_pause::pallet::Event<T>
   **/
  PalletTxPauseEvent: {
    _enum: {
      CallPaused: {
        fullName: '(Bytes,Bytes)',
      },
      CallUnpaused: {
        fullName: '(Bytes,Bytes)'
      }
    }
  },
  /**
   * Lookup109: pallet_transaction_payment::pallet::Event<T>
   **/
  PalletTransactionPaymentEvent: {
    _enum: {
      TransactionFeePaid: {
        who: 'AccountId32',
        actualFee: 'u128',
        tip: 'u128'
      }
    }
  },
  /**
   * Lookup110: pallet_sudo::pallet::Event<T>
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
        sudoResult: 'Result<Null, SpRuntimeDispatchError>'
      }
    }
  },
  /**
   * Lookup112: frame_system::Phase
   **/
  FrameSystemPhase: {
    _enum: {
      ApplyExtrinsic: 'u32',
      Finalization: 'Null',
      Initialization: 'Null'
    }
  },
  /**
   * Lookup116: frame_system::LastRuntimeUpgradeInfo
   **/
  FrameSystemLastRuntimeUpgradeInfo: {
    specVersion: 'Compact<u32>',
    specName: 'Text'
  },
  /**
   * Lookup117: frame_system::CodeUpgradeAuthorization<T>
   **/
  FrameSystemCodeUpgradeAuthorization: {
    codeHash: 'H256',
    checkVersion: 'bool'
  },
  /**
   * Lookup118: frame_system::pallet::Call<T>
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
        code: 'Bytes'
      }
    }
  },
  /**
   * Lookup122: frame_system::limits::BlockWeights
   **/
  FrameSystemLimitsBlockWeights: {
    baseBlock: 'SpWeightsWeightV2Weight',
    maxBlock: 'SpWeightsWeightV2Weight',
    perClass: 'FrameSupportDispatchPerDispatchClassWeightsPerClass'
  },
  /**
   * Lookup123: frame_support::dispatch::PerDispatchClass<frame_system::limits::WeightsPerClass>
   **/
  FrameSupportDispatchPerDispatchClassWeightsPerClass: {
    normal: 'FrameSystemLimitsWeightsPerClass',
    operational: 'FrameSystemLimitsWeightsPerClass',
    mandatory: 'FrameSystemLimitsWeightsPerClass'
  },
  /**
   * Lookup124: frame_system::limits::WeightsPerClass
   **/
  FrameSystemLimitsWeightsPerClass: {
    baseExtrinsic: 'SpWeightsWeightV2Weight',
    maxExtrinsic: 'Option<SpWeightsWeightV2Weight>',
    maxTotal: 'Option<SpWeightsWeightV2Weight>',
    reserved: 'Option<SpWeightsWeightV2Weight>'
  },
  /**
   * Lookup126: frame_system::limits::BlockLength
   **/
  FrameSystemLimitsBlockLength: {
    max: 'FrameSupportDispatchPerDispatchClassU32'
  },
  /**
   * Lookup127: frame_support::dispatch::PerDispatchClass<T>
   **/
  FrameSupportDispatchPerDispatchClassU32: {
    normal: 'u32',
    operational: 'u32',
    mandatory: 'u32'
  },
  /**
   * Lookup128: sp_weights::RuntimeDbWeight
   **/
  SpWeightsRuntimeDbWeight: {
    read: 'u64',
    write: 'u64'
  },
  /**
   * Lookup129: sp_version::RuntimeVersion
   **/
  SpVersionRuntimeVersion: {
    specName: 'Text',
    implName: 'Text',
    authoringVersion: 'u32',
    specVersion: 'u32',
    implVersion: 'u32',
    apis: 'Vec<([u8;8],u32)>',
    transactionVersion: 'u32',
    stateVersion: 'u8'
  },
  /**
   * Lookup134: frame_system::pallet::Error<T>
   **/
  FrameSystemError: {
    _enum: ['InvalidSpecName', 'SpecVersionNeedsToIncrease', 'FailedToExtractRuntimeVersion', 'NonDefaultComposite', 'NonZeroRefCount', 'CallFiltered', 'MultiBlockMigrationsOngoing', 'NothingAuthorized', 'Unauthorized']
  },
  /**
   * Lookup135: pallet_timestamp::pallet::Call<T>
   **/
  PalletTimestampCall: {
    _enum: {
      set: {
        now: 'Compact<u64>'
      }
    }
  },
  /**
   * Lookup137: pallet_multisig::Multisig<BlockNumber, Balance, sp_core::crypto::AccountId32, MaxApprovals>
   **/
  PalletMultisigMultisig: {
    when: 'PalletMultisigTimepoint',
    deposit: 'u128',
    depositor: 'AccountId32',
    approvals: 'Vec<AccountId32>'
  },
  /**
   * Lookup140: pallet_multisig::pallet::Call<T>
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
        callHash: '[u8;32]'
      }
    }
  },
  /**
   * Lookup142: pallet_proxy::pallet::Call<T>
   **/
  PalletProxyCall: {
    _enum: {
      proxy: {
        real: 'MultiAddress',
        forceProxyType: 'Option<UlxNodeRuntimeProxyType>',
        call: 'Call',
      },
      add_proxy: {
        delegate: 'MultiAddress',
        proxyType: 'UlxNodeRuntimeProxyType',
        delay: 'u32',
      },
      remove_proxy: {
        delegate: 'MultiAddress',
        proxyType: 'UlxNodeRuntimeProxyType',
        delay: 'u32',
      },
      remove_proxies: 'Null',
      create_pure: {
        proxyType: 'UlxNodeRuntimeProxyType',
        delay: 'u32',
        index: 'u16',
      },
      kill_pure: {
        spawner: 'MultiAddress',
        proxyType: 'UlxNodeRuntimeProxyType',
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
        forceProxyType: 'Option<UlxNodeRuntimeProxyType>',
        call: 'Call'
      }
    }
  },
  /**
   * Lookup147: pallet_ticks::pallet::Call<T>
   **/
  PalletTicksCall: 'Null',
  /**
   * Lookup148: pallet_mining_slot::pallet::Call<T>
   **/
  PalletMiningSlotCall: {
    _enum: {
      bid: {
        bondInfo: 'Option<PalletMiningSlotMiningSlotBid>',
        rewardDestination: 'UlxPrimitivesBlockSealRewardDestination'
      }
    }
  },
  /**
   * Lookup150: pallet_mining_slot::MiningSlotBid<VaultId, Balance>
   **/
  PalletMiningSlotMiningSlotBid: {
    vaultId: 'u32',
    amount: 'u128'
  },
  /**
   * Lookup151: pallet_bitcoin_utxos::pallet::Call<T>
   **/
  PalletBitcoinUtxosCall: {
    _enum: {
      sync: {
        utxoSync: 'UlxPrimitivesInherentsBitcoinUtxoSync',
      },
      set_confirmed_block: {
        bitcoinHeight: 'u64',
        bitcoinBlockHash: 'UlxPrimitivesBitcoinH256Le',
      },
      set_operator: {
        accountId: 'AccountId32'
      }
    }
  },
  /**
   * Lookup152: ulx_primitives::inherents::BitcoinUtxoSync
   **/
  UlxPrimitivesInherentsBitcoinUtxoSync: {
    spent: 'BTreeMap<u64, u64>',
    verified: 'BTreeMap<u64, UlxPrimitivesBitcoinUtxoRef>',
    invalid: 'BTreeMap<u64, UlxPrimitivesBitcoinBitcoinRejectedReason>',
    syncToBlock: 'UlxPrimitivesBitcoinBitcoinBlock'
  },
  /**
   * Lookup162: ulx_primitives::bitcoin::BitcoinBlock
   **/
  UlxPrimitivesBitcoinBitcoinBlock: {
    blockHeight: 'Compact<u64>',
    blockHash: 'UlxPrimitivesBitcoinH256Le'
  },
  /**
   * Lookup163: pallet_vaults::pallet::Call<T>
   **/
  PalletVaultsCall: {
    _enum: {
      create: {
        vaultConfig: 'PalletVaultsVaultConfig',
      },
      modify_funding: {
        vaultId: 'u32',
        totalMiningAmountOffered: 'u128',
        totalBitcoinAmountOffered: 'u128',
        securitizationPercent: 'u128',
      },
      modify_terms: {
        vaultId: 'u32',
        terms: 'UlxPrimitivesBondVaultTerms',
      },
      close: {
        vaultId: 'u32',
      },
      add_bitcoin_pubkey_hashes: {
        vaultId: 'u32',
        bitcoinPubkeyHashes: 'Vec<UlxPrimitivesBitcoinBitcoinPubkeyHash>'
      }
    }
  },
  /**
   * Lookup164: pallet_vaults::pallet::VaultConfig<Balance, MaxPendingVaultBitcoinPubkeys>
   **/
  PalletVaultsVaultConfig: {
    terms: 'UlxPrimitivesBondVaultTerms',
    bitcoinAmountAllocated: 'Compact<u128>',
    bitcoinPubkeyHashes: 'Vec<UlxPrimitivesBitcoinBitcoinPubkeyHash>',
    miningAmountAllocated: 'Compact<u128>',
    securitizationPercent: 'Compact<u128>'
  },
  /**
   * Lookup165: ulx_primitives::bond::VaultTerms<Balance>
   **/
  UlxPrimitivesBondVaultTerms: {
    bitcoinAnnualPercentRate: 'Compact<u128>',
    bitcoinBaseFee: 'Compact<u128>',
    miningAnnualPercentRate: 'Compact<u128>',
    miningBaseFee: 'Compact<u128>',
    miningRewardSharingPercentTake: 'Compact<Percent>'
  },
  /**
   * Lookup168: ulx_primitives::bitcoin::BitcoinPubkeyHash
   **/
  UlxPrimitivesBitcoinBitcoinPubkeyHash: '[u8;20]',
  /**
   * Lookup170: pallet_bond::pallet::Call<T>
   **/
  PalletBondCall: {
    _enum: {
      bond_bitcoin: {
        vaultId: 'u32',
        satoshis: 'Compact<u64>',
        bitcoinPubkeyHash: 'UlxPrimitivesBitcoinBitcoinPubkeyHash',
      },
      __Unused1: 'Null',
      __Unused2: 'Null',
      __Unused3: 'Null',
      unlock_bitcoin_bond: {
        bondId: 'u64',
        toScriptPubkey: 'Bytes',
        bitcoinNetworkFee: 'u64',
      },
      cosign_bitcoin_unlock: {
        bondId: 'u64',
        pubkey: 'UlxPrimitivesBitcoinCompressedBitcoinPubkey',
        signature: 'Bytes'
      }
    }
  },
  /**
   * Lookup173: pallet_notaries::pallet::Call<T>
   **/
  PalletNotariesCall: {
    _enum: {
      propose: {
        meta: 'UlxPrimitivesNotaryNotaryMeta',
      },
      activate: {
        operatorAccount: 'AccountId32',
      },
      update: {
        notaryId: 'Compact<u32>',
        meta: 'UlxPrimitivesNotaryNotaryMeta',
        effectiveTick: 'Compact<u32>'
      }
    }
  },
  /**
   * Lookup174: pallet_notebook::pallet::Call<T>
   **/
  PalletNotebookCall: {
    _enum: {
      submit: {
        notebooks: 'Vec<UlxPrimitivesNotebookSignedNotebookHeader>'
      }
    }
  },
  /**
   * Lookup176: ulx_primitives::notebook::SignedNotebookHeader
   **/
  UlxPrimitivesNotebookSignedNotebookHeader: {
    header: 'UlxPrimitivesNotebookNotebookHeader',
    signature: '[u8;64]'
  },
  /**
   * Lookup177: ulx_primitives::notebook::NotebookHeader
   **/
  UlxPrimitivesNotebookNotebookHeader: {
    version: 'Compact<u16>',
    notebookNumber: 'Compact<u32>',
    tick: 'Compact<u32>',
    tax: 'Compact<u128>',
    notaryId: 'Compact<u32>',
    chainTransfers: 'Vec<UlxPrimitivesNotebookChainTransfer>',
    changedAccountsRoot: 'H256',
    changedAccountOrigins: 'Vec<UlxPrimitivesBalanceChangeAccountOrigin>',
    blockVotesRoot: 'H256',
    blockVotesCount: 'Compact<u32>',
    blocksWithVotes: 'Vec<H256>',
    blockVotingPower: 'Compact<u128>',
    secretHash: 'H256',
    parentSecret: 'Option<H256>',
    dataDomains: 'Vec<(H256,AccountId32)>'
  },
  /**
   * Lookup180: ulx_primitives::notebook::ChainTransfer
   **/
  UlxPrimitivesNotebookChainTransfer: {
    _enum: {
      ToMainchain: {
        accountId: 'AccountId32',
        amount: 'Compact<u128>',
      },
      ToLocalchain: {
        transferId: 'Compact<u32>'
      }
    }
  },
  /**
   * Lookup183: ulx_primitives::balance_change::AccountOrigin
   **/
  UlxPrimitivesBalanceChangeAccountOrigin: {
    notebookNumber: 'Compact<u32>',
    accountUid: 'Compact<u32>'
  },
  /**
   * Lookup191: pallet_chain_transfer::pallet::Call<T>
   **/
  PalletChainTransferCall: {
    _enum: {
      send_to_localchain: {
        amount: 'Compact<u128>',
        notaryId: 'u32'
      }
    }
  },
  /**
   * Lookup192: pallet_block_seal_spec::pallet::Call<T>
   **/
  PalletBlockSealSpecCall: {
    _enum: {
      configure: {
        voteMinimum: 'Option<u128>',
        computeDifficulty: 'Option<u128>'
      }
    }
  },
  /**
   * Lookup193: pallet_data_domain::pallet::Call<T>
   **/
  PalletDataDomainCall: {
    _enum: {
      set_zone_record: {
        domainHash: 'H256',
        zoneRecord: 'UlxPrimitivesDataDomainZoneRecord'
      }
    }
  },
  /**
   * Lookup194: pallet_price_index::pallet::Call<T>
   **/
  PalletPriceIndexCall: {
    _enum: {
      submit: {
        index: 'PalletPriceIndexPriceIndex',
      },
      set_operator: {
        accountId: 'AccountId32'
      }
    }
  },
  /**
   * Lookup195: pallet_price_index::PriceIndex
   **/
  PalletPriceIndexPriceIndex: {
    btcUsdPrice: 'Compact<u128>',
    argonUsdPrice: 'Compact<u128>',
    argonUsdTargetPrice: 'u128',
    tick: 'Compact<u32>'
  },
  /**
   * Lookup196: pallet_session::pallet::Call<T>
   **/
  PalletSessionCall: {
    _enum: {
      set_keys: {
        _alias: {
          keys_: 'keys',
        },
        keys_: 'UlxNodeRuntimeOpaqueSessionKeys',
        proof: 'Bytes',
      },
      purge_keys: 'Null'
    }
  },
  /**
   * Lookup197: ulx_node_runtime::opaque::SessionKeys
   **/
  UlxNodeRuntimeOpaqueSessionKeys: {
    grandpa: 'SpConsensusGrandpaAppPublic',
    blockSealAuthority: 'UlxPrimitivesBlockSealAppPublic'
  },
  /**
   * Lookup198: ulx_primitives::block_seal::app::Public
   **/
  UlxPrimitivesBlockSealAppPublic: '[u8;32]',
  /**
   * Lookup199: pallet_block_seal::pallet::Call<T>
   **/
  PalletBlockSealCall: {
    _enum: {
      apply: {
        seal: 'UlxPrimitivesInherentsBlockSealInherent'
      }
    }
  },
  /**
   * Lookup200: ulx_primitives::inherents::BlockSealInherent
   **/
  UlxPrimitivesInherentsBlockSealInherent: {
    _enum: {
      Vote: {
        sealStrength: 'U256',
        notaryId: 'Compact<u32>',
        sourceNotebookNumber: 'Compact<u32>',
        sourceNotebookProof: 'UlxPrimitivesBalanceChangeMerkleProof',
        blockVote: 'UlxPrimitivesBlockVoteBlockVoteT',
        minerSignature: 'UlxPrimitivesBlockSealAppSignature',
      },
      Compute: 'Null'
    }
  },
  /**
   * Lookup203: ulx_primitives::balance_change::MerkleProof
   **/
  UlxPrimitivesBalanceChangeMerkleProof: {
    proof: 'Vec<H256>',
    numberOfLeaves: 'Compact<u32>',
    leafIndex: 'Compact<u32>'
  },
  /**
   * Lookup205: ulx_primitives::block_vote::BlockVoteT<primitive_types::H256>
   **/
  UlxPrimitivesBlockVoteBlockVoteT: {
    accountId: 'AccountId32',
    blockHash: 'H256',
    index: 'Compact<u32>',
    power: 'Compact<u128>',
    dataDomainHash: 'H256',
    dataDomainAccount: 'AccountId32',
    signature: 'SpRuntimeMultiSignature',
    blockRewardsAccountId: 'AccountId32'
  },
  /**
   * Lookup206: sp_runtime::MultiSignature
   **/
  SpRuntimeMultiSignature: {
    _enum: {
      Ed25519: '[u8;64]',
      Sr25519: '[u8;64]',
      Ecdsa: '[u8;65]'
    }
  },
  /**
   * Lookup208: ulx_primitives::block_seal::app::Signature
   **/
  UlxPrimitivesBlockSealAppSignature: '[u8;64]',
  /**
   * Lookup209: pallet_block_rewards::pallet::Call<T>
   **/
  PalletBlockRewardsCall: 'Null',
  /**
   * Lookup210: pallet_grandpa::pallet::Call<T>
   **/
  PalletGrandpaCall: {
    _enum: {
      report_equivocation: {
        equivocationProof: 'SpConsensusGrandpaEquivocationProof',
        keyOwnerProof: 'SpSessionMembershipProof',
      },
      report_equivocation_unsigned: {
        equivocationProof: 'SpConsensusGrandpaEquivocationProof',
        keyOwnerProof: 'SpSessionMembershipProof',
      },
      note_stalled: {
        delay: 'u32',
        bestFinalizedBlockNumber: 'u32'
      }
    }
  },
  /**
   * Lookup211: sp_consensus_grandpa::EquivocationProof<primitive_types::H256, N>
   **/
  SpConsensusGrandpaEquivocationProof: {
    setId: 'u64',
    equivocation: 'SpConsensusGrandpaEquivocation'
  },
  /**
   * Lookup212: sp_consensus_grandpa::Equivocation<primitive_types::H256, N>
   **/
  SpConsensusGrandpaEquivocation: {
    _enum: {
      Prevote: 'FinalityGrandpaEquivocationPrevote',
      Precommit: 'FinalityGrandpaEquivocationPrecommit'
    }
  },
  /**
   * Lookup213: finality_grandpa::Equivocation<sp_consensus_grandpa::app::Public, finality_grandpa::Prevote<primitive_types::H256, N>, sp_consensus_grandpa::app::Signature>
   **/
  FinalityGrandpaEquivocationPrevote: {
    roundNumber: 'u64',
    identity: 'SpConsensusGrandpaAppPublic',
    first: '(FinalityGrandpaPrevote,SpConsensusGrandpaAppSignature)',
    second: '(FinalityGrandpaPrevote,SpConsensusGrandpaAppSignature)'
  },
  /**
   * Lookup214: finality_grandpa::Prevote<primitive_types::H256, N>
   **/
  FinalityGrandpaPrevote: {
    targetHash: 'H256',
    targetNumber: 'u32'
  },
  /**
   * Lookup215: sp_consensus_grandpa::app::Signature
   **/
  SpConsensusGrandpaAppSignature: '[u8;64]',
  /**
   * Lookup217: finality_grandpa::Equivocation<sp_consensus_grandpa::app::Public, finality_grandpa::Precommit<primitive_types::H256, N>, sp_consensus_grandpa::app::Signature>
   **/
  FinalityGrandpaEquivocationPrecommit: {
    roundNumber: 'u64',
    identity: 'SpConsensusGrandpaAppPublic',
    first: '(FinalityGrandpaPrecommit,SpConsensusGrandpaAppSignature)',
    second: '(FinalityGrandpaPrecommit,SpConsensusGrandpaAppSignature)'
  },
  /**
   * Lookup218: finality_grandpa::Precommit<primitive_types::H256, N>
   **/
  FinalityGrandpaPrecommit: {
    targetHash: 'H256',
    targetNumber: 'u32'
  },
  /**
   * Lookup220: sp_session::MembershipProof
   **/
  SpSessionMembershipProof: {
    session: 'u32',
    trieNodes: 'Vec<Bytes>',
    validatorCount: 'u32'
  },
  /**
   * Lookup221: pallet_mint::pallet::Call<T>
   **/
  PalletMintCall: 'Null',
  /**
   * Lookup222: pallet_balances::pallet::Call<T, I>
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
        keepAlive: 'bool'
      }
    }
  },
  /**
   * Lookup223: pallet_balances::types::AdjustmentDirection
   **/
  PalletBalancesAdjustmentDirection: {
    _enum: ['Increase', 'Decrease']
  },
  /**
   * Lookup225: pallet_tx_pause::pallet::Call<T>
   **/
  PalletTxPauseCall: {
    _enum: {
      pause: {
        fullName: '(Bytes,Bytes)',
      },
      unpause: {
        ident: '(Bytes,Bytes)'
      }
    }
  },
  /**
   * Lookup226: pallet_sudo::pallet::Call<T>
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
      remove_key: 'Null'
    }
  },
  /**
   * Lookup228: pallet_multisig::pallet::Error<T>
   **/
  PalletMultisigError: {
    _enum: ['MinimumThreshold', 'AlreadyApproved', 'NoApprovalsNeeded', 'TooFewSignatories', 'TooManySignatories', 'SignatoriesOutOfOrder', 'SenderInSignatories', 'NotFound', 'NotOwner', 'NoTimepoint', 'WrongTimepoint', 'UnexpectedTimepoint', 'MaxWeightTooLow', 'AlreadyStored']
  },
  /**
   * Lookup231: pallet_proxy::ProxyDefinition<sp_core::crypto::AccountId32, ulx_node_runtime::ProxyType, BlockNumber>
   **/
  PalletProxyProxyDefinition: {
    delegate: 'AccountId32',
    proxyType: 'UlxNodeRuntimeProxyType',
    delay: 'u32'
  },
  /**
   * Lookup235: pallet_proxy::Announcement<sp_core::crypto::AccountId32, primitive_types::H256, BlockNumber>
   **/
  PalletProxyAnnouncement: {
    real: 'AccountId32',
    callHash: 'H256',
    height: 'u32'
  },
  /**
   * Lookup237: pallet_proxy::pallet::Error<T>
   **/
  PalletProxyError: {
    _enum: ['TooMany', 'NotFound', 'NotProxy', 'Unproxyable', 'Duplicate', 'NoPermission', 'Unannounced', 'NoSelfProxy']
  },
  /**
   * Lookup239: pallet_ticks::pallet::Error<T>
   **/
  PalletTicksError: 'Null',
  /**
   * Lookup247: pallet_mining_slot::pallet::Error<T>
   **/
  PalletMiningSlotError: {
    _enum: {
      SlotNotTakingBids: 'Null',
      TooManyBlockRegistrants: 'Null',
      InsufficientOwnershipTokens: 'Null',
      BidTooLow: 'Null',
      CannotRegisterOverlappingSessions: 'Null',
      BondNotFound: 'Null',
      NoMoreBondIds: 'Null',
      VaultClosed: 'Null',
      MinimumBondAmountNotMet: 'Null',
      ExpirationAtBlockOverflow: 'Null',
      InsufficientFunds: 'Null',
      InsufficientVaultFunds: 'Null',
      ExpirationTooSoon: 'Null',
      NoPermissions: 'Null',
      HoldUnexpectedlyModified: 'Null',
      UnrecoverableHold: 'Null',
      VaultNotFound: 'Null',
      BondAlreadyClosed: 'Null',
      FeeExceedsBondAmount: 'Null',
      AccountWouldBeBelowMinimum: 'Null',
      GenericBondError: 'UlxPrimitivesBondBondError'
    }
  },
  /**
   * Lookup248: ulx_primitives::bond::BondError
   **/
  UlxPrimitivesBondBondError: {
    _enum: ['BondNotFound', 'NoMoreBondIds', 'MinimumBondAmountNotMet', 'VaultClosed', 'ExpirationAtBlockOverflow', 'AccountWouldBeBelowMinimum', 'InsufficientFunds', 'InsufficientVaultFunds', 'InsufficientBitcoinsForMining', 'ExpirationTooSoon', 'NoPermissions', 'HoldUnexpectedlyModified', 'UnrecoverableHold', 'VaultNotFound', 'NoVaultBitcoinPubkeysAvailable', 'FeeExceedsBondAmount', 'InvalidBitcoinScript']
  },
  /**
   * Lookup249: ulx_primitives::bitcoin::UtxoValue
   **/
  UlxPrimitivesBitcoinUtxoValue: {
    utxoId: 'u64',
    scriptPubkey: 'UlxPrimitivesBitcoinBitcoinCosignScriptPubkey',
    satoshis: 'Compact<u64>',
    submittedAtHeight: 'Compact<u64>',
    watchForSpentUntilHeight: 'Compact<u64>'
  },
  /**
   * Lookup250: ulx_primitives::bitcoin::BitcoinCosignScriptPubkey
   **/
  UlxPrimitivesBitcoinBitcoinCosignScriptPubkey: {
    _enum: {
      P2WSH: {
        wscriptHash: 'H256'
      }
    }
  },
  /**
   * Lookup257: pallet_bitcoin_utxos::pallet::Error<T>
   **/
  PalletBitcoinUtxosError: {
    _enum: ['NoPermissions', 'NoBitcoinConfirmedBlock', 'InsufficientBitcoinAmount', 'NoBitcoinPricesAvailable', 'ScriptPubkeyConflict', 'UtxoNotLocked', 'RedemptionsUnavailable', 'InvalidBitcoinSyncHeight', 'BitcoinHeightNotConfirmed', 'MaxUtxosExceeded', 'InvalidBitcoinScript']
  },
  /**
   * Lookup258: ulx_primitives::bond::Vault<sp_core::crypto::AccountId32, Balance, BlockNumber>
   **/
  UlxPrimitivesBondVault: {
    operatorAccountId: 'AccountId32',
    bitcoinArgons: 'UlxPrimitivesBondVaultArgons',
    securitizationPercent: 'Compact<u128>',
    securitizedArgons: 'Compact<u128>',
    miningArgons: 'UlxPrimitivesBondVaultArgons',
    miningRewardSharingPercentTake: 'Compact<Percent>',
    isClosed: 'bool',
    pendingTerms: 'Option<(u32,UlxPrimitivesBondVaultTerms)>'
  },
  /**
   * Lookup259: ulx_primitives::bond::VaultArgons<Balance>
   **/
  UlxPrimitivesBondVaultArgons: {
    annualPercentRate: 'Compact<u128>',
    allocated: 'Compact<u128>',
    bonded: 'Compact<u128>',
    baseFee: 'Compact<u128>'
  },
  /**
   * Lookup263: pallet_vaults::pallet::Error<T>
   **/
  PalletVaultsError: {
    _enum: ['BondNotFound', 'NoMoreVaultIds', 'NoMoreBondIds', 'MinimumBondAmountNotMet', 'ExpirationAtBlockOverflow', 'InsufficientFunds', 'InsufficientVaultFunds', 'InsufficientBitcoinsForMining', 'AccountBelowMinimumBalance', 'VaultClosed', 'InvalidVaultAmount', 'VaultReductionBelowAllocatedFunds', 'InvalidSecuritization', 'MaxPendingVaultBitcoinPubkeys', 'MaxSecuritizationPercentExceeded', 'InvalidBondType', 'BitcoinUtxoNotFound', 'InsufficientSatoshisBonded', 'NoBitcoinPricesAvailable', 'InvalidBitcoinScript', 'ExpirationTooSoon', 'NoPermissions', 'HoldUnexpectedlyModified', 'UnrecoverableHold', 'VaultNotFound', 'FeeExceedsBondAmount', 'NoVaultBitcoinPubkeysAvailable', 'TermsModificationOverflow', 'TermsChangeAlreadyScheduled']
  },
  /**
   * Lookup264: ulx_primitives::bond::Bond<sp_core::crypto::AccountId32, Balance, BlockNumber>
   **/
  UlxPrimitivesBond: {
    bondType: 'UlxPrimitivesBondBondType',
    vaultId: 'Compact<u32>',
    utxoId: 'Option<u64>',
    bondedAccountId: 'AccountId32',
    totalFee: 'Compact<u128>',
    prepaidFee: 'Compact<u128>',
    amount: 'Compact<u128>',
    startBlock: 'Compact<u32>',
    expiration: 'UlxPrimitivesBondBondExpiration'
  },
  /**
   * Lookup267: pallet_bond::pallet::UtxoState
   **/
  PalletBondUtxoState: {
    bondId: 'u64',
    satoshis: 'u64',
    vaultPubkeyHash: 'UlxPrimitivesBitcoinBitcoinPubkeyHash',
    ownerPubkeyHash: 'UlxPrimitivesBitcoinBitcoinPubkeyHash',
    vaultClaimHeight: 'u64',
    openClaimHeight: 'u64',
    registerBlock: 'u64',
    utxoScriptPubkey: 'UlxPrimitivesBitcoinBitcoinCosignScriptPubkey',
    isVerified: 'bool'
  },
  /**
   * Lookup270: pallet_bond::pallet::UtxoCosignRequest<Balance>
   **/
  PalletBondUtxoCosignRequest: {
    bitcoinNetworkFee: 'u64',
    cosignDueBlock: 'u64',
    toScriptPubkey: 'Bytes',
    redemptionPrice: 'u128'
  },
  /**
   * Lookup274: pallet_bond::pallet::Error<T>
   **/
  PalletBondError: {
    _enum: {
      BondNotFound: 'Null',
      NoMoreBondIds: 'Null',
      MinimumBondAmountNotMet: 'Null',
      ExpirationAtBlockOverflow: 'Null',
      InsufficientFunds: 'Null',
      InsufficientVaultFunds: 'Null',
      InsufficientBitcoinsForMining: 'Null',
      AccountWouldGoBelowMinimumBalance: 'Null',
      VaultClosed: 'Null',
      InvalidVaultAmount: 'Null',
      BondRedemptionNotLocked: 'Null',
      BitcoinUnlockInitiationDeadlinePassed: 'Null',
      BitcoinFeeTooHigh: 'Null',
      InvalidBondType: 'Null',
      BitcoinUtxoNotFound: 'Null',
      InsufficientSatoshisBonded: 'Null',
      NoBitcoinPricesAvailable: 'Null',
      InvalidBitcoinScript: 'Null',
      ExpirationTooSoon: 'Null',
      NoPermissions: 'Null',
      HoldUnexpectedlyModified: 'Null',
      UnrecoverableHold: 'Null',
      VaultNotFound: 'Null',
      FeeExceedsBondAmount: 'Null',
      GenericBondError: 'UlxPrimitivesBondBondError'
    }
  },
  /**
   * Lookup286: pallet_notaries::pallet::Error<T>
   **/
  PalletNotariesError: {
    _enum: ['ProposalNotFound', 'MaxNotariesExceeded', 'MaxProposalsPerBlockExceeded', 'NotAnActiveNotary', 'InvalidNotaryOperator', 'NoMoreNotaryIds', 'EffectiveTickTooSoon', 'TooManyKeys', 'InvalidNotary']
  },
  /**
   * Lookup290: ulx_primitives::notary::NotaryNotebookKeyDetails
   **/
  UlxPrimitivesNotaryNotaryNotebookKeyDetails: {
    notebookNumber: 'Compact<u32>',
    tick: 'Compact<u32>',
    blockVotesRoot: 'H256',
    secretHash: 'H256',
    parentSecret: 'Option<H256>'
  },
  /**
   * Lookup292: ulx_primitives::digests::NotebookDigest<ulx_notary_audit::error::VerifyError>
   **/
  UlxPrimitivesDigestsNotebookDigest: {
    notebooks: 'Vec<UlxPrimitivesDigestsNotebookDigestRecord>'
  },
  /**
   * Lookup294: ulx_primitives::digests::NotebookDigestRecord<ulx_notary_audit::error::VerifyError>
   **/
  UlxPrimitivesDigestsNotebookDigestRecord: {
    notaryId: 'Compact<u32>',
    notebookNumber: 'Compact<u32>',
    tick: 'Compact<u32>',
    auditFirstFailure: 'Option<UlxNotaryAuditErrorVerifyError>'
  },
  /**
   * Lookup297: pallet_notebook::pallet::Error<T>
   **/
  PalletNotebookError: {
    _enum: ['DuplicateNotebookNumber', 'MissingNotebookNumber', 'NotebookTickAlreadyUsed', 'InvalidNotebookSignature', 'InvalidSecretProvided', 'CouldNotDecodeNotebook', 'DuplicateNotebookDigest', 'MissingNotebookDigest', 'InvalidNotebookDigest', 'MultipleNotebookInherentsProvided', 'InternalError']
  },
  /**
   * Lookup298: pallet_chain_transfer::QueuedTransferOut<sp_core::crypto::AccountId32, Balance>
   **/
  PalletChainTransferQueuedTransferOut: {
    accountId: 'AccountId32',
    amount: 'u128',
    expirationTick: 'u32',
    notaryId: 'u32'
  },
  /**
   * Lookup303: frame_support::PalletId
   **/
  FrameSupportPalletId: '[u8;8]',
  /**
   * Lookup304: pallet_chain_transfer::pallet::Error<T>
   **/
  PalletChainTransferError: {
    _enum: ['MaxBlockTransfersExceeded', 'InsufficientFunds', 'InsufficientNotarizedFunds', 'InvalidOrDuplicatedLocalchainTransfer', 'NotebookIncludesExpiredLocalchainTransfer', 'InvalidNotaryUsedForTransfer', 'NotaryLocked']
  },
  /**
   * Lookup309: ulx_primitives::notary::NotaryNotebookVoteDigestDetails
   **/
  UlxPrimitivesNotaryNotaryNotebookVoteDigestDetails: {
    notaryId: 'Compact<u32>',
    notebookNumber: 'Compact<u32>',
    tick: 'Compact<u32>',
    blockVotesCount: 'Compact<u32>',
    blockVotingPower: 'Compact<u128>'
  },
  /**
   * Lookup311: ulx_primitives::digests::BlockVoteDigest
   **/
  UlxPrimitivesDigestsBlockVoteDigest: {
    votingPower: 'Compact<u128>',
    votesCount: 'Compact<u32>'
  },
  /**
   * Lookup315: pallet_block_seal_spec::pallet::Error<T>
   **/
  PalletBlockSealSpecError: {
    _enum: ['MaxNotebooksAtTickExceeded']
  },
  /**
   * Lookup318: pallet_data_domain::pallet::Error<T>
   **/
  PalletDataDomainError: {
    _enum: ['DomainNotRegistered', 'NotDomainOwner', 'FailedToAddToAddressHistory', 'FailedToAddExpiringDomain', 'AccountDecodingError']
  },
  /**
   * Lookup319: pallet_price_index::pallet::Error<T>
   **/
  PalletPriceIndexError: {
    _enum: ['NotAuthorizedOperator', 'MissingValue', 'PricesTooOld', 'MaxPriceChangePerTickExceeded']
  },
  /**
   * Lookup324: sp_core::crypto::KeyTypeId
   **/
  SpCoreCryptoKeyTypeId: '[u8;4]',
  /**
   * Lookup325: pallet_session::pallet::Error<T>
   **/
  PalletSessionError: {
    _enum: ['InvalidProof', 'NoAssociatedValidatorId', 'DuplicatedKey', 'NoKeys', 'NoAccount']
  },
  /**
   * Lookup326: ulx_primitives::providers::BlockSealerInfo<sp_core::crypto::AccountId32>
   **/
  UlxPrimitivesProvidersBlockSealerInfo: {
    blockAuthorAccountId: 'AccountId32',
    blockVoteRewardsAccount: 'Option<AccountId32>'
  },
  /**
   * Lookup327: ulx_primitives::digests::ParentVotingKeyDigest
   **/
  UlxPrimitivesDigestsParentVotingKeyDigest: {
    parentVotingKey: 'Option<H256>'
  },
  /**
   * Lookup328: pallet_block_seal::pallet::Error<T>
   **/
  PalletBlockSealError: {
    _enum: ['InvalidVoteSealStrength', 'InvalidSubmitter', 'UnableToDecodeVoteAccount', 'UnregisteredBlockAuthor', 'InvalidBlockVoteProof', 'NoGrandparentVoteMinimum', 'DuplicateBlockSealProvided', 'InsufficientVotingPower', 'ParentVotingKeyNotFound', 'InvalidVoteGrandparentHash', 'IneligibleNotebookUsed', 'NoEligibleVotingRoot', 'UnregisteredDataDomain', 'InvalidDataDomainAccount', 'InvalidAuthoritySignature', 'CouldNotDecodeVote', 'MaxNotebooksAtTickExceeded', 'NoClosestMinerFoundForVote', 'BlockVoteInvalidSignature']
  },
  /**
   * Lookup330: pallet_block_rewards::pallet::Error<T>
   **/
  PalletBlockRewardsError: 'Null',
  /**
   * Lookup331: pallet_grandpa::StoredState<N>
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
        delay: 'u32'
      }
    }
  },
  /**
   * Lookup332: pallet_grandpa::StoredPendingChange<N, Limit>
   **/
  PalletGrandpaStoredPendingChange: {
    scheduledAt: 'u32',
    delay: 'u32',
    nextAuthorities: 'Vec<(SpConsensusGrandpaAppPublic,u64)>',
    forced: 'Option<u32>'
  },
  /**
   * Lookup335: pallet_grandpa::pallet::Error<T>
   **/
  PalletGrandpaError: {
    _enum: ['PauseFailed', 'ResumeFailed', 'ChangePending', 'TooSoon', 'InvalidKeyOwnershipProof', 'InvalidEquivocationProof', 'DuplicateOffenceReport']
  },
  /**
   * Lookup336: sp_staking::offence::OffenceDetails<sp_core::crypto::AccountId32, Offender>
   **/
  SpStakingOffenceOffenceDetails: {
    offender: '(AccountId32,PalletMiningSlotMinerHistory)',
    reporters: 'Vec<AccountId32>'
  },
  /**
   * Lookup338: pallet_mining_slot::MinerHistory
   **/
  PalletMiningSlotMinerHistory: {
    authorityIndex: 'u32'
  },
  /**
   * Lookup343: pallet_mint::pallet::Error<T>
   **/
  PalletMintError: {
    _enum: ['TooManyPendingMints']
  },
  /**
   * Lookup345: pallet_balances::types::BalanceLock<Balance>
   **/
  PalletBalancesBalanceLock: {
    id: '[u8;8]',
    amount: 'u128',
    reasons: 'PalletBalancesReasons'
  },
  /**
   * Lookup346: pallet_balances::types::Reasons
   **/
  PalletBalancesReasons: {
    _enum: ['Fee', 'Misc', 'All']
  },
  /**
   * Lookup349: pallet_balances::types::ReserveData<ReserveIdentifier, Balance>
   **/
  PalletBalancesReserveData: {
    id: '[u8;8]',
    amount: 'u128'
  },
  /**
   * Lookup352: pallet_balances::types::IdAmount<ulx_node_runtime::RuntimeHoldReason, Balance>
   **/
  PalletBalancesIdAmountRuntimeHoldReason: {
    id: 'UlxNodeRuntimeRuntimeHoldReason',
    amount: 'u128'
  },
  /**
   * Lookup353: ulx_node_runtime::RuntimeHoldReason
   **/
  UlxNodeRuntimeRuntimeHoldReason: {
    _enum: {
      __Unused0: 'Null',
      __Unused1: 'Null',
      __Unused2: 'Null',
      __Unused3: 'Null',
      __Unused4: 'Null',
      MiningSlot: 'PalletMiningSlotHoldReason',
      __Unused6: 'Null',
      Vaults: 'PalletVaultsHoldReason',
      Bonds: 'PalletBondHoldReason',
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
      BlockRewards: 'PalletBlockRewardsHoldReason'
    }
  },
  /**
   * Lookup354: pallet_mining_slot::pallet::HoldReason
   **/
  PalletMiningSlotHoldReason: {
    _enum: ['RegisterAsMiner']
  },
  /**
   * Lookup355: pallet_vaults::pallet::HoldReason
   **/
  PalletVaultsHoldReason: {
    _enum: ['EnterVault', 'BondFee']
  },
  /**
   * Lookup356: pallet_bond::pallet::HoldReason
   **/
  PalletBondHoldReason: {
    _enum: ['UnlockingBitcoin']
  },
  /**
   * Lookup357: pallet_block_rewards::pallet::HoldReason
   **/
  PalletBlockRewardsHoldReason: {
    _enum: ['MaturationPeriod']
  },
  /**
   * Lookup360: pallet_balances::types::IdAmount<ulx_node_runtime::RuntimeFreezeReason, Balance>
   **/
  PalletBalancesIdAmountRuntimeFreezeReason: {
    id: 'UlxNodeRuntimeRuntimeFreezeReason',
    amount: 'u128'
  },
  /**
   * Lookup361: ulx_node_runtime::RuntimeFreezeReason
   **/
  UlxNodeRuntimeRuntimeFreezeReason: {
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
      BlockRewards: 'PalletBlockRewardsFreezeReason'
    }
  },
  /**
   * Lookup362: pallet_block_rewards::pallet::FreezeReason
   **/
  PalletBlockRewardsFreezeReason: {
    _enum: ['MaturationPeriod']
  },
  /**
   * Lookup364: pallet_balances::pallet::Error<T, I>
   **/
  PalletBalancesError: {
    _enum: ['VestingBalance', 'LiquidityRestrictions', 'InsufficientBalance', 'ExistentialDeposit', 'Expendability', 'ExistingVestingSchedule', 'DeadAccount', 'TooManyReserves', 'TooManyHolds', 'TooManyFreezes', 'IssuanceDeactivated', 'DeltaZero']
  },
  /**
   * Lookup366: pallet_tx_pause::pallet::Error<T>
   **/
  PalletTxPauseError: {
    _enum: ['IsPaused', 'IsUnpaused', 'Unpausable', 'NotFound']
  },
  /**
   * Lookup367: pallet_transaction_payment::Releases
   **/
  PalletTransactionPaymentReleases: {
    _enum: ['V1Ancient', 'V2']
  },
  /**
   * Lookup368: pallet_sudo::pallet::Error<T>
   **/
  PalletSudoError: {
    _enum: ['RequireSudo']
  },
  /**
   * Lookup371: frame_system::extensions::check_non_zero_sender::CheckNonZeroSender<T>
   **/
  FrameSystemExtensionsCheckNonZeroSender: 'Null',
  /**
   * Lookup372: frame_system::extensions::check_spec_version::CheckSpecVersion<T>
   **/
  FrameSystemExtensionsCheckSpecVersion: 'Null',
  /**
   * Lookup373: frame_system::extensions::check_tx_version::CheckTxVersion<T>
   **/
  FrameSystemExtensionsCheckTxVersion: 'Null',
  /**
   * Lookup374: frame_system::extensions::check_genesis::CheckGenesis<T>
   **/
  FrameSystemExtensionsCheckGenesis: 'Null',
  /**
   * Lookup377: frame_system::extensions::check_nonce::CheckNonce<T>
   **/
  FrameSystemExtensionsCheckNonce: 'Compact<u32>',
  /**
   * Lookup378: frame_system::extensions::check_weight::CheckWeight<T>
   **/
  FrameSystemExtensionsCheckWeight: 'Null',
  /**
   * Lookup379: pallet_transaction_payment::ChargeTransactionPayment<T>
   **/
  PalletTransactionPaymentChargeTransactionPayment: 'Compact<u128>',
  /**
   * Lookup380: ulx_node_runtime::Runtime
   **/
  UlxNodeRuntimeRuntime: 'Null'
};
