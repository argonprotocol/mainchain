// Auto-generated via `yarn polkadot-types-from-chain`, do not edit
/* eslint-disable */

// import type lookup before we augment - in some environments
// this is required to allow for ambient/previous definitions
import type {} from '@polkadot/api-base/types/errors';

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
      /**
       * The Bitcoin Unspect Transaction Output (UTXO) was not found
       **/
      BitcoinUtxoNotFound: AugmentedError<ApiType>;
      /**
       * An overflow occurred recording a lock expiration
       **/
      ExpirationAtBlockOverflow: AugmentedError<ApiType>;
      /**
       * Cannot request an orphaned release for the funding UTXO
       **/
      FundingUtxoCannotBeReleased: AugmentedError<ApiType>;
      /**
       * An error occurred in the vault module
       **/
      GenericVaultError: AugmentedError<ApiType>;
      /**
       * The expected amount of funds to return from hold was not available
       **/
      HoldUnexpectedlyModified: AugmentedError<ApiType>;
      /**
       * An ineligible microgon rate per btc was requested
       **/
      IneligibleMicrogonRateRequested: AugmentedError<ApiType>;
      InsufficientFunds: AugmentedError<ApiType>;
      /**
       * The minimum number of satoshis was not met
       **/
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
      /**
       * The Bitcoin Lock record was not found
       **/
      LockNotFound: AugmentedError<ApiType>;
      /**
       * The lock funding has not been confirmed on bitcoin
       **/
      LockPendingFunding: AugmentedError<ApiType>;
      /**
       * Too many orphaned utxo release requests for a lock
       **/
      MaxOrphanedUtxoReleaseRequestsExceeded: AugmentedError<ApiType>;
      /**
       * The price provider has no bitcoin prices available. This is a temporary error
       **/
      NoBitcoinPricesAvailable: AugmentedError<ApiType>;
      /**
       * The user does not have permissions to perform this action
       **/
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
       * Cannot fund with an orphaned utxo after lock funding is confirmed
       **/
      OrphanedUtxoFundingConflict: AugmentedError<ApiType>;
      /**
       * Cannot lock an orphaned utxo with a pending release request
       **/
      OrphanedUtxoReleaseRequested: AugmentedError<ApiType>;
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
      /**
       * The hold on funds could not be recovered
       **/
      UnrecoverableHold: AugmentedError<ApiType>;
      /**
       * This vault is closed
       **/
      VaultClosed: AugmentedError<ApiType>;
      /**
       * The vault was not found
       **/
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
       * This Lock already has an attached funding UTXO
       **/
      LockAlreadyFunded: AugmentedError<ApiType>;
      /**
       * Too many candidate UTXOs are being stored for this lock
       **/
      MaxCandidateUtxosExceeded: AugmentedError<ApiType>;
      /**
       * Too many UTXOs are being watched
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
       * The UTXO reference does not map to a candidate entry
       **/
      UtxoNotCandidate: AugmentedError<ApiType>;
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
       * Duplicate vote block
       **/
      DuplicateVoteBlockAtTick: AugmentedError<ApiType>;
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
       * The nonce score distance supplied is invalid
       **/
      InvalidMinerNonceScore: AugmentedError<ApiType>;
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
    crosschainTransfer: {
      /**
       * The force-set cut would discard a queue entry that already has local quorum.
       **/
      CannotForceSetQuorumApprovedQueueEntry: AugmentedError<ApiType>;
      /**
       * This council member already approved the queued Minting Authority.
       **/
      CouncilApprovalAlreadyRecorded: AugmentedError<ApiType>;
      /**
       * The Minting Authority approval queue entry does not exist.
       **/
      CouncilApprovalQueueEntryNotFound: AugmentedError<ApiType>;
      /**
       * The account has not registered a council signer for this destination chain.
       **/
      CouncilSignerNotRegistered: AugmentedError<ApiType>;
      /**
       * The supplied Global Issuance Council contains the same account more than once.
       **/
      DuplicateGlobalIssuanceCouncilAccount: AugmentedError<ApiType>;
      /**
       * The supplied Global Issuance Council contains the same signer more than once.
       **/
      DuplicateGlobalIssuanceCouncilSigner: AugmentedError<ApiType>;
      /**
       * This source chain is paused due to gateway sync misalignment and needs operator
       * recovery.
       **/
      GatewaySyncPaused: AugmentedError<ApiType>;
      /**
       * The origin is not an active Global Issuance Council member for the destination chain.
       **/
      GlobalIssuanceCouncilMemberNotFound: AugmentedError<ApiType>;
      /**
       * The active Global Issuance Council has not been seeded for this destination chain.
       **/
      GlobalIssuanceCouncilNotFound: AugmentedError<ApiType>;
      /**
       * The operator does not have enough remaining committed Argonot collateral for this
       * Minting Authority.
       **/
      InsufficientCommittedArgonotCollateral: AugmentedError<ApiType>;
      /**
       * The operator does not have enough remaining committed microgon collateral for this
       * Minting Authority.
       **/
      InsufficientCommittedMicrogonCollateral: AugmentedError<ApiType>;
      /**
       * The burn account lacks enough balance for the payout.
       **/
      InsufficientLiquidity: AugmentedError<ApiType>;
      /**
       * The authority does not have enough remaining gateway collateral for this transfer row.
       **/
      InsufficientMintingAuthorityCollateral: AugmentedError<ApiType>;
      /**
       * The configured source-chain shape is incomplete or malformed.
       **/
      InvalidChainConfig: AugmentedError<ApiType>;
      /**
       * The provided council approval signature did not match the expected Ethereum signer.
       **/
      InvalidCouncilApprovalSignature: AugmentedError<ApiType>;
      /**
       * The provided council signer proof did not match the recovered Ethereum signer.
       **/
      InvalidCouncilSignerProof: AugmentedError<ApiType>;
      /**
       * The force-set cut point was behind already confirmed queue progress.
       **/
      InvalidForceSetAfterNonce: AugmentedError<ApiType>;
      /**
       * The Ethereum event topics or payload do not match a supported gateway activity.
       **/
      InvalidGatewayActivity: AugmentedError<ApiType>;
      /**
       * The supplied Global Issuance Council is empty or internally inconsistent.
       **/
      InvalidGlobalIssuanceCouncil: AugmentedError<ApiType>;
      /**
       * The council-managed Argonot floor could not be determined from current pricing.
       **/
      InvalidMicrogonsPerArgonot: AugmentedError<ApiType>;
      /**
       * The configured activation repayment pricing is internally invalid.
       **/
      InvalidMintingAuthorityActivationRepaymentPricing: AugmentedError<ApiType>;
      /**
       * The supplied Minting Authority collateral is invalid.
       **/
      InvalidMintingAuthorityCollateral: AugmentedError<ApiType>;
      /**
       * Reserved legacy error for invalid signer-keyed deactivation signatures.
       **/
      InvalidMintingAuthorityDeactivationSignature: AugmentedError<ApiType>;
      /**
       * The supplied Minting Authority signing key is invalid.
       **/
      InvalidMintingAuthoritySigningKey: AugmentedError<ApiType>;
      /**
       * The provided Minting Authority signer proof did not match the recovered Ethereum
       * signer.
       **/
      InvalidMintingAuthoritySigningKeyProof: AugmentedError<ApiType>;
      /**
       * The Ethereum verifier rejected the supplied proof.
       **/
      InvalidProof: AugmentedError<ApiType>;
      /**
       * The provided transfer collateral row is invalid for this transfer asset.
       **/
      InvalidTransferCollateral: AugmentedError<ApiType>;
      /**
       * The transfer collateral signature did not match the authority signing key.
       **/
      InvalidTransferCollateralSignature: AugmentedError<ApiType>;
      /**
       * The provided transfer collateral row did not advance the signer's local reservation.
       **/
      InvalidTransferCollateralUpdate: AugmentedError<ApiType>;
      /**
       * The transfer-out amount must be nonzero.
       **/
      InvalidTransferOutAmount: AugmentedError<ApiType>;
      /**
       * The transfer-out recipient must be nonzero for the destination chain.
       **/
      InvalidTransferOutRecipient: AugmentedError<ApiType>;
      /**
       * The Ethereum event topics or payload do not match `TransferToArgonStarted`.
       **/
      InvalidTransferToArgonActivity: AugmentedError<ApiType>;
      /**
       * The supplied Minting Authority signing key already has a live local authority record.
       **/
      MintingAuthorityAlreadyRegistered: AugmentedError<ApiType>;
      /**
       * The supplied Minting Authority collateral is below the configured per-chain minimum
       * normalized microgon value.
       **/
      MintingAuthorityCollateralBelowMinimum: AugmentedError<ApiType>;
      /**
       * The local Minting Authority did not match the proven Ethereum activity.
       **/
      MintingAuthorityMismatch: AugmentedError<ApiType>;
      /**
       * The supplied Minting Authority signing key does not exist locally.
       **/
      MintingAuthorityNotFound: AugmentedError<ApiType>;
      /**
       * The configured activation repayment pricing is missing for this source chain.
       **/
      MissingMintingAuthorityActivationRepaymentPricing: AugmentedError<ApiType>;
      /**
       * No verifier-backed Ethereum execution block is available to anchor a transfer-out
       * expiry window.
       **/
      MissingVerifiedExecutionBlock: AugmentedError<ApiType>;
      /**
       * At least one council approval signature must be supplied.
       **/
      NoCouncilApprovalSignaturesProvided: AugmentedError<ApiType>;
      /**
       * At least one gateway activity log must be supplied with the receipt proof.
       **/
      NoGatewayActivitiesProvided: AugmentedError<ApiType>;
      /**
       * At least one proven gateway-activity block must be supplied.
       **/
      NoGatewayProofBlocksProvided: AugmentedError<ApiType>;
      /**
       * The latest verifier-backed Ethereum execution block is too old to safely open a new
       * transfer out.
       **/
      StaleVerifiedExecutionBlock: AugmentedError<ApiType>;
      /**
       * The destination chain already tracks the maximum number of non-terminal transfer-out
       * requests.
       **/
      TooManyPendingTransferOuts: AugmentedError<ApiType>;
      /**
       * The collateral increment is below the configured minimum and does not complete the
       * transfer.
       **/
      TransferCollateralIncrementTooSmall: AugmentedError<ApiType>;
      /**
       * The outbound transfer cannot accept more collateral because it is already fully
       * covered.
       **/
      TransferOutAlreadyReady: AugmentedError<ApiType>;
      /**
       * The outbound transfer request is already expired on the latest verified Ethereum block.
       **/
      TransferOutExpired: AugmentedError<ApiType>;
      /**
       * The outbound transfer record does not exist.
       **/
      TransferOutNotFound: AugmentedError<ApiType>;
      /**
       * The proven gateway activity nonce is not the next contiguous nonce.
       **/
      UnexpectedGatewayActivityNonce: AugmentedError<ApiType>;
      /**
       * The local Minting Authority was not in the expected lifecycle state.
       **/
      UnexpectedMintingAuthorityState: AugmentedError<ApiType>;
      /**
       * The caller's expected already-proven gateway activity nonce is stale or incorrect.
       **/
      UnexpectedPreviousGatewayActivityNonce: AugmentedError<ApiType>;
      /**
       * The local owner vault could not be resolved.
       **/
      UnknownOwnerVault: AugmentedError<ApiType>;
      /**
       * The gateway does not match the configured gateway address.
       **/
      UnsupportedGateway: AugmentedError<ApiType>;
      /**
       * The source chain is not configured for inbound claims.
       **/
      UnsupportedSource: AugmentedError<ApiType>;
      /**
       * The token is not supported under the configured gateway.
       **/
      UnsupportedToken: AugmentedError<ApiType>;
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
       * Duplicate FrameInfo found
       **/
      DuplicateFrameInfoDigest: AugmentedError<ApiType>;
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
    ethereumVerifier: {
      BlockBodyHashTreeRootFailed: AugmentedError<ApiType>;
      BLSPreparePublicKeysFailed: AugmentedError<ApiType>;
      BLSVerificationFailed: AugmentedError<ApiType>;
      ExecutionHeaderAnchorAlreadyImported: AugmentedError<ApiType>;
      ExecutionHeaderAnchorNotHistorical: AugmentedError<ApiType>;
      ExpectedFinalizedHeaderNotStored: AugmentedError<ApiType>;
      ForkDataHashTreeRootFailed: AugmentedError<ApiType>;
      Halted: AugmentedError<ApiType>;
      HeaderHashTreeRootFailed: AugmentedError<ApiType>;
      InvalidBackfillHeaderRoot: AugmentedError<ApiType>;
      InvalidExecutionHeaderProof: AugmentedError<ApiType>;
      /**
       * The gap between finalized headers is larger than the retained historical window.
       **/
      InvalidFinalizedHeaderGap: AugmentedError<ApiType>;
      InvalidHeaderMerkleProof: AugmentedError<ApiType>;
      InvalidSyncCommitteeMerkleProof: AugmentedError<ApiType>;
      /**
       * The given update is not in the expected period, or the given next sync committee does
       * not match the next sync committee in storage.
       **/
      InvalidSyncCommitteeUpdate: AugmentedError<ApiType>;
      InvalidUpdateSlot: AugmentedError<ApiType>;
      /**
       * Attested header is older than latest finalized header.
       **/
      IrrelevantUpdate: AugmentedError<ApiType>;
      NotBootstrapped: AugmentedError<ApiType>;
      SigningRootHashTreeRootFailed: AugmentedError<ApiType>;
      SkippedSyncCommitteePeriod: AugmentedError<ApiType>;
      SyncCommitteeHashTreeRootFailed: AugmentedError<ApiType>;
      SyncCommitteeParticipantsNotSupermajority: AugmentedError<ApiType>;
      SyncCommitteeUpdateRequired: AugmentedError<ApiType>;
      UnexpectedBeaconPreset: AugmentedError<ApiType>;
    };
    feeControl: {
      /**
       * The requested tip + fee is higher than the maximum allowed by the sponsor
       **/
      SponsoredFeeTooHigh: AugmentedError<ApiType>;
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
    localchainTransfer: {
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
       * No transfer IDs are currently available
       **/
      NoAvailableTransferId: AugmentedError<ApiType>;
      /**
       * The notary is currently locked and cannot process transfers
       **/
      NotaryLockedForTransfer: AugmentedError<ApiType>;
      /**
       * A transfer was submitted in a previous block but the expiration block has passed
       **/
      NotebookIncludesExpiredLocalchainTransfer: AugmentedError<ApiType>;
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
       * Cannot re-register an account with a different funding account
       **/
      CannotChangeFundingAccount: AugmentedError<ApiType>;
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
       * Mining slot bidding currently requires prior operational-account registration.
       **/
      OperationalAccountRegistrationRequired: AugmentedError<ApiType>;
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
    operationalAccounts: {
      /**
       * One of the provided accounts is already linked to an operational account.
       **/
      AccountAlreadyLinked: AugmentedError<ApiType>;
      /**
       * The account is already operational.
       **/
      AlreadyOperational: AugmentedError<ApiType>;
      /**
       * The caller already registered an operational account.
       **/
      AlreadyRegistered: AugmentedError<ApiType>;
      /**
       * The encrypted server payload exceeds the configured max length.
       **/
      EncryptedServerTooLong: AugmentedError<ApiType>;
      /**
       * One of the linked account ownership proofs is invalid.
       **/
      InvalidAccountProof: AugmentedError<ApiType>;
      /**
       * The referral proof or sponsor proof is invalid.
       **/
      InvalidReferralProof: AugmentedError<ApiType>;
      /**
       * The caller is not one of the accounts included in the registration.
       **/
      InvalidRegistrationSubmitter: AugmentedError<ApiType>;
      /**
       * The operational account has no pending rewards to claim.
       **/
      NoPendingRewards: AugmentedError<ApiType>;
      /**
       * The requested progress patch does not contain any updates.
       **/
      NoProgressUpdateProvided: AugmentedError<ApiType>;
      /**
       * The account has not satisfied operational requirements yet.
       **/
      NotEligibleForActivation: AugmentedError<ApiType>;
      /**
       * The caller has not registered an operational account.
       **/
      NotOperationalAccount: AugmentedError<ApiType>;
      /**
       * The caller is not the sponsor of the requested sponsee.
       **/
      NotSponsorOfSponsee: AugmentedError<ApiType>;
      /**
       * The referral proof has expired.
       **/
      ReferralProofExpired: AugmentedError<ApiType>;
      /**
       * A valid invite is required to register an operational account.
       **/
      RegistrationInviteRequired: AugmentedError<ApiType>;
      /**
       * Reward claims must be at least one Argon.
       **/
      RewardClaimBelowMinimum: AugmentedError<ApiType>;
      /**
       * The requested reward claim exceeds pending rewards.
       **/
      RewardClaimExceedsPending: AugmentedError<ApiType>;
      /**
       * Reward claims must be whole Argon increments.
       **/
      RewardClaimNotWholeArgon: AugmentedError<ApiType>;
      /**
       * The treasury does not currently have enough available reserves for the claim.
       **/
      TreasuryInsufficientFunds: AugmentedError<ApiType>;
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
    treasury: {
      /**
       * Liquidating this bond lot would take the account below its crosschain-encumbered
       * treasury backing.
       **/
      ActiveBondAmountBelowEncumberedBacking: AugmentedError<ApiType>;
      /**
       * The Argonot bond purchase would exceed the active circulation cap.
       **/
      ArgonotBondPurchaseAboveCap: AugmentedError<ApiType>;
      /**
       * The Argonot bond purchase did not beat the current active-set cutoff.
       **/
      ArgonotBondPurchaseBelowCutoff: AugmentedError<ApiType>;
      /**
       * The bond lot is already scheduled for release.
       **/
      BondLotAlreadyReleasing: AugmentedError<ApiType>;
      /**
       * The bond lot could not be found.
       **/
      BondLotNotFound: AugmentedError<ApiType>;
      /**
       * The vault doesn't have enough bitcoin security to support this bond purchase
       **/
      BondPurchaseAboveSecurity: AugmentedError<ApiType>;
      /**
       * The purchase is below the minimum amount.
       **/
      BondPurchaseBelowMinimum: AugmentedError<ApiType>;
      /**
       * The purchase would not enter the vault's accepted list.
       **/
      BondPurchaseRejected: AugmentedError<ApiType>;
      /**
       * The beneficiary already has a bond lot for this vault.
       **/
      BonusApprovalExistingBondLot: AugmentedError<ApiType>;
      /**
       * The bonus approval already expired.
       **/
      BonusApprovalExpired: AugmentedError<ApiType>;
      /**
       * The bonus approval was signed for a different beneficiary.
       **/
      BonusApprovalWrongAccount: AugmentedError<ApiType>;
      /**
       * The bonus approval was signed for a different vault.
       **/
      BonusApprovalWrongVault: AugmentedError<ApiType>;
      /**
       * An internal error occurred.
       **/
      InternalError: AugmentedError<ApiType>;
      /**
       * The bonus approval signature is invalid or unauthorized.
       **/
      InvalidBonusApprovalSignature: AugmentedError<ApiType>;
      /**
       * The vault already has the maximum number of accepted bond lots.
       **/
      MaxAcceptedBondLotsExceeded: AugmentedError<ApiType>;
      /**
       * Too many bond lot releases are scheduled for the same frame.
       **/
      MaxPendingBondReleasesExceeded: AugmentedError<ApiType>;
      /**
       * The caller does not own the bond lot.
       **/
      NotBondLotOwner: AugmentedError<ApiType>;
      /**
       * The vault is not accepting bond purchases.
       **/
      VaultNotAcceptingBondPurchases: AugmentedError<ApiType>;
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
       * Committed Argonots cannot be reduced below the amount already crosschain-encumbered.
       **/
      CommittedArgonotsBelowEncumberedBacking: AugmentedError<ApiType>;
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
       * Treasury bond sharing plus bonus cannot exceed 100%.
       **/
      InvalidBondSharingTerms: AugmentedError<ApiType>;
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
       * Vault names must start with an uppercase ASCII letter and otherwise be ASCII
       * alphanumeric.
       **/
      InvalidVaultName: AugmentedError<ApiType>;
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
       * Vault creation currently requires prior operational-account registration.
       **/
      OperationalAccountRegistrationRequired: AugmentedError<ApiType>;
      /**
       * A vault must clear out all overdue external collect blockers before it can collect.
       **/
      OverdueCollectBlockersBeforeCollect: AugmentedError<ApiType>;
      /**
       * A vault must clear out all pending cosigns before it can collect
       **/
      PendingCosignsBeforeCollect: AugmentedError<ApiType>;
      /**
       * A vault must clear out all pending orphan cosigns before it can collect
       **/
      PendingOrphanedUtxoCosignsBeforeCollect: AugmentedError<ApiType>;
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
