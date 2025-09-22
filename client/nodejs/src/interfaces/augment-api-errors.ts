// Auto-generated via `yarn polkadot-types-from-chain`, do not edit
/* eslint-disable */

// import type lookup before we augment - in some environments
// this is required to allow for ambient/previous definitions
import '@polkadot/api-base/types/errors';

import type { ApiTypes, AugmentedError } from '@polkadot/api-base/types';

export type __AugmentedError<ApiType extends ApiTypes> = AugmentedError<ApiType>;

declare module '@polkadot/api-base/types/errors' {
  interface AugmentedErrors<ApiType extends ApiTypes> {
    balances: {
      /**
       * Beneficiary account must pre-exist.
       **/
      DeadAccount: AugmentedError<ApiType>;
      /**
       * The delta cannot be zero.
       **/
      DeltaZero: AugmentedError<ApiType>;
      /**
       * Value too low to create account due to existential deposit.
       **/
      ExistentialDeposit: AugmentedError<ApiType>;
      /**
       * A vesting schedule already exists for this account.
       **/
      ExistingVestingSchedule: AugmentedError<ApiType>;
      /**
       * Transfer/payment would kill account.
       **/
      Expendability: AugmentedError<ApiType>;
      /**
       * Balance too low to send value.
       **/
      InsufficientBalance: AugmentedError<ApiType>;
      /**
       * The issuance cannot be modified since it is already deactivated.
       **/
      IssuanceDeactivated: AugmentedError<ApiType>;
      /**
       * Account liquidity restrictions prevent withdrawal.
       **/
      LiquidityRestrictions: AugmentedError<ApiType>;
      /**
       * Number of freezes exceed `MaxFreezes`.
       **/
      TooManyFreezes: AugmentedError<ApiType>;
      /**
       * Number of holds exceed `VariantCountOf<T::RuntimeHoldReason>`.
       **/
      TooManyHolds: AugmentedError<ApiType>;
      /**
       * Number of named reserves exceed `MaxReserves`.
       **/
      TooManyReserves: AugmentedError<ApiType>;
      /**
       * Vesting balance too high to send value.
       **/
      VestingBalance: AugmentedError<ApiType>;
    };
    bitcoinLocks: {
      /**
       * The proposed transaction would take the account below the minimum (existential) balance
       **/
      AccountWouldGoBelowMinimumBalance: AugmentedError<ApiType>;
      /**
       * The fee for this bitcoin release is too high
       **/
      BitcoinFeeTooHigh: AugmentedError<ApiType>;
      /**
       * The cosign signature is not valid for the bitcoin release
       **/
      BitcoinInvalidCosignature: AugmentedError<ApiType>;
      /**
       * This bitcoin pubkey couldn't be decoded for release
       **/
      BitcoinPubkeyUnableToBeDecoded: AugmentedError<ApiType>;
      /**
       * The bitcoin has passed the deadline to release it
       **/
      BitcoinReleaseInitiationDeadlinePassed: AugmentedError<ApiType>;
      /**
       * This bitcoin signature couldn't be decoded for release
       **/
      BitcoinSignatureUnableToBeDecoded: AugmentedError<ApiType>;
      /**
       * This bitcoin cosign script couldn't be decoded for release
       **/
      BitcoinUnableToBeDecodedForRelease: AugmentedError<ApiType>;
      BitcoinUtxoNotFound: AugmentedError<ApiType>;
      /**
       * An overflow occurred recording a lock expiration
       **/
      ExpirationAtBlockOverflow: AugmentedError<ApiType>;
      GenericVaultError: AugmentedError<ApiType>;
      HoldUnexpectedlyModified: AugmentedError<ApiType>;
      InsufficientFunds: AugmentedError<ApiType>;
      InsufficientSatoshisLocked: AugmentedError<ApiType>;
      InsufficientVaultFunds: AugmentedError<ApiType>;
      /**
       * The bitcoin script to lock this bitcoin has errors
       **/
      InvalidBitcoinScript: AugmentedError<ApiType>;
      /**
       * Funding would result in an overflow of the balance type
       **/
      InvalidVaultAmount: AugmentedError<ApiType>;
      /**
       * A lock in process of release cannot be ratcheted
       **/
      LockInProcessOfRelease: AugmentedError<ApiType>;
      LockNotFound: AugmentedError<ApiType>;
      NoBitcoinPricesAvailable: AugmentedError<ApiType>;
      NoPermissions: AugmentedError<ApiType>;
      /**
       * Nothing to ratchet
       **/
      NoRatchetingAvailable: AugmentedError<ApiType>;
      /**
       * No Vault public keys are available
       **/
      NoVaultBitcoinPubkeysAvailable: AugmentedError<ApiType>;
      /**
       * An overflow or underflow occurred while calculating the redemption price
       **/
      OverflowError: AugmentedError<ApiType>;
      /**
       * This bitcoin redemption has not been locked in
       **/
      RedemptionNotLocked: AugmentedError<ApiType>;
      /**
       * Unable to generate a new vault public key
       **/
      UnableToGenerateVaultBitcoinPubkey: AugmentedError<ApiType>;
      UnrecoverableHold: AugmentedError<ApiType>;
      /**
       * The lock is not verified
       **/
      UnverifiedLock: AugmentedError<ApiType>;
      VaultClosed: AugmentedError<ApiType>;
      VaultNotFound: AugmentedError<ApiType>;
      /**
       * This vault is not yet active
       **/
      VaultNotYetActive: AugmentedError<ApiType>;
    };
    bitcoinUtxos: {
      /**
       * Bitcoin height not confirmed yet
       **/
      BitcoinHeightNotConfirmed: AugmentedError<ApiType>;
      /**
       * Duplicated UtxoId. Already in use
       **/
      DuplicateUtxoId: AugmentedError<ApiType>;
      /**
       * Insufficient bitcoin amount
       **/
      InsufficientBitcoinAmount: AugmentedError<ApiType>;
      /**
       * Locking script has errors
       **/
      InvalidBitcoinScript: AugmentedError<ApiType>;
      /**
       * Invalid bitcoin sync height attempted
       **/
      InvalidBitcoinSyncHeight: AugmentedError<ApiType>;
      /**
       * Too many UTXOs are being tracked
       **/
      MaxUtxosExceeded: AugmentedError<ApiType>;
      /**
       * No Oracle-provided bitcoin block has been provided to the network
       **/
      NoBitcoinConfirmedBlock: AugmentedError<ApiType>;
      /**
       * No prices are available to mint bitcoins
       **/
      NoBitcoinPricesAvailable: AugmentedError<ApiType>;
      /**
       * Only an Oracle Operator can perform this action
       **/
      NoPermissions: AugmentedError<ApiType>;
      /**
       * Redemptions not currently available
       **/
      RedemptionsUnavailable: AugmentedError<ApiType>;
      /**
       * ScriptPubKey is already being waited for
       **/
      ScriptPubkeyConflict: AugmentedError<ApiType>;
      /**
       * Locked Utxo Not Found
       **/
      UtxoNotLocked: AugmentedError<ApiType>;
    };
    blockRewards: {};
    blockSeal: {
      /**
       * A block seal authority could not be properly decoded
       **/
      BlockSealDecodeError: AugmentedError<ApiType>;
      /**
       * The vote signature was invalid
       **/
      BlockVoteInvalidSignature: AugmentedError<ApiType>;
      /**
       * Could not decode the scale bytes of the votes
       **/
      CouldNotDecodeVote: AugmentedError<ApiType>;
      /**
       * Too many block seals submitted
       **/
      DuplicateBlockSealProvided: AugmentedError<ApiType>;
      /**
       * The notebook for this vote was not eligible to vote
       **/
      IneligibleNotebookUsed: AugmentedError<ApiType>;
      /**
       * The block vote did not reach the minimum voting power at time of the grandparent block
       **/
      InsufficientVotingPower: AugmentedError<ApiType>;
      /**
       * The merkle proof of vote inclusion in the notebook is invalid
       **/
      InvalidBlockVoteProof: AugmentedError<ApiType>;
      /**
       * Compute blocks cant be added in the same tick as a vote
       **/
      InvalidComputeBlockTick: AugmentedError<ApiType>;
      /**
       * Invalid fork power parent
       **/
      InvalidForkPowerParent: AugmentedError<ApiType>;
      /**
       * The xor distance supplied is invalid
       **/
      InvalidMinerXorDistance: AugmentedError<ApiType>;
      /**
       * Vote not submitted by the right miner
       **/
      InvalidSubmitter: AugmentedError<ApiType>;
      /**
       * The block vote was not for a valid block
       **/
      InvalidVoteGrandparentHash: AugmentedError<ApiType>;
      /**
       * The strength of the given seal did not match calculations
       **/
      InvalidVoteSealStrength: AugmentedError<ApiType>;
      /**
       * Too many notebooks were submitted for the current tick. Should not be possible
       **/
      MaxNotebooksAtTickExceeded: AugmentedError<ApiType>;
      /**
       * No closest miner found for vote
       **/
      NoClosestMinerFoundForVote: AugmentedError<ApiType>;
      /**
       * The lookup to verify a vote's authenticity is not available for the given block
       **/
      NoEligibleVotingRoot: AugmentedError<ApiType>;
      /**
       * No vote minimum found at grandparent height
       **/
      NoGrandparentVoteMinimum: AugmentedError<ApiType>;
      /**
       * No registered voting key found for the parent block
       **/
      ParentVotingKeyNotFound: AugmentedError<ApiType>;
      /**
       * Could not decode the vote bytes
       **/
      UnableToDecodeVoteAccount: AugmentedError<ApiType>;
      /**
       * The block author is not a registered miner
       **/
      UnregisteredBlockAuthor: AugmentedError<ApiType>;
    };
    blockSealSpec: {
      /**
       * The maximum number of notebooks at the current tick has been exceeded
       **/
      MaxNotebooksAtTickExceeded: AugmentedError<ApiType>;
    };
    chainTransfer: {
      /**
       * Insufficient balance to create this transfer
       **/
      InsufficientFunds: AugmentedError<ApiType>;
      /**
       * Insufficient balance to fulfill a mainchain transfer
       **/
      InsufficientNotarizedFunds: AugmentedError<ApiType>;
      /**
       * The notary id is not registered
       **/
      InvalidNotaryUsedForTransfer: AugmentedError<ApiType>;
      /**
       * The transfer was already submitted in a previous block
       **/
      InvalidOrDuplicatedLocalchainTransfer: AugmentedError<ApiType>;
      MaxBlockTransfersExceeded: AugmentedError<ApiType>;
      /**
       * A transfer was submitted in a previous block but the expiration block has passed
       **/
      NotebookIncludesExpiredLocalchainTransfer: AugmentedError<ApiType>;
    };
    digests: {
      /**
       * Failed to decode digests
       **/
      CouldNotDecodeDigest: AugmentedError<ApiType>;
      /**
       * Duplicate AuthorDigest found
       **/
      DuplicateAuthorDigest: AugmentedError<ApiType>;
      /**
       * Duplicate BlockVoteDigest found
       **/
      DuplicateBlockVoteDigest: AugmentedError<ApiType>;
      /**
       * Duplicate ForkPowerDigest found
       **/
      DuplicateForkPowerDigest: AugmentedError<ApiType>;
      /**
       * Duplicate NotebookDigest found
       **/
      DuplicateNotebookDigest: AugmentedError<ApiType>;
      /**
       * Duplicate ParentVotingKeyDigest found
       **/
      DuplicateParentVotingKeyDigest: AugmentedError<ApiType>;
      /**
       * Duplicate TickDigest found
       **/
      DuplicateTickDigest: AugmentedError<ApiType>;
      /**
       * Missing AuthorDigest
       **/
      MissingAuthorDigest: AugmentedError<ApiType>;
      /**
       * Missing BlockVoteDigest
       **/
      MissingBlockVoteDigest: AugmentedError<ApiType>;
      /**
       * Missing NotebookDigest
       **/
      MissingNotebookDigest: AugmentedError<ApiType>;
      /**
       * Missing ParentVotingKeyDigest
       **/
      MissingParentVotingKeyDigest: AugmentedError<ApiType>;
      /**
       * Missing TickDigest
       **/
      MissingTickDigest: AugmentedError<ApiType>;
    };
    domains: {
      /**
       * Error decoding account from notary
       **/
      AccountDecodingError: AugmentedError<ApiType>;
      /**
       * The domain is not registered.
       **/
      DomainNotRegistered: AugmentedError<ApiType>;
      /**
       * Failed to add to the expiring domain list
       **/
      FailedToAddExpiringDomain: AugmentedError<ApiType>;
      /**
       * Failed to add to the address history.
       **/
      FailedToAddToAddressHistory: AugmentedError<ApiType>;
      /**
       * The sender is not the owner of the domain.
       **/
      NotDomainOwner: AugmentedError<ApiType>;
    };
    grandpa: {
      /**
       * Attempt to signal GRANDPA change with one already pending.
       **/
      ChangePending: AugmentedError<ApiType>;
      /**
       * A given equivocation report is valid but already previously reported.
       **/
      DuplicateOffenceReport: AugmentedError<ApiType>;
      /**
       * An equivocation proof provided as part of an equivocation report is invalid.
       **/
      InvalidEquivocationProof: AugmentedError<ApiType>;
      /**
       * A key ownership proof provided as part of an equivocation report is invalid.
       **/
      InvalidKeyOwnershipProof: AugmentedError<ApiType>;
      /**
       * Attempt to signal GRANDPA pause when the authority set isn't live
       * (either paused or already pending pause).
       **/
      PauseFailed: AugmentedError<ApiType>;
      /**
       * Attempt to signal GRANDPA resume when the authority set isn't paused
       * (either live or already pending resume).
       **/
      ResumeFailed: AugmentedError<ApiType>;
      /**
       * Cannot signal forced change so soon after last.
       **/
      TooSoon: AugmentedError<ApiType>;
    };
    hyperbridge: {};
    ismp: {
      /**
       * Couldn't update challenge period
       **/
      ChallengePeriodUpdateFailed: AugmentedError<ApiType>;
      /**
       * Encountered an error while creating the consensus client.
       **/
      ConsensusClientCreationFailed: AugmentedError<ApiType>;
      /**
       * Error charging fee
       **/
      ErrorChargingFee: AugmentedError<ApiType>;
      /**
       * Invalid ISMP message
       **/
      InvalidMessage: AugmentedError<ApiType>;
      /**
       * Requested message was not found
       **/
      MessageNotFound: AugmentedError<ApiType>;
      /**
       * Couldn't update unbonding period
       **/
      UnbondingPeriodUpdateFailed: AugmentedError<ApiType>;
    };
    miningSlot: {
      /**
       * The mining bid cannot be reduced
       **/
      BidCannotBeReduced: AugmentedError<ApiType>;
      /**
       * The given bid isn't high enough to be in the cohort
       **/
      BidTooLow: AugmentedError<ApiType>;
      /**
       * An account can only have one active registration
       **/
      CannotRegisterOverlappingSessions: AugmentedError<ApiType>;
      /**
       * The funding account does not have enough funds to cover the bid
       **/
      InsufficientFunds: AugmentedError<ApiType>;
      /**
       * This funding account does not hold the minimum argonots needed
       **/
      InsufficientOwnershipTokens: AugmentedError<ApiType>;
      /**
       * Bids must be in allowed increments
       **/
      InvalidBidAmount: AugmentedError<ApiType>;
      /**
       * Bidding for the next cohort has closed
       **/
      SlotNotTakingBids: AugmentedError<ApiType>;
      /**
       * The cohort registration overflowed
       **/
      TooManyBlockRegistrants: AugmentedError<ApiType>;
      /**
       * The argonots on hold cannot be released
       **/
      UnrecoverableHold: AugmentedError<ApiType>;
    };
    mint: {
      TooManyPendingMints: AugmentedError<ApiType>;
    };
    multisig: {
      /**
       * Call is already approved by this signatory.
       **/
      AlreadyApproved: AugmentedError<ApiType>;
      /**
       * The data to be stored is already stored.
       **/
      AlreadyStored: AugmentedError<ApiType>;
      /**
       * The maximum weight information provided was too low.
       **/
      MaxWeightTooLow: AugmentedError<ApiType>;
      /**
       * Threshold must be 2 or greater.
       **/
      MinimumThreshold: AugmentedError<ApiType>;
      /**
       * Call doesn't need any (more) approvals.
       **/
      NoApprovalsNeeded: AugmentedError<ApiType>;
      /**
       * Multisig operation not found in storage.
       **/
      NotFound: AugmentedError<ApiType>;
      /**
       * No timepoint was given, yet the multisig operation is already underway.
       **/
      NoTimepoint: AugmentedError<ApiType>;
      /**
       * Only the account that originally created the multisig is able to cancel it or update
       * its deposits.
       **/
      NotOwner: AugmentedError<ApiType>;
      /**
       * The sender was contained in the other signatories; it shouldn't be.
       **/
      SenderInSignatories: AugmentedError<ApiType>;
      /**
       * The signatories were provided out of order; they should be ordered.
       **/
      SignatoriesOutOfOrder: AugmentedError<ApiType>;
      /**
       * There are too few signatories in the list.
       **/
      TooFewSignatories: AugmentedError<ApiType>;
      /**
       * There are too many signatories in the list.
       **/
      TooManySignatories: AugmentedError<ApiType>;
      /**
       * A timepoint was given, yet no multisig operation is underway.
       **/
      UnexpectedTimepoint: AugmentedError<ApiType>;
      /**
       * A different timepoint was given to the multisig operation that is underway.
       **/
      WrongTimepoint: AugmentedError<ApiType>;
    };
    notaries: {
      /**
       * The proposed effective tick is too soon
       **/
      EffectiveTickTooSoon: AugmentedError<ApiType>;
      /**
       * The notary is invalid
       **/
      InvalidNotary: AugmentedError<ApiType>;
      /**
       * Invalid notary operator for this operation
       **/
      InvalidNotaryOperator: AugmentedError<ApiType>;
      /**
       * Maximum number of notaries exceeded
       **/
      MaxNotariesExceeded: AugmentedError<ApiType>;
      /**
       * Maximum number of proposals per block exceeded
       **/
      MaxProposalsPerBlockExceeded: AugmentedError<ApiType>;
      /**
       * An internal error has occurred. The notary ids are exhausted.
       **/
      NoMoreNotaryIds: AugmentedError<ApiType>;
      /**
       * This notary is not active, so this change cannot be made yet
       **/
      NotAnActiveNotary: AugmentedError<ApiType>;
      /**
       * The proposal to activate was not found
       **/
      ProposalNotFound: AugmentedError<ApiType>;
      /**
       * Too many internal keys
       **/
      TooManyKeys: AugmentedError<ApiType>;
    };
    notebook: {
      /**
       * Could not decode the scale bytes of the notebook
       **/
      CouldNotDecodeNotebook: AugmentedError<ApiType>;
      /**
       * The notebook digest was included more than once
       **/
      DuplicateNotebookDigest: AugmentedError<ApiType>;
      /**
       * This notebook has already been submitted
       **/
      DuplicateNotebookNumber: AugmentedError<ApiType>;
      /**
       * Unable to track the notebook change list
       **/
      InternalError: AugmentedError<ApiType>;
      /**
       * Invalid notary operator
       **/
      InvalidNotaryOperator: AugmentedError<ApiType>;
      /**
       * The notebook digest did not match the included notebooks
       **/
      InvalidNotebookDigest: AugmentedError<ApiType>;
      /**
       * The signature of the notebook is invalid
       **/
      InvalidNotebookSignature: AugmentedError<ApiType>;
      /**
       * Invalid notebook submission tick
       **/
      InvalidNotebookSubmissionTick: AugmentedError<ApiType>;
      /**
       * Invalid reprocess notebook
       **/
      InvalidReprocessNotebook: AugmentedError<ApiType>;
      /**
       * The secret or secret hash of the parent notebook do not match
       **/
      InvalidSecretProvided: AugmentedError<ApiType>;
      /**
       * The notebook digest was not included
       **/
      MissingNotebookDigest: AugmentedError<ApiType>;
      /**
       * Notebooks received out of order
       **/
      MissingNotebookNumber: AugmentedError<ApiType>;
      /**
       * Multiple inherents provided
       **/
      MultipleNotebookInherentsProvided: AugmentedError<ApiType>;
      /**
       * A notebook was submitted for a notary that failed audit, which is not allowed
       **/
      NotebookSubmittedForLockedNotary: AugmentedError<ApiType>;
      /**
       * A notebook was already provided at this tick
       **/
      NotebookTickAlreadyUsed: AugmentedError<ApiType>;
    };
    ownership: {
      /**
       * Beneficiary account must pre-exist.
       **/
      DeadAccount: AugmentedError<ApiType>;
      /**
       * The delta cannot be zero.
       **/
      DeltaZero: AugmentedError<ApiType>;
      /**
       * Value too low to create account due to existential deposit.
       **/
      ExistentialDeposit: AugmentedError<ApiType>;
      /**
       * A vesting schedule already exists for this account.
       **/
      ExistingVestingSchedule: AugmentedError<ApiType>;
      /**
       * Transfer/payment would kill account.
       **/
      Expendability: AugmentedError<ApiType>;
      /**
       * Balance too low to send value.
       **/
      InsufficientBalance: AugmentedError<ApiType>;
      /**
       * The issuance cannot be modified since it is already deactivated.
       **/
      IssuanceDeactivated: AugmentedError<ApiType>;
      /**
       * Account liquidity restrictions prevent withdrawal.
       **/
      LiquidityRestrictions: AugmentedError<ApiType>;
      /**
       * Number of freezes exceed `MaxFreezes`.
       **/
      TooManyFreezes: AugmentedError<ApiType>;
      /**
       * Number of holds exceed `VariantCountOf<T::RuntimeHoldReason>`.
       **/
      TooManyHolds: AugmentedError<ApiType>;
      /**
       * Number of named reserves exceed `MaxReserves`.
       **/
      TooManyReserves: AugmentedError<ApiType>;
      /**
       * Vesting balance too high to send value.
       **/
      VestingBalance: AugmentedError<ApiType>;
    };
    priceIndex: {
      /**
       * Change in argon price is too large
       **/
      MaxPriceChangePerTickExceeded: AugmentedError<ApiType>;
      /**
       * Missing value
       **/
      MissingValue: AugmentedError<ApiType>;
      /**
       * Not authorized as an oracle operator
       **/
      NotAuthorizedOperator: AugmentedError<ApiType>;
      /**
       * The submitted prices are too old
       **/
      PricesTooOld: AugmentedError<ApiType>;
    };
    proxy: {
      /**
       * Account is already a proxy.
       **/
      Duplicate: AugmentedError<ApiType>;
      /**
       * Call may not be made by proxy because it may escalate its privileges.
       **/
      NoPermission: AugmentedError<ApiType>;
      /**
       * Cannot add self as proxy.
       **/
      NoSelfProxy: AugmentedError<ApiType>;
      /**
       * Proxy registration not found.
       **/
      NotFound: AugmentedError<ApiType>;
      /**
       * Sender is not a proxy of the account to be proxied.
       **/
      NotProxy: AugmentedError<ApiType>;
      /**
       * There are too many proxies registered or too many announcements pending.
       **/
      TooMany: AugmentedError<ApiType>;
      /**
       * Announcement, if made at all, was made too recently.
       **/
      Unannounced: AugmentedError<ApiType>;
      /**
       * A call which is incompatible with the proxy type's filter was attempted.
       **/
      Unproxyable: AugmentedError<ApiType>;
    };
    sudo: {
      /**
       * Sender must be the Sudo account.
       **/
      RequireSudo: AugmentedError<ApiType>;
    };
    system: {
      /**
       * The origin filter prevent the call to be dispatched.
       **/
      CallFiltered: AugmentedError<ApiType>;
      /**
       * Failed to extract the runtime version from the new runtime.
       *
       * Either calling `Core_version` or decoding `RuntimeVersion` failed.
       **/
      FailedToExtractRuntimeVersion: AugmentedError<ApiType>;
      /**
       * The name of specification does not match between the current runtime
       * and the new runtime.
       **/
      InvalidSpecName: AugmentedError<ApiType>;
      /**
       * A multi-block migration is ongoing and prevents the current code from being replaced.
       **/
      MultiBlockMigrationsOngoing: AugmentedError<ApiType>;
      /**
       * Suicide called when the account has non-default composite data.
       **/
      NonDefaultComposite: AugmentedError<ApiType>;
      /**
       * There is a non-zero reference count preventing the account from being purged.
       **/
      NonZeroRefCount: AugmentedError<ApiType>;
      /**
       * No upgrade authorized.
       **/
      NothingAuthorized: AugmentedError<ApiType>;
      /**
       * The specification version is not allowed to decrease between the current runtime
       * and the new runtime.
       **/
      SpecVersionNeedsToIncrease: AugmentedError<ApiType>;
      /**
       * The submitted code is not authorized.
       **/
      Unauthorized: AugmentedError<ApiType>;
    };
    ticks: {};
    tokenGateway: {
      /**
       * Asset Id creation failed
       **/
      AssetCreationError: AugmentedError<ApiType>;
      /**
       * Asset decimals not found
       **/
      AssetDecimalsNotFound: AugmentedError<ApiType>;
      /**
       * Error while teleporting asset
       **/
      AssetTeleportError: AugmentedError<ApiType>;
      /**
       * Coprocessor was not configured in the runtime
       **/
      CoprocessorNotConfigured: AugmentedError<ApiType>;
      /**
       * Asset or update Dispatch Error
       **/
      DispatchError: AugmentedError<ApiType>;
      /**
       * Only root or asset owner can update asset
       **/
      NotAssetOwner: AugmentedError<ApiType>;
      /**
       * Protocol Params have not been initialized
       **/
      NotInitialized: AugmentedError<ApiType>;
      /**
       * Unknown Asset
       **/
      UnknownAsset: AugmentedError<ApiType>;
      /**
       * A asset that has not been registered
       **/
      UnregisteredAsset: AugmentedError<ApiType>;
    };
    treasury: {
      /**
       * The added amount would exceed the activated securitization
       **/
      ActivatedSecuritizationExceeded: AugmentedError<ApiType>;
      /**
       * This fund has already been renewed
       **/
      AlreadyRenewed: AugmentedError<ApiType>;
      /**
       * Below the minimum amount of argons per contributor
       **/
      BelowMinimum: AugmentedError<ApiType>;
      /**
       * The contributed amount would not make this account a contributor
       **/
      ContributionTooLow: AugmentedError<ApiType>;
      /**
       * Unable to update the vault fund
       **/
      CouldNotFindTreasury: AugmentedError<ApiType>;
      /**
       * An internal error occurred (like an overflow)
       **/
      InternalError: AugmentedError<ApiType>;
      /**
       * The prebond amount cannot be reduced and this takes it below the previous allocation
       **/
      MaxAmountBelowMinimum: AugmentedError<ApiType>;
      /**
       * Max contributors for a fund exceeded
       **/
      MaxContributorsExceeded: AugmentedError<ApiType>;
      /**
       * Max Vaults exceeded
       **/
      MaxVaultsExceeded: AugmentedError<ApiType>;
      /**
       * This account is not an active mining fund contributor
       **/
      NotAFundContributor: AugmentedError<ApiType>;
      /**
       * Vault operator only
       **/
      NotAVaultOperator: AugmentedError<ApiType>;
      /**
       * The given vault is not accepting mining bonds
       **/
      VaultNotAcceptingMiningBonds: AugmentedError<ApiType>;
    };
    txPause: {
      /**
       * The call is paused.
       **/
      IsPaused: AugmentedError<ApiType>;
      /**
       * The call is unpaused.
       **/
      IsUnpaused: AugmentedError<ApiType>;
      NotFound: AugmentedError<ApiType>;
      /**
       * The call is whitelisted and cannot be paused.
       **/
      Unpausable: AugmentedError<ApiType>;
    };
    utility: {
      /**
       * Too many calls batched.
       **/
      TooManyCalls: AugmentedError<ApiType>;
    };
    vaults: {
      /**
       * An account may only be associated with a single vault
       **/
      AccountAlreadyHasVault: AugmentedError<ApiType>;
      /**
       * The proposed transaction would take the account below the minimum (existential) balance
       **/
      AccountBelowMinimumBalance: AugmentedError<ApiType>;
      /**
       * Bitcoin conversion to compressed pubkey failed
       **/
      BitcoinConversionFailed: AugmentedError<ApiType>;
      /**
       * A funding change is already scheduled
       **/
      FundingChangeAlreadyScheduled: AugmentedError<ApiType>;
      HoldUnexpectedlyModified: AugmentedError<ApiType>;
      /**
       * The user doesn't have enough funds to complete this request
       **/
      InsufficientFunds: AugmentedError<ApiType>;
      /**
       * There aren't enough funds in the vault to cover the requested bitcoin lock
       **/
      InsufficientVaultFunds: AugmentedError<ApiType>;
      /**
       * An internal processing error occurred
       **/
      InternalError: AugmentedError<ApiType>;
      /**
       * The bitcoin script to lock this bitcoin has errors
       **/
      InvalidBitcoinScript: AugmentedError<ApiType>;
      /**
       * An invalid securitization percent was provided for the vault. NOTE: it cannot be
       * decreased (or negative)
       **/
      InvalidSecuritization: AugmentedError<ApiType>;
      /**
       * Funding would result in an overflow of the balance type
       **/
      InvalidVaultAmount: AugmentedError<ApiType>;
      /**
       * Unable to decode xpubkey
       **/
      InvalidXpubkey: AugmentedError<ApiType>;
      /**
       * Internally, the vault ids are maxed out
       **/
      NoMoreVaultIds: AugmentedError<ApiType>;
      NoPermissions: AugmentedError<ApiType>;
      /**
       * No Vault public keys are available
       **/
      NoVaultBitcoinPubkeysAvailable: AugmentedError<ApiType>;
      /**
       * A vault must clear out all pending cosigns before it can collect
       **/
      PendingCosignsBeforeCollect: AugmentedError<ApiType>;
      /**
       * The vault bitcoin xpubkey has already been used
       **/
      ReusedVaultBitcoinXpub: AugmentedError<ApiType>;
      /**
       * Terms are already scheduled to be changed
       **/
      TermsChangeAlreadyScheduled: AugmentedError<ApiType>;
      /**
       * The terms modification list could not handle any more items
       **/
      TermsModificationOverflow: AugmentedError<ApiType>;
      /**
       * Unable to derive xpubkey child
       **/
      UnableToDeriveVaultXpubChild: AugmentedError<ApiType>;
      /**
       * Unable to generate a new vault bitcoin pubkey
       **/
      UnableToGenerateVaultBitcoinPubkey: AugmentedError<ApiType>;
      UnrecoverableHold: AugmentedError<ApiType>;
      /**
       * The XPub is unsafe to use in a public blockchain (aka, unhardened)
       **/
      UnsafeXpubkey: AugmentedError<ApiType>;
      /**
       * This vault is closed
       **/
      VaultClosed: AugmentedError<ApiType>;
      VaultNotFound: AugmentedError<ApiType>;
      /**
       * The vault is not yet active
       **/
      VaultNotYetActive: AugmentedError<ApiType>;
      /**
       * This reduction in vault securitization goes below the amount already committed
       **/
      VaultReductionBelowSecuritization: AugmentedError<ApiType>;
      /**
       * Wrong Xpub Network
       **/
      WrongXpubNetwork: AugmentedError<ApiType>;
    };
  } // AugmentedErrors
} // declare module
