#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::large_enum_variant)]

extern crate alloc;

pub use pallet::*;
use pallet_prelude::frame_support;
pub use weights::{WeightInfo, WithProviderWeights};

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
pub mod migrations;
mod weights;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use alloc::vec::Vec;
	use argon_primitives::{
		MICROGONS_PER_ARGON, MiningFrameTransitionProvider, MiningSlotProvider,
		OperationalAccountsHook, OperationalRewardKind, OperationalRewardsPayer,
		RecentArgonTransferLookup, Signature, TreasuryPoolProvider,
		UniswapTransferRequirementProvider, vault::BitcoinVaultProvider,
	};
	use codec::{Decode, Encode, EncodeLike};
	use core::marker::PhantomData;
	use pallet_prelude::*;
	use polkadot_sdk::frame_system::ensure_root;
	use sp_core::sr25519;
	use sp_runtime::{
		AccountId32,
		traits::{Verify, Zero},
	};

	/// Domain separator for referral claim proofs.
	pub const REFERRAL_CLAIM_PROOF_MESSAGE_KEY: &[u8; 14] = b"referral_claim";
	pub const REFERRAL_SPONSOR_GRANT_MESSAGE_KEY: &[u8; 22] = b"referral_sponsor_grant";
	pub const OPERATIONAL_ACCOUNT_PROOF_MESSAGE_KEY: &[u8; 27] = b"operational_primary_account";
	pub const VAULT_ACCOUNT_PROOF_MESSAGE_KEY: &[u8; 25] = b"operational_vault_account";
	pub const MINING_FUNDING_ACCOUNT_PROOF_MESSAGE_KEY: &[u8; 26] = b"operational_mining_funding";
	pub const MINING_BOT_ACCOUNT_PROOF_MESSAGE_KEY: &[u8; 22] = b"operational_mining_bot";
	const STORAGE_VERSION: StorageVersion = StorageVersion::new(1);

	#[pallet::pallet]
	#[pallet::storage_version(STORAGE_VERSION)]
	pub struct Pallet<T>(_);

	#[pallet::config]
	#[pallet::disable_frame_system_supertrait_check]
	pub trait Config: polkadot_sdk::frame_system::Config {
		/// The balance of an account.
		type Balance: AtLeast32BitUnsigned
			+ Member
			+ codec::FullCodec
			+ Copy
			+ MaybeSerializeDeserialize
			+ DecodeWithMemTracking
			+ core::fmt::Debug
			+ Default
			+ From<u128>
			+ Into<u128>
			+ TypeInfo
			+ MaxEncodedLen;

		/// Provides access to the current mining frame id.
		type FrameProvider: MiningFrameTransitionProvider;

		/// Additional argon amount (base units) required per referral after operational.
		#[pallet::constant]
		type BitcoinLockSizeForReferral: Get<Self::Balance>;
		/// Default reward paid when an account becomes operational.
		#[pallet::constant]
		type OperationalReferralReward: Get<Self::Balance>;
		/// Default bonus reward paid every referral threshold.
		#[pallet::constant]
		type OperationalReferralBonusReward: Get<Self::Balance>;
		/// Number of operational sponsees required per referral bonus reward.
		#[pallet::constant]
		type ReferralBonusEveryXOperationalSponsees: Get<u32>;
		/// Maximum number of available referrals allowed at once.
		#[pallet::constant]
		type MaxAvailableReferrals: Get<u32>;
		/// Maximum number of expired referral codes cleared per block.
		#[pallet::constant]
		type MaxExpiredReferralCodeCleanupsPerBlock: Get<u32>;
		/// Maximum number of encrypted server bytes stored per sponsee.
		#[pallet::constant]
		type MaxEncryptedServerLen: Get<u32>;
		/// Minimum vault securitization required to become operational.
		#[pallet::constant]
		type OperationalMinimumVaultSecuritization: Get<Self::Balance>;
		/// Mining seats required to become operational.
		#[pallet::constant]
		type MiningSeatsForOperational: Get<u32>;
		/// Mining seats required per referral after operational.
		#[pallet::constant]
		type MiningSeatsPerReferral: Get<u32>;

		/// Provider for current vault state used to initialize registration.
		type VaultProvider: BitcoinVaultProvider<AccountId = Self::AccountId, Balance = Self::Balance>;
		/// Provider for whether a linked mining rewards account currently has an active seat.
		type MiningSlotProvider: MiningSlotProvider<Self::AccountId>;
		/// Provider for whether a linked vault account currently has treasury pool participation.
		type TreasuryPoolProvider: TreasuryPoolProvider<Self::AccountId>;
		/// Provider for whether new operational accounts should require a Uniswap-backed transfer.
		type UniswapTransferRequirementProvider: UniswapTransferRequirementProvider;
		/// Provider for recent qualifying inbound Argon transfer lookup.
		type RecentArgonTransferLookup: RecentArgonTransferLookup<Self::AccountId>;
		/// Payout adapter for explicitly claimed operational rewards.
		type OperationalRewardsPayer: OperationalRewardsPayer<Self::AccountId, Self::Balance>;

		/// Weight information for this pallet.
		type WeightInfo: WeightInfo;
	}

	#[derive(
		Decode,
		DecodeWithMemTracking,
		Encode,
		CloneNoBound,
		PartialEqNoBound,
		EqNoBound,
		RuntimeDebug,
		TypeInfo,
		MaxEncodedLen,
	)]
	pub struct ReferralProof<AccountId>
	where
		AccountId: Clone + PartialEq + Eq,
	{
		/// Public referral code shared by the sponsor.
		pub referral_code: sr25519::Public,
		/// Signature from the referral code over the registration claim.
		pub referral_signature: sr25519::Signature,
		/// Operational account sponsoring this referral.
		pub sponsor: AccountId,
		/// Mining frame where this referral grant expires.
		#[codec(compact)]
		pub expires_at_frame: FrameId,
		/// Signature from the sponsor over the referral grant.
		pub sponsor_signature: Signature,
	}

	impl<AccountId> ReferralProof<AccountId>
	where
		AccountId: Clone + Encode + Eq + PartialEq,
	{
		pub fn verify_claim<Owner: EncodeLike>(&self, account_id: &Owner) -> bool {
			let message = (REFERRAL_CLAIM_PROOF_MESSAGE_KEY, self.referral_code, account_id)
				.using_encoded(blake2_256);
			let verified = self.referral_signature.verify(message.as_slice(), &self.referral_code);
			#[cfg(feature = "runtime-benchmarks")]
			{
				let _ = verified;
				return true;
			}
			#[cfg(not(feature = "runtime-benchmarks"))]
			{
				verified
			}
		}

		pub fn verify_sponsor(&self) -> bool {
			let Ok(sponsor) = AccountId32::decode(&mut self.sponsor.encode().as_slice()) else {
				return false;
			};
			let message = (
				REFERRAL_SPONSOR_GRANT_MESSAGE_KEY,
				&self.sponsor,
				self.referral_code,
				self.expires_at_frame,
			)
				.using_encoded(blake2_256);
			let verified = self.sponsor_signature.verify(message.as_slice(), &sponsor);
			#[cfg(feature = "runtime-benchmarks")]
			{
				let _ = verified;
				return true;
			}
			#[cfg(not(feature = "runtime-benchmarks"))]
			{
				verified
			}
		}
	}

	#[derive(
		Decode,
		DecodeWithMemTracking,
		Encode,
		CloneNoBound,
		PartialEqNoBound,
		EqNoBound,
		RuntimeDebugNoBound,
		TypeInfo,
		MaxEncodedLen,
	)]
	pub struct AccountOwnershipProof {
		pub signature: Signature,
	}

	impl AccountOwnershipProof {
		pub fn verify<AccountId: Encode>(
			&self,
			owner: &AccountId,
			account_id: &AccountId,
			domain: &[u8],
		) -> bool {
			let Ok(account_id) = AccountId32::decode(&mut account_id.encode().as_slice()) else {
				return false;
			};
			let message = (domain, owner, &account_id).using_encoded(blake2_256);
			let verified = self.signature.verify(message.as_slice(), &account_id);
			#[cfg(feature = "runtime-benchmarks")]
			{
				let _ = verified;
				return true;
			}
			#[cfg(not(feature = "runtime-benchmarks"))]
			{
				verified
			}
		}
	}

	#[derive(
		Encode,
		Decode,
		DecodeWithMemTracking,
		Clone,
		PartialEq,
		Eq,
		TypeInfo,
		RuntimeDebug,
		MaxEncodedLen,
		Default,
	)]
	pub struct OpaqueEncryptionPubkey(pub [u8; 32]);

	#[derive(
		Encode,
		Decode,
		DecodeWithMemTracking,
		Clone,
		PartialEq,
		Eq,
		TypeInfo,
		RuntimeDebugNoBound,
		MaxEncodedLen,
	)]
	#[scale_info(skip_type_params(T))]
	pub struct RegistrationV1<T: Config> {
		/// Primary operational account that owns the linked accounts.
		pub operational_account: T::AccountId,
		/// Opaque public encryption key for this operational account, currently x25519 bytes.
		pub encryption_pubkey: OpaqueEncryptionPubkey,
		/// Proof that the primary operational account is controlled by the registrant.
		pub operational_account_proof: AccountOwnershipProof,
		/// Vault account associated with this operational account.
		pub vault_account: T::AccountId,
		/// Mining funding account associated with this operational account.
		pub mining_funding_account: T::AccountId,
		/// Bot account used for mining operations.
		pub mining_bot_account: T::AccountId,
		/// Proof that the vault account is controlled by the registrant.
		pub vault_account_proof: AccountOwnershipProof,
		/// Proof that the mining funding account is controlled by the registrant.
		pub mining_funding_account_proof: AccountOwnershipProof,
		/// Proof that the mining bot account is controlled by the registrant.
		pub mining_bot_account_proof: AccountOwnershipProof,
		/// Optional referral proof used to link this registration to a sponsor.
		pub referral_proof: Option<ReferralProof<T::AccountId>>,
	}

	#[derive(
		Encode,
		Decode,
		DecodeWithMemTracking,
		Clone,
		PartialEq,
		Eq,
		TypeInfo,
		RuntimeDebugNoBound,
		MaxEncodedLen,
	)]
	#[scale_info(skip_type_params(T))]
	pub enum Registration<T: Config> {
		V1(RegistrationV1<T>),
	}

	#[derive(
		Encode, Decode, Clone, PartialEq, Eq, TypeInfo, RuntimeDebugNoBound, MaxEncodedLen,
	)]
	#[scale_info(skip_type_params(T))]
	pub struct OperationalAccount<T: Config> {
		/// Vault account associated with this operational account.
		pub vault_account: T::AccountId,
		/// Mining funding account associated with this operational account.
		/// This account is also used as the mining account.
		pub mining_funding_account: T::AccountId,
		/// Bot account used for mining operations.
		pub mining_bot_account: T::AccountId,
		/// Opaque public encryption key for this operational account, currently x25519 bytes.
		pub encryption_pubkey: OpaqueEncryptionPubkey,
		/// Sponsor account, if known.
		pub sponsor: Option<T::AccountId>,
		/// Whether at least one qualifying Uniswap transfer has been observed.
		pub has_uniswap_transfer: bool,
		/// Whether the vault has been created for this operational account.
		pub vault_created: bool,
		/// Bitcoin amount accrued above the bitcoin already applied to referral issuance.
		pub bitcoin_accrual: T::Balance,
		/// Bitcoin already applied to previously issued referrals.
		pub bitcoin_applied_total: T::Balance,
		/// Whether the account has participated in a treasury pool.
		pub has_treasury_pool_participation: bool,
		/// Mining seats accrued since the last mining referral issuance.
		#[codec(compact)]
		pub mining_seat_accrual: u32,
		/// Mining seats already applied to previously issued referrals.
		#[codec(compact)]
		pub mining_seat_applied_total: u32,
		/// Number of sponsored accounts that have become operational.
		#[codec(compact)]
		pub operational_referrals_count: u32,
		/// Whether one earned referral is pending materialization.
		pub referral_pending: bool,
		/// Number of referrals this account can still have redeemed.
		#[codec(compact)]
		pub available_referrals: u32,
		/// Number of rewards earned.
		#[codec(compact)]
		pub rewards_earned_count: u32,
		/// Aggregate amount of rewards earned.
		pub rewards_earned_amount: T::Balance,
		/// Aggregate amount of rewards collected.
		pub rewards_collected_amount: T::Balance,
		/// Whether the account is operational.
		pub is_operational: bool,
	}

	#[derive(
		Decode,
		DecodeWithMemTracking,
		Encode,
		Clone,
		PartialEq,
		Eq,
		TypeInfo,
		RuntimeDebug,
		MaxEncodedLen,
	)]
	pub struct OperationalProgressPatch<Balance: Member + MaxEncodedLen + Default> {
		/// Override for whether at least one qualifying Uniswap transfer has been observed.
		pub has_uniswap_transfer: Option<bool>,
		/// Override for whether the vault has been created.
		pub vault_created: Option<bool>,
		/// Override for whether the account has participated in a treasury pool.
		pub has_treasury_pool_participation: Option<bool>,
		/// Requested minimum for the total observed bitcoin lock value.
		///
		/// This is treated as a monotonic applied-total override: the effective stored
		/// `observed_bitcoin_total` will be at least this value, while preserving the
		/// bitcoin already applied to issued referrals. If the provided value is
		/// lower than the applied total, the current total is retained.
		pub observed_bitcoin_total: Option<Balance>,
		/// Requested minimum for the total observed mining seats won.
		///
		/// This is treated as a monotonic applied-total override: the effective stored
		/// `observed_mining_seat_total` will be at least this value, while preserving
		/// the seats already applied to issued referrals. If the provided value is
		/// lower than the applied total, the current total is retained.
		pub observed_mining_seat_total: Option<u32>,
	}

	impl<Balance: Member + MaxEncodedLen + Default> OperationalProgressPatch<Balance> {
		fn has_updates(&self) -> bool {
			self.has_uniswap_transfer.is_some() ||
				self.vault_created.is_some() ||
				self.has_treasury_pool_participation.is_some() ||
				self.observed_bitcoin_total.is_some() ||
				self.observed_mining_seat_total.is_some()
		}
	}

	#[pallet::storage]
	/// Registered operational accounts keyed by the primary account id.
	pub type OperationalAccounts<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, OperationalAccount<T>, OptionQuery>;

	#[pallet::storage]
	/// Reverse lookup of any linked account to its operational account id.
	pub type OperationalAccountBySubAccount<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, T::AccountId, OptionQuery>;

	#[pallet::storage]
	/// Referral codes that have already been linked, keyed to their proof expiration frame.
	pub type ConsumedReferralCodes<T: Config> =
		StorageMap<_, Blake2_128Concat, sr25519::Public, FrameId, OptionQuery>;

	#[pallet::storage]
	/// Referral codes to clear after their referral proof expiration frame.
	pub type ConsumedReferralCodesByExpiration<T: Config> = StorageDoubleMap<
		_,
		Twox64Concat,
		FrameId,
		Blake2_128Concat,
		sr25519::Public,
		(),
		OptionQuery,
	>;

	#[pallet::storage]
	/// Oldest referral expiration frame that still has cleanup work to resume.
	pub type ExpiredReferralCodeCleanupFrame<T: Config> = StorageValue<_, FrameId, OptionQuery>;

	#[derive(
		Encode,
		Decode,
		Clone,
		PartialEq,
		Eq,
		RuntimeDebug,
		TypeInfo,
		MaxEncodedLen,
		Default,
		serde::Deserialize,
		serde::Serialize,
	)]
	pub struct RewardsConfig<Balance: Member + MaxEncodedLen + Default> {
		/// Reward paid when an account becomes operational.
		#[codec(compact)]
		pub operational_referral_reward: Balance,
		/// Bonus reward paid for every follow-on threshold met.
		#[codec(compact)]
		pub referral_bonus_reward: Balance,
	}

	#[pallet::genesis_config]
	#[derive(frame_support::DefaultNoBound)]
	pub struct GenesisConfig<T: Config> {
		#[serde(skip)]
		pub _phantom: PhantomData<T>,
	}

	#[pallet::genesis_build]
	impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
		fn build(&self) {
			Rewards::<T>::put(RewardsConfig {
				operational_referral_reward: T::OperationalReferralReward::get(),
				referral_bonus_reward: T::OperationalReferralBonusReward::get(),
			});
		}
	}

	#[pallet::storage]
	/// Configured reward amounts for operational accounts.
	pub type Rewards<T: Config> = StorageValue<_, RewardsConfig<T::Balance>, ValueQuery>;

	#[pallet::storage]
	/// Opaque encrypted sponsor server payload keyed by the sponsee operational account.
	pub type EncryptedServerBySponsee<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		T::AccountId,
		BoundedVec<u8, T::MaxEncryptedServerLen>,
		OptionQuery,
	>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// An operational account was registered with its linked accounts.
		OperationalAccountRegistered {
			operational_account: T::AccountId,
			vault_account: T::AccountId,
			mining_funding_account: T::AccountId,
			mining_bot_account: T::AccountId,
			sponsor: Option<T::AccountId>,
		},
		/// Account has become operational.
		AccountWentOperational { account: T::AccountId },
		/// A reward is earned for an operational account, but not yet claimed.
		OperationalRewardEarned {
			account: T::AccountId,
			reward_kind: OperationalRewardKind,
			amount: T::Balance,
		},
		/// Claimable operational rewards were paid to a managed account.
		OperationalRewardsClaimed {
			operational_account: T::AccountId,
			claimant: T::AccountId,
			amount: T::Balance,
			remaining_pending: T::Balance,
		},
		/// Reward config values were updated.
		RewardsConfigUpdated {
			operational_referral_reward: T::Balance,
			referral_bonus_reward: T::Balance,
		},
		/// Operational progress was forced by root.
		OperationalProgressForced {
			account: T::AccountId,
			update_operational_progress: bool,
			has_uniswap_transfer: bool,
			vault_created: bool,
			has_treasury_pool_participation: bool,
			observed_bitcoin_total: T::Balance,
			observed_mining_seat_total: u32,
		},
		/// A sponsor updated the encrypted server payload for a sponsee.
		EncryptedServerUpdated { sponsor: T::AccountId, sponsee: T::AccountId },
	}

	#[pallet::error]
	pub enum Error<T> {
		/// The caller already registered an operational account.
		AlreadyRegistered,
		/// The caller is not one of the accounts included in the registration.
		InvalidRegistrationSubmitter,
		/// One of the provided accounts is already linked to an operational account.
		AccountAlreadyLinked,
		/// One of the linked account ownership proofs is invalid.
		InvalidAccountProof,
		/// The caller has not registered an operational account.
		NotOperationalAccount,
		/// The referral proof or sponsor proof is invalid.
		InvalidReferralProof,
		/// The referral proof has expired.
		ReferralProofExpired,
		/// The requested progress patch does not contain any updates.
		NoProgressUpdateProvided,
		/// The encrypted server payload exceeds the configured max length.
		EncryptedServerTooLong,
		/// The caller is not the sponsor of the requested sponsee.
		NotSponsorOfSponsee,
		/// The operational account has no pending rewards to claim.
		NoPendingRewards,
		/// Reward claims must be at least one Argon.
		RewardClaimBelowMinimum,
		/// Reward claims must be whole Argon increments.
		RewardClaimNotWholeArgon,
		/// The requested reward claim exceeds pending rewards.
		RewardClaimExceedsPending,
		/// The treasury does not currently have enough available reserves for the claim.
		TreasuryInsufficientFunds,
		/// The account is already operational.
		AlreadyOperational,
		/// The account has not satisfied operational requirements yet.
		NotEligibleForActivation,
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(_n: BlockNumberFor<T>) -> Weight {
			let current_frame = T::FrameProvider::get_current_frame_id();
			let cleanup_limit = T::MaxExpiredReferralCodeCleanupsPerBlock::get() as usize;
			let mut cleaned = 0u64;
			let mut reads = 2u64;

			let start_frame = ExpiredReferralCodeCleanupFrame::<T>::get().unwrap_or(current_frame);

			for frame in start_frame..=current_frame {
				let remaining = cleanup_limit.saturating_sub(cleaned as usize);
				reads.saturating_accrue(1);

				let referral_codes = ConsumedReferralCodesByExpiration::<T>::iter_key_prefix(frame)
					.take(remaining.saturating_add(1))
					.collect::<Vec<_>>();

				reads.saturating_accrue(referral_codes.len() as u64);

				for referral_code in referral_codes.iter().take(remaining) {
					ConsumedReferralCodes::<T>::remove(referral_code);
					ConsumedReferralCodesByExpiration::<T>::remove(frame, referral_code);
					cleaned.saturating_accrue(1);
				}

				if referral_codes.len() > remaining {
					ExpiredReferralCodeCleanupFrame::<T>::put(frame);
					return T::DbWeight::get()
						.reads_writes(reads, cleaned.saturating_mul(2).saturating_add(1));
				}
			}

			ExpiredReferralCodeCleanupFrame::<T>::kill();
			T::DbWeight::get().reads_writes(reads, cleaned.saturating_mul(2).saturating_add(1))
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Register vault, mining funding, and bot accounts for an operational account.
		/// Any account in the registration may submit the transaction.
		/// If a referral proof is provided, the registration records the sponsor relationship.
		#[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::register())]
		pub fn register(origin: OriginFor<T>, registration: Registration<T>) -> DispatchResult {
			let Registration::V1(RegistrationV1 {
				operational_account,
				encryption_pubkey,
				operational_account_proof,
				vault_account,
				mining_funding_account,
				mining_bot_account,
				vault_account_proof,
				mining_funding_account_proof,
				mining_bot_account_proof,
				referral_proof,
			}) = registration;
			let submitter = ensure_signed(origin)?;
			ensure!(
				submitter == operational_account ||
					submitter == vault_account ||
					submitter == mining_funding_account ||
					submitter == mining_bot_account,
				Error::<T>::InvalidRegistrationSubmitter
			);
			ensure!(
				!OperationalAccounts::<T>::contains_key(&operational_account),
				Error::<T>::AlreadyRegistered
			);
			ensure!(
				!OperationalAccountBySubAccount::<T>::contains_key(&operational_account),
				Error::<T>::AccountAlreadyLinked
			);
			ensure!(
				!OperationalAccountBySubAccount::<T>::contains_key(&vault_account),
				Error::<T>::AccountAlreadyLinked
			);
			ensure!(
				!OperationalAccountBySubAccount::<T>::contains_key(&mining_funding_account),
				Error::<T>::AccountAlreadyLinked
			);
			ensure!(
				!OperationalAccountBySubAccount::<T>::contains_key(&mining_bot_account),
				Error::<T>::AccountAlreadyLinked
			);
			ensure!(
				operational_account_proof.verify(
					&operational_account,
					&operational_account,
					OPERATIONAL_ACCOUNT_PROOF_MESSAGE_KEY
				) && vault_account_proof.verify(
					&operational_account,
					&vault_account,
					VAULT_ACCOUNT_PROOF_MESSAGE_KEY
				) && mining_funding_account_proof.verify(
					&operational_account,
					&mining_funding_account,
					MINING_FUNDING_ACCOUNT_PROOF_MESSAGE_KEY,
				) && mining_bot_account_proof.verify(
					&operational_account,
					&mining_bot_account,
					MINING_BOT_ACCOUNT_PROOF_MESSAGE_KEY,
				),
				Error::<T>::InvalidAccountProof
			);

			let current_frame = T::FrameProvider::get_current_frame_id();
			let sponsor = if let Some(referral_proof) = referral_proof.as_ref() {
				ensure!(
					current_frame < referral_proof.expires_at_frame,
					Error::<T>::ReferralProofExpired
				);
				ensure!(
					referral_proof.verify_claim(&operational_account) &&
						referral_proof.verify_sponsor(),
					Error::<T>::InvalidReferralProof
				);
				let referral_code = referral_proof.referral_code;
				OperationalAccounts::<T>::try_mutate(
					&referral_proof.sponsor,
					|maybe_account| -> Result<Option<T::AccountId>, Error<T>> {
						let Some(sponsor_account) = maybe_account else {
							return Ok(None);
						};
						if sponsor_account.available_referrals == 0 {
							return Ok(None);
						}
						if ConsumedReferralCodes::<T>::contains_key(referral_code) {
							return Ok(None);
						}
						sponsor_account.available_referrals.saturating_reduce(1);
						Self::materialize_available_referrals(sponsor_account);
						ConsumedReferralCodes::<T>::insert(
							referral_code,
							referral_proof.expires_at_frame,
						);
						ConsumedReferralCodesByExpiration::<T>::insert(
							referral_proof.expires_at_frame,
							referral_code,
							(),
						);
						Ok(Some(referral_proof.sponsor.clone()))
					},
				)?
			} else {
				None
			};
			let vault_registration = T::VaultProvider::get_registration_vault_data(&vault_account);
			let has_treasury_pool_participation = vault_registration
				.as_ref()
				.map(|vault| {
					T::TreasuryPoolProvider::has_bond_participation(vault.vault_id, &vault_account)
				})
				.unwrap_or(false);
			let observed_mining_seat_total = u32::from(
				T::MiningSlotProvider::has_active_rewards_account_seat(&mining_funding_account),
			);
			let has_uniswap_transfer =
				!T::UniswapTransferRequirementProvider::requires_uniswap_transfer() ||
					[
						&operational_account,
						&vault_account,
						&mining_funding_account,
						&mining_bot_account,
					]
					.iter()
					.copied()
					.any(|account_id| {
						T::RecentArgonTransferLookup::has_recent_argon_transfer(account_id)
					});

			OperationalAccounts::<T>::insert(
				&operational_account,
				OperationalAccount {
					vault_account: vault_account.clone(),
					mining_funding_account: mining_funding_account.clone(),
					mining_bot_account: mining_bot_account.clone(),
					encryption_pubkey,
					sponsor: sponsor.clone(),
					has_uniswap_transfer,
					vault_created: vault_registration.is_some(),
					// Bootstrap lookup seeds current observed totals as live accrual so
					// registration matches the normal hook-driven activation path.
					bitcoin_accrual: vault_registration
						.map(|vault| vault.activated_securitization)
						.unwrap_or_else(Zero::zero),
					bitcoin_applied_total: T::Balance::zero(),
					has_treasury_pool_participation,
					mining_seat_accrual: observed_mining_seat_total,
					mining_seat_applied_total: 0,
					operational_referrals_count: 0,
					referral_pending: false,
					available_referrals: 0,
					rewards_earned_count: 0,
					rewards_earned_amount: T::Balance::zero(),
					rewards_collected_amount: T::Balance::zero(),
					is_operational: false,
				},
			);

			OperationalAccountBySubAccount::<T>::insert(&vault_account, &operational_account);
			OperationalAccountBySubAccount::<T>::insert(
				&mining_funding_account,
				&operational_account,
			);
			OperationalAccountBySubAccount::<T>::insert(&mining_bot_account, &operational_account);

			Self::deposit_event(Event::OperationalAccountRegistered {
				operational_account: operational_account.clone(),
				vault_account: vault_account.clone(),
				mining_funding_account: mining_funding_account.clone(),
				mining_bot_account: mining_bot_account.clone(),
				sponsor: sponsor.clone(),
			});
			Ok(())
		}

		/// Update reward amounts for operational accounts.
		#[pallet::call_index(2)]
		#[pallet::weight(T::WeightInfo::set_reward_config())]
		pub fn set_reward_config(
			origin: OriginFor<T>,
			operational_referral_reward: T::Balance,
			referral_bonus_reward: T::Balance,
		) -> DispatchResult {
			ensure_root(origin)?;
			Rewards::<T>::put(RewardsConfig { operational_referral_reward, referral_bonus_reward });
			Self::deposit_event(Event::RewardsConfigUpdated {
				operational_referral_reward,
				referral_bonus_reward,
			});
			Ok(())
		}

		/// Force-update operational progress markers for an account.
		#[pallet::call_index(3)]
		#[pallet::weight(T::WeightInfo::force_set_progress())]
		pub fn force_set_progress(
			origin: OriginFor<T>,
			owner: T::AccountId,
			patch: OperationalProgressPatch<T::Balance>,
			update_operational_progress: bool,
		) -> DispatchResult {
			ensure_root(origin)?;
			ensure!(patch.has_updates(), Error::<T>::NoProgressUpdateProvided);

			let mut has_uniswap_transfer = false;
			let mut vault_created = false;
			let mut has_treasury_pool_participation = false;
			let mut observed_bitcoin_total = T::Balance::zero();
			let mut observed_mining_seat_total = 0u32;

			OperationalAccounts::<T>::try_mutate(
				&owner,
				|maybe_account| -> Result<(), Error<T>> {
					let account =
						maybe_account.as_mut().ok_or(Error::<T>::NotOperationalAccount)?;

					if let Some(value) = patch.has_uniswap_transfer {
						account.has_uniswap_transfer = value;
					}
					if let Some(value) = patch.vault_created {
						account.vault_created = value;
					}
					if let Some(value) = patch.has_treasury_pool_participation {
						account.has_treasury_pool_participation = value;
					}
					if let Some(value) = patch.observed_bitcoin_total {
						Self::recalculate_bitcoin_accrual(account, value);
					}
					if let Some(value) = patch.observed_mining_seat_total {
						Self::set_observed_mining_seat_total(account, value);
					}

					if update_operational_progress {
						Self::materialize_available_referrals(account);
					}

					has_uniswap_transfer = account.has_uniswap_transfer;
					vault_created = account.vault_created;
					has_treasury_pool_participation = account.has_treasury_pool_participation;
					observed_bitcoin_total = Self::observed_bitcoin_total(account);
					observed_mining_seat_total = Self::observed_mining_seat_total(account);
					Ok(())
				},
			)?;

			Self::deposit_event(Event::OperationalProgressForced {
				account: owner,
				update_operational_progress,
				has_uniswap_transfer,
				vault_created,
				has_treasury_pool_participation,
				observed_bitcoin_total,
				observed_mining_seat_total,
			});
			Ok(())
		}

		/// Store an opaque encrypted sponsor server payload for a sponsored operational account.
		#[pallet::call_index(4)]
		#[pallet::weight(T::WeightInfo::set_encrypted_server_for_sponsee())]
		pub fn set_encrypted_server_for_sponsee(
			origin: OriginFor<T>,
			sponsee: T::AccountId,
			encrypted_server: Vec<u8>,
		) -> DispatchResult {
			let sponsor = ensure_signed(origin)?;
			ensure!(
				OperationalAccounts::<T>::contains_key(&sponsor),
				Error::<T>::NotOperationalAccount
			);
			let sponsee_account =
				OperationalAccounts::<T>::get(&sponsee).ok_or(Error::<T>::NotOperationalAccount)?;
			ensure!(
				sponsee_account.sponsor == Some(sponsor.clone()),
				Error::<T>::NotSponsorOfSponsee
			);

			let encrypted_server: BoundedVec<u8, T::MaxEncryptedServerLen> =
				encrypted_server.try_into().map_err(|_| Error::<T>::EncryptedServerTooLong)?;
			EncryptedServerBySponsee::<T>::insert(&sponsee, encrypted_server);
			Self::deposit_event(Event::EncryptedServerUpdated { sponsor, sponsee });
			Ok(())
		}

		/// Activate an eligible operational account from any managed account.
		#[pallet::call_index(5)]
		#[pallet::weight(T::WeightInfo::activate())]
		pub fn activate(origin: OriginFor<T>) -> DispatchResult {
			let signer = ensure_signed(origin)?;
			let owner = Self::operational_owner_for(&signer);

			OperationalAccounts::<T>::try_mutate(&owner, |maybe_account| -> DispatchResult {
				let account = maybe_account.as_mut().ok_or(Error::<T>::NotOperationalAccount)?;
				ensure!(!account.is_operational, Error::<T>::AlreadyOperational);
				ensure!(
					Self::has_achieved_operational(account),
					Error::<T>::NotEligibleForActivation
				);

				let reward_config = Rewards::<T>::get();
				account.is_operational = true;
				Self::increment_available_referrals(account);
				Self::materialize_available_referrals(account);
				T::VaultProvider::account_became_operational(&account.vault_account);
				Self::deposit_event(Event::AccountWentOperational { account: owner.clone() });
				Self::record_reward(
					account,
					&owner,
					OperationalRewardKind::Activation,
					reward_config.operational_referral_reward,
				);

				if let Some(sponsor) = account.sponsor.as_ref() {
					OperationalAccounts::<T>::mutate(sponsor, |maybe_account| {
						let Some(sponsor_account) = maybe_account else {
							return;
						};
						if !sponsor_account.is_operational {
							return;
						}
						sponsor_account.referral_pending = true;
						Self::materialize_available_referrals(sponsor_account);
						sponsor_account.operational_referrals_count.saturating_accrue(1);
						Self::record_reward(
							sponsor_account,
							sponsor,
							OperationalRewardKind::Activation,
							reward_config.operational_referral_reward,
						);
						let bonus_every = T::ReferralBonusEveryXOperationalSponsees::get();
						if bonus_every > 0 &&
							sponsor_account.operational_referrals_count % bonus_every == 0
						{
							Self::record_reward(
								sponsor_account,
								sponsor,
								OperationalRewardKind::ReferralBonus,
								reward_config.referral_bonus_reward,
							);
						}
					});
				}

				Ok(())
			})
		}

		/// Claim pending operational rewards to any managed account.
		#[pallet::call_index(6)]
		#[pallet::weight(T::WeightInfo::claim_rewards())]
		pub fn claim_rewards(origin: OriginFor<T>, amount: T::Balance) -> DispatchResult {
			let claimant = ensure_signed(origin)?;
			let owner = Self::operational_owner_for(&claimant);
			let claim_increment = T::Balance::from(MICROGONS_PER_ARGON);
			let amount_u128: u128 = amount.into();

			ensure!(amount >= claim_increment, Error::<T>::RewardClaimBelowMinimum);
			ensure!(amount_u128 % MICROGONS_PER_ARGON == 0, Error::<T>::RewardClaimNotWholeArgon);

			OperationalAccounts::<T>::try_mutate(&owner, |maybe_account| -> DispatchResult {
				let account = maybe_account.as_mut().ok_or(Error::<T>::NotOperationalAccount)?;
				let pending_rewards =
					account.rewards_earned_amount.saturating_sub(account.rewards_collected_amount);
				ensure!(!pending_rewards.is_zero(), Error::<T>::NoPendingRewards);
				ensure!(amount <= pending_rewards, Error::<T>::RewardClaimExceedsPending);

				T::OperationalRewardsPayer::claim_reward(&claimant, amount)
					.map_err(|_| Error::<T>::TreasuryInsufficientFunds)?;

				account.rewards_collected_amount.saturating_accrue(amount);
				let remaining_pending =
					account.rewards_earned_amount.saturating_sub(account.rewards_collected_amount);

				Self::deposit_event(Event::OperationalRewardsClaimed {
					operational_account: owner.clone(),
					claimant,
					amount,
					remaining_pending,
				});
				Ok(())
			})
		}
	}

	impl<T: Config> Pallet<T> {
		/// Record a confirmed Uniswap transfer to a linked vault account.
		pub fn on_uniswap_transfer(account_id: &T::AccountId, _amount: T::Balance) {
			let Some(owner) = OperationalAccountBySubAccount::<T>::get(account_id) else {
				return;
			};

			OperationalAccounts::<T>::mutate(&owner, |maybe_account| {
				let Some(account) = maybe_account else {
					return;
				};

				account.has_uniswap_transfer = true;
			});
		}

		fn operational_owner_for(account_id: &T::AccountId) -> T::AccountId {
			OperationalAccountBySubAccount::<T>::get(account_id)
				.unwrap_or_else(|| account_id.clone())
		}

		fn record_reward(
			operational_account: &mut OperationalAccount<T>,
			owner: &T::AccountId,
			reward_kind: OperationalRewardKind,
			amount: T::Balance,
		) {
			if amount.is_zero() {
				return;
			}

			operational_account.rewards_earned_count.saturating_accrue(1);
			operational_account.rewards_earned_amount.saturating_accrue(amount);
			Self::deposit_event(Event::OperationalRewardEarned {
				account: owner.clone(),
				reward_kind,
				amount,
			});
		}

		fn has_achieved_operational(operational_account: &OperationalAccount<T>) -> bool {
			if !operational_account.vault_created ||
				!operational_account.has_uniswap_transfer ||
				Self::observed_bitcoin_total(operational_account).is_zero() ||
				Self::observed_mining_seat_total(operational_account) <
					T::MiningSeatsForOperational::get() ||
				!operational_account.has_treasury_pool_participation
			{
				return false;
			}

			T::VaultProvider::get_registration_vault_data(&operational_account.vault_account)
				.map(|vault| {
					vault.securitization >= T::OperationalMinimumVaultSecuritization::get()
				})
				.unwrap_or(false)
		}

		fn increment_available_referrals(account: &mut OperationalAccount<T>) {
			if account.available_referrals < T::MaxAvailableReferrals::get() {
				account.available_referrals.saturating_accrue(1);
			}
		}

		fn materialize_available_referrals(account: &mut OperationalAccount<T>) {
			if !account.is_operational {
				return;
			}
			let max_available_referrals = T::MaxAvailableReferrals::get();
			let bitcoin_threshold = T::BitcoinLockSizeForReferral::get();
			let mining_seat_threshold = T::MiningSeatsPerReferral::get();
			while account.available_referrals < max_available_referrals {
				if account.referral_pending {
					account.referral_pending = false;
					account.available_referrals.saturating_accrue(1);
					continue;
				}
				if bitcoin_threshold > T::Balance::zero() &&
					account.bitcoin_accrual >= bitcoin_threshold
				{
					account.bitcoin_applied_total = Self::observed_bitcoin_total(account);
					account.bitcoin_accrual = T::Balance::zero();
					account.available_referrals.saturating_accrue(1);
					continue;
				}
				if mining_seat_threshold > 0 && account.mining_seat_accrual >= mining_seat_threshold
				{
					account.mining_seat_applied_total = Self::observed_mining_seat_total(account);
					account.mining_seat_accrual = 0;
					account.available_referrals.saturating_accrue(1);
					continue;
				}
				break;
			}
		}

		fn observed_bitcoin_total(account: &OperationalAccount<T>) -> T::Balance {
			account.bitcoin_applied_total.saturating_add(account.bitcoin_accrual)
		}

		fn observed_mining_seat_total(account: &OperationalAccount<T>) -> u32 {
			account.mining_seat_applied_total.saturating_add(account.mining_seat_accrual)
		}

		fn set_observed_mining_seat_total(
			account: &mut OperationalAccount<T>,
			observed_total: u32,
		) {
			let prior_applied_total = account.mining_seat_applied_total;
			let next_accrual = observed_total.saturating_sub(prior_applied_total);
			account.mining_seat_accrual = next_accrual;
			account.mining_seat_applied_total = prior_applied_total;
		}

		fn recalculate_bitcoin_accrual(
			account: &mut OperationalAccount<T>,
			total_locked: T::Balance,
		) {
			account.bitcoin_accrual = total_locked.saturating_sub(account.bitcoin_applied_total);
		}
	}

	impl<T: Config> OperationalAccountsHook<T::AccountId, T::Balance> for Pallet<T> {
		fn vault_created_weight() -> Weight {
			<T as Config>::WeightInfo::on_vault_created()
		}

		fn vault_created(account_id: &T::AccountId) {
			let Some(owner) = OperationalAccountBySubAccount::<T>::get(account_id) else {
				return;
			};

			OperationalAccounts::<T>::mutate(&owner, |maybe_account| {
				let Some(account) = maybe_account else {
					return;
				};

				account.vault_created = true;
			});
		}

		fn bitcoin_lock_funded_weight() -> Weight {
			<T as Config>::WeightInfo::on_bitcoin_lock_funded()
		}

		fn bitcoin_lock_funded(vault_operator_account: &T::AccountId, total_locked: T::Balance) {
			let Some(owner) = OperationalAccountBySubAccount::<T>::get(vault_operator_account)
			else {
				return;
			};

			OperationalAccounts::<T>::mutate(&owner, |maybe_account| {
				let Some(account) = maybe_account else {
					return;
				};
				Self::recalculate_bitcoin_accrual(account, total_locked);
				Self::materialize_available_referrals(account);
			});
		}

		fn mining_seat_won_weight() -> Weight {
			<T as Config>::WeightInfo::on_mining_seat_won()
		}

		fn mining_seat_won(miner_account: &T::AccountId) {
			let Some(owner) = OperationalAccountBySubAccount::<T>::get(miner_account) else {
				return;
			};
			OperationalAccounts::<T>::mutate(&owner, |maybe_account| {
				let Some(account) = maybe_account else {
					return;
				};
				account.mining_seat_accrual.saturating_accrue(1);
				Self::materialize_available_referrals(account);
			});
		}

		fn treasury_pool_participated_weight() -> Weight {
			<T as Config>::WeightInfo::on_treasury_pool_participated()
		}

		fn treasury_pool_participated(vault_operator_account: &T::AccountId, _amount: T::Balance) {
			let Some(owner) = OperationalAccountBySubAccount::<T>::get(vault_operator_account)
			else {
				return;
			};
			OperationalAccounts::<T>::mutate(&owner, |maybe_account| {
				let Some(account) = maybe_account else {
					return;
				};
				account.has_treasury_pool_participation = true;
			});
		}

		fn uniswap_transfer_confirmed_weight() -> Weight {
			<T as Config>::WeightInfo::on_uniswap_transfer()
		}

		fn uniswap_transfer_confirmed(account_id: &T::AccountId, amount: T::Balance) {
			Self::on_uniswap_transfer(account_id, amount)
		}
	}
}
