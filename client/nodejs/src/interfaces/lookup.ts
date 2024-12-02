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
   * Lookup20: frame_system::EventRecord<argon_runtime::RuntimeEvent, primitive_types::H256>
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
   * Lookup31: pallet_digests::pallet::Event<T>
   **/
  PalletDigestsEvent: 'Null',
  /**
   * Lookup32: pallet_multisig::pallet::Event<T>
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
   * Lookup33: pallet_multisig::Timepoint<BlockNumber>
   **/
  PalletMultisigTimepoint: {
    height: 'u32',
    index: 'u32'
  },
  /**
   * Lookup36: pallet_proxy::pallet::Event<T>
   **/
  PalletProxyEvent: {
    _enum: {
      ProxyExecuted: {
        result: 'Result<Null, SpRuntimeDispatchError>',
      },
      PureCreated: {
        pure: 'AccountId32',
        who: 'AccountId32',
        proxyType: 'ArgonRuntimeConfigsProxyType',
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
        proxyType: 'ArgonRuntimeConfigsProxyType',
        delay: 'u32',
      },
      ProxyRemoved: {
        delegator: 'AccountId32',
        delegatee: 'AccountId32',
        proxyType: 'ArgonRuntimeConfigsProxyType',
        delay: 'u32'
      }
    }
  },
  /**
   * Lookup37: argon_runtime::configs::ProxyType
   **/
  ArgonRuntimeConfigsProxyType: {
    _enum: ['Any', 'NonTransfer', 'PriceIndex']
  },
  /**
   * Lookup39: pallet_mining_slot::pallet::Event<T>
   **/
  PalletMiningSlotEvent: {
    _enum: {
      NewMiners: {
        startIndex: 'u32',
        newMiners: 'Vec<ArgonPrimitivesBlockSealMiningRegistration>',
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
   * Lookup41: argon_primitives::block_seal::MiningRegistration<sp_core::crypto::AccountId32, Balance, argon_runtime::SessionKeys>
   **/
  ArgonPrimitivesBlockSealMiningRegistration: {
    accountId: 'AccountId32',
    rewardDestination: 'ArgonPrimitivesBlockSealRewardDestination',
    bondId: 'Option<u64>',
    bondAmount: 'Compact<u128>',
    ownershipTokens: 'Compact<u128>',
    rewardSharing: 'Option<ArgonPrimitivesBlockSealRewardSharing>',
    authorityKeys: 'ArgonRuntimeSessionKeys'
  },
  /**
   * Lookup42: argon_runtime::SessionKeys
   **/
  ArgonRuntimeSessionKeys: {
    grandpa: 'SpConsensusGrandpaAppPublic',
    blockSealAuthority: 'ArgonPrimitivesBlockSealAppPublic'
  },
  /**
   * Lookup43: sp_consensus_grandpa::app::Public
   **/
  SpConsensusGrandpaAppPublic: '[u8;32]',
  /**
   * Lookup44: argon_primitives::block_seal::app::Public
   **/
  ArgonPrimitivesBlockSealAppPublic: '[u8;32]',
  /**
   * Lookup45: argon_primitives::block_seal::RewardDestination<sp_core::crypto::AccountId32>
   **/
  ArgonPrimitivesBlockSealRewardDestination: {
    _enum: {
      Owner: 'Null',
      Account: 'AccountId32'
    }
  },
  /**
   * Lookup49: argon_primitives::block_seal::RewardSharing<sp_core::crypto::AccountId32>
   **/
  ArgonPrimitivesBlockSealRewardSharing: {
    accountId: 'AccountId32',
    percentTake: 'Compact<u128>'
  },
  /**
   * Lookup53: pallet_bitcoin_utxos::pallet::Event<T>
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
        error: 'SpRuntimeDispatchError'
      }
    }
  },
  /**
   * Lookup54: argon_primitives::bitcoin::BitcoinRejectedReason
   **/
  ArgonPrimitivesBitcoinBitcoinRejectedReason: {
    _enum: ['SatoshisMismatch', 'Spent', 'LookupExpired', 'DuplicateUtxo']
  },
  /**
   * Lookup55: argon_primitives::bitcoin::UtxoRef
   **/
  ArgonPrimitivesBitcoinUtxoRef: {
    txid: 'ArgonPrimitivesBitcoinH256Le',
    outputIndex: 'Compact<u32>'
  },
  /**
   * Lookup56: argon_primitives::bitcoin::H256Le
   **/
  ArgonPrimitivesBitcoinH256Le: '[u8;32]',
  /**
   * Lookup58: pallet_vaults::pallet::Event<T>
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
        securitizationStillBonded: 'u128',
      },
      VaultBitcoinXpubChange: {
        vaultId: 'u32'
      }
    }
  },
  /**
   * Lookup59: pallet_bond::pallet::Event<T>
   **/
  PalletBondEvent: {
    _enum: {
      BondCreated: {
        vaultId: 'u32',
        bondId: 'u64',
        bondType: 'ArgonPrimitivesBondBondType',
        bondedAccountId: 'AccountId32',
        utxoId: 'Option<u64>',
        amount: 'u128',
        expiration: 'ArgonPrimitivesBondBondExpiration',
      },
      BondCompleted: {
        vaultId: 'u32',
        bondId: 'u64',
      },
      BondCanceled: {
        vaultId: 'u32',
        bondId: 'u64',
        bondedAccountId: 'AccountId32',
        bondType: 'ArgonPrimitivesBondBondType',
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
   * Lookup60: argon_primitives::bond::BondType
   **/
  ArgonPrimitivesBondBondType: {
    _enum: ['Mining', 'Bitcoin']
  },
  /**
   * Lookup61: argon_primitives::bond::BondExpiration<BlockNumber>
   **/
  ArgonPrimitivesBondBondExpiration: {
    _enum: {
      ArgonBlock: 'Compact<u32>',
      BitcoinBlock: 'Compact<u64>'
    }
  },
  /**
   * Lookup64: pallet_notaries::pallet::Event<T>
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
        meta: 'ArgonPrimitivesNotaryNotaryMeta'
      }
    }
  },
  /**
   * Lookup65: argon_primitives::notary::NotaryMeta<MaxHosts>
   **/
  ArgonPrimitivesNotaryNotaryMeta: {
    name: 'Bytes',
    public: '[u8;32]',
    hosts: 'Vec<Bytes>'
  },
  /**
   * Lookup72: argon_primitives::notary::NotaryRecord<sp_core::crypto::AccountId32, BlockNumber, MaxHosts>
   **/
  ArgonPrimitivesNotaryNotaryRecord: {
    notaryId: 'Compact<u32>',
    operatorAccountId: 'AccountId32',
    activatedBlock: 'Compact<u32>',
    metaUpdatedBlock: 'Compact<u32>',
    metaUpdatedTick: 'Compact<u64>',
    meta: 'ArgonPrimitivesNotaryNotaryMeta'
  },
  /**
   * Lookup73: pallet_notebook::pallet::Event<T>
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
        notebookNumber: 'u32'
      }
    }
  },
  /**
   * Lookup74: argon_notary_audit::error::VerifyError
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
      NoDefaultBlockVote: 'Null'
    }
  },
  /**
   * Lookup75: argon_primitives::account::AccountType
   **/
  ArgonPrimitivesAccountAccountType: {
    _enum: ['Tax', 'Deposit']
  },
  /**
   * Lookup76: argon_notary_audit::AccountHistoryLookupError
   **/
  ArgonNotaryAuditAccountHistoryLookupError: {
    _enum: ['RootNotFound', 'LastChangeNotFound', 'InvalidTransferToLocalchain', 'BlockSpecificationNotFound']
  },
  /**
   * Lookup79: pallet_chain_transfer::pallet::Event<T>
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
      TransferToEvm: {
        from: 'AccountId32',
        to: 'H160',
        amount: 'u128',
        evmChain: 'PalletChainTransferIsmpModuleEvmChain',
        asset: 'PalletChainTransferIsmpModuleAsset',
        commitment: 'H256',
      },
      TransferToEvmExpired: {
        from: 'AccountId32',
        to: 'H160',
        amount: 'u128',
        evmChain: 'PalletChainTransferIsmpModuleEvmChain',
        asset: 'PalletChainTransferIsmpModuleAsset',
      },
      TransferFromEvm: {
        from: 'H160',
        to: 'AccountId32',
        asset: 'PalletChainTransferIsmpModuleAsset',
        amount: 'u128',
        evmChain: 'PalletChainTransferIsmpModuleEvmChain',
      },
      TransferFromEvmWhilePaused: {
        from: 'H160',
        to: 'AccountId32',
        asset: 'PalletChainTransferIsmpModuleAsset',
        amount: 'u128',
        evmChain: 'PalletChainTransferIsmpModuleEvmChain',
      },
      ERC6160AssetRegistrationDispatched: {
        commitment: 'H256',
        asset: 'PalletChainTransferIsmpModuleAsset',
        addedChains: 'Vec<IsmpHostStateMachine>',
        removedChains: 'Vec<IsmpHostStateMachine>'
      }
    }
  },
  /**
   * Lookup82: pallet_chain_transfer::ismp_module::EvmChain
   **/
  PalletChainTransferIsmpModuleEvmChain: {
    _enum: ['Ethereum', 'Base', 'Arbitrum', 'Optimism', 'Gnosis', 'BinanceSmartChain']
  },
  /**
   * Lookup83: pallet_chain_transfer::ismp_module::Asset
   **/
  PalletChainTransferIsmpModuleAsset: {
    _enum: ['Argon', 'OwnershipToken']
  },
  /**
   * Lookup85: ismp::host::StateMachine
   **/
  IsmpHostStateMachine: {
    _enum: {
      Evm: 'u32',
      Polkadot: 'u32',
      Kusama: 'u32',
      Substrate: '[u8;4]',
      Tendermint: '[u8;4]'
    }
  },
  /**
   * Lookup87: pallet_block_seal_spec::pallet::Event<T>
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
   * Lookup88: pallet_domains::pallet::Event<T>
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
        error: 'SpRuntimeDispatchError'
      }
    }
  },
  /**
   * Lookup89: argon_primitives::domain::ZoneRecord<sp_core::crypto::AccountId32>
   **/
  ArgonPrimitivesDomainZoneRecord: {
    paymentAccount: 'AccountId32',
    notaryId: 'u32',
    versions: 'BTreeMap<ArgonPrimitivesDomainSemver, ArgonPrimitivesDomainVersionHost>'
  },
  /**
   * Lookup91: argon_primitives::domain::Semver
   **/
  ArgonPrimitivesDomainSemver: {
    major: 'u32',
    minor: 'u32',
    patch: 'u32'
  },
  /**
   * Lookup92: argon_primitives::domain::VersionHost
   **/
  ArgonPrimitivesDomainVersionHost: {
    datastoreId: 'Bytes',
    host: 'Bytes'
  },
  /**
   * Lookup96: pallet_domains::DomainRegistration<sp_core::crypto::AccountId32>
   **/
  PalletDomainsDomainRegistration: {
    accountId: 'AccountId32',
    registeredAtTick: 'u64'
  },
  /**
   * Lookup97: pallet_price_index::pallet::Event<T>
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
   * Lookup98: pallet_grandpa::pallet::Event
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
   * Lookup101: pallet_block_rewards::pallet::Event<T>
   **/
  PalletBlockRewardsEvent: {
    _enum: {
      RewardCreated: {
        maturationBlock: 'u32',
        rewards: 'Vec<ArgonPrimitivesBlockSealBlockPayout>',
      },
      RewardUnlocked: {
        rewards: 'Vec<ArgonPrimitivesBlockSealBlockPayout>',
      },
      RewardUnlockError: {
        accountId: 'AccountId32',
        argons: 'Option<u128>',
        ownership: 'Option<u128>',
        error: 'SpRuntimeDispatchError',
      },
      RewardCreateError: {
        accountId: 'AccountId32',
        argons: 'Option<u128>',
        ownership: 'Option<u128>',
        error: 'SpRuntimeDispatchError'
      }
    }
  },
  /**
   * Lookup103: argon_primitives::block_seal::BlockPayout<sp_core::crypto::AccountId32, Balance>
   **/
  ArgonPrimitivesBlockSealBlockPayout: {
    accountId: 'AccountId32',
    ownership: 'Compact<u128>',
    argons: 'Compact<u128>'
  },
  /**
   * Lookup105: pallet_mint::pallet::Event<T>
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
   * Lookup106: pallet_mint::pallet::MintType
   **/
  PalletMintMintType: {
    _enum: ['Bitcoin', 'Mining']
  },
  /**
   * Lookup107: pallet_balances::pallet::Event<T, I>
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
   * Lookup108: frame_support::traits::tokens::misc::BalanceStatus
   **/
  FrameSupportTokensMiscBalanceStatus: {
    _enum: ['Free', 'Reserved']
  },
  /**
   * Lookup110: pallet_tx_pause::pallet::Event<T>
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
   * Lookup113: pallet_transaction_payment::pallet::Event<T>
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
   * Lookup114: pallet_utility::pallet::Event
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
        result: 'Result<Null, SpRuntimeDispatchError>'
      }
    }
  },
  /**
   * Lookup115: pallet_sudo::pallet::Event<T>
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
   * Lookup117: pallet_ismp::pallet::Event<T>
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
      GetRequestTimeoutHandled: 'IsmpEventsTimeoutHandled'
    }
  },
  /**
   * Lookup118: ismp::consensus::StateMachineId
   **/
  IsmpConsensusStateMachineId: {
    stateId: 'IsmpHostStateMachine',
    consensusStateId: '[u8;4]'
  },
  /**
   * Lookup119: ismp::consensus::StateMachineHeight
   **/
  IsmpConsensusStateMachineHeight: {
    id: 'IsmpConsensusStateMachineId',
    height: 'u64'
  },
  /**
   * Lookup122: pallet_ismp::errors::HandlingError
   **/
  PalletIsmpErrorsHandlingError: {
    message: 'Bytes'
  },
  /**
   * Lookup124: ismp::events::RequestResponseHandled
   **/
  IsmpEventsRequestResponseHandled: {
    commitment: 'H256',
    relayer: 'Bytes'
  },
  /**
   * Lookup125: ismp::events::TimeoutHandled
   **/
  IsmpEventsTimeoutHandled: {
    commitment: 'H256',
    source: 'IsmpHostStateMachine',
    dest: 'IsmpHostStateMachine'
  },
  /**
   * Lookup126: ismp_grandpa::pallet::Event<T>
   **/
  IsmpGrandpaEvent: {
    _enum: {
      StateMachineAdded: {
        stateMachines: 'Vec<IsmpHostStateMachine>',
      },
      StateMachineRemoved: {
        stateMachines: 'Vec<IsmpHostStateMachine>'
      }
    }
  },
  /**
   * Lookup127: pallet_hyperbridge::pallet::Event<T>
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
        account: 'AccountId32'
      }
    }
  },
  /**
   * Lookup128: pallet_hyperbridge::VersionedHostParams<Balance>
   **/
  PalletHyperbridgeVersionedHostParams: {
    _enum: {
      V1: 'PalletHyperbridgeSubstrateHostParams'
    }
  },
  /**
   * Lookup129: pallet_hyperbridge::SubstrateHostParams<B>
   **/
  PalletHyperbridgeSubstrateHostParams: {
    defaultPerByteFee: 'u128',
    perByteFees: 'BTreeMap<IsmpHostStateMachine, u128>',
    assetRegistrationFee: 'u128'
  },
  /**
   * Lookup133: frame_system::Phase
   **/
  FrameSystemPhase: {
    _enum: {
      ApplyExtrinsic: 'u32',
      Finalization: 'Null',
      Initialization: 'Null'
    }
  },
  /**
   * Lookup137: frame_system::LastRuntimeUpgradeInfo
   **/
  FrameSystemLastRuntimeUpgradeInfo: {
    specVersion: 'Compact<u32>',
    specName: 'Text'
  },
  /**
   * Lookup138: frame_system::CodeUpgradeAuthorization<T>
   **/
  FrameSystemCodeUpgradeAuthorization: {
    codeHash: 'H256',
    checkVersion: 'bool'
  },
  /**
   * Lookup139: frame_system::pallet::Call<T>
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
   * Lookup143: frame_system::limits::BlockWeights
   **/
  FrameSystemLimitsBlockWeights: {
    baseBlock: 'SpWeightsWeightV2Weight',
    maxBlock: 'SpWeightsWeightV2Weight',
    perClass: 'FrameSupportDispatchPerDispatchClassWeightsPerClass'
  },
  /**
   * Lookup144: frame_support::dispatch::PerDispatchClass<frame_system::limits::WeightsPerClass>
   **/
  FrameSupportDispatchPerDispatchClassWeightsPerClass: {
    normal: 'FrameSystemLimitsWeightsPerClass',
    operational: 'FrameSystemLimitsWeightsPerClass',
    mandatory: 'FrameSystemLimitsWeightsPerClass'
  },
  /**
   * Lookup145: frame_system::limits::WeightsPerClass
   **/
  FrameSystemLimitsWeightsPerClass: {
    baseExtrinsic: 'SpWeightsWeightV2Weight',
    maxExtrinsic: 'Option<SpWeightsWeightV2Weight>',
    maxTotal: 'Option<SpWeightsWeightV2Weight>',
    reserved: 'Option<SpWeightsWeightV2Weight>'
  },
  /**
   * Lookup147: frame_system::limits::BlockLength
   **/
  FrameSystemLimitsBlockLength: {
    max: 'FrameSupportDispatchPerDispatchClassU32'
  },
  /**
   * Lookup148: frame_support::dispatch::PerDispatchClass<T>
   **/
  FrameSupportDispatchPerDispatchClassU32: {
    normal: 'u32',
    operational: 'u32',
    mandatory: 'u32'
  },
  /**
   * Lookup149: sp_weights::RuntimeDbWeight
   **/
  SpWeightsRuntimeDbWeight: {
    read: 'u64',
    write: 'u64'
  },
  /**
   * Lookup150: sp_version::RuntimeVersion
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
   * Lookup155: frame_system::pallet::Error<T>
   **/
  FrameSystemError: {
    _enum: ['InvalidSpecName', 'SpecVersionNeedsToIncrease', 'FailedToExtractRuntimeVersion', 'NonDefaultComposite', 'NonZeroRefCount', 'CallFiltered', 'MultiBlockMigrationsOngoing', 'NothingAuthorized', 'Unauthorized']
  },
  /**
   * Lookup156: argon_primitives::digests::Digestset<argon_notary_audit::error::VerifyError, sp_core::crypto::AccountId32>
   **/
  ArgonPrimitivesDigestsDigestset: {
    author: 'AccountId32',
    blockVote: 'ArgonPrimitivesDigestsBlockVoteDigest',
    votingKey: 'Option<ArgonPrimitivesDigestsParentVotingKeyDigest>',
    forkPower: 'Option<ArgonPrimitivesForkPower>',
    tick: 'u64',
    notebooks: 'ArgonPrimitivesDigestsNotebookDigest'
  },
  /**
   * Lookup157: argon_primitives::digests::BlockVoteDigest
   **/
  ArgonPrimitivesDigestsBlockVoteDigest: {
    votingPower: 'Compact<u128>',
    votesCount: 'Compact<u32>'
  },
  /**
   * Lookup159: argon_primitives::digests::ParentVotingKeyDigest
   **/
  ArgonPrimitivesDigestsParentVotingKeyDigest: {
    parentVotingKey: 'Option<H256>'
  },
  /**
   * Lookup162: argon_primitives::fork_power::ForkPower
   **/
  ArgonPrimitivesForkPower: {
    isLatestVote: 'bool',
    notebooks: 'Compact<u64>',
    votingPower: 'U256',
    sealStrength: 'U256',
    totalComputeDifficulty: 'U256',
    voteCreatedBlocks: 'Compact<u128>'
  },
  /**
   * Lookup166: argon_primitives::digests::NotebookDigest<argon_notary_audit::error::VerifyError>
   **/
  ArgonPrimitivesDigestsNotebookDigest: {
    notebooks: 'Vec<ArgonPrimitivesNotebookNotebookAuditResult>'
  },
  /**
   * Lookup168: argon_primitives::notebook::NotebookAuditResult<argon_notary_audit::error::VerifyError>
   **/
  ArgonPrimitivesNotebookNotebookAuditResult: {
    notaryId: 'Compact<u32>',
    notebookNumber: 'Compact<u32>',
    tick: 'Compact<u64>',
    auditFirstFailure: 'Option<ArgonNotaryAuditErrorVerifyError>'
  },
  /**
   * Lookup170: pallet_digests::pallet::Error<T>
   **/
  PalletDigestsError: {
    _enum: ['DuplicateBlockVoteDigest', 'DuplicateAuthorDigest', 'DuplicateTickDigest', 'DuplicateParentVotingKeyDigest', 'DuplicateNotebookDigest', 'DuplicateForkPowerDigest', 'MissingBlockVoteDigest', 'MissingAuthorDigest', 'MissingTickDigest', 'MissingParentVotingKeyDigest', 'MissingNotebookDigest', 'CouldNotDecodeDigest']
  },
  /**
   * Lookup171: pallet_timestamp::pallet::Call<T>
   **/
  PalletTimestampCall: {
    _enum: {
      set: {
        now: 'Compact<u64>'
      }
    }
  },
  /**
   * Lookup173: pallet_multisig::Multisig<BlockNumber, Balance, sp_core::crypto::AccountId32, MaxApprovals>
   **/
  PalletMultisigMultisig: {
    when: 'PalletMultisigTimepoint',
    deposit: 'u128',
    depositor: 'AccountId32',
    approvals: 'Vec<AccountId32>'
  },
  /**
   * Lookup176: pallet_multisig::pallet::Call<T>
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
   * Lookup178: pallet_proxy::pallet::Call<T>
   **/
  PalletProxyCall: {
    _enum: {
      proxy: {
        real: 'MultiAddress',
        forceProxyType: 'Option<ArgonRuntimeConfigsProxyType>',
        call: 'Call',
      },
      add_proxy: {
        delegate: 'MultiAddress',
        proxyType: 'ArgonRuntimeConfigsProxyType',
        delay: 'u32',
      },
      remove_proxy: {
        delegate: 'MultiAddress',
        proxyType: 'ArgonRuntimeConfigsProxyType',
        delay: 'u32',
      },
      remove_proxies: 'Null',
      create_pure: {
        proxyType: 'ArgonRuntimeConfigsProxyType',
        delay: 'u32',
        index: 'u16',
      },
      kill_pure: {
        spawner: 'MultiAddress',
        proxyType: 'ArgonRuntimeConfigsProxyType',
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
        forceProxyType: 'Option<ArgonRuntimeConfigsProxyType>',
        call: 'Call'
      }
    }
  },
  /**
   * Lookup182: pallet_ticks::pallet::Call<T>
   **/
  PalletTicksCall: 'Null',
  /**
   * Lookup183: pallet_mining_slot::pallet::Call<T>
   **/
  PalletMiningSlotCall: {
    _enum: {
      bid: {
        _alias: {
          keys_: 'keys',
        },
        bondInfo: 'Option<PalletMiningSlotMiningSlotBid>',
        rewardDestination: 'ArgonPrimitivesBlockSealRewardDestination',
        keys_: 'ArgonRuntimeSessionKeys'
      }
    }
  },
  /**
   * Lookup185: pallet_mining_slot::MiningSlotBid<VaultId, Balance>
   **/
  PalletMiningSlotMiningSlotBid: {
    vaultId: 'u32',
    amount: 'u128'
  },
  /**
   * Lookup186: pallet_bitcoin_utxos::pallet::Call<T>
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
        accountId: 'AccountId32'
      }
    }
  },
  /**
   * Lookup187: argon_primitives::inherents::BitcoinUtxoSync
   **/
  ArgonPrimitivesInherentsBitcoinUtxoSync: {
    spent: 'BTreeMap<u64, u64>',
    verified: 'BTreeMap<u64, ArgonPrimitivesBitcoinUtxoRef>',
    invalid: 'BTreeMap<u64, ArgonPrimitivesBitcoinBitcoinRejectedReason>',
    syncToBlock: 'ArgonPrimitivesBitcoinBitcoinBlock'
  },
  /**
   * Lookup197: argon_primitives::bitcoin::BitcoinBlock
   **/
  ArgonPrimitivesBitcoinBitcoinBlock: {
    blockHeight: 'Compact<u64>',
    blockHash: 'ArgonPrimitivesBitcoinH256Le'
  },
  /**
   * Lookup198: pallet_vaults::pallet::Call<T>
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
        terms: 'ArgonPrimitivesBondVaultTerms',
      },
      close: {
        vaultId: 'u32',
      },
      replace_bitcoin_xpub: {
        vaultId: 'u32',
        bitcoinXpub: 'ArgonPrimitivesBitcoinOpaqueBitcoinXpub'
      }
    }
  },
  /**
   * Lookup199: pallet_vaults::pallet::VaultConfig<Balance>
   **/
  PalletVaultsVaultConfig: {
    terms: 'ArgonPrimitivesBondVaultTerms',
    bitcoinAmountAllocated: 'Compact<u128>',
    bitcoinXpubkey: 'ArgonPrimitivesBitcoinOpaqueBitcoinXpub',
    miningAmountAllocated: 'Compact<u128>',
    securitizationPercent: 'Compact<u128>'
  },
  /**
   * Lookup200: argon_primitives::bond::VaultTerms<Balance>
   **/
  ArgonPrimitivesBondVaultTerms: {
    bitcoinAnnualPercentRate: 'Compact<u128>',
    bitcoinBaseFee: 'Compact<u128>',
    miningAnnualPercentRate: 'Compact<u128>',
    miningBaseFee: 'Compact<u128>',
    miningRewardSharingPercentTake: 'Compact<u128>'
  },
  /**
   * Lookup201: argon_primitives::bitcoin::OpaqueBitcoinXpub
   **/
  ArgonPrimitivesBitcoinOpaqueBitcoinXpub: '[u8;78]',
  /**
   * Lookup203: pallet_bond::pallet::Call<T>
   **/
  PalletBondCall: {
    _enum: {
      bond_bitcoin: {
        vaultId: 'u32',
        satoshis: 'Compact<u64>',
        bitcoinPubkey: 'ArgonPrimitivesBitcoinCompressedBitcoinPubkey',
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
        signature: 'Bytes'
      }
    }
  },
  /**
   * Lookup204: argon_primitives::bitcoin::CompressedBitcoinPubkey
   **/
  ArgonPrimitivesBitcoinCompressedBitcoinPubkey: '[u8;33]',
  /**
   * Lookup208: pallet_notaries::pallet::Call<T>
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
        effectiveTick: 'Compact<u64>'
      }
    }
  },
  /**
   * Lookup209: pallet_notebook::pallet::Call<T>
   **/
  PalletNotebookCall: {
    _enum: {
      submit: {
        notebooks: 'Vec<ArgonPrimitivesNotebookSignedNotebookHeader>',
      },
      unlock: {
        notaryId: 'u32'
      }
    }
  },
  /**
   * Lookup211: argon_primitives::notebook::SignedNotebookHeader
   **/
  ArgonPrimitivesNotebookSignedNotebookHeader: {
    header: 'ArgonPrimitivesNotebookNotebookHeader',
    signature: '[u8;64]'
  },
  /**
   * Lookup212: argon_primitives::notebook::NotebookHeader
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
    domains: 'Vec<(H256,AccountId32)>'
  },
  /**
   * Lookup215: argon_primitives::notebook::ChainTransfer
   **/
  ArgonPrimitivesNotebookChainTransfer: {
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
   * Lookup218: argon_primitives::balance_change::AccountOrigin
   **/
  ArgonPrimitivesBalanceChangeAccountOrigin: {
    notebookNumber: 'Compact<u32>',
    accountUid: 'Compact<u32>'
  },
  /**
   * Lookup225: pallet_chain_transfer::pallet::Call<T>
   **/
  PalletChainTransferCall: {
    _enum: {
      send_to_localchain: {
        amount: 'Compact<u128>',
        notaryId: 'u32',
      },
      send_to_evm_chain: {
        params: 'PalletChainTransferTransferToEvm',
      },
      register_hyperbridge_assets: {
        chains: 'Vec<(IsmpHostStateMachine,Bytes)>',
      },
      update_hyperbridge_assets: {
        addChains: 'Vec<(IsmpHostStateMachine,Bytes)>',
        removeChains: 'Vec<IsmpHostStateMachine>',
      },
      replace_hyperbridge_admins: {
        newAdmins: 'Vec<(IsmpHostStateMachine,H160)>',
      },
      set_bridge_enabled: {
        enabled: 'bool'
      }
    }
  },
  /**
   * Lookup226: pallet_chain_transfer::TransferToEvm<Balance>
   **/
  PalletChainTransferTransferToEvm: {
    asset: 'PalletChainTransferIsmpModuleAsset',
    evmChain: 'PalletChainTransferIsmpModuleEvmChain',
    recipient: 'H160',
    amount: 'u128',
    timeout: 'u64',
    relayerFee: 'u128'
  },
  /**
   * Lookup235: pallet_block_seal_spec::pallet::Call<T>
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
   * Lookup236: pallet_domains::pallet::Call<T>
   **/
  PalletDomainsCall: {
    _enum: {
      set_zone_record: {
        domainHash: 'H256',
        zoneRecord: 'ArgonPrimitivesDomainZoneRecord'
      }
    }
  },
  /**
   * Lookup237: pallet_price_index::pallet::Call<T>
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
   * Lookup238: pallet_price_index::PriceIndex
   **/
  PalletPriceIndexPriceIndex: {
    btcUsdPrice: 'Compact<u128>',
    argonUsdPrice: 'Compact<u128>',
    argonUsdTargetPrice: 'u128',
    tick: 'Compact<u64>'
  },
  /**
   * Lookup239: pallet_grandpa::pallet::Call<T>
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
        bestFinalizedBlockNumber: 'u32'
      }
    }
  },
  /**
   * Lookup240: sp_consensus_grandpa::EquivocationProof<primitive_types::H256, N>
   **/
  SpConsensusGrandpaEquivocationProof: {
    setId: 'u64',
    equivocation: 'SpConsensusGrandpaEquivocation'
  },
  /**
   * Lookup241: sp_consensus_grandpa::Equivocation<primitive_types::H256, N>
   **/
  SpConsensusGrandpaEquivocation: {
    _enum: {
      Prevote: 'FinalityGrandpaEquivocationPrevote',
      Precommit: 'FinalityGrandpaEquivocationPrecommit'
    }
  },
  /**
   * Lookup242: finality_grandpa::Equivocation<sp_consensus_grandpa::app::Public, finality_grandpa::Prevote<primitive_types::H256, N>, sp_consensus_grandpa::app::Signature>
   **/
  FinalityGrandpaEquivocationPrevote: {
    roundNumber: 'u64',
    identity: 'SpConsensusGrandpaAppPublic',
    first: '(FinalityGrandpaPrevote,SpConsensusGrandpaAppSignature)',
    second: '(FinalityGrandpaPrevote,SpConsensusGrandpaAppSignature)'
  },
  /**
   * Lookup243: finality_grandpa::Prevote<primitive_types::H256, N>
   **/
  FinalityGrandpaPrevote: {
    targetHash: 'H256',
    targetNumber: 'u32'
  },
  /**
   * Lookup244: sp_consensus_grandpa::app::Signature
   **/
  SpConsensusGrandpaAppSignature: '[u8;64]',
  /**
   * Lookup246: finality_grandpa::Equivocation<sp_consensus_grandpa::app::Public, finality_grandpa::Precommit<primitive_types::H256, N>, sp_consensus_grandpa::app::Signature>
   **/
  FinalityGrandpaEquivocationPrecommit: {
    roundNumber: 'u64',
    identity: 'SpConsensusGrandpaAppPublic',
    first: '(FinalityGrandpaPrecommit,SpConsensusGrandpaAppSignature)',
    second: '(FinalityGrandpaPrecommit,SpConsensusGrandpaAppSignature)'
  },
  /**
   * Lookup247: finality_grandpa::Precommit<primitive_types::H256, N>
   **/
  FinalityGrandpaPrecommit: {
    targetHash: 'H256',
    targetNumber: 'u32'
  },
  /**
   * Lookup249: sp_core::Void
   **/
  SpCoreVoid: 'Null',
  /**
   * Lookup250: pallet_block_seal::pallet::Call<T>
   **/
  PalletBlockSealCall: {
    _enum: {
      apply: {
        seal: 'ArgonPrimitivesInherentsBlockSealInherent'
      }
    }
  },
  /**
   * Lookup251: argon_primitives::inherents::BlockSealInherent
   **/
  ArgonPrimitivesInherentsBlockSealInherent: {
    _enum: {
      Vote: {
        sealStrength: 'U256',
        notaryId: 'Compact<u32>',
        sourceNotebookNumber: 'Compact<u32>',
        sourceNotebookProof: 'ArgonPrimitivesBalanceChangeMerkleProof',
        blockVote: 'ArgonPrimitivesBlockVoteBlockVoteT',
      },
      Compute: 'Null'
    }
  },
  /**
   * Lookup252: argon_primitives::balance_change::MerkleProof
   **/
  ArgonPrimitivesBalanceChangeMerkleProof: {
    proof: 'Vec<H256>',
    numberOfLeaves: 'Compact<u32>',
    leafIndex: 'Compact<u32>'
  },
  /**
   * Lookup254: argon_primitives::block_vote::BlockVoteT<primitive_types::H256>
   **/
  ArgonPrimitivesBlockVoteBlockVoteT: {
    accountId: 'AccountId32',
    blockHash: 'H256',
    index: 'Compact<u32>',
    power: 'Compact<u128>',
    signature: 'SpRuntimeMultiSignature',
    blockRewardsAccountId: 'AccountId32',
    tick: 'Compact<u64>'
  },
  /**
   * Lookup255: sp_runtime::MultiSignature
   **/
  SpRuntimeMultiSignature: {
    _enum: {
      Ed25519: '[u8;64]',
      Sr25519: '[u8;64]',
      Ecdsa: '[u8;65]'
    }
  },
  /**
   * Lookup257: pallet_block_rewards::pallet::Call<T>
   **/
  PalletBlockRewardsCall: 'Null',
  /**
   * Lookup258: pallet_mint::pallet::Call<T>
   **/
  PalletMintCall: 'Null',
  /**
   * Lookup259: pallet_balances::pallet::Call<T, I>
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
   * Lookup260: pallet_balances::types::AdjustmentDirection
   **/
  PalletBalancesAdjustmentDirection: {
    _enum: ['Increase', 'Decrease']
  },
  /**
   * Lookup262: pallet_tx_pause::pallet::Call<T>
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
   * Lookup263: pallet_utility::pallet::Call<T>
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
        weight: 'SpWeightsWeightV2Weight'
      }
    }
  },
  /**
   * Lookup265: argon_runtime::OriginCaller
   **/
  ArgonRuntimeOriginCaller: {
    _enum: {
      system: 'FrameSupportDispatchRawOrigin',
      Void: 'SpCoreVoid'
    }
  },
  /**
   * Lookup266: frame_support::dispatch::RawOrigin<sp_core::crypto::AccountId32>
   **/
  FrameSupportDispatchRawOrigin: {
    _enum: {
      Root: 'Null',
      Signed: 'AccountId32',
      None: 'Null'
    }
  },
  /**
   * Lookup267: pallet_sudo::pallet::Call<T>
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
   * Lookup268: pallet_ismp::pallet::Call<T>
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
        message: 'PalletIsmpUtilsFundMessageParams'
      }
    }
  },
  /**
   * Lookup270: ismp::messaging::Message
   **/
  IsmpMessagingMessage: {
    _enum: {
      Consensus: 'IsmpMessagingConsensusMessage',
      FraudProof: 'IsmpMessagingFraudProofMessage',
      Request: 'IsmpMessagingRequestMessage',
      Response: 'IsmpMessagingResponseMessage',
      Timeout: 'IsmpMessagingTimeoutMessage'
    }
  },
  /**
   * Lookup271: ismp::messaging::ConsensusMessage
   **/
  IsmpMessagingConsensusMessage: {
    consensusProof: 'Bytes',
    consensusStateId: '[u8;4]',
    signer: 'Bytes'
  },
  /**
   * Lookup272: ismp::messaging::FraudProofMessage
   **/
  IsmpMessagingFraudProofMessage: {
    proof1: 'Bytes',
    proof2: 'Bytes',
    consensusStateId: '[u8;4]'
  },
  /**
   * Lookup273: ismp::messaging::RequestMessage
   **/
  IsmpMessagingRequestMessage: {
    requests: 'Vec<IsmpRouterPostRequest>',
    proof: 'IsmpMessagingProof',
    signer: 'Bytes'
  },
  /**
   * Lookup275: ismp::router::PostRequest
   **/
  IsmpRouterPostRequest: {
    source: 'IsmpHostStateMachine',
    dest: 'IsmpHostStateMachine',
    nonce: 'u64',
    from: 'Bytes',
    to: 'Bytes',
    timeoutTimestamp: 'u64',
    body: 'Bytes'
  },
  /**
   * Lookup276: ismp::messaging::Proof
   **/
  IsmpMessagingProof: {
    height: 'IsmpConsensusStateMachineHeight',
    proof: 'Bytes'
  },
  /**
   * Lookup277: ismp::messaging::ResponseMessage
   **/
  IsmpMessagingResponseMessage: {
    datagram: 'IsmpRouterRequestResponse',
    proof: 'IsmpMessagingProof',
    signer: 'Bytes'
  },
  /**
   * Lookup278: ismp::router::RequestResponse
   **/
  IsmpRouterRequestResponse: {
    _enum: {
      Request: 'Vec<IsmpRouterRequest>',
      Response: 'Vec<IsmpRouterResponse>'
    }
  },
  /**
   * Lookup280: ismp::router::Request
   **/
  IsmpRouterRequest: {
    _enum: {
      Post: 'IsmpRouterPostRequest',
      Get: 'IsmpRouterGetRequest'
    }
  },
  /**
   * Lookup281: ismp::router::GetRequest
   **/
  IsmpRouterGetRequest: {
    _alias: {
      keys_: 'keys'
    },
    source: 'IsmpHostStateMachine',
    dest: 'IsmpHostStateMachine',
    nonce: 'u64',
    from: 'Bytes',
    keys_: 'Vec<Bytes>',
    height: 'u64',
    context: 'Bytes',
    timeoutTimestamp: 'u64'
  },
  /**
   * Lookup283: ismp::router::Response
   **/
  IsmpRouterResponse: {
    _enum: {
      Post: 'IsmpRouterPostResponse',
      Get: 'IsmpRouterGetResponse'
    }
  },
  /**
   * Lookup284: ismp::router::PostResponse
   **/
  IsmpRouterPostResponse: {
    post: 'IsmpRouterPostRequest',
    response: 'Bytes',
    timeoutTimestamp: 'u64'
  },
  /**
   * Lookup285: ismp::router::GetResponse
   **/
  IsmpRouterGetResponse: {
    get: 'IsmpRouterGetRequest',
    values: 'Vec<IsmpRouterStorageValue>'
  },
  /**
   * Lookup287: ismp::router::StorageValue
   **/
  IsmpRouterStorageValue: {
    key: 'Bytes',
    value: 'Option<Bytes>'
  },
  /**
   * Lookup289: ismp::messaging::TimeoutMessage
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
        requests: 'Vec<IsmpRouterRequest>'
      }
    }
  },
  /**
   * Lookup291: ismp::messaging::CreateConsensusState
   **/
  IsmpMessagingCreateConsensusState: {
    consensusState: 'Bytes',
    consensusClientId: '[u8;4]',
    consensusStateId: '[u8;4]',
    unbondingPeriod: 'u64',
    challengePeriods: 'BTreeMap<IsmpHostStateMachine, u64>',
    stateMachineCommitments: 'Vec<(IsmpConsensusStateMachineId,IsmpMessagingStateCommitmentHeight)>'
  },
  /**
   * Lookup297: ismp::messaging::StateCommitmentHeight
   **/
  IsmpMessagingStateCommitmentHeight: {
    commitment: 'IsmpConsensusStateCommitment',
    height: 'u64'
  },
  /**
   * Lookup298: ismp::consensus::StateCommitment
   **/
  IsmpConsensusStateCommitment: {
    timestamp: 'u64',
    overlayRoot: 'Option<H256>',
    stateRoot: 'H256'
  },
  /**
   * Lookup299: pallet_ismp::utils::UpdateConsensusState
   **/
  PalletIsmpUtilsUpdateConsensusState: {
    consensusStateId: '[u8;4]',
    unbondingPeriod: 'Option<u64>',
    challengePeriods: 'BTreeMap<IsmpHostStateMachine, u64>'
  },
  /**
   * Lookup300: pallet_ismp::utils::FundMessageParams<Balance>
   **/
  PalletIsmpUtilsFundMessageParams: {
    commitment: 'PalletIsmpUtilsMessageCommitment',
    amount: 'u128'
  },
  /**
   * Lookup301: pallet_ismp::utils::MessageCommitment
   **/
  PalletIsmpUtilsMessageCommitment: {
    _enum: {
      Request: 'H256',
      Response: 'H256'
    }
  },
  /**
   * Lookup302: ismp_grandpa::pallet::Call<T>
   **/
  IsmpGrandpaCall: {
    _enum: {
      add_state_machines: {
        newStateMachines: 'Vec<IsmpGrandpaAddStateMachine>',
      },
      remove_state_machines: {
        stateMachines: 'Vec<IsmpHostStateMachine>'
      }
    }
  },
  /**
   * Lookup304: ismp_grandpa::AddStateMachine
   **/
  IsmpGrandpaAddStateMachine: {
    stateMachine: 'IsmpHostStateMachine',
    slotDuration: 'u64'
  },
  /**
   * Lookup306: pallet_multisig::pallet::Error<T>
   **/
  PalletMultisigError: {
    _enum: ['MinimumThreshold', 'AlreadyApproved', 'NoApprovalsNeeded', 'TooFewSignatories', 'TooManySignatories', 'SignatoriesOutOfOrder', 'SenderInSignatories', 'NotFound', 'NotOwner', 'NoTimepoint', 'WrongTimepoint', 'UnexpectedTimepoint', 'MaxWeightTooLow', 'AlreadyStored']
  },
  /**
   * Lookup309: pallet_proxy::ProxyDefinition<sp_core::crypto::AccountId32, argon_runtime::configs::ProxyType, BlockNumber>
   **/
  PalletProxyProxyDefinition: {
    delegate: 'AccountId32',
    proxyType: 'ArgonRuntimeConfigsProxyType',
    delay: 'u32'
  },
  /**
   * Lookup313: pallet_proxy::Announcement<sp_core::crypto::AccountId32, primitive_types::H256, BlockNumber>
   **/
  PalletProxyAnnouncement: {
    real: 'AccountId32',
    callHash: 'H256',
    height: 'u32'
  },
  /**
   * Lookup315: pallet_proxy::pallet::Error<T>
   **/
  PalletProxyError: {
    _enum: ['TooMany', 'NotFound', 'NotProxy', 'Unproxyable', 'Duplicate', 'NoPermission', 'Unannounced', 'NoSelfProxy']
  },
  /**
   * Lookup316: argon_primitives::tick::Ticker
   **/
  ArgonPrimitivesTickTicker: {
    tickDurationMillis: 'Compact<u64>',
    channelHoldExpirationTicks: 'Compact<u64>'
  },
  /**
   * Lookup318: pallet_ticks::pallet::Error<T>
   **/
  PalletTicksError: 'Null',
  /**
   * Lookup325: argon_primitives::block_seal::MiningSlotConfig<BlockNumber>
   **/
  ArgonPrimitivesBlockSealMiningSlotConfig: {
    blocksBeforeBidEndForVrfClose: 'Compact<u32>',
    blocksBetweenSlots: 'Compact<u32>',
    slotBiddingStartBlock: 'Compact<u32>'
  },
  /**
   * Lookup326: pallet_mining_slot::pallet::Error<T>
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
      GenericBondError: 'ArgonPrimitivesBondBondError'
    }
  },
  /**
   * Lookup327: argon_primitives::bond::BondError
   **/
  ArgonPrimitivesBondBondError: {
    _enum: ['BondNotFound', 'NoMoreBondIds', 'MinimumBondAmountNotMet', 'VaultClosed', 'ExpirationAtBlockOverflow', 'AccountWouldBeBelowMinimum', 'InsufficientFunds', 'InsufficientVaultFunds', 'InsufficientBitcoinsForMining', 'ExpirationTooSoon', 'NoPermissions', 'HoldUnexpectedlyModified', 'UnrecoverableHold', 'VaultNotFound', 'NoVaultBitcoinPubkeysAvailable', 'UnableToGenerateVaultBitcoinPubkey', 'UnableToDecodeVaultBitcoinPubkey', 'FeeExceedsBondAmount', 'InvalidBitcoinScript', 'InternalError']
  },
  /**
   * Lookup328: argon_primitives::bitcoin::UtxoValue
   **/
  ArgonPrimitivesBitcoinUtxoValue: {
    utxoId: 'u64',
    scriptPubkey: 'ArgonPrimitivesBitcoinBitcoinCosignScriptPubkey',
    satoshis: 'Compact<u64>',
    submittedAtHeight: 'Compact<u64>',
    watchForSpentUntilHeight: 'Compact<u64>'
  },
  /**
   * Lookup329: argon_primitives::bitcoin::BitcoinCosignScriptPubkey
   **/
  ArgonPrimitivesBitcoinBitcoinCosignScriptPubkey: {
    _enum: {
      P2WSH: {
        wscriptHash: 'H256'
      }
    }
  },
  /**
   * Lookup334: argon_primitives::bitcoin::BitcoinNetwork
   **/
  ArgonPrimitivesBitcoinBitcoinNetwork: {
    _enum: ['Bitcoin', 'Testnet', 'Signet', 'Regtest']
  },
  /**
   * Lookup337: pallet_bitcoin_utxos::pallet::Error<T>
   **/
  PalletBitcoinUtxosError: {
    _enum: ['NoPermissions', 'NoBitcoinConfirmedBlock', 'InsufficientBitcoinAmount', 'NoBitcoinPricesAvailable', 'ScriptPubkeyConflict', 'UtxoNotLocked', 'RedemptionsUnavailable', 'InvalidBitcoinSyncHeight', 'BitcoinHeightNotConfirmed', 'MaxUtxosExceeded', 'InvalidBitcoinScript']
  },
  /**
   * Lookup338: argon_primitives::bond::Vault<sp_core::crypto::AccountId32, Balance, BlockNumber>
   **/
  ArgonPrimitivesBondVault: {
    operatorAccountId: 'AccountId32',
    bitcoinArgons: 'ArgonPrimitivesBondVaultArgons',
    securitizationPercent: 'Compact<u128>',
    securitizedArgons: 'Compact<u128>',
    miningArgons: 'ArgonPrimitivesBondVaultArgons',
    miningRewardSharingPercentTake: 'Compact<u128>',
    isClosed: 'bool',
    pendingTerms: 'Option<(u32,ArgonPrimitivesBondVaultTerms)>'
  },
  /**
   * Lookup339: argon_primitives::bond::VaultArgons<Balance>
   **/
  ArgonPrimitivesBondVaultArgons: {
    annualPercentRate: 'Compact<u128>',
    allocated: 'Compact<u128>',
    bonded: 'Compact<u128>',
    baseFee: 'Compact<u128>'
  },
  /**
   * Lookup343: argon_primitives::bitcoin::BitcoinXPub
   **/
  ArgonPrimitivesBitcoinBitcoinXPub: {
    publicKey: 'ArgonPrimitivesBitcoinCompressedBitcoinPubkey',
    depth: 'Compact<u8>',
    parentFingerprint: '[u8;4]',
    childNumber: 'Compact<u32>',
    chainCode: '[u8;32]',
    network: 'ArgonPrimitivesBitcoinNetworkKind'
  },
  /**
   * Lookup345: argon_primitives::bitcoin::NetworkKind
   **/
  ArgonPrimitivesBitcoinNetworkKind: {
    _enum: ['Main', 'Test']
  },
  /**
   * Lookup347: pallet_vaults::pallet::Error<T>
   **/
  PalletVaultsError: {
    _enum: ['BondNotFound', 'NoMoreVaultIds', 'NoMoreBondIds', 'MinimumBondAmountNotMet', 'ExpirationAtBlockOverflow', 'InsufficientFunds', 'InsufficientVaultFunds', 'InsufficientBitcoinsForMining', 'AccountBelowMinimumBalance', 'VaultClosed', 'InvalidVaultAmount', 'VaultReductionBelowAllocatedFunds', 'InvalidSecuritization', 'ReusedVaultBitcoinXpub', 'MaxSecuritizationPercentExceeded', 'InvalidBondType', 'BitcoinUtxoNotFound', 'InsufficientSatoshisBonded', 'NoBitcoinPricesAvailable', 'InvalidBitcoinScript', 'InvalidXpubkey', 'WrongXpubNetwork', 'UnsafeXpubkey', 'UnableToDeriveVaultXpubChild', 'BitcoinConversionFailed', 'ExpirationTooSoon', 'NoPermissions', 'HoldUnexpectedlyModified', 'UnrecoverableHold', 'VaultNotFound', 'FeeExceedsBondAmount', 'NoVaultBitcoinPubkeysAvailable', 'TermsModificationOverflow', 'TermsChangeAlreadyScheduled', 'InternalError', 'UnableToGenerateVaultBitcoinPubkey', 'UnableToDecodeVaultBitcoinPubkey']
  },
  /**
   * Lookup348: argon_primitives::bond::Bond<sp_core::crypto::AccountId32, Balance, BlockNumber>
   **/
  ArgonPrimitivesBond: {
    bondType: 'ArgonPrimitivesBondBondType',
    vaultId: 'Compact<u32>',
    utxoId: 'Option<u64>',
    bondedAccountId: 'AccountId32',
    totalFee: 'Compact<u128>',
    prepaidFee: 'Compact<u128>',
    amount: 'Compact<u128>',
    startBlock: 'Compact<u32>',
    expiration: 'ArgonPrimitivesBondBondExpiration'
  },
  /**
   * Lookup351: pallet_bond::pallet::UtxoState
   **/
  PalletBondUtxoState: {
    bondId: 'Compact<u64>',
    satoshis: 'Compact<u64>',
    vaultPubkey: 'ArgonPrimitivesBitcoinCompressedBitcoinPubkey',
    vaultClaimPubkey: 'ArgonPrimitivesBitcoinCompressedBitcoinPubkey',
    vaultXpubSources: '([u8;4],u32,u32)',
    ownerPubkey: 'ArgonPrimitivesBitcoinCompressedBitcoinPubkey',
    vaultClaimHeight: 'Compact<u64>',
    openClaimHeight: 'Compact<u64>',
    createdAtHeight: 'Compact<u64>',
    utxoScriptPubkey: 'ArgonPrimitivesBitcoinBitcoinCosignScriptPubkey',
    isVerified: 'bool'
  },
  /**
   * Lookup355: pallet_bond::pallet::UtxoCosignRequest<Balance>
   **/
  PalletBondUtxoCosignRequest: {
    bondId: 'Compact<u64>',
    vaultId: 'Compact<u32>',
    bitcoinNetworkFee: 'Compact<u64>',
    cosignDueBlock: 'Compact<u64>',
    toScriptPubkey: 'Bytes',
    redemptionPrice: 'Compact<u128>'
  },
  /**
   * Lookup359: pallet_bond::pallet::Error<T>
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
      BitcoinUnableToBeDecodedForUnlock: 'Null',
      BitcoinSignatureUnableToBeDecoded: 'Null',
      BitcoinPubkeyUnableToBeDecoded: 'Null',
      BitcoinInvalidCosignature: 'Null',
      InsufficientSatoshisBonded: 'Null',
      NoBitcoinPricesAvailable: 'Null',
      InvalidBitcoinScript: 'Null',
      ExpirationTooSoon: 'Null',
      NoPermissions: 'Null',
      HoldUnexpectedlyModified: 'Null',
      UnrecoverableHold: 'Null',
      VaultNotFound: 'Null',
      FeeExceedsBondAmount: 'Null',
      GenericBondError: 'ArgonPrimitivesBondBondError'
    }
  },
  /**
   * Lookup371: pallet_notaries::pallet::Error<T>
   **/
  PalletNotariesError: {
    _enum: ['ProposalNotFound', 'MaxNotariesExceeded', 'MaxProposalsPerBlockExceeded', 'NotAnActiveNotary', 'InvalidNotaryOperator', 'NoMoreNotaryIds', 'EffectiveTickTooSoon', 'TooManyKeys', 'InvalidNotary']
  },
  /**
   * Lookup375: argon_primitives::notary::NotaryNotebookKeyDetails
   **/
  ArgonPrimitivesNotaryNotaryNotebookKeyDetails: {
    notebookNumber: 'Compact<u32>',
    tick: 'Compact<u64>',
    blockVotesRoot: 'H256',
    secretHash: 'H256',
    parentSecret: 'Option<H256>'
  },
  /**
   * Lookup378: pallet_notebook::pallet::Error<T>
   **/
  PalletNotebookError: {
    _enum: ['DuplicateNotebookNumber', 'MissingNotebookNumber', 'NotebookTickAlreadyUsed', 'InvalidNotebookSignature', 'InvalidSecretProvided', 'CouldNotDecodeNotebook', 'DuplicateNotebookDigest', 'MissingNotebookDigest', 'InvalidNotebookDigest', 'MultipleNotebookInherentsProvided', 'InternalError', 'NotebookSubmittedForLockedNotary', 'InvalidReprocessNotebook', 'InvalidNotaryOperator']
  },
  /**
   * Lookup379: pallet_chain_transfer::QueuedTransferOut<sp_core::crypto::AccountId32, Balance>
   **/
  PalletChainTransferQueuedTransferOut: {
    accountId: 'AccountId32',
    amount: 'u128',
    expirationTick: 'u64',
    notaryId: 'u32'
  },
  /**
   * Lookup388: frame_support::PalletId
   **/
  FrameSupportPalletId: '[u8;8]',
  /**
   * Lookup389: pallet_chain_transfer::pallet::Error<T>
   **/
  PalletChainTransferError: {
    _enum: ['MaxBlockTransfersExceeded', 'InsufficientFunds', 'InsufficientNotarizedFunds', 'InvalidOrDuplicatedLocalchainTransfer', 'NotebookIncludesExpiredLocalchainTransfer', 'InvalidNotaryUsedForTransfer', 'FailedToTransferToEvm', 'NotATokenAdmin', 'Erc6160RegistrationFailed', 'CoprocessorNotConfigured', 'InvalidEvmChain', 'EvmChainNotSupported', 'EvmChainNotConfigured', 'EvmBridgePaused', 'MaxChainsExceeded']
  },
  /**
   * Lookup394: argon_primitives::notary::NotaryNotebookVoteDigestDetails
   **/
  ArgonPrimitivesNotaryNotaryNotebookVoteDigestDetails: {
    notaryId: 'Compact<u32>',
    notebookNumber: 'Compact<u32>',
    tick: 'Compact<u64>',
    blockVotesCount: 'Compact<u32>',
    blockVotingPower: 'Compact<u128>'
  },
  /**
   * Lookup399: pallet_block_seal_spec::pallet::Error<T>
   **/
  PalletBlockSealSpecError: {
    _enum: ['MaxNotebooksAtTickExceeded']
  },
  /**
   * Lookup401: pallet_domains::pallet::Error<T>
   **/
  PalletDomainsError: {
    _enum: ['DomainNotRegistered', 'NotDomainOwner', 'FailedToAddToAddressHistory', 'FailedToAddExpiringDomain', 'AccountDecodingError']
  },
  /**
   * Lookup402: pallet_price_index::pallet::Error<T>
   **/
  PalletPriceIndexError: {
    _enum: ['NotAuthorizedOperator', 'MissingValue', 'PricesTooOld', 'MaxPriceChangePerTickExceeded']
  },
  /**
   * Lookup403: pallet_grandpa::StoredState<N>
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
   * Lookup404: pallet_grandpa::StoredPendingChange<N, Limit>
   **/
  PalletGrandpaStoredPendingChange: {
    scheduledAt: 'u32',
    delay: 'u32',
    nextAuthorities: 'Vec<(SpConsensusGrandpaAppPublic,u64)>',
    forced: 'Option<u32>'
  },
  /**
   * Lookup407: pallet_grandpa::pallet::Error<T>
   **/
  PalletGrandpaError: {
    _enum: ['PauseFailed', 'ResumeFailed', 'ChangePending', 'TooSoon', 'InvalidKeyOwnershipProof', 'InvalidEquivocationProof', 'DuplicateOffenceReport']
  },
  /**
   * Lookup408: argon_primitives::providers::BlockSealerInfo<sp_core::crypto::AccountId32>
   **/
  ArgonPrimitivesProvidersBlockSealerInfo: {
    blockAuthorAccountId: 'AccountId32',
    blockVoteRewardsAccount: 'Option<AccountId32>'
  },
  /**
   * Lookup412: pallet_block_seal::pallet::Error<T>
   **/
  PalletBlockSealError: {
    _enum: ['InvalidVoteSealStrength', 'InvalidSubmitter', 'UnableToDecodeVoteAccount', 'UnregisteredBlockAuthor', 'InvalidBlockVoteProof', 'NoGrandparentVoteMinimum', 'DuplicateBlockSealProvided', 'InsufficientVotingPower', 'ParentVotingKeyNotFound', 'InvalidVoteGrandparentHash', 'IneligibleNotebookUsed', 'NoEligibleVotingRoot', 'CouldNotDecodeVote', 'MaxNotebooksAtTickExceeded', 'NoClosestMinerFoundForVote', 'BlockVoteInvalidSignature', 'InvalidForkPowerParent']
  },
  /**
   * Lookup414: pallet_block_rewards::pallet::Error<T>
   **/
  PalletBlockRewardsError: 'Null',
  /**
   * Lookup418: pallet_mint::pallet::Error<T>
   **/
  PalletMintError: {
    _enum: ['TooManyPendingMints']
  },
  /**
   * Lookup420: pallet_balances::types::BalanceLock<Balance>
   **/
  PalletBalancesBalanceLock: {
    id: '[u8;8]',
    amount: 'u128',
    reasons: 'PalletBalancesReasons'
  },
  /**
   * Lookup421: pallet_balances::types::Reasons
   **/
  PalletBalancesReasons: {
    _enum: ['Fee', 'Misc', 'All']
  },
  /**
   * Lookup424: pallet_balances::types::ReserveData<ReserveIdentifier, Balance>
   **/
  PalletBalancesReserveData: {
    id: '[u8;8]',
    amount: 'u128'
  },
  /**
   * Lookup427: frame_support::traits::tokens::misc::IdAmount<argon_runtime::RuntimeHoldReason, Balance>
   **/
  FrameSupportTokensMiscIdAmountRuntimeHoldReason: {
    id: 'ArgonRuntimeRuntimeHoldReason',
    amount: 'u128'
  },
  /**
   * Lookup428: argon_runtime::RuntimeHoldReason
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
      Bonds: 'PalletBondHoldReason',
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
   * Lookup429: pallet_mining_slot::pallet::HoldReason
   **/
  PalletMiningSlotHoldReason: {
    _enum: ['RegisterAsMiner']
  },
  /**
   * Lookup430: pallet_vaults::pallet::HoldReason
   **/
  PalletVaultsHoldReason: {
    _enum: ['EnterVault', 'BondFee']
  },
  /**
   * Lookup431: pallet_bond::pallet::HoldReason
   **/
  PalletBondHoldReason: {
    _enum: ['UnlockingBitcoin']
  },
  /**
   * Lookup432: pallet_block_rewards::pallet::HoldReason
   **/
  PalletBlockRewardsHoldReason: {
    _enum: ['MaturationPeriod']
  },
  /**
   * Lookup435: frame_support::traits::tokens::misc::IdAmount<argon_runtime::RuntimeFreezeReason, Balance>
   **/
  FrameSupportTokensMiscIdAmountRuntimeFreezeReason: {
    id: 'ArgonRuntimeRuntimeFreezeReason',
    amount: 'u128'
  },
  /**
   * Lookup436: argon_runtime::RuntimeFreezeReason
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
      BlockRewards: 'PalletBlockRewardsFreezeReason'
    }
  },
  /**
   * Lookup437: pallet_block_rewards::pallet::FreezeReason
   **/
  PalletBlockRewardsFreezeReason: {
    _enum: ['MaturationPeriod']
  },
  /**
   * Lookup439: pallet_balances::pallet::Error<T, I>
   **/
  PalletBalancesError: {
    _enum: ['VestingBalance', 'LiquidityRestrictions', 'InsufficientBalance', 'ExistentialDeposit', 'Expendability', 'ExistingVestingSchedule', 'DeadAccount', 'TooManyReserves', 'TooManyHolds', 'TooManyFreezes', 'IssuanceDeactivated', 'DeltaZero']
  },
  /**
   * Lookup441: pallet_tx_pause::pallet::Error<T>
   **/
  PalletTxPauseError: {
    _enum: ['IsPaused', 'IsUnpaused', 'Unpausable', 'NotFound']
  },
  /**
   * Lookup442: pallet_transaction_payment::Releases
   **/
  PalletTransactionPaymentReleases: {
    _enum: ['V1Ancient', 'V2']
  },
  /**
   * Lookup443: pallet_utility::pallet::Error<T>
   **/
  PalletUtilityError: {
    _enum: ['TooManyCalls']
  },
  /**
   * Lookup444: pallet_sudo::pallet::Error<T>
   **/
  PalletSudoError: {
    _enum: ['RequireSudo']
  },
  /**
   * Lookup445: pallet_ismp::pallet::Error<T>
   **/
  PalletIsmpError: {
    _enum: ['InvalidMessage', 'MessageNotFound', 'ConsensusClientCreationFailed', 'UnbondingPeriodUpdateFailed', 'ChallengePeriodUpdateFailed']
  },
  /**
   * Lookup446: pallet_hyperbridge::pallet::Error<T>
   **/
  PalletHyperbridgeError: 'Null',
  /**
   * Lookup449: frame_system::extensions::check_non_zero_sender::CheckNonZeroSender<T>
   **/
  FrameSystemExtensionsCheckNonZeroSender: 'Null',
  /**
   * Lookup450: frame_system::extensions::check_spec_version::CheckSpecVersion<T>
   **/
  FrameSystemExtensionsCheckSpecVersion: 'Null',
  /**
   * Lookup451: frame_system::extensions::check_tx_version::CheckTxVersion<T>
   **/
  FrameSystemExtensionsCheckTxVersion: 'Null',
  /**
   * Lookup452: frame_system::extensions::check_genesis::CheckGenesis<T>
   **/
  FrameSystemExtensionsCheckGenesis: 'Null',
  /**
   * Lookup455: frame_system::extensions::check_nonce::CheckNonce<T>
   **/
  FrameSystemExtensionsCheckNonce: 'Compact<u32>',
  /**
   * Lookup456: frame_system::extensions::check_weight::CheckWeight<T>
   **/
  FrameSystemExtensionsCheckWeight: 'Null',
  /**
   * Lookup457: pallet_transaction_payment::ChargeTransactionPayment<T>
   **/
  PalletTransactionPaymentChargeTransactionPayment: 'Compact<u128>',
  /**
   * Lookup458: frame_metadata_hash_extension::CheckMetadataHash<T>
   **/
  FrameMetadataHashExtensionCheckMetadataHash: {
    mode: 'FrameMetadataHashExtensionMode'
  },
  /**
   * Lookup459: frame_metadata_hash_extension::Mode
   **/
  FrameMetadataHashExtensionMode: {
    _enum: ['Disabled', 'Enabled']
  },
  /**
   * Lookup461: argon_runtime::Runtime
   **/
  ArgonRuntimeRuntime: 'Null'
};
