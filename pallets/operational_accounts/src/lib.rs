#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::large_enum_variant)]

extern crate alloc;

pub use pallet::*;
use pallet_prelude::frame_support;
pub use weights::WeightInfo;

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
	use argon_primitives::{
		MiningFrameTransitionProvider, OperationalAccountsHook, OperationalRewardKind,
		OperationalRewardPayout, OperationalRewardsPayer, OperationalRewardsProvider, Signature,
		TransactionSponsorProvider, TxSponsor,
	};
	use codec::{Decode, Encode, EncodeLike};
	use pallet_prelude::*;
	use polkadot_sdk::{frame_support::traits::IsSubType, frame_system::ensure_root};
	use sp_core::sr25519;
	use sp_runtime::{AccountId32, traits::Verify};

	/// Domain separator for access code activation proofs.
	pub const ACCESS_CODE_PROOF_MESSAGE_KEY: &[u8; 17] = b"access_code_claim";
	pub const VAULT_ACCOUNT_PROOF_MESSAGE_KEY: &[u8; 25] = b"operational_vault_account";
	pub const MINING_FUNDING_ACCOUNT_PROOF_MESSAGE_KEY: &[u8; 26] = b"operational_mining_funding";
	pub const MINING_BOT_ACCOUNT_PROOF_MESSAGE_KEY: &[u8; 22] = b"operational_mining_bot";
	// Upper bound of immediate treasury payout attempts from one operation:
	// account activation reward + sponsor activation reward + sponsor referral bonus.
	const MAX_IMMEDIATE_REWARD_PAYOUTS_PER_OPERATION: u64 = 3;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[derive(Encode, Decode, Clone, PartialEq, Eq, TypeInfo, MaxEncodedLen)]
	pub struct LegacyVaultInfo<AccountId, Balance> {
		pub vault_account: AccountId,
		pub activated_securitization: Balance,
		pub has_treasury_pool_participation: bool,
	}

	pub trait LegacyVaultProvider<AccountId, Balance> {
		fn legacy_vaults() -> alloc::vec::Vec<LegacyVaultInfo<AccountId, Balance>>;
	}

	impl<AccountId, Balance> LegacyVaultProvider<AccountId, Balance> for () {
		fn legacy_vaults() -> alloc::vec::Vec<LegacyVaultInfo<AccountId, Balance>> {
			alloc::vec::Vec::new()
		}
	}

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
		/// Maximum number of legacy vault records to hydrate on registration.
		#[pallet::constant]
		type MaxLegacyVaultRegistrations: Get<u32>;
		/// Maximum number of queued operational rewards.
		#[pallet::constant]
		type MaxOperationalRewardsQueued: Get<u32>;
		/// Maximum number of unactivated (issued but unused) access codes allowed at once.
		#[pallet::constant]
		type MaxUnactivatedAccessCodes: Get<u32>;
		/// Minimum argon amount (base units) required to mark a bitcoin lock as qualifying.
		#[pallet::constant]
		type MinBitcoinLockSizeForOperational: Get<Self::Balance>;
		/// Mining seats required to become operational.
		#[pallet::constant]
		type MiningSeatsForOperational: Get<u32>;
		/// Mining seats required per access code after operational.
		#[pallet::constant]
		type MiningSeatsPerAccessCode: Get<u32>;

		/// Provider for legacy vault hydration data.
		type LegacyVaultProvider: LegacyVaultProvider<Self::AccountId, Self::Balance>;
		/// Pays operational rewards immediately when possible.
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
			self.signature.verify(message.as_slice(), &self.public)
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
			self.signature.verify(message.as_slice(), &account_id)
		}
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
		/// Sponsor account, if known.
		pub sponsor: Option<T::AccountId>,
		/// Whether at least one qualifying Uniswap transfer has been observed.
		pub has_uniswap_transfer: bool,
		/// Whether the vault has been created for this operational account.
		pub vault_created: bool,
		/// Bitcoin amount accrued since the last bitcoin high watermark.
		pub bitcoin_accrual: T::Balance,
		/// Bitcoin locked high watermark consumed by previously issued bitcoin access codes.
		pub bitcoin_high_watermark: T::Balance,
		/// Whether the account has participated in a treasury pool.
		pub has_treasury_pool_participation: bool,
		/// Mining seats accrued since the last mining high watermark.
		#[codec(compact)]
		pub mining_seat_accrual: u32,
		/// Mining seats high watermark consumed by previously issued mining access codes.
		#[codec(compact)]
		pub mining_seat_high_watermark: u32,
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

	#[pallet::storage]
	/// Legacy vault data used to hydrate accounts as they register.
	pub type LegacyVaultRegistrations<T: Config> = StorageValue<
		_,
		BoundedVec<LegacyVaultInfo<T::AccountId, T::Balance>, T::MaxLegacyVaultRegistrations>,
		ValueQuery,
	>;

	#[pallet::storage]
	/// Tracks whether the initial migration has already run.
	#[pallet::storage_prefix = "LegacyVaultHydrationComplete"]
	pub type HasInitialMigrationRun<T: Config> = StorageValue<_, bool, ValueQuery>;

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

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// An operational account was registered with its linked accounts.
		OperationalAccountRegistered {
			account: T::AccountId,
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
	}

	#[pallet::error]
	pub enum Error<T> {
		/// The caller already registered an operational account.
		AlreadyRegistered,
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
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_runtime_upgrade() -> Weight {
			Self::run_initial_migration()
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
		/// Register vault, mining funding, and bot accounts for the signer.
		/// If an access code is provided, the sponsor pays the transaction fee.
		#[pallet::call_index(0)]
		#[pallet::weight(
			T::WeightInfo::register().saturating_add(
				T::OperationalRewardsPayer::try_pay_reward_weight()
					.saturating_mul(MAX_IMMEDIATE_REWARD_PAYOUTS_PER_OPERATION),
			)
		)]
		#[allow(clippy::too_many_arguments)]
		pub fn register(
			origin: OriginFor<T>,
			vault_account: T::AccountId,
			mining_funding_account: T::AccountId,
			mining_bot_account: T::AccountId,
			vault_account_proof: AccountOwnershipProof,
			mining_funding_account_proof: AccountOwnershipProof,
			mining_bot_account_proof: AccountOwnershipProof,
			access_code: Option<AccessCodeProof>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			ensure!(!OperationalAccounts::<T>::contains_key(&who), Error::<T>::AlreadyRegistered);
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
				vault_account_proof.verify(&who, &vault_account, VAULT_ACCOUNT_PROOF_MESSAGE_KEY),
				Error::<T>::InvalidAccountProof
			);
			ensure!(
				mining_funding_account_proof.verify(
					&who,
					&mining_funding_account,
					MINING_FUNDING_ACCOUNT_PROOF_MESSAGE_KEY,
				),
				Error::<T>::InvalidAccountProof
			);
			ensure!(
				mining_bot_account_proof.verify(
					&who,
					&mining_bot_account,
					MINING_BOT_ACCOUNT_PROOF_MESSAGE_KEY,
				),
				Error::<T>::InvalidAccountProof
			);

			let sponsor = if let Some(access_code) = access_code {
				let code = AccessCodesByPublic::<T>::take(access_code.public)
					.ok_or(Error::<T>::InvalidAccessCode)?;
				ensure!(access_code.verify(&who), Error::<T>::InvalidAccessCodeProof);
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
			let legacy_info = LegacyVaultRegistrations::<T>::mutate(|entries| {
				let pos = entries.iter().position(|entry| entry.vault_account == vault_account)?;
				Some(entries.remove(pos))
			});
			let (legacy_bitcoin_total, legacy_has_treasury, legacy_vault_created) =
				if let Some(info) = legacy_info {
					(info.activated_securitization, info.has_treasury_pool_participation, true)
				} else {
					(T::Balance::zero(), false, false)
				};
			let has_uniswap_transfer = legacy_bitcoin_total > T::Balance::zero();

			OperationalAccounts::<T>::insert(
				&who,
				OperationalAccount {
					vault_account: vault_account.clone(),
					mining_funding_account: mining_funding_account.clone(),
					mining_bot_account: mining_bot_account.clone(),
					sponsor: sponsor.clone(),
					has_uniswap_transfer,
					vault_created: legacy_vault_created,
					bitcoin_accrual: legacy_bitcoin_total,
					bitcoin_high_watermark: T::Balance::zero(),
					has_treasury_pool_participation: legacy_has_treasury,
					mining_seat_accrual: 0,
					mining_seat_high_watermark: 0,
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

			OperationalAccountBySubAccount::<T>::insert(&vault_account, &who);
			OperationalAccountBySubAccount::<T>::insert(&mining_funding_account, &who);
			OperationalAccountBySubAccount::<T>::insert(&mining_bot_account, &who);
			OperationalAccounts::<T>::mutate(&who, |maybe_account| {
				if let Some(account) = maybe_account {
					Self::maybe_activate_operational(&who, account);
				}
			});

			Self::deposit_event(Event::OperationalAccountRegistered {
				account: who,
				vault_account,
				mining_funding_account,
				mining_bot_account,
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

		fn immediate_reward_payout_weight() -> Weight {
			T::OperationalRewardsPayer::try_pay_reward_weight()
				.saturating_mul(MAX_IMMEDIATE_REWARD_PAYOUTS_PER_OPERATION)
		}

		fn run_initial_migration() -> Weight {
			if HasInitialMigrationRun::<T>::get() {
				return T::DbWeight::get().reads(1);
			}
			Rewards::<T>::put(RewardsConfig {
				operational_referral_reward: T::OperationalReferralReward::get(),
				referral_bonus_reward: T::OperationalReferralBonusReward::get(),
			});
			let legacy_entries = T::LegacyVaultProvider::legacy_vaults();
			let mut writes = 2u64;
			if !legacy_entries.is_empty() {
				let mut registrations: BoundedVec<
					LegacyVaultInfo<T::AccountId, T::Balance>,
					T::MaxLegacyVaultRegistrations,
				> = BoundedVec::default();
				for entry in legacy_entries {
					if registrations.try_push(entry).is_err() {
						break;
					}
				}
				LegacyVaultRegistrations::<T>::put(registrations);
				writes = writes.saturating_add(1);
			}
			HasInitialMigrationRun::<T>::put(true);
			T::DbWeight::get().reads_writes(1, writes)
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

			if T::OperationalRewardsPayer::try_pay_reward(&payout) &&
				Self::remove_reward_from_queue(&payout)
			{
				operational_account.rewards_collected_amount.saturating_accrue(payout.amount);
			}
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
					account.bitcoin_high_watermark = Self::observed_bitcoin_total(account);
					account.bitcoin_accrual = T::Balance::zero();
					account.issuable_access_codes.saturating_accrue(1);
					continue;
				}
				if mining_seat_threshold > 0 && account.mining_seat_accrual >= mining_seat_threshold
				{
					account.mining_seat_high_watermark = Self::observed_mining_seat_total(account);
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
					if sponsor_account.operational_referrals_count % bonus_every == 0 {
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
			account.bitcoin_high_watermark.saturating_add(account.bitcoin_accrual)
		}

		fn observed_mining_seat_total(account: &OperationalAccount<T>) -> u32 {
			account.mining_seat_high_watermark.saturating_add(account.mining_seat_accrual)
		}

		fn remove_reward_from_queue(
			reward: &OperationalRewardPayout<T::AccountId, T::Balance>,
		) -> bool {
			OperationalRewardsQueue::<T>::mutate(|queue| {
				let Some(pos) = queue.iter().position(|entry| entry == reward) else {
					return false;
				};
				queue.remove(pos);
				true
			})
		}
	}

	impl<T: Config> OperationalAccountsHook<T::AccountId, T::Balance> for Pallet<T> {
		fn vault_created_weight() -> Weight {
			<T as Config>::WeightInfo::on_vault_created()
				.saturating_add(Self::immediate_reward_payout_weight())
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
				.saturating_add(Self::immediate_reward_payout_weight())
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
				account.bitcoin_accrual =
					total_locked.saturating_sub(account.bitcoin_high_watermark);
				Self::maybe_activate_operational(&owner, account);
				Self::materialize_issuable_access_codes(account);
			});
		}

		fn mining_seat_won_weight() -> Weight {
			<T as Config>::WeightInfo::on_mining_seat_won()
				.saturating_add(Self::immediate_reward_payout_weight())
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
				.saturating_add(Self::immediate_reward_payout_weight())
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
				.saturating_add(Self::immediate_reward_payout_weight())
		}

		fn uniswap_transfer_confirmed(account_id: &T::AccountId, amount: T::Balance) {
			Self::on_uniswap_transfer(account_id, amount)
		}
	}

	impl<T: Config> OperationalRewardsProvider<T::AccountId, T::Balance> for Pallet<T> {
		fn pending_rewards() -> Vec<OperationalRewardPayout<T::AccountId, T::Balance>> {
			OperationalRewardsQueue::<T>::get().into_iter().collect()
		}

		fn mark_reward_paid(reward: &OperationalRewardPayout<T::AccountId, T::Balance>) {
			if !Self::remove_reward_from_queue(reward) {
				return;
			}
			OperationalAccounts::<T>::mutate(&reward.operational_account, |maybe_account| {
				if let Some(account) = maybe_account {
					account.rewards_collected_amount.saturating_accrue(reward.amount);
				}
			});
		}
	}

	impl<T: Config> TransactionSponsorProvider<T::AccountId, T::RuntimeCall, T::Balance> for Pallet<T>
	where
		T::RuntimeCall: IsSubType<Call<T>>,
	{
		fn get_transaction_sponsor(
			signer: &T::AccountId,
			call: &T::RuntimeCall,
		) -> Option<TxSponsor<T::AccountId, T::Balance>> {
			let pallet_call: &Call<T> = <T::RuntimeCall as IsSubType<Call<T>>>::is_sub_type(call)?;
			match pallet_call {
				Call::register { access_code, .. } => {
					let access_code = access_code.as_ref()?;
					let code = AccessCodesByPublic::<T>::get(access_code.public)?;
					if !access_code.verify(signer) {
						return None;
					}
					Some(TxSponsor {
						payer: code.sponsor,
						max_fee_with_tip: None,
						unique_tx_key: Some(access_code.public.encode()),
					})
				},
				_ => None,
			}
		}
	}
}
