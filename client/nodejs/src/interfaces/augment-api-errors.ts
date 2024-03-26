// Auto-generated via `yarn polkadot-types-from-chain`, do not edit
/* eslint-disable */

// import type lookup before we augment - in some environments
// this is required to allow for ambient/previous definitions
import '@polkadot/api-base/types/errors';

import type { ApiTypes, AugmentedError } from '@polkadot/api-base/types';

export type __AugmentedError<ApiType extends ApiTypes> = AugmentedError<ApiType>;

declare module '@polkadot/api-base/types/errors' {
  interface AugmentedErrors<ApiType extends ApiTypes> {
    argonBalances: {
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
    blockRewards: {
    };
    blockSeal: {
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
       * Message was not signed by a registered miner
       **/
      InvalidAuthoritySignature: AugmentedError<ApiType>;
      /**
       * The merkle proof of vote inclusion in the notebook is invalid
       **/
      InvalidBlockVoteProof: AugmentedError<ApiType>;
      /**
       * The data domain account is mismatched with the block reward seeker
       **/
      InvalidDataDomainAccount: AugmentedError<ApiType>;
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
      /**
       * The data domain was not registered
       **/
      UnregisteredDataDomain: AugmentedError<ApiType>;
    };
    blockSealSpec: {
      /**
       * The maximum number of notebooks at the current tick has been exceeded
       **/
      MaxNotebooksAtTickExceeded: AugmentedError<ApiType>;
    };
    bond: {
      BadState: AugmentedError<ApiType>;
      BondAlreadyLocked: AugmentedError<ApiType>;
      BondFundClosed: AugmentedError<ApiType>;
      BondFundMaximumBondsExceeded: AugmentedError<ApiType>;
      BondFundNotFound: AugmentedError<ApiType>;
      /**
       * This reduction in bond funds offered goes below the amount that is already committed to
       * bond
       **/
      BondFundReductionExceedsAllocatedFunds: AugmentedError<ApiType>;
      BondLockedCannotModify: AugmentedError<ApiType>;
      BondNotFound: AugmentedError<ApiType>;
      /**
       * There are too many bond or bond funds expiring in the given expiration block
       **/
      ExpirationAtBlockOverflow: AugmentedError<ApiType>;
      ExpirationTooSoon: AugmentedError<ApiType>;
      /**
       * The fee for this bond exceeds the amount of the bond, which is unsafe
       **/
      FeeExceedsBondAmount: AugmentedError<ApiType>;
      FundExtensionMustBeLater: AugmentedError<ApiType>;
      HoldUnexpectedlyModified: AugmentedError<ApiType>;
      InsufficientBondFunds: AugmentedError<ApiType>;
      InsufficientFunds: AugmentedError<ApiType>;
      LeaseUntilBlockTooSoon: AugmentedError<ApiType>;
      LeaseUntilPastFundExpiration: AugmentedError<ApiType>;
      MinimumBondAmountNotMet: AugmentedError<ApiType>;
      NoBondFundFound: AugmentedError<ApiType>;
      NoMoreBondFundIds: AugmentedError<ApiType>;
      NoMoreBondIds: AugmentedError<ApiType>;
      NoPermissions: AugmentedError<ApiType>;
      TransactionWouldTakeAccountBelowMinimumBalance: AugmentedError<ApiType>;
      UnrecoverableHold: AugmentedError<ApiType>;
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
       * The account nonce used for this transfer is no longer valid
       **/
      InvalidAccountNonce: AugmentedError<ApiType>;
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
    dataDomain: {
      /**
       * The domain is not registered.
       **/
      DomainNotRegistered: AugmentedError<ApiType>;
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
    miningSlot: {
      AccountWouldBeBelowMinimum: AugmentedError<ApiType>;
      /**
       * Internal state has become somehow corrupted and the operation cannot continue.
       **/
      BadInternalState: AugmentedError<ApiType>;
      BadState: AugmentedError<ApiType>;
      BidBondDurationTooShort: AugmentedError<ApiType>;
      BidTooLow: AugmentedError<ApiType>;
      BondAlreadyClosed: AugmentedError<ApiType>;
      BondAlreadyLocked: AugmentedError<ApiType>;
      BondFundClosed: AugmentedError<ApiType>;
      BondFundMaximumBondsExceeded: AugmentedError<ApiType>;
      BondFundNotFound: AugmentedError<ApiType>;
      BondLockedCannotModify: AugmentedError<ApiType>;
      BondNotFound: AugmentedError<ApiType>;
      CannotRegisteredOverlappingSessions: AugmentedError<ApiType>;
      /**
       * There are too many bond or bond funds expiring in the given expiration block
       **/
      ExpirationAtBlockOverflow: AugmentedError<ApiType>;
      ExpirationTooSoon: AugmentedError<ApiType>;
      /**
       * The fee for this bond exceeds the amount of the bond, which is unsafe
       **/
      FeeExceedsBondAmount: AugmentedError<ApiType>;
      HoldUnexpectedlyModified: AugmentedError<ApiType>;
      InsufficientBalanceForBid: AugmentedError<ApiType>;
      InsufficientBondFunds: AugmentedError<ApiType>;
      InsufficientFunds: AugmentedError<ApiType>;
      InsufficientOwnershipTokens: AugmentedError<ApiType>;
      LeaseUntilBlockTooSoon: AugmentedError<ApiType>;
      LeaseUntilPastFundExpiration: AugmentedError<ApiType>;
      MinimumBondAmountNotMet: AugmentedError<ApiType>;
      NoBondFundFound: AugmentedError<ApiType>;
      NoMoreBondIds: AugmentedError<ApiType>;
      NoPermissions: AugmentedError<ApiType>;
      /**
       * You must register with rpc hosts so that your miner can be reached for block seal
       * auditing
       **/
      RpcHostsAreRequired: AugmentedError<ApiType>;
      SlotNotTakingBids: AugmentedError<ApiType>;
      TooManyBlockRegistrants: AugmentedError<ApiType>;
      UnableToRotateAuthority: AugmentedError<ApiType>;
      UnrecoverableHold: AugmentedError<ApiType>;
    };
    mint: {
    };
    notaries: {
      InvalidNotaryOperator: AugmentedError<ApiType>;
      MaxNotariesExceeded: AugmentedError<ApiType>;
      MaxProposalsPerBlockExceeded: AugmentedError<ApiType>;
      NoMoreNotaryIds: AugmentedError<ApiType>;
      NotAnActiveNotary: AugmentedError<ApiType>;
      ProposalNotFound: AugmentedError<ApiType>;
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
       * The notebook digest did not match the included notebooks
       **/
      InvalidNotebookDigest: AugmentedError<ApiType>;
      /**
       * The signature of the notebook is invalid
       **/
      InvalidNotebookSignature: AugmentedError<ApiType>;
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
       * A notebook was already provided at this tick
       **/
      NotebookTickAlreadyUsed: AugmentedError<ApiType>;
    };
    session: {
      /**
       * Registered duplicate key.
       **/
      DuplicatedKey: AugmentedError<ApiType>;
      /**
       * Invalid ownership proof.
       **/
      InvalidProof: AugmentedError<ApiType>;
      /**
       * Key setting account is not live, so it's impossible to associate keys.
       **/
      NoAccount: AugmentedError<ApiType>;
      /**
       * No associated validator ID for account.
       **/
      NoAssociatedValidatorId: AugmentedError<ApiType>;
      /**
       * No keys are associated with this account.
       **/
      NoKeys: AugmentedError<ApiType>;
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
    ticks: {
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
    ulixeeBalances: {
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
  } // AugmentedErrors
} // declare module
