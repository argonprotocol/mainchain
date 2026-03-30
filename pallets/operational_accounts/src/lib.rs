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
mod weights;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use alloc::vec::Vec;
	use argon_primitives::{
		MiningFrameTransitionProvider, MiningSlotProvider, OperationalAccountsHook,
		OperationalRewardKind, OperationalRewardPayout, OperationalRewardsPayer,
		OperationalRewardsProvider, RecentArgonTransferLookup, Signature, TreasuryPoolProvider,
		vault::BitcoinVaultProvider,
	};
	use codec::{Decode, Encode, EncodeLike};
	use pallet_prelude::*;
	use polkadot_sdk::frame_system::ensure_root;
	use sp_core::sr25519;
	use sp_runtime::{
		AccountId32,
		traits::{Verify, Zero},
	};

	/// Domain separator for access code activation proofs.
	pub const ACCESS_CODE_PROOF_MESSAGE_KEY: &[u8; 17] = b"access_code_claim";
	pub const OPERATIONAL_ACCOUNT_PROOF_MESSAGE_KEY: &[u8; 27] = b"operational_primary_account";
	pub const VAULT_ACCOUNT_PROOF_MESSAGE_KEY: &[u8; 25] = b"operational_vault_account";
	pub const MINING_FUNDING_ACCOUNT_PROOF_MESSAGE_KEY: &[u8; 26] = b"operational_mining_funding";
	pub const MINING_BOT_ACCOUNT_PROOF_MESSAGE_KEY: &[u8; 22] = b"operational_mining_bot";

	#[pallet::pallet]
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

		/// How many frames an access code remains valid.
		#[pallet::constant]
		type AccessCodeExpirationFrames: Get<FrameId>;
		/// Maximum number of access codes that may expire in the same frame.
		#[pallet::constant]
		type MaxAccessCodesExpiringPerFrame: Get<u32>;

		/// Additional argon amount (base units) required per access code after operational.
		#[pallet::constant]
		type BitcoinLockSizeForAccessCode: Get<Self::Balance>;
		/// Default reward paid when an account becomes operational.
		#[pallet::constant]
		type OperationalReferralReward: Get<Self::Balance>;
		/// Default bonus reward paid every referral threshold.
		#[pallet::constant]
		type OperationalReferralBonusReward: Get<Self::Balance>;
		/// Number of operational sponsees required per referral bonus reward.
		#[pallet::constant]
		type ReferralBonusEveryXOperationalSponsees: Get<u32>;
		/// Maximum number of issuable access codes allowed at once.
		#[pallet::constant]
		type MaxIssuableAccessCodes: Get<u32>;
		/// Maximum number of queued operational rewards.
		#[pallet::constant]
		type MaxOperationalRewardsQueued: Get<u32>;
		/// Maximum number of unactivated (issued but unused) access codes allowed at once.
		#[pallet::constant]
		type MaxUnactivatedAccessCodes: Get<u32>;
		/// Maximum number of encrypted server bytes stored per sponsee.
		#[pallet::constant]
		type MaxEncryptedServerLen: Get<u32>;
		/// Minimum argon amount (base units) required to mark a bitcoin lock as qualifying.
		#[pallet::constant]
		type MinBitcoinLockSizeForOperational: Get<Self::Balance>;
		/// Mining seats required to become operational.
		#[pallet::constant]
		type MiningSeatsForOperational: Get<u32>;
		/// Mining seats required per access code after operational.
		#[pallet::constant]
		type MiningSeatsPerAccessCode: Get<u32>;

		/// Provider for current vault state used to initialize registration.
		type VaultProvider: BitcoinVaultProvider<AccountId = Self::AccountId, Balance = Self::Balance>;
		/// Provider for whether a linked mining rewards account currently has an active seat.
		type MiningSlotProvider: MiningSlotProvider<Self::AccountId>;
		/// Provider for whether a linked vault account currently has treasury pool participation.
		type TreasuryPoolProvider: TreasuryPoolProvider<Self::AccountId>;
		/// Provider for recent qualifying inbound Argon transfer lookup.
		type RecentArgonTransferLookup: RecentArgonTransferLookup<Self::AccountId>;
		/// Reserved payout adapter for runtime compatibility (rewards are settled on frame
		/// transition via treasury queue processing).
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
	pub struct AccessCodeProof {
		/// The public key that serves as the access code.
		pub public: sr25519::Public,
		/// Signature over the activation message.
		pub signature: sr25519::Signature,
	}

	impl AccessCodeProof {
		pub fn verify<AccountId: EncodeLike>(&self, account_id: &AccountId) -> bool {
			let message =
				(ACCESS_CODE_PROOF_MESSAGE_KEY, self.public, account_id).using_encoded(blake2_256);
			let verified = self.signature.verify(message.as_slice(), &self.public);
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
		/// Optional sponsor access code used to link this registration to a sponsor.
		pub access_code: Option<AccessCodeProof>,
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
		/// Bitcoin amount accrued above the bitcoin already applied to access code issuance.
		pub bitcoin_accrual: T::Balance,
		/// Bitcoin already applied to previously issued bitcoin access codes.
		pub bitcoin_applied_total: T::Balance,
		/// Whether the account has participated in a treasury pool.
		pub has_treasury_pool_participation: bool,
		/// Mining seats accrued since the last mining access code issuance.
		#[codec(compact)]
		pub mining_seat_accrual: u32,
		/// Mining seats already applied to previously issued mining access codes.
		#[codec(compact)]
		pub mining_seat_applied_total: u32,
		/// Number of sponsored accounts that have become operational.
		#[codec(compact)]
		pub operational_referrals_count: u32,
		/// Whether one referral-earned access code is pending materialization.
		pub referral_access_code_pending: bool,
		/// Number of access codes this account can issue right now.
		#[codec(compact)]
		pub issuable_access_codes: u32,
		/// Number of issued access codes that have not yet been activated.
		#[codec(compact)]
		pub unactivated_access_codes: u32,
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
		/// bitcoin already applied to issued access codes. If the provided value is
		/// lower than the applied total, the current total is retained.
		pub observed_bitcoin_total: Option<Balance>,
		/// Requested minimum for the total observed mining seats won.
		///
		/// This is treated as a monotonic applied-total override: the effective stored
		/// `observed_mining_seat_total` will be at least this value, while preserving
		/// the seats already applied to issued access codes. If the provided value is
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
	/// Registered access codes keyed by their public key.
	pub type AccessCodesByPublic<T: Config> =
		StorageMap<_, Blake2_128Concat, sr25519::Public, AccessCodeMetadata<T>, OptionQuery>;

	#[pallet::storage]
	/// Registered access codes expiring at a given mining frame.
	pub type AccessCodesExpiringByFrame<T: Config> = StorageMap<
		_,
		Twox64Concat,
		FrameId,
		BoundedVec<sr25519::Public, T::MaxAccessCodesExpiringPerFrame>,
		ValueQuery,
	>;

	#[derive(
		Encode, Decode, Clone, PartialEq, Eq, TypeInfo, RuntimeDebugNoBound, MaxEncodedLen,
	)]
	#[scale_info(skip_type_params(T))]
	pub struct AccessCodeMetadata<T: Config> {
		/// Operational account that sponsored this access code.
		pub sponsor: T::AccountId,
		/// Expiration frame for this access code.
		#[codec(compact)]
		pub expiration_frame: FrameId,
	}

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

	#[pallet::storage]
	/// Configured reward amounts for operational accounts.
	pub type Rewards<T: Config> = StorageValue<_, RewardsConfig<T::Balance>, ValueQuery>;

	#[pallet::storage]
	/// Pending operational account rewards waiting on treasury payout (FIFO queue).
	pub type OperationalRewardsQueue<T: Config> = StorageValue<
		_,
		BoundedVec<
			OperationalRewardPayout<T::AccountId, T::Balance>,
			T::MaxOperationalRewardsQueued,
		>,
		ValueQuery,
	>;

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
		/// A reward has been queued for treasury payout.
		OperationalRewardEarned {
			account: T::AccountId,
			reward_kind: OperationalRewardKind,
			amount: T::Balance,
		},
		/// Reward enqueue failed because the pending queue is full.
		OperationalRewardEnqueueFailed {
			account: T::AccountId,
			reward_kind: OperationalRewardKind,
			amount: T::Balance,
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
		/// The access code is already registered.
		AccessCodeAlreadyRegistered,
		/// The access code provided is not registered.
		InvalidAccessCode,
		/// The access code activation proof is invalid.
		InvalidAccessCodeProof,
		/// No access codes are currently issuable.
		NoIssuableAccessCodes,
		/// Too many unactivated access codes are outstanding.
		MaxUnactivatedAccessCodesReached,
		/// Too many access codes are already scheduled to expire in this frame.
		MaxAccessCodesExpiringPerFrameReached,
		/// The requested progress patch does not contain any updates.
		NoProgressUpdateProvided,
		/// The encrypted server payload exceeds the configured max length.
		EncryptedServerTooLong,
		/// The caller is not the sponsor of the requested sponsee.
		NotSponsorOfSponsee,
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_runtime_upgrade() -> Weight {
			if Rewards::<T>::exists() {
				return T::DbWeight::get().reads(1);
			}
			Rewards::<T>::put(RewardsConfig {
				operational_referral_reward: T::OperationalReferralReward::get(),
				referral_bonus_reward: T::OperationalReferralBonusReward::get(),
			});
			T::DbWeight::get().reads_writes(1, 1)
		}

		fn on_initialize(_n: BlockNumberFor<T>) -> Weight {
			let current_frame = T::FrameProvider::get_current_frame_id();
			let expiring_codes = AccessCodesExpiringByFrame::<T>::take(current_frame);
			let processed = expiring_codes.len() as u64;
			for public in expiring_codes {
				if let Some(access_code_metadata) = AccessCodesByPublic::<T>::take(public) {
					OperationalAccounts::<T>::mutate(
						&access_code_metadata.sponsor,
						|maybe_account| {
							if let Some(account) = maybe_account {
								Self::decrement_unactivated_access_codes(account);
								Self::increment_issuable_access_codes(account);
							}
						},
					);
				}
			}
			T::DbWeight::get()
				.reads_writes(1, 1)
				.saturating_add(T::DbWeight::get().reads_writes(2, 2).saturating_mul(processed))
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Register vault, mining funding, and bot accounts for an operational account.
		/// Any account in the registration may submit the transaction.
		/// If an access code is provided, the registration records the sponsor relationship.
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
				access_code,
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

			let sponsor = if let Some(access_code) = access_code.as_ref() {
				let code = AccessCodesByPublic::<T>::take(access_code.public)
					.ok_or(Error::<T>::InvalidAccessCode)?;
				ensure!(
					access_code.verify(&operational_account),
					Error::<T>::InvalidAccessCodeProof
				);
				AccessCodesExpiringByFrame::<T>::mutate(code.expiration_frame, |expiring_codes| {
					if let Some(index) =
						expiring_codes.iter().position(|public| *public == access_code.public)
					{
						expiring_codes.remove(index);
					}
				});
				Some(code.sponsor)
			} else {
				None
			};
			let vault_registration = T::VaultProvider::get_registration_vault_data(&vault_account);
			let has_treasury_pool_participation = vault_registration
				.as_ref()
				.map(|vault| {
					T::TreasuryPoolProvider::has_pool_participation(vault.vault_id, &vault_account)
				})
				.unwrap_or(false);
			let observed_mining_seat_total = u32::from(
				T::MiningSlotProvider::has_active_rewards_account_seat(&mining_funding_account),
			);
			let has_uniswap_transfer =
				T::RecentArgonTransferLookup::has_recent_argon_transfer(&operational_account) ||
					T::RecentArgonTransferLookup::has_recent_argon_transfer(&vault_account) ||
					T::RecentArgonTransferLookup::has_recent_argon_transfer(
						&mining_funding_account,
					) || T::RecentArgonTransferLookup::has_recent_argon_transfer(&mining_bot_account);

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
					referral_access_code_pending: false,
					issuable_access_codes: 0,
					unactivated_access_codes: 0,
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
			OperationalAccounts::<T>::mutate(&operational_account, |maybe_account| {
				if let Some(stored_account) = maybe_account {
					Self::maybe_activate_operational(&operational_account, stored_account);
				}
			});

			Self::deposit_event(Event::OperationalAccountRegistered {
				operational_account: operational_account.clone(),
				vault_account: vault_account.clone(),
				mining_funding_account: mining_funding_account.clone(),
				mining_bot_account: mining_bot_account.clone(),
				sponsor: sponsor.clone(),
			});
			if let Some(sponsor) = sponsor {
				OperationalAccounts::<T>::mutate(&sponsor, |maybe_account| {
					if let Some(sponsor_account) = maybe_account {
						sponsor_account.unactivated_access_codes.saturating_reduce(1);
						Self::materialize_issuable_access_codes(sponsor_account);
					}
				});
			}
			Ok(())
		}

		/// Issue an access code (the public key itself) for this operational account.
		/// The access code expires after `AccessCodeExpirationFrames`.
		#[pallet::call_index(1)]
		#[pallet::weight(T::WeightInfo::issue_access_code())]
		pub fn issue_access_code(
			origin: OriginFor<T>,
			access_code_public: sr25519::Public,
		) -> DispatchResult {
			let sponsor = ensure_signed(origin)?;
			ensure!(
				!AccessCodesByPublic::<T>::contains_key(access_code_public),
				Error::<T>::AccessCodeAlreadyRegistered
			);
			OperationalAccounts::<T>::try_mutate(
				&sponsor,
				|maybe_account| -> Result<(), Error<T>> {
					let account =
						maybe_account.as_mut().ok_or(Error::<T>::NotOperationalAccount)?;
					ensure!(account.issuable_access_codes > 0, Error::<T>::NoIssuableAccessCodes);
					ensure!(
						account.unactivated_access_codes < T::MaxUnactivatedAccessCodes::get(),
						Error::<T>::MaxUnactivatedAccessCodesReached
					);
					account.issuable_access_codes.saturating_reduce(1);
					account.unactivated_access_codes.saturating_accrue(1);
					Self::materialize_issuable_access_codes(account);
					Ok(())
				},
			)?;

			let current_frame = T::FrameProvider::get_current_frame_id();
			let mut expiration_frame = current_frame;
			expiration_frame.saturating_accrue(T::AccessCodeExpirationFrames::get());
			AccessCodesExpiringByFrame::<T>::try_mutate(
				expiration_frame,
				|expiring_codes| -> Result<(), Error<T>> {
					expiring_codes
						.try_push(access_code_public)
						.map_err(|_| Error::<T>::MaxAccessCodesExpiringPerFrameReached)
				},
			)?;

			AccessCodesByPublic::<T>::insert(
				access_code_public,
				AccessCodeMetadata { sponsor: sponsor.clone(), expiration_frame },
			);
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
						let activated = Self::maybe_activate_operational(&owner, account);
						if !activated {
							Self::materialize_issuable_access_codes(account);
						}
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
				Self::maybe_activate_operational(&owner, account);
			});
		}

		fn decrement_unactivated_access_codes(account: &mut OperationalAccount<T>) {
			account.unactivated_access_codes.saturating_reduce(1);
		}

		fn enqueue_reward(
			operational_account: &mut OperationalAccount<T>,
			owner: &T::AccountId,
			payout_account: &T::AccountId,
			reward_kind: OperationalRewardKind,
			amount: T::Balance,
		) {
			if amount.is_zero() {
				return;
			}
			let reward_kind_event = reward_kind.clone();
			let payout = OperationalRewardPayout {
				operational_account: owner.clone(),
				payout_account: payout_account.clone(),
				reward_kind,
				amount,
			};
			let pushed = OperationalRewardsQueue::<T>::try_mutate(|queue| {
				queue.try_push(payout.clone()).map_err(|_| ())
			})
			.is_ok();
			if !pushed {
				Self::deposit_event(Event::OperationalRewardEnqueueFailed {
					account: owner.clone(),
					reward_kind: reward_kind_event,
					amount,
				});
				return;
			}
			operational_account.rewards_earned_count.saturating_accrue(1);
			operational_account.rewards_earned_amount.saturating_accrue(amount);
			Self::deposit_event(Event::OperationalRewardEarned {
				account: owner.clone(),
				reward_kind: reward_kind_event,
				amount,
			});
		}

		fn has_achieved_operational(operational_account: &OperationalAccount<T>) -> bool {
			operational_account.vault_created &&
				operational_account.has_uniswap_transfer &&
				Self::observed_bitcoin_total(operational_account) >=
					T::MinBitcoinLockSizeForOperational::get() &&
				Self::observed_mining_seat_total(operational_account) >=
					T::MiningSeatsForOperational::get() &&
				operational_account.has_treasury_pool_participation
		}

		fn increment_issuable_access_codes(account: &mut OperationalAccount<T>) {
			if account.issuable_access_codes < T::MaxIssuableAccessCodes::get() {
				account.issuable_access_codes.saturating_accrue(1);
			}
		}

		fn materialize_issuable_access_codes(account: &mut OperationalAccount<T>) {
			if !account.is_operational {
				return;
			}
			let max_issuable_access_codes = T::MaxIssuableAccessCodes::get();
			let bitcoin_threshold = T::BitcoinLockSizeForAccessCode::get();
			let mining_seat_threshold = T::MiningSeatsPerAccessCode::get();
			while account.issuable_access_codes < max_issuable_access_codes {
				if account.referral_access_code_pending {
					account.referral_access_code_pending = false;
					account.issuable_access_codes.saturating_accrue(1);
					continue;
				}
				if bitcoin_threshold > T::Balance::zero() &&
					account.bitcoin_accrual >= bitcoin_threshold
				{
					account.bitcoin_applied_total = Self::observed_bitcoin_total(account);
					account.bitcoin_accrual = T::Balance::zero();
					account.issuable_access_codes.saturating_accrue(1);
					continue;
				}
				if mining_seat_threshold > 0 && account.mining_seat_accrual >= mining_seat_threshold
				{
					account.mining_seat_applied_total = Self::observed_mining_seat_total(account);
					account.mining_seat_accrual = 0;
					account.issuable_access_codes.saturating_accrue(1);
					continue;
				}
				break;
			}
		}

		fn maybe_activate_operational(
			owner: &T::AccountId,
			operational_account: &mut OperationalAccount<T>,
		) -> bool {
			if operational_account.is_operational ||
				!Self::has_achieved_operational(operational_account)
			{
				return false;
			}

			let reward_config = Rewards::<T>::get();
			operational_account.is_operational = true;
			Self::increment_issuable_access_codes(operational_account);
			Self::materialize_issuable_access_codes(operational_account);
			Self::deposit_event(Event::AccountWentOperational { account: owner.clone() });
			let payout_account = operational_account.mining_funding_account.clone();
			Self::enqueue_reward(
				operational_account,
				owner,
				&payout_account,
				OperationalRewardKind::Activation,
				reward_config.operational_referral_reward,
			);

			if let Some(sponsor) = operational_account.sponsor.as_ref() {
				OperationalAccounts::<T>::mutate(sponsor, |maybe_account| {
					let Some(sponsor_account) = maybe_account else {
						return;
					};
					if !sponsor_account.is_operational {
						return;
					}
					sponsor_account.referral_access_code_pending = true;
					Self::materialize_issuable_access_codes(sponsor_account);
					sponsor_account.operational_referrals_count.saturating_accrue(1);
					let payout_account = sponsor_account.mining_funding_account.clone();
					Self::enqueue_reward(
						sponsor_account,
						sponsor,
						&payout_account,
						OperationalRewardKind::Activation,
						reward_config.operational_referral_reward,
					);
					let bonus_every = T::ReferralBonusEveryXOperationalSponsees::get();
					if bonus_every > 0 &&
						sponsor_account.operational_referrals_count % bonus_every == 0
					{
						Self::enqueue_reward(
							sponsor_account,
							sponsor,
							&payout_account,
							OperationalRewardKind::ReferralBonus,
							reward_config.referral_bonus_reward,
						);
					}
				});
			}
			true
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

		fn apply_reward_payment(
			reward: &OperationalRewardPayout<T::AccountId, T::Balance>,
			amount_paid: T::Balance,
		) -> T::Balance {
			OperationalRewardsQueue::<T>::mutate(|queue| {
				// `reward` comes from a copied snapshot; resolve its live queue index before
				// removal.
				let Some(queue_index) = queue.iter().position(|entry| entry == reward) else {
					return T::Balance::zero();
				};
				let queued_amount = queue[queue_index].amount;
				queue.remove(queue_index);
				amount_paid.min(queued_amount)
			})
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
				Self::maybe_activate_operational(&owner, account);
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
				Self::maybe_activate_operational(&owner, account);
				Self::materialize_issuable_access_codes(account);
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
				Self::maybe_activate_operational(&owner, account);
				Self::materialize_issuable_access_codes(account);
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
				Self::maybe_activate_operational(&owner, account);
			});
		}

		fn uniswap_transfer_confirmed_weight() -> Weight {
			<T as Config>::WeightInfo::on_uniswap_transfer()
		}

		fn uniswap_transfer_confirmed(account_id: &T::AccountId, amount: T::Balance) {
			Self::on_uniswap_transfer(account_id, amount)
		}
	}

	impl<T: Config> OperationalRewardsProvider<T::AccountId, T::Balance> for Pallet<T> {
		type Weights = crate::weights::ProviderWeightAdapter<T>;

		fn pending_rewards() -> Vec<OperationalRewardPayout<T::AccountId, T::Balance>> {
			OperationalRewardsQueue::<T>::get().into_iter().collect()
		}

		fn max_pending_rewards() -> u32 {
			T::MaxOperationalRewardsQueued::get()
		}

		fn mark_reward_paid(
			reward: &OperationalRewardPayout<T::AccountId, T::Balance>,
			amount_paid: T::Balance,
		) {
			let amount_paid = Self::apply_reward_payment(reward, amount_paid);
			if amount_paid.is_zero() {
				return;
			}
			OperationalAccounts::<T>::mutate(&reward.operational_account, |maybe_account| {
				if let Some(account) = maybe_account {
					account.rewards_collected_amount.saturating_accrue(amount_paid);
				}
			});
		}
	}
}
