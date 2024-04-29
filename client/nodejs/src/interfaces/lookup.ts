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
   * Lookup31: pallet_proxy::pallet::Event<T>
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
   * Lookup34: ulx_node_runtime::ProxyType
   **/
  UlxNodeRuntimeProxyType: {
    _enum: ['Any', 'NonTransfer', 'PriceIndex']
  },
  /**
   * Lookup36: pallet_mining_slot::pallet::Event<T>
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
        keptOwnershipBond: 'bool'
      }
    }
  },
  /**
   * Lookup38: ulx_primitives::block_seal::MiningRegistration<sp_core::crypto::AccountId32, BondId, Balance>
   **/
  UlxPrimitivesBlockSealMiningRegistration: {
    accountId: 'AccountId32',
    rewardDestination: 'UlxPrimitivesBlockSealRewardDestination',
    bondId: 'Option<u64>',
    bondAmount: 'Compact<u128>',
    ownershipTokens: 'Compact<u128>'
  },
  /**
   * Lookup39: ulx_primitives::block_seal::RewardDestination<sp_core::crypto::AccountId32>
   **/
  UlxPrimitivesBlockSealRewardDestination: {
    _enum: {
      Owner: 'Null',
      Account: 'AccountId32'
    }
  },
  /**
   * Lookup43: pallet_bond::pallet::Event<T>
   **/
  PalletBondEvent: {
    _enum: {
      BondFundOffered: {
        bondFundId: 'u32',
        amountOffered: 'u128',
        expirationBlock: 'u32',
        offerAccountId: 'AccountId32',
      },
      BondFundExtended: {
        bondFundId: 'u32',
        amountOffered: 'u128',
        expirationBlock: 'u32',
      },
      BondFundEnded: {
        bondFundId: 'u32',
        amountStillBonded: 'u128',
      },
      BondFundExpired: {
        bondFundId: 'u32',
        offerAccountId: 'AccountId32',
      },
      BondedSelf: {
        bondId: 'u64',
        bondedAccountId: 'AccountId32',
        amount: 'u128',
        completionBlock: 'u32',
      },
      BondLeased: {
        bondFundId: 'u32',
        bondId: 'u64',
        bondedAccountId: 'AccountId32',
        amount: 'u128',
        totalFee: 'u128',
        annualPercentRate: 'u32',
        completionBlock: 'u32',
      },
      BondExtended: {
        bondFundId: 'Option<u32>',
        bondId: 'u64',
        amount: 'u128',
        completionBlock: 'u32',
        feeChange: 'u128',
        annualPercentRate: 'u32',
      },
      BondCompleted: {
        bondFundId: 'Option<u32>',
        bondId: 'u64',
      },
      BondBurned: {
        bondFundId: 'Option<u32>',
        bondId: 'u64',
        amount: 'u128',
      },
      BondFeeRefund: {
        bondFundId: 'u32',
        bondId: 'u64',
        bondedAccountId: 'AccountId32',
        bondFundReductionForPayment: 'u128',
        finalFee: 'u128',
        refundAmount: 'u128',
      },
      BondLocked: {
        bondId: 'u64',
        bondedAccountId: 'AccountId32',
      },
      BondUnlocked: {
        bondId: 'u64',
        bondedAccountId: 'AccountId32'
      }
    }
  },
  /**
   * Lookup45: pallet_notaries::pallet::Event<T>
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
        effectiveBlock: 'u32',
      },
      NotaryMetaUpdated: {
        notaryId: 'u32',
        meta: 'UlxPrimitivesNotaryNotaryMeta'
      }
    }
  },
  /**
   * Lookup46: ulx_primitives::notary::NotaryMeta<MaxHosts>
   **/
  UlxPrimitivesNotaryNotaryMeta: {
    public: '[u8;32]',
    hosts: 'Vec<Bytes>'
  },
  /**
   * Lookup51: ulx_primitives::notary::NotaryRecord<sp_core::crypto::AccountId32, BlockNumber, MaxHosts>
   **/
  UlxPrimitivesNotaryNotaryRecord: {
    notaryId: 'Compact<u32>',
    operatorAccountId: 'AccountId32',
    activatedBlock: 'Compact<u32>',
    metaUpdatedBlock: 'Compact<u32>',
    meta: 'UlxPrimitivesNotaryNotaryMeta'
  },
  /**
   * Lookup53: pallet_notebook::pallet::Event<T>
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
   * Lookup54: ulx_notary_audit::error::VerifyError
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
   * Lookup55: ulx_primitives::account::AccountType
   **/
  UlxPrimitivesAccountAccountType: {
    _enum: ['Tax', 'Deposit']
  },
  /**
   * Lookup56: ulx_notary_audit::AccountHistoryLookupError
   **/
  UlxNotaryAuditAccountHistoryLookupError: {
    _enum: ['RootNotFound', 'LastChangeNotFound', 'InvalidTransferToLocalchain', 'BlockSpecificationNotFound']
  },
  /**
   * Lookup59: pallet_chain_transfer::pallet::Event<T>
   **/
  PalletChainTransferEvent: {
    _enum: {
      TransferToLocalchain: {
        accountId: 'AccountId32',
        amount: 'u128',
        transferId: 'u32',
        notaryId: 'u32',
        expirationBlock: 'u32',
      },
      TransferToLocalchainExpired: {
        accountId: 'AccountId32',
        transferId: 'u32',
        notaryId: 'u32',
      },
      TransferIn: {
        accountId: 'AccountId32',
        amount: 'u128',
        notaryId: 'u32'
      }
    }
  },
  /**
   * Lookup60: pallet_block_seal_spec::pallet::Event<T>
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
   * Lookup61: pallet_data_domain::pallet::Event<T>
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
        registration: 'PalletDataDomainDataDomainRegistration'
      }
    }
  },
  /**
   * Lookup62: ulx_primitives::data_domain::ZoneRecord<sp_core::crypto::AccountId32>
   **/
  UlxPrimitivesDataDomainZoneRecord: {
    paymentAccount: 'AccountId32',
    notaryId: 'u32',
    versions: 'BTreeMap<UlxPrimitivesDataDomainSemver, UlxPrimitivesDataDomainVersionHost>'
  },
  /**
   * Lookup64: ulx_primitives::data_domain::Semver
   **/
  UlxPrimitivesDataDomainSemver: {
    major: 'u32',
    minor: 'u32',
    patch: 'u32'
  },
  /**
   * Lookup65: ulx_primitives::data_domain::VersionHost
   **/
  UlxPrimitivesDataDomainVersionHost: {
    datastoreId: 'Bytes',
    host: 'Bytes'
  },
  /**
   * Lookup69: pallet_data_domain::DataDomainRegistration<sp_core::crypto::AccountId32>
   **/
  PalletDataDomainDataDomainRegistration: {
    accountId: 'AccountId32',
    registeredAtTick: 'u32'
  },
  /**
   * Lookup70: pallet_price_index::pallet::Event<T>
   **/
  PalletPriceIndexEvent: {
    _enum: {
      NewIndex: {
        priceIndex: 'PalletPriceIndexPriceIndex',
      },
      OperatorChanged: {
        operatorId: 'AccountId32'
      }
    }
  },
  /**
   * Lookup71: pallet_price_index::PriceIndex<Moment>
   **/
  PalletPriceIndexPriceIndex: {
    btcUsdPrice: 'Compact<u64>',
    argonUsdPrice: 'Compact<u64>',
    argonCpi: 'i16',
    timestamp: 'Compact<u64>'
  },
  /**
   * Lookup73: pallet_session::pallet::Event
   **/
  PalletSessionEvent: {
    _enum: {
      NewSession: {
        sessionIndex: 'u32'
      }
    }
  },
  /**
   * Lookup74: pallet_block_rewards::pallet::Event<T>
   **/
  PalletBlockRewardsEvent: {
    _enum: {
      RewardCreated: {
        maturationBlock: 'u32',
        rewards: 'Vec<UlxPrimitivesBlockSealBlockPayout>',
      },
      RewardUnlocked: {
        rewards: 'Vec<UlxPrimitivesBlockSealBlockPayout>'
      }
    }
  },
  /**
   * Lookup76: ulx_primitives::block_seal::BlockPayout<sp_core::crypto::AccountId32, Balance>
   **/
  UlxPrimitivesBlockSealBlockPayout: {
    accountId: 'AccountId32',
    ulixees: 'u128',
    argons: 'u128'
  },
  /**
   * Lookup77: pallet_grandpa::pallet::Event
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
   * Lookup80: sp_consensus_grandpa::app::Public
   **/
  SpConsensusGrandpaAppPublic: '[u8;32]',
  /**
   * Lookup81: pallet_offences::pallet::Event
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
   * Lookup83: pallet_bitcoin_mint::pallet::Event<T>
   **/
  PalletBitcoinMintEvent: {
    _enum: {
      UtxoOwnershipConfirmed: {
        utxo: 'UlxPrimitivesBitcoinBitcoinUtxoId',
        accountId: 'AccountId32',
        bondId: 'u64',
        amount: 'u128',
        expirationBlock: 'u32',
      },
      UtxoUnlocked: {
        utxo: 'UlxPrimitivesBitcoinBitcoinUtxoId',
        accountId: 'AccountId32',
        bondId: 'u64',
        amount: 'u128',
      },
      UtxoOwnershipDenied: {
        utxo: 'UlxPrimitivesBitcoinBitcoinUtxoId',
        accountId: 'AccountId32',
        bondId: 'u64',
        amount: 'u128',
        expirationBlock: 'u32',
      },
      UtxoMovedWithBurn: {
        utxo: 'UlxPrimitivesBitcoinBitcoinUtxoId',
        bondId: 'u64'
      }
    }
  },
  /**
   * Lookup84: ulx_primitives::bitcoin::BitcoinUtxoId
   **/
  UlxPrimitivesBitcoinBitcoinUtxoId: {
    txid: 'UlxPrimitivesBitcoinH256Le',
    outputIndex: 'u32'
  },
  /**
   * Lookup85: ulx_primitives::bitcoin::H256Le
   **/
  UlxPrimitivesBitcoinH256Le: '[u8;32]',
  /**
   * Lookup86: pallet_balances::pallet::Event<T, I>
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
   * Lookup87: frame_support::traits::tokens::misc::BalanceStatus
   **/
  FrameSupportTokensMiscBalanceStatus: {
    _enum: ['Free', 'Reserved']
  },
  /**
   * Lookup88: pallet_ulixee_mint::pallet::Event<T>
   **/
  PalletUlixeeMintEvent: 'Null',
  /**
   * Lookup90: pallet_tx_pause::pallet::Event<T>
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
   * Lookup93: pallet_transaction_payment::pallet::Event<T>
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
   * Lookup94: pallet_sudo::pallet::Event<T>
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
   * Lookup96: frame_system::Phase
   **/
  FrameSystemPhase: {
    _enum: {
      ApplyExtrinsic: 'u32',
      Finalization: 'Null',
      Initialization: 'Null'
    }
  },
  /**
   * Lookup100: frame_system::LastRuntimeUpgradeInfo
   **/
  FrameSystemLastRuntimeUpgradeInfo: {
    specVersion: 'Compact<u32>',
    specName: 'Text'
  },
  /**
   * Lookup101: frame_system::CodeUpgradeAuthorization<T>
   **/
  FrameSystemCodeUpgradeAuthorization: {
    codeHash: 'H256',
    checkVersion: 'bool'
  },
  /**
   * Lookup102: frame_system::pallet::Call<T>
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
   * Lookup106: frame_system::limits::BlockWeights
   **/
  FrameSystemLimitsBlockWeights: {
    baseBlock: 'SpWeightsWeightV2Weight',
    maxBlock: 'SpWeightsWeightV2Weight',
    perClass: 'FrameSupportDispatchPerDispatchClassWeightsPerClass'
  },
  /**
   * Lookup107: frame_support::dispatch::PerDispatchClass<frame_system::limits::WeightsPerClass>
   **/
  FrameSupportDispatchPerDispatchClassWeightsPerClass: {
    normal: 'FrameSystemLimitsWeightsPerClass',
    operational: 'FrameSystemLimitsWeightsPerClass',
    mandatory: 'FrameSystemLimitsWeightsPerClass'
  },
  /**
   * Lookup108: frame_system::limits::WeightsPerClass
   **/
  FrameSystemLimitsWeightsPerClass: {
    baseExtrinsic: 'SpWeightsWeightV2Weight',
    maxExtrinsic: 'Option<SpWeightsWeightV2Weight>',
    maxTotal: 'Option<SpWeightsWeightV2Weight>',
    reserved: 'Option<SpWeightsWeightV2Weight>'
  },
  /**
   * Lookup110: frame_system::limits::BlockLength
   **/
  FrameSystemLimitsBlockLength: {
    max: 'FrameSupportDispatchPerDispatchClassU32'
  },
  /**
   * Lookup111: frame_support::dispatch::PerDispatchClass<T>
   **/
  FrameSupportDispatchPerDispatchClassU32: {
    normal: 'u32',
    operational: 'u32',
    mandatory: 'u32'
  },
  /**
   * Lookup112: sp_weights::RuntimeDbWeight
   **/
  SpWeightsRuntimeDbWeight: {
    read: 'u64',
    write: 'u64'
  },
  /**
   * Lookup113: sp_version::RuntimeVersion
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
   * Lookup118: frame_system::pallet::Error<T>
   **/
  FrameSystemError: {
    _enum: ['InvalidSpecName', 'SpecVersionNeedsToIncrease', 'FailedToExtractRuntimeVersion', 'NonDefaultComposite', 'NonZeroRefCount', 'CallFiltered', 'MultiBlockMigrationsOngoing', 'NothingAuthorized', 'Unauthorized']
  },
  /**
   * Lookup119: pallet_timestamp::pallet::Call<T>
   **/
  PalletTimestampCall: {
    _enum: {
      set: {
        now: 'Compact<u64>'
      }
    }
  },
  /**
   * Lookup122: pallet_proxy::ProxyDefinition<sp_core::crypto::AccountId32, ulx_node_runtime::ProxyType, BlockNumber>
   **/
  PalletProxyProxyDefinition: {
    delegate: 'AccountId32',
    proxyType: 'UlxNodeRuntimeProxyType',
    delay: 'u32'
  },
  /**
   * Lookup126: pallet_proxy::Announcement<sp_core::crypto::AccountId32, primitive_types::H256, BlockNumber>
   **/
  PalletProxyAnnouncement: {
    real: 'AccountId32',
    callHash: 'H256',
    height: 'u32'
  },
  /**
   * Lookup128: pallet_proxy::pallet::Call<T>
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
   * Lookup134: pallet_ticks::pallet::Call<T>
   **/
  PalletTicksCall: 'Null',
  /**
   * Lookup135: pallet_mining_slot::pallet::Call<T>
   **/
  PalletMiningSlotCall: {
    _enum: {
      bid: {
        bondId: 'Option<u64>',
        rewardDestination: 'UlxPrimitivesBlockSealRewardDestination'
      }
    }
  },
  /**
   * Lookup136: pallet_bond::pallet::Call<T>
   **/
  PalletBondCall: {
    _enum: {
      offer_fund: {
        leaseAnnualPercentRate: 'Compact<u32>',
        leaseBaseFee: 'Compact<u128>',
        amountOffered: 'Compact<u128>',
        expirationBlock: 'u32',
      },
      end_fund: {
        bondFundId: 'u32',
      },
      extend_fund: {
        bondFundId: 'u32',
        totalAmountOffered: 'u128',
        expirationBlock: 'u32',
      },
      bond_self: {
        amount: 'u128',
        bondUntilBlock: 'u32',
      },
      lease: {
        bondFundId: 'u32',
        amount: 'u128',
        leaseUntilBlock: 'u32',
      },
      return_bond: {
        bondId: 'u64',
      },
      extend_bond: {
        bondId: 'u64',
        totalAmount: 'u128',
        bondUntilBlock: 'u32'
      }
    }
  },
  /**
   * Lookup137: pallet_notaries::pallet::Call<T>
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
        meta: 'UlxPrimitivesNotaryNotaryMeta'
      }
    }
  },
  /**
   * Lookup138: pallet_notebook::pallet::Call<T>
   **/
  PalletNotebookCall: {
    _enum: {
      submit: {
        notebooks: 'Vec<UlxPrimitivesNotebookSignedNotebookHeader>'
      }
    }
  },
  /**
   * Lookup140: ulx_primitives::notebook::SignedNotebookHeader
   **/
  UlxPrimitivesNotebookSignedNotebookHeader: {
    header: 'UlxPrimitivesNotebookNotebookHeader',
    signature: '[u8;64]'
  },
  /**
   * Lookup141: ulx_primitives::notebook::NotebookHeader
   **/
  UlxPrimitivesNotebookNotebookHeader: {
    version: 'Compact<u16>',
    notebookNumber: 'Compact<u32>',
    tick: 'Compact<u32>',
    finalizedBlockNumber: 'Compact<u32>',
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
   * Lookup144: ulx_primitives::notebook::ChainTransfer
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
   * Lookup147: ulx_primitives::balance_change::AccountOrigin
   **/
  UlxPrimitivesBalanceChangeAccountOrigin: {
    notebookNumber: 'Compact<u32>',
    accountUid: 'Compact<u32>'
  },
  /**
   * Lookup155: pallet_chain_transfer::pallet::Call<T>
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
   * Lookup156: pallet_block_seal_spec::pallet::Call<T>
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
   * Lookup158: pallet_data_domain::pallet::Call<T>
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
   * Lookup159: pallet_price_index::pallet::Call<T>
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
   * Lookup160: pallet_session::pallet::Call<T>
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
   * Lookup161: ulx_node_runtime::opaque::SessionKeys
   **/
  UlxNodeRuntimeOpaqueSessionKeys: {
    grandpa: 'SpConsensusGrandpaAppPublic',
    blockSealAuthority: 'UlxPrimitivesBlockSealAppPublic'
  },
  /**
   * Lookup162: ulx_primitives::block_seal::app::Public
   **/
  UlxPrimitivesBlockSealAppPublic: '[u8;32]',
  /**
   * Lookup163: pallet_block_seal::pallet::Call<T>
   **/
  PalletBlockSealCall: {
    _enum: {
      apply: {
        seal: 'UlxPrimitivesInherentsBlockSealInherent'
      }
    }
  },
  /**
   * Lookup164: ulx_primitives::inherents::BlockSealInherent
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
   * Lookup167: ulx_primitives::balance_change::MerkleProof
   **/
  UlxPrimitivesBalanceChangeMerkleProof: {
    proof: 'Vec<H256>',
    numberOfLeaves: 'Compact<u32>',
    leafIndex: 'Compact<u32>'
  },
  /**
   * Lookup169: ulx_primitives::block_vote::BlockVoteT<primitive_types::H256>
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
   * Lookup170: sp_runtime::MultiSignature
   **/
  SpRuntimeMultiSignature: {
    _enum: {
      Ed25519: '[u8;64]',
      Sr25519: '[u8;64]',
      Ecdsa: '[u8;65]'
    }
  },
  /**
   * Lookup172: ulx_primitives::block_seal::app::Signature
   **/
  UlxPrimitivesBlockSealAppSignature: '[u8;64]',
  /**
   * Lookup173: pallet_block_rewards::pallet::Call<T>
   **/
  PalletBlockRewardsCall: 'Null',
  /**
   * Lookup174: pallet_grandpa::pallet::Call<T>
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
   * Lookup175: sp_consensus_grandpa::EquivocationProof<primitive_types::H256, N>
   **/
  SpConsensusGrandpaEquivocationProof: {
    setId: 'u64',
    equivocation: 'SpConsensusGrandpaEquivocation'
  },
  /**
   * Lookup176: sp_consensus_grandpa::Equivocation<primitive_types::H256, N>
   **/
  SpConsensusGrandpaEquivocation: {
    _enum: {
      Prevote: 'FinalityGrandpaEquivocationPrevote',
      Precommit: 'FinalityGrandpaEquivocationPrecommit'
    }
  },
  /**
   * Lookup177: finality_grandpa::Equivocation<sp_consensus_grandpa::app::Public, finality_grandpa::Prevote<primitive_types::H256, N>, sp_consensus_grandpa::app::Signature>
   **/
  FinalityGrandpaEquivocationPrevote: {
    roundNumber: 'u64',
    identity: 'SpConsensusGrandpaAppPublic',
    first: '(FinalityGrandpaPrevote,SpConsensusGrandpaAppSignature)',
    second: '(FinalityGrandpaPrevote,SpConsensusGrandpaAppSignature)'
  },
  /**
   * Lookup178: finality_grandpa::Prevote<primitive_types::H256, N>
   **/
  FinalityGrandpaPrevote: {
    targetHash: 'H256',
    targetNumber: 'u32'
  },
  /**
   * Lookup179: sp_consensus_grandpa::app::Signature
   **/
  SpConsensusGrandpaAppSignature: '[u8;64]',
  /**
   * Lookup181: finality_grandpa::Equivocation<sp_consensus_grandpa::app::Public, finality_grandpa::Precommit<primitive_types::H256, N>, sp_consensus_grandpa::app::Signature>
   **/
  FinalityGrandpaEquivocationPrecommit: {
    roundNumber: 'u64',
    identity: 'SpConsensusGrandpaAppPublic',
    first: '(FinalityGrandpaPrecommit,SpConsensusGrandpaAppSignature)',
    second: '(FinalityGrandpaPrecommit,SpConsensusGrandpaAppSignature)'
  },
  /**
   * Lookup182: finality_grandpa::Precommit<primitive_types::H256, N>
   **/
  FinalityGrandpaPrecommit: {
    targetHash: 'H256',
    targetNumber: 'u32'
  },
  /**
   * Lookup184: sp_session::MembershipProof
   **/
  SpSessionMembershipProof: {
    session: 'u32',
    trieNodes: 'Vec<Bytes>',
    validatorCount: 'u32'
  },
  /**
   * Lookup185: pallet_bitcoin_mint::pallet::Call<T>
   **/
  PalletBitcoinMintCall: {
    _enum: {
      sync: {
        utxoSync: 'UlxPrimitivesInherentsBitcoinUtxoSync',
      },
      lock: {
        bondId: 'Option<u64>',
        txid: 'UlxPrimitivesBitcoinH256Le',
        outputIndex: 'u32',
        satoshis: 'u64',
        pubkey: 'UlxPrimitivesBitcoinCompressedPublicKey',
        ownershipProofSignature: '[u8;65]',
      },
      unlock: {
        txid: 'UlxPrimitivesBitcoinH256Le',
        outputIndex: 'u32'
      }
    }
  },
  /**
   * Lookup186: ulx_primitives::inherents::BitcoinUtxoSync
   **/
  UlxPrimitivesInherentsBitcoinUtxoSync: {
    moved: 'Vec<UlxPrimitivesBitcoinBitcoinUtxoId>',
    confirmed: 'Vec<UlxPrimitivesBitcoinBitcoinUtxoId>',
    blockHash: 'UlxPrimitivesBitcoinH256Le',
    blockHeight: 'u32'
  },
  /**
   * Lookup189: ulx_primitives::bitcoin::CompressedPublicKey
   **/
  UlxPrimitivesBitcoinCompressedPublicKey: '[u8;33]',
  /**
   * Lookup191: pallet_balances::pallet::Call<T, I>
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
        delta: 'Compact<u128>'
      }
    }
  },
  /**
   * Lookup193: pallet_balances::types::AdjustmentDirection
   **/
  PalletBalancesAdjustmentDirection: {
    _enum: ['Increase', 'Decrease']
  },
  /**
   * Lookup194: pallet_ulixee_mint::pallet::Call<T>
   **/
  PalletUlixeeMintCall: 'Null',
  /**
   * Lookup196: pallet_tx_pause::pallet::Call<T>
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
   * Lookup197: pallet_sudo::pallet::Call<T>
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
   * Lookup198: pallet_proxy::pallet::Error<T>
   **/
  PalletProxyError: {
    _enum: ['TooMany', 'NotFound', 'NotProxy', 'Unproxyable', 'Duplicate', 'NoPermission', 'Unannounced', 'NoSelfProxy']
  },
  /**
   * Lookup200: pallet_ticks::pallet::Error<T>
   **/
  PalletTicksError: 'Null',
  /**
   * Lookup206: pallet_mining_slot::pallet::Error<T>
   **/
  PalletMiningSlotError: {
    _enum: ['SlotNotTakingBids', 'TooManyBlockRegistrants', 'UnableToRotateAuthority', 'InsufficientOwnershipTokens', 'InsufficientBalanceForBid', 'BidTooLow', 'BadInternalState', 'RpcHostsAreRequired', 'BidBondDurationTooShort', 'CannotRegisteredOverlappingSessions', 'BadState', 'BondNotFound', 'NoMoreBondIds', 'BondFundClosed', 'MinimumBondAmountNotMet', 'LeaseUntilBlockTooSoon', 'LeaseUntilPastFundExpiration', 'ExpirationAtBlockOverflow', 'InsufficientFunds', 'InsufficientBondFunds', 'ExpirationTooSoon', 'NoPermissions', 'NoBondFundFound', 'HoldUnexpectedlyModified', 'BondFundMaximumBondsExceeded', 'UnrecoverableHold', 'BondFundNotFound', 'BondAlreadyClosed', 'BondAlreadyLocked', 'BondLockedCannotModify', 'FeeExceedsBondAmount', 'AccountWouldBeBelowMinimum']
  },
  /**
   * Lookup207: ulx_primitives::bond::BondFund<sp_core::crypto::AccountId32, Balance, BlockNumber>
   **/
  UlxPrimitivesBondBondFund: {
    leaseAnnualPercentRate: 'Compact<u32>',
    leaseBaseFee: 'Compact<u128>',
    offerAccountId: 'AccountId32',
    amountReserved: 'Compact<u128>',
    offerExpirationBlock: 'u32',
    amountBonded: 'Compact<u128>',
    isEnded: 'bool'
  },
  /**
   * Lookup210: ulx_primitives::bond::Bond<sp_core::crypto::AccountId32, Balance, BlockNumber, BondFundId>
   **/
  UlxPrimitivesBond: {
    bondFundId: 'Option<u32>',
    bondedAccountId: 'AccountId32',
    annualPercentRate: 'Compact<u32>',
    baseFee: 'Compact<u128>',
    fee: 'Compact<u128>',
    amount: 'Compact<u128>',
    startBlock: 'Compact<u32>',
    completionBlock: 'Compact<u32>',
    isLocked: 'bool'
  },
  /**
   * Lookup213: pallet_bond::pallet::Error<T>
   **/
  PalletBondError: {
    _enum: ['BadState', 'BondNotFound', 'NoMoreBondFundIds', 'NoMoreBondIds', 'MinimumBondAmountNotMet', 'ExpirationAtBlockOverflow', 'InsufficientFunds', 'InsufficientBondFunds', 'TransactionWouldTakeAccountBelowMinimumBalance', 'BondFundClosed', 'BondFundReductionExceedsAllocatedFunds', 'ExpirationTooSoon', 'LeaseUntilBlockTooSoon', 'LeaseUntilPastFundExpiration', 'NoPermissions', 'NoBondFundFound', 'FundExtensionMustBeLater', 'HoldUnexpectedlyModified', 'BondFundMaximumBondsExceeded', 'UnrecoverableHold', 'BondFundNotFound', 'BondAlreadyLocked', 'BondLockedCannotModify', 'FeeExceedsBondAmount']
  },
  /**
   * Lookup225: pallet_notaries::pallet::Error<T>
   **/
  PalletNotariesError: {
    _enum: ['ProposalNotFound', 'MaxNotariesExceeded', 'MaxProposalsPerBlockExceeded', 'NotAnActiveNotary', 'InvalidNotaryOperator', 'NoMoreNotaryIds']
  },
  /**
   * Lookup229: ulx_primitives::notary::NotaryNotebookKeyDetails
   **/
  UlxPrimitivesNotaryNotaryNotebookKeyDetails: {
    notebookNumber: 'Compact<u32>',
    tick: 'Compact<u32>',
    blockVotesRoot: 'H256',
    secretHash: 'H256',
    parentSecret: 'Option<H256>'
  },
  /**
   * Lookup231: ulx_primitives::digests::NotebookDigest<ulx_notary_audit::error::VerifyError>
   **/
  UlxPrimitivesDigestsNotebookDigest: {
    notebooks: 'Vec<UlxPrimitivesDigestsNotebookDigestRecord>'
  },
  /**
   * Lookup233: ulx_primitives::digests::NotebookDigestRecord<ulx_notary_audit::error::VerifyError>
   **/
  UlxPrimitivesDigestsNotebookDigestRecord: {
    notaryId: 'Compact<u32>',
    notebookNumber: 'Compact<u32>',
    tick: 'Compact<u32>',
    auditFirstFailure: 'Option<UlxNotaryAuditErrorVerifyError>'
  },
  /**
   * Lookup236: pallet_notebook::pallet::Error<T>
   **/
  PalletNotebookError: {
    _enum: ['DuplicateNotebookNumber', 'MissingNotebookNumber', 'NotebookTickAlreadyUsed', 'InvalidNotebookSignature', 'InvalidSecretProvided', 'CouldNotDecodeNotebook', 'DuplicateNotebookDigest', 'MissingNotebookDigest', 'InvalidNotebookDigest', 'MultipleNotebookInherentsProvided', 'InternalError']
  },
  /**
   * Lookup237: pallet_chain_transfer::QueuedTransferOut<sp_core::crypto::AccountId32, Balance, BlockNumber>
   **/
  PalletChainTransferQueuedTransferOut: {
    accountId: 'AccountId32',
    amount: 'u128',
    expirationBlock: 'u32',
    notaryId: 'u32'
  },
  /**
   * Lookup242: frame_support::PalletId
   **/
  FrameSupportPalletId: '[u8;8]',
  /**
   * Lookup243: pallet_chain_transfer::pallet::Error<T>
   **/
  PalletChainTransferError: {
    _enum: ['MaxBlockTransfersExceeded', 'InsufficientFunds', 'InsufficientNotarizedFunds', 'InvalidOrDuplicatedLocalchainTransfer', 'NotebookIncludesExpiredLocalchainTransfer', 'InvalidNotaryUsedForTransfer']
  },
  /**
   * Lookup248: ulx_primitives::notary::NotaryNotebookVoteDigestDetails
   **/
  UlxPrimitivesNotaryNotaryNotebookVoteDigestDetails: {
    notaryId: 'Compact<u32>',
    notebookNumber: 'Compact<u32>',
    tick: 'Compact<u32>',
    blockVotesCount: 'Compact<u32>',
    blockVotingPower: 'Compact<u128>'
  },
  /**
   * Lookup250: ulx_primitives::digests::BlockVoteDigest
   **/
  UlxPrimitivesDigestsBlockVoteDigest: {
    votingPower: 'Compact<u128>',
    votesCount: 'Compact<u32>'
  },
  /**
   * Lookup254: pallet_block_seal_spec::pallet::Error<T>
   **/
  PalletBlockSealSpecError: {
    _enum: ['MaxNotebooksAtTickExceeded']
  },
  /**
   * Lookup257: pallet_data_domain::pallet::Error<T>
   **/
  PalletDataDomainError: {
    _enum: ['DomainNotRegistered', 'NotDomainOwner']
  },
  /**
   * Lookup260: pallet_price_index::pallet::Error<T>
   **/
  PalletPriceIndexError: {
    _enum: ['NotAuthorizedOperator', 'MissingValue', 'HistoryRecordingError']
  },
  /**
   * Lookup265: sp_core::crypto::KeyTypeId
   **/
  SpCoreCryptoKeyTypeId: '[u8;4]',
  /**
   * Lookup266: pallet_session::pallet::Error<T>
   **/
  PalletSessionError: {
    _enum: ['InvalidProof', 'NoAssociatedValidatorId', 'DuplicatedKey', 'NoKeys', 'NoAccount']
  },
  /**
   * Lookup267: ulx_primitives::providers::BlockSealerInfo<sp_core::crypto::AccountId32>
   **/
  UlxPrimitivesProvidersBlockSealerInfo: {
    minerRewardsAccount: 'AccountId32',
    blockVoteRewardsAccount: 'AccountId32'
  },
  /**
   * Lookup268: ulx_primitives::digests::ParentVotingKeyDigest
   **/
  UlxPrimitivesDigestsParentVotingKeyDigest: {
    parentVotingKey: 'Option<H256>'
  },
  /**
   * Lookup269: pallet_block_seal::pallet::Error<T>
   **/
  PalletBlockSealError: {
    _enum: ['InvalidVoteSealStrength', 'InvalidSubmitter', 'UnableToDecodeVoteAccount', 'UnregisteredBlockAuthor', 'InvalidBlockVoteProof', 'NoGrandparentVoteMinimum', 'DuplicateBlockSealProvided', 'InsufficientVotingPower', 'ParentVotingKeyNotFound', 'InvalidVoteGrandparentHash', 'IneligibleNotebookUsed', 'NoEligibleVotingRoot', 'UnregisteredDataDomain', 'InvalidDataDomainAccount', 'InvalidAuthoritySignature', 'CouldNotDecodeVote', 'MaxNotebooksAtTickExceeded', 'NoClosestMinerFoundForVote', 'BlockVoteInvalidSignature']
  },
  /**
   * Lookup271: pallet_block_rewards::pallet::Error<T>
   **/
  PalletBlockRewardsError: 'Null',
  /**
   * Lookup272: pallet_grandpa::StoredState<N>
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
   * Lookup273: pallet_grandpa::StoredPendingChange<N, Limit>
   **/
  PalletGrandpaStoredPendingChange: {
    scheduledAt: 'u32',
    delay: 'u32',
    nextAuthorities: 'Vec<(SpConsensusGrandpaAppPublic,u64)>',
    forced: 'Option<u32>'
  },
  /**
   * Lookup275: pallet_grandpa::pallet::Error<T>
   **/
  PalletGrandpaError: {
    _enum: ['PauseFailed', 'ResumeFailed', 'ChangePending', 'TooSoon', 'InvalidKeyOwnershipProof', 'InvalidEquivocationProof', 'DuplicateOffenceReport']
  },
  /**
   * Lookup276: sp_staking::offence::OffenceDetails<sp_core::crypto::AccountId32, Offender>
   **/
  SpStakingOffenceOffenceDetails: {
    offender: '(AccountId32,PalletMiningSlotMinerHistory)',
    reporters: 'Vec<AccountId32>'
  },
  /**
   * Lookup278: pallet_mining_slot::MinerHistory
   **/
  PalletMiningSlotMinerHistory: {
    authorityIndex: 'u32'
  },
  /**
   * Lookup280: pallet_bitcoin_mint::LockedUtxo<sp_core::crypto::AccountId32, BondId, Balance, BlockNumber>
   **/
  PalletBitcoinMintLockedUtxo: {
    accountId: 'AccountId32',
    bondId: 'u64',
    amount: 'u128',
    satoshis: 'u64',
    expirationBlock: 'u32'
  },
  /**
   * Lookup284: pallet_bitcoin_mint::LockedUtxoPendingConfirmation<sp_core::crypto::AccountId32, BondId, Balance, BlockNumber>
   **/
  PalletBitcoinMintLockedUtxoPendingConfirmation: {
    accountId: 'AccountId32',
    bondId: 'u64',
    amount: 'u128',
    satoshis: 'u64',
    expirationBlock: 'u32',
    publicKey: 'UlxPrimitivesBitcoinCompressedPublicKey',
    ownershipProofSignature: '[u8;65]'
  },
  /**
   * Lookup292: pallet_bitcoin_mint::pallet::Error<T>
   **/
  PalletBitcoinMintError: {
    _enum: ['InvalidBondSubmitted', 'InsufficientBitcoinAmount', 'InsufficientBondAmount', 'PrematureBondExpiration', 'NoBitcoinPricesAvailable', 'BitcoinAlreadyLocked', 'MaxPendingMintUtxosExceeded', 'UtxoNotLocked', 'RedemptionsUnavailable', 'BadState', 'BondNotFound', 'NoMoreBondIds', 'BondFundClosed', 'MinimumBondAmountNotMet', 'LeaseUntilBlockTooSoon', 'LeaseUntilPastFundExpiration', 'ExpirationAtBlockOverflow', 'InsufficientFunds', 'InsufficientBondFunds', 'ExpirationTooSoon', 'NoPermissions', 'NoBondFundFound', 'HoldUnexpectedlyModified', 'BondFundMaximumBondsExceeded', 'UnrecoverableHold', 'BondFundNotFound', 'BondAlreadyClosed', 'BondAlreadyLocked', 'BondLockedCannotModify', 'FeeExceedsBondAmount', 'AccountWouldBeBelowMinimum']
  },
  /**
   * Lookup294: pallet_balances::types::BalanceLock<Balance>
   **/
  PalletBalancesBalanceLock: {
    id: '[u8;8]',
    amount: 'u128',
    reasons: 'PalletBalancesReasons'
  },
  /**
   * Lookup295: pallet_balances::types::Reasons
   **/
  PalletBalancesReasons: {
    _enum: ['Fee', 'Misc', 'All']
  },
  /**
   * Lookup298: pallet_balances::types::ReserveData<ReserveIdentifier, Balance>
   **/
  PalletBalancesReserveData: {
    id: '[u8;8]',
    amount: 'u128'
  },
  /**
   * Lookup301: pallet_balances::types::IdAmount<ulx_node_runtime::RuntimeHoldReason, Balance>
   **/
  PalletBalancesIdAmountRuntimeHoldReason: {
    id: 'UlxNodeRuntimeRuntimeHoldReason',
    amount: 'u128'
  },
  /**
   * Lookup302: ulx_node_runtime::RuntimeHoldReason
   **/
  UlxNodeRuntimeRuntimeHoldReason: {
    _enum: {
      __Unused0: 'Null',
      __Unused1: 'Null',
      __Unused2: 'Null',
      __Unused3: 'Null',
      MiningSlot: 'PalletMiningSlotHoldReason',
      Bond: 'PalletBondHoldReason',
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
      BlockRewards: 'PalletBlockRewardsHoldReason',
      __Unused17: 'Null',
      __Unused18: 'Null',
      BitcoinMint: 'PalletBitcoinMintHoldReason',
      __Unused20: 'Null',
      UlixeeMint: 'PalletUlixeeMintHoldReason'
    }
  },
  /**
   * Lookup303: pallet_mining_slot::pallet::HoldReason
   **/
  PalletMiningSlotHoldReason: {
    _enum: ['RegisterAsMiner']
  },
  /**
   * Lookup304: pallet_bond::pallet::HoldReason
   **/
  PalletBondHoldReason: {
    _enum: ['EnterBondFund']
  },
  /**
   * Lookup305: pallet_block_rewards::pallet::HoldReason
   **/
  PalletBlockRewardsHoldReason: {
    _enum: ['MaturationPeriod']
  },
  /**
   * Lookup306: pallet_bitcoin_mint::pallet::HoldReason
   **/
  PalletBitcoinMintHoldReason: 'Null',
  /**
   * Lookup307: pallet_ulixee_mint::pallet::HoldReason
   **/
  PalletUlixeeMintHoldReason: 'Null',
  /**
   * Lookup310: pallet_balances::types::IdAmount<ulx_node_runtime::RuntimeFreezeReason, Balance>
   **/
  PalletBalancesIdAmountRuntimeFreezeReason: {
    id: 'UlxNodeRuntimeRuntimeFreezeReason',
    amount: 'u128'
  },
  /**
   * Lookup311: ulx_node_runtime::RuntimeFreezeReason
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
      BlockRewards: 'PalletBlockRewardsFreezeReason'
    }
  },
  /**
   * Lookup312: pallet_block_rewards::pallet::FreezeReason
   **/
  PalletBlockRewardsFreezeReason: {
    _enum: ['MaturationPeriod']
  },
  /**
   * Lookup314: pallet_balances::pallet::Error<T, I>
   **/
  PalletBalancesError: {
    _enum: ['VestingBalance', 'LiquidityRestrictions', 'InsufficientBalance', 'ExistentialDeposit', 'Expendability', 'ExistingVestingSchedule', 'DeadAccount', 'TooManyReserves', 'TooManyHolds', 'TooManyFreezes', 'IssuanceDeactivated', 'DeltaZero']
  },
  /**
   * Lookup315: pallet_ulixee_mint::pallet::Error<T>
   **/
  PalletUlixeeMintError: 'Null',
  /**
   * Lookup317: pallet_tx_pause::pallet::Error<T>
   **/
  PalletTxPauseError: {
    _enum: ['IsPaused', 'IsUnpaused', 'Unpausable', 'NotFound']
  },
  /**
   * Lookup319: pallet_transaction_payment::Releases
   **/
  PalletTransactionPaymentReleases: {
    _enum: ['V1Ancient', 'V2']
  },
  /**
   * Lookup320: pallet_sudo::pallet::Error<T>
   **/
  PalletSudoError: {
    _enum: ['RequireSudo']
  },
  /**
   * Lookup323: frame_system::extensions::check_non_zero_sender::CheckNonZeroSender<T>
   **/
  FrameSystemExtensionsCheckNonZeroSender: 'Null',
  /**
   * Lookup324: frame_system::extensions::check_spec_version::CheckSpecVersion<T>
   **/
  FrameSystemExtensionsCheckSpecVersion: 'Null',
  /**
   * Lookup325: frame_system::extensions::check_tx_version::CheckTxVersion<T>
   **/
  FrameSystemExtensionsCheckTxVersion: 'Null',
  /**
   * Lookup326: frame_system::extensions::check_genesis::CheckGenesis<T>
   **/
  FrameSystemExtensionsCheckGenesis: 'Null',
  /**
   * Lookup329: frame_system::extensions::check_nonce::CheckNonce<T>
   **/
  FrameSystemExtensionsCheckNonce: 'Compact<u32>',
  /**
   * Lookup330: frame_system::extensions::check_weight::CheckWeight<T>
   **/
  FrameSystemExtensionsCheckWeight: 'Null',
  /**
   * Lookup331: pallet_transaction_payment::ChargeTransactionPayment<T>
   **/
  PalletTransactionPaymentChargeTransactionPayment: 'Compact<u128>',
  /**
   * Lookup332: ulx_node_runtime::Runtime
   **/
  UlxNodeRuntimeRuntime: 'Null'
};
