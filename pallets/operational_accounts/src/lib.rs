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
		bitcoin::UtxoId, vault::BitcoinVaultProvider, BitcoinLocksProvider, MiningSlotProvider,
		OperationalAccountProvider, OperationalAccountsHook, OperationalRewardKind,
		OperationalRewardsPayer, Signature, TreasuryPoolProvider, UniswapTransferProvider,
		UtxoLockEvents, MICROGONS_PER_ARGON,
	};
	use codec::{Decode, Encode, EncodeLike};
	use core::{fmt::Debug, marker::PhantomData};
	use frame_support::traits::fungible::Mutate;
	use pallet_prelude::*;
	use polkadot_sdk::frame_system::ensure_root;
	use sp_runtime::{
		traits::{Verify, Zero},
		AccountId32,
	};

	pub const UPSTREAM_ACCESS_PROOF_MESSAGE_KEY: &[u8] = b"operational_access_proof";
	pub const OPERATIONAL_ACCOUNT_PROOF_MESSAGE_KEY: &[u8; 27] = b"operational_primary_account";
	pub const VAULT_ACCOUNT_PROOF_MESSAGE_KEY: &[u8; 25] = b"operational_vault_account";
	pub const MINING_ACCOUNT_PROOF_MESSAGE_KEY: &[u8; 26] = b"operational_mining_account";
	const STORAGE_VERSION: StorageVersion = StorageVersion::new(2);

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
			+ Debug
			+ Default
			+ From<u128>
			+ Into<u128>
			+ TypeInfo
			+ MaxEncodedLen;

		/// Additional argon amount (base units) required per follow-on access code after an account
		/// becomes operational.
		#[pallet::constant]
		type BitcoinLockSizeForAccessCode: Get<Self::Balance>;
		/// Default reward paid when an account becomes operationally certified.
		#[pallet::constant]
		type OperationalCertificationReward: Get<Self::Balance>;
		/// Default bonus reward paid every operational certification threshold.
		#[pallet::constant]
		type OperationalCertificationBonusReward: Get<Self::Balance>;
		/// Number of operational certifications required per bonus reward.
		#[pallet::constant]
		type OperationalCertificationsPerBonusReward: Get<u32>;
		/// Maximum number of available access codes allowed at once.
		#[pallet::constant]
		type MaxAvailableAccessCodes: Get<u32>;
		/// Maximum number of encrypted server bytes stored per network account.
		#[pallet::constant]
		type MaxEncryptedServerLen: Get<u32>;
		/// Minimum Uniswap transfer amount required to register.
		#[pallet::constant]
		type MinimumUniswapTransfer: Get<Self::Balance>;
		/// Minimum bitcoin amount required to register.
		#[pallet::constant]
		type MinimumBitcoin: Get<Self::Balance>;
		/// Minimum bond amount required to register.
		#[pallet::constant]
		type MinimumBonds: Get<Self::Balance>;
		/// Minimum total Uniswap transfer amount required for operational certification.
		#[pallet::constant]
		type OperationalMinimumUniswapTransfer: Get<Self::Balance>;
		/// Minimum vault securitization required to become operational.
		#[pallet::constant]
		type OperationalMinimumVaultSecuritization: Get<Self::Balance>;
		/// Mining seats required to become operational.
		#[pallet::constant]
		type MiningSeatsForOperational: Get<u32>;
		/// Mining seats required per follow-on access code after an account becomes
		/// operational.
		#[pallet::constant]
		type MiningSeatsPerAccessCode: Get<u32>;

		/// Provider for current vault state used to initialize registration.
		type VaultProvider: BitcoinVaultProvider<
			AccountId = Self::AccountId,
			Balance = Self::Balance,
		>;
		/// Provider for whether a linked mining rewards account currently has an active seat.
		type MiningSlotProvider: MiningSlotProvider<Self::AccountId>;
		/// Provider for an account's currently funded bitcoin lock amount at registration time.
		type BitcoinLocksProvider: BitcoinLocksProvider<Self::AccountId, Self::Balance>;
		/// Provider for current account bond participation.
		type TreasuryPoolProvider: TreasuryPoolProvider<Self::AccountId, Balance = Self::Balance>;
		/// Provider for whether crosschain transfer tracking is active and whether linked accounts
		/// already satisfy it.
		type UniswapTransferProvider: UniswapTransferProvider<
			Self::AccountId,
			Balance = Self::Balance,
		>;
		/// Balance source used to confirm current Argon holdings during operational activation.
		type Currency: Mutate<Self::AccountId, Balance = Self::Balance>;
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
		DebugNoBound,
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
		Debug,
		MaxEncodedLen,
		Default,
	)]
	pub struct OpaqueEncryptionPubkey(pub [u8; 32]);

	#[derive(
		Decode,
		DecodeWithMemTracking,
		Encode,
		CloneNoBound,
		PartialEqNoBound,
		EqNoBound,
		DebugNoBound,
		TypeInfo,
		MaxEncodedLen,
	)]
	pub struct UpstreamAccessProof<AccountId>
	where
		AccountId: Clone + PartialEq + Eq + Debug,
	{
		/// Upstream account granting access to this downstream registration.
		pub upstream_account: AccountId,
		/// Signature from the upstream account over the downstream operational account.
		pub signature: Signature,
	}

	impl<AccountId> UpstreamAccessProof<AccountId>
	where
		AccountId: Clone + Encode + Eq + PartialEq + Debug,
	{
		pub fn verify<Owner: EncodeLike>(&self, operational_account: &Owner) -> bool {
			let Ok(upstream_account) =
				AccountId32::decode(&mut self.upstream_account.encode().as_slice())
			else {
				return false;
			};
			let message =
				(UPSTREAM_ACCESS_PROOF_MESSAGE_KEY, &self.upstream_account, operational_account)
					.using_encoded(blake2_256);
			let verified = self.signature.verify(message.as_slice(), &upstream_account);
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
		DebugNoBound,
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
		/// Mining account associated with this operational account.
		pub mining_account: T::AccountId,
		/// Proof that the vault account is controlled by the registrant.
		pub vault_account_proof: AccountOwnershipProof,
		/// Proof that the mining account is controlled by the registrant.
		pub mining_account_proof: AccountOwnershipProof,
		/// Optional upstream access proof used to link this registration to an upstream account.
		pub access_proof: Option<UpstreamAccessProof<T::AccountId>>,
	}

	#[derive(
		Encode,
		Decode,
		DecodeWithMemTracking,
		Clone,
		PartialEq,
		Eq,
		TypeInfo,
		DebugNoBound,
		MaxEncodedLen,
	)]
	#[scale_info(skip_type_params(T))]
	pub enum Registration<T: Config> {
		V1(RegistrationV1<T>),
	}

	#[derive(Encode, Decode, Clone, PartialEq, Eq, TypeInfo, DebugNoBound, MaxEncodedLen)]
	#[scale_info(skip_type_params(T))]
	pub struct OperationalAccount<T: Config> {
		/// Vault account associated with this operational account.
		pub vault_account: T::AccountId,
		/// Mining account associated with this operational account.
		pub mining_account: T::AccountId,
		/// Opaque public encryption key for this operational account, currently x25519 bytes.
		pub encryption_pubkey: OpaqueEncryptionPubkey,
		/// Upstream account, if known.
		pub upstream_account: Option<T::AccountId>,
		/// Cumulative linked-account Uniswap transfers-in amount counted toward the minimum
		/// requirements.
		pub uniswap_argon_transfers_in_amount: T::Balance,
		/// Account bitcoin amount currently counted toward the minimum requirements.
		pub account_bitcoin_amount: T::Balance,
		/// Account vault bond amount currently counted toward the minimum requirements.
		pub account_vault_bond_amount: T::Balance,
		/// Whether the vault has been created for this operational account.
		pub vault_created: bool,
		/// Vault bitcoin amount currently tracked above the stored baseline.
		pub vault_bitcoin_accrual: T::Balance,
		/// Vault bitcoin baseline already included in the tracked total.
		pub vault_bitcoin_applied_total: T::Balance,
		/// Mining seats accrued since the stored baseline.
		#[codec(compact)]
		pub mining_seat_accrual: u32,
		/// Mining seats already included in the stored baseline.
		#[codec(compact)]
		pub mining_seat_applied_total: u32,
		/// Number of downstream operational certifications credited to this account.
		#[codec(compact)]
		pub operational_certifications_count: u32,
		/// Whether one earned access code is pending materialization.
		pub access_code_pending: bool,
		/// Number of access codes this account can still grant.
		#[codec(compact)]
		pub available_access_codes: u32,
		/// Number of rewards earned.
		#[codec(compact)]
		pub rewards_earned_count: u32,
		/// Aggregate amount of rewards earned.
		pub rewards_earned_amount: T::Balance,
		/// Aggregate amount of rewards collected.
		pub rewards_collected_amount: T::Balance,
		/// Whether the account is operationally certified.
		pub is_operationally_certified: bool,
	}

	#[derive(
		Decode, DecodeWithMemTracking, Encode, Clone, PartialEq, Eq, TypeInfo, Debug, MaxEncodedLen,
	)]
	pub struct OperationalProgressPatch<Balance: Member + MaxEncodedLen + Default> {
		/// Requested Uniswap Argon transfers-in amount.
		pub uniswap_argon_transfers_in_amount: Option<Balance>,
		/// Requested minimum for the account bitcoin amount.
		pub account_bitcoin_amount: Option<Balance>,
		/// Requested minimum for the account vault bond amount.
		pub account_vault_bond_amount: Option<Balance>,
		/// Override for whether the vault has been created.
		pub vault_created: Option<bool>,
		/// Requested minimum for the operator vault bitcoin amount.
		///
		/// This is treated as a monotonic applied-total override: the effective stored total will
		/// be at least this value while preserving the previously applied baseline. If the
		/// provided value is lower than the applied total, the current total is retained.
		pub vault_bitcoin_amount: Option<Balance>,
		/// Requested minimum for the total mining seats won.
		///
		/// This is treated as a monotonic applied-total override: the effective stored total will
		/// be at least this value while preserving the previously applied baseline. If the
		/// provided value is lower than the applied total, the current total is retained.
		pub mining_seat_count: Option<u32>,
	}

	impl<Balance: Member + MaxEncodedLen + Default> OperationalProgressPatch<Balance> {
		fn has_updates(&self) -> bool {
			self.uniswap_argon_transfers_in_amount.is_some() ||
				self.account_bitcoin_amount.is_some() ||
				self.account_vault_bond_amount.is_some() ||
				self.vault_created.is_some() ||
				self.vault_bitcoin_amount.is_some() ||
				self.mining_seat_count.is_some()
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

	#[pallet::type_value]
	pub fn DefaultIsOperationalAccountInviteOnly<T: Config>() -> bool {
		true
	}

	#[pallet::storage]
	/// Whether operational-account access is invite-only.
	///
	/// When enabled, registration requires an upstream invite and vault creation plus
	/// mining-slot bidding remain restricted to registered operational accounts.
	///
	/// Existing live raw chain specs do not contain this key, so the default remains invite-only
	/// unless a development chain overrides it in genesis.
	pub type IsOperationalAccountInviteOnly<T: Config> =
		StorageValue<_, bool, ValueQuery, DefaultIsOperationalAccountInviteOnly<T>>;

	#[derive(
		Encode,
		Decode,
		Clone,
		PartialEq,
		Eq,
		Debug,
		TypeInfo,
		MaxEncodedLen,
		Default,
		serde::Deserialize,
		serde::Serialize,
	)]
	pub struct RewardsConfig<Balance: Member + MaxEncodedLen + Default> {
		/// Reward paid when an account becomes operationally certified.
		#[codec(compact)]
		pub operational_certification_reward: Balance,
		/// Bonus reward paid for every operational certification threshold met.
		#[codec(compact)]
		pub operational_certification_bonus_reward: Balance,
	}

	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		pub is_operational_account_invite_only: bool,
		#[serde(skip)]
		pub _phantom: PhantomData<T>,
	}

	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> Self {
			Self { is_operational_account_invite_only: true, _phantom: Default::default() }
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
		fn build(&self) {
			Rewards::<T>::put(RewardsConfig {
				operational_certification_reward: T::OperationalCertificationReward::get(),
				operational_certification_bonus_reward: T::OperationalCertificationBonusReward::get(
				),
			});
			IsOperationalAccountInviteOnly::<T>::put(self.is_operational_account_invite_only);
		}
	}

	#[pallet::storage]
	/// Configured reward amounts for operational accounts.
	pub type Rewards<T: Config> = StorageValue<_, RewardsConfig<T::Balance>, ValueQuery>;

	#[pallet::storage]
	/// Opaque encrypted upstream server payload keyed by the downstream account.
	pub type EncryptedServerByDownstreamAccount<T: Config> = StorageMap<
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
			mining_account: T::AccountId,
			upstream_account: Option<T::AccountId>,
		},
		/// Account has met the registration minimums.
		AccountMeetsMinimums { account: T::AccountId },
		/// Account has become operationally certified.
		AccountOperationallyCertified { account: T::AccountId },
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
			operational_certification_reward: T::Balance,
			operational_certification_bonus_reward: T::Balance,
		},
		/// Operational progress was forced by root.
		OperationalProgressForced {
			account: T::AccountId,
			update_operational_progress: bool,
			meets_minimums: bool,
			uniswap_argon_transfers_in_amount: T::Balance,
			account_bitcoin_amount: T::Balance,
			account_vault_bond_amount: T::Balance,
			vault_created: bool,
			operator_vault_bitcoin_amount: T::Balance,
			mining_seat_count: u32,
		},
		/// An upstream account updated the encrypted server payload for a downstream account.
		EncryptedServerUpdated { upstream_account: T::AccountId, downstream_account: T::AccountId },
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
		/// The requested account has not registered operational accounts.
		NotOperationalAccount,
		/// A valid invite is required to register an operational account.
		RegistrationInviteRequired,
		/// The upstream access proof is invalid.
		InvalidAccessProof,
		/// The requested upstream account does not have a registered operational account.
		UpstreamNotOperationalAccount,
		/// The requested account does not currently satisfy the minimums.
		MinimumsNotMet,
		/// An account cannot grant access to itself.
		CannotGrantAccessToSelf,
		/// The requested progress patch does not contain any updates.
		NoProgressUpdateProvided,
		/// The encrypted server payload exceeds the configured max length.
		EncryptedServerTooLong,
		/// The caller is not the upstream account for the requested downstream account.
		NotUpstreamOfDownstream,
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
		/// The account is already operationally certified.
		AlreadyOperationallyCertified,
		/// The account has not satisfied operational requirements yet.
		NotEligibleForActivation,
		/// The requested upstream account does not have any available access codes.
		UpstreamHasNoAvailableAccessCodes,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Register vault and mining accounts for an operational account.
		/// Any account in the registration may submit the transaction.
		/// When invite-only is enabled, the registration must include an upstream access proof.
		#[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::register())]
		pub fn register(origin: OriginFor<T>, registration: Registration<T>) -> DispatchResult {
			let Registration::V1(RegistrationV1 {
				operational_account,
				encryption_pubkey,
				operational_account_proof,
				vault_account,
				mining_account,
				vault_account_proof,
				mining_account_proof,
				access_proof,
			}) = registration;
			let submitter = ensure_signed(origin)?;
			ensure!(
				submitter == operational_account ||
					submitter == vault_account ||
					submitter == mining_account,
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
				!OperationalAccountBySubAccount::<T>::contains_key(&mining_account),
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
				) && mining_account_proof.verify(
					&operational_account,
					&mining_account,
					MINING_ACCOUNT_PROOF_MESSAGE_KEY,
				),
				Error::<T>::InvalidAccountProof
			);

			let invite_only = IsOperationalAccountInviteOnly::<T>::get();
			let upstream_account = if let Some(access_proof) = access_proof.as_ref() {
				ensure!(access_proof.verify(&operational_account), Error::<T>::InvalidAccessProof);

				let upstream_account = access_proof.upstream_account.clone();
				ensure!(
					OperationalAccounts::<T>::contains_key(&upstream_account),
					Error::<T>::UpstreamNotOperationalAccount
				);
				ensure!(
					upstream_account != operational_account,
					Error::<T>::CannotGrantAccessToSelf
				);
				OperationalAccounts::<T>::try_mutate(
					&upstream_account,
					|maybe_account| -> Result<T::AccountId, Error<T>> {
						let Some(upstream_account_data) = maybe_account.as_mut() else {
							return Err(Error::<T>::UpstreamNotOperationalAccount);
						};
						if upstream_account_data.available_access_codes == 0 {
							return Err(Error::<T>::UpstreamHasNoAvailableAccessCodes);
						}
						upstream_account_data.available_access_codes.saturating_reduce(1);
						Self::materialize_available_access_codes(upstream_account_data);

						Ok(upstream_account.clone())
					},
				)
				.map(Some)?
			} else {
				ensure!(!invite_only, Error::<T>::RegistrationInviteRequired);
				None
			};
			let vault_registration = T::VaultProvider::get_registration_vault_data(&vault_account);
			let mining_seat_count =
				u32::from(T::MiningSlotProvider::has_active_rewards_account_seat(&mining_account));
			let vault_created = vault_registration.is_some();
			let vault_bitcoin_accrual = vault_registration
				.map(|vault| vault.activated_securitization)
				.unwrap_or_else(Zero::zero);
			let mut account = OperationalAccount {
				vault_account: vault_account.clone(),
				mining_account: mining_account.clone(),
				encryption_pubkey,
				upstream_account: upstream_account.clone(),
				uniswap_argon_transfers_in_amount: T::Balance::zero(),
				account_bitcoin_amount: T::BitcoinLocksProvider::get_account_funded_bitcoin_amount(
					&vault_account,
				),
				account_vault_bond_amount:
					T::TreasuryPoolProvider::active_account_vault_bond_amount(&vault_account),
				vault_created,
				// Bootstrap lookup seeds current vault bitcoin progress so already-funded vaults
				// preserve their tracked vault bitcoin progress after registration.
				vault_bitcoin_accrual,
				vault_bitcoin_applied_total: T::Balance::zero(),
				mining_seat_accrual: mining_seat_count,
				mining_seat_applied_total: 0,
				operational_certifications_count: 0,
				access_code_pending: false,
				available_access_codes: 0,
				rewards_earned_count: 0,
				rewards_earned_amount: T::Balance::zero(),
				rewards_collected_amount: T::Balance::zero(),
				is_operationally_certified: false,
			};
			account.uniswap_argon_transfers_in_amount =
				Self::current_linked_account_uniswap_argon_transfers_in_amount(
					&operational_account,
					&account,
				);
			ensure!(Self::meets_minimums(&account), Error::<T>::MinimumsNotMet);
			Self::emit_meets_minimums_event_if_newly_met(&operational_account, false, &account);

			OperationalAccounts::<T>::insert(&operational_account, account);

			OperationalAccountBySubAccount::<T>::insert(&vault_account, &operational_account);
			OperationalAccountBySubAccount::<T>::insert(&mining_account, &operational_account);

			Self::deposit_event(Event::OperationalAccountRegistered {
				operational_account: operational_account.clone(),
				vault_account: vault_account.clone(),
				mining_account: mining_account.clone(),
				upstream_account,
			});
			Ok(())
		}

		/// Update certification reward amounts for operational accounts.
		#[pallet::call_index(2)]
		#[pallet::weight(T::WeightInfo::set_reward_config())]
		pub fn set_reward_config(
			origin: OriginFor<T>,
			operational_certification_reward: T::Balance,
			operational_certification_bonus_reward: T::Balance,
		) -> DispatchResult {
			ensure_root(origin)?;
			Rewards::<T>::put(RewardsConfig {
				operational_certification_reward,
				operational_certification_bonus_reward,
			});
			Self::deposit_event(Event::RewardsConfigUpdated {
				operational_certification_reward,
				operational_certification_bonus_reward,
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

			let mut meets_minimums = false;
			let mut uniswap_argon_transfers_in_amount = T::Balance::zero();
			let mut account_bitcoin_amount = T::Balance::zero();
			let mut account_vault_bond_amount = T::Balance::zero();
			let mut vault_created = false;
			let mut operator_vault_bitcoin_amount = T::Balance::zero();
			let mut mining_seat_count = 0u32;

			OperationalAccounts::<T>::try_mutate(
				&owner,
				|maybe_account| -> Result<(), Error<T>> {
					let account =
						maybe_account.as_mut().ok_or(Error::<T>::NotOperationalAccount)?;
					let previously_met_minimums = Self::meets_minimums(account);

					if let Some(value) = patch.uniswap_argon_transfers_in_amount {
						account.uniswap_argon_transfers_in_amount = value;
					}
					if let Some(value) = patch.account_bitcoin_amount {
						account.account_bitcoin_amount = value;
					}
					if let Some(value) = patch.account_vault_bond_amount {
						account.account_vault_bond_amount = value;
					}
					if let Some(value) = patch.vault_created {
						account.vault_created = value;
					}
					if let Some(value) = patch.vault_bitcoin_amount {
						Self::recalculate_vault_bitcoin_accrual(account, value);
					}
					if let Some(value) = patch.mining_seat_count {
						Self::set_mining_seat_count(account, value);
					}
					meets_minimums = Self::emit_meets_minimums_event_if_newly_met(
						&owner,
						previously_met_minimums,
						account,
					);

					if update_operational_progress {
						Self::materialize_available_access_codes(account);
					}

					uniswap_argon_transfers_in_amount = account.uniswap_argon_transfers_in_amount;
					account_bitcoin_amount = account.account_bitcoin_amount;
					account_vault_bond_amount = account.account_vault_bond_amount;
					vault_created = account.vault_created;
					operator_vault_bitcoin_amount = Self::current_vault_bitcoin_amount(account);
					mining_seat_count = Self::mining_seat_count(account);
					Ok(())
				},
			)?;

			Self::deposit_event(Event::OperationalProgressForced {
				account: owner,
				update_operational_progress,
				meets_minimums,
				uniswap_argon_transfers_in_amount,
				account_bitcoin_amount,
				account_vault_bond_amount,
				vault_created,
				operator_vault_bitcoin_amount,
				mining_seat_count,
			});
			Ok(())
		}

		/// Store an opaque encrypted upstream server payload for a downstream account.
		#[pallet::call_index(4)]
		#[pallet::weight(T::WeightInfo::set_encrypted_server_for_downstream_account())]
		pub fn set_encrypted_server_for_downstream_account(
			origin: OriginFor<T>,
			downstream_account: T::AccountId,
			encrypted_server: Vec<u8>,
		) -> DispatchResult {
			let upstream_account = ensure_signed(origin)?;
			ensure!(
				OperationalAccounts::<T>::contains_key(&upstream_account),
				Error::<T>::NotOperationalAccount
			);
			let downstream_account_data = OperationalAccounts::<T>::get(&downstream_account)
				.ok_or(Error::<T>::NotOperationalAccount)?;
			ensure!(
				downstream_account_data.upstream_account == Some(upstream_account.clone()),
				Error::<T>::NotUpstreamOfDownstream
			);

			let encrypted_server: BoundedVec<u8, T::MaxEncryptedServerLen> =
				encrypted_server.try_into().map_err(|_| Error::<T>::EncryptedServerTooLong)?;
			EncryptedServerByDownstreamAccount::<T>::insert(&downstream_account, encrypted_server);
			Self::deposit_event(Event::EncryptedServerUpdated {
				upstream_account,
				downstream_account,
			});
			Ok(())
		}

		/// Activate an eligible operational account from any managed account.
		#[pallet::call_index(5)]
		#[pallet::weight(T::WeightInfo::activate())]
		pub fn activate(origin: OriginFor<T>) -> DispatchResult {
			let signer = ensure_signed(origin)?;
			let owner =
				Self::operational_owner_for(&signer).ok_or(Error::<T>::NotOperationalAccount)?;

			OperationalAccounts::<T>::try_mutate(&owner, |maybe_account| -> DispatchResult {
				let account = maybe_account.as_mut().ok_or(Error::<T>::NotOperationalAccount)?;
				ensure!(
					!account.is_operationally_certified,
					Error::<T>::AlreadyOperationallyCertified
				);
				ensure!(Self::meets_minimums(account), Error::<T>::MinimumsNotMet);
				ensure!(
					Self::meets_activation_requirements(account),
					Error::<T>::NotEligibleForActivation
				);

				let reward_config = Rewards::<T>::get();
				account.is_operationally_certified = true;
				Self::increment_available_access_codes(account);
				Self::materialize_available_access_codes(account);
				T::VaultProvider::account_became_operational(&account.vault_account);
				Self::deposit_event(Event::AccountOperationallyCertified {
					account: owner.clone(),
				});
				Self::record_reward(
					account,
					&owner,
					OperationalRewardKind::Certification,
					reward_config.operational_certification_reward,
				);

				if let Some(upstream_account) = account.upstream_account.as_ref() {
					OperationalAccounts::<T>::mutate(upstream_account, |maybe_account| {
						let Some(upstream_account_data) = maybe_account else {
							return;
						};
						if !upstream_account_data.is_operationally_certified {
							return;
						}
						upstream_account_data.access_code_pending = true;
						Self::materialize_available_access_codes(upstream_account_data);
						upstream_account_data.operational_certifications_count.saturating_accrue(1);
						Self::record_reward(
							upstream_account_data,
							upstream_account,
							OperationalRewardKind::Certification,
							reward_config.operational_certification_reward,
						);
						let bonus_every = T::OperationalCertificationsPerBonusReward::get();
						if bonus_every > 0 &&
							upstream_account_data.operational_certifications_count % bonus_every ==
								0
						{
							Self::record_reward(
								upstream_account_data,
								upstream_account,
								OperationalRewardKind::OperationalCertificationBonus,
								reward_config.operational_certification_bonus_reward,
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
			let owner =
				Self::operational_owner_for(&claimant).ok_or(Error::<T>::NotOperationalAccount)?;
			let claim_increment = T::Balance::from(MICROGONS_PER_ARGON);
			let amount_u128: u128 = amount.into();

			ensure!(amount >= claim_increment, Error::<T>::RewardClaimBelowMinimum);
			ensure!(
				amount_u128.is_multiple_of(MICROGONS_PER_ARGON),
				Error::<T>::RewardClaimNotWholeArgon
			);

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
		/// Refresh the stored linked-account Uniswap Argon transfers-in amount for a linked
		/// operational account.
		pub fn refresh_account_uniswap_argon_transfers_in_amount(account_id: &T::AccountId) {
			let Some(owner) = Self::operational_owner_for(account_id) else {
				return;
			};

			OperationalAccounts::<T>::mutate(&owner, |maybe_account| {
				let Some(account) = maybe_account else {
					return;
				};
				let previously_met_minimums = Self::meets_minimums(account);

				account.uniswap_argon_transfers_in_amount =
					Self::current_linked_account_uniswap_argon_transfers_in_amount(&owner, account);
				Self::emit_meets_minimums_event_if_newly_met(
					&owner,
					previously_met_minimums,
					account,
				);
			});
		}

		fn operational_owner_for(account_id: &T::AccountId) -> Option<T::AccountId> {
			if OperationalAccounts::<T>::contains_key(account_id) {
				return Some(account_id.clone());
			}

			OperationalAccountBySubAccount::<T>::get(account_id)
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

		fn has_uniswap_argon_transfers_in_requirement(
			operational_account: &OperationalAccount<T>,
			minimum: T::Balance,
		) -> bool {
			!T::UniswapTransferProvider::is_crosschain_activated() ||
				operational_account.uniswap_argon_transfers_in_amount >= minimum
		}

		pub(crate) fn meets_minimums(operational_account: &OperationalAccount<T>) -> bool {
			let has_uniswap_argon_transfers_in = Self::has_uniswap_argon_transfers_in_requirement(
				operational_account,
				T::MinimumUniswapTransfer::get(),
			);
			let has_account_bitcoin =
				operational_account.account_bitcoin_amount >= T::MinimumBitcoin::get();
			let has_account_vault_bonds =
				operational_account.account_vault_bond_amount >= T::MinimumBonds::get();

			has_uniswap_argon_transfers_in && has_account_bitcoin && has_account_vault_bonds
		}

		fn meets_activation_requirements(operational_account: &OperationalAccount<T>) -> bool {
			let has_operational_uniswap_argon_transfers_in =
				Self::has_uniswap_argon_transfers_in_requirement(
					operational_account,
					T::OperationalMinimumUniswapTransfer::get(),
				);
			let has_vault_securitization = Self::vault_amount(operational_account) >=
				T::OperationalMinimumVaultSecuritization::get();
			let has_mining_seats =
				Self::mining_seat_count(operational_account) >= T::MiningSeatsForOperational::get();

			has_operational_uniswap_argon_transfers_in &&
				has_vault_securitization &&
				has_mining_seats
		}

		fn emit_meets_minimums_event_if_newly_met(
			owner: &T::AccountId,
			previously_met_minimums: bool,
			account: &OperationalAccount<T>,
		) -> bool {
			let meets_minimums = Self::meets_minimums(account);
			if !previously_met_minimums && meets_minimums {
				Self::deposit_event(Event::AccountMeetsMinimums { account: owner.clone() });
			}
			meets_minimums
		}

		fn increment_available_access_codes(account: &mut OperationalAccount<T>) {
			if account.available_access_codes < T::MaxAvailableAccessCodes::get() {
				account.available_access_codes.saturating_accrue(1);
			}
		}

		fn materialize_available_access_codes(account: &mut OperationalAccount<T>) {
			if !account.is_operationally_certified {
				return;
			}
			let max_available_access_codes = T::MaxAvailableAccessCodes::get();
			let bitcoin_threshold = T::BitcoinLockSizeForAccessCode::get();
			let mining_seat_threshold = T::MiningSeatsPerAccessCode::get();
			while account.available_access_codes < max_available_access_codes {
				if account.access_code_pending {
					account.access_code_pending = false;
					account.available_access_codes.saturating_accrue(1);
					continue;
				}
				if bitcoin_threshold > T::Balance::zero() &&
					account.vault_bitcoin_accrual >= bitcoin_threshold
				{
					account.vault_bitcoin_applied_total =
						Self::current_vault_bitcoin_amount(account);
					account.vault_bitcoin_accrual = T::Balance::zero();
					account.available_access_codes.saturating_accrue(1);
					continue;
				}
				if mining_seat_threshold > 0 && account.mining_seat_accrual >= mining_seat_threshold
				{
					account.mining_seat_applied_total = Self::mining_seat_count(account);
					account.mining_seat_accrual = 0;
					account.available_access_codes.saturating_accrue(1);
					continue;
				}
				break;
			}
		}

		fn current_vault_bitcoin_amount(account: &OperationalAccount<T>) -> T::Balance {
			// Current vault bitcoin equals the stored baseline plus the new amount accrued since
			// then.
			let previously_applied = account.vault_bitcoin_applied_total;
			let newly_accrued = account.vault_bitcoin_accrual;

			previously_applied.saturating_add(newly_accrued)
		}

		fn current_linked_account_uniswap_argon_transfers_in_amount(
			owner: &T::AccountId,
			account: &OperationalAccount<T>,
		) -> T::Balance {
			let linked_accounts =
				[owner.clone(), account.vault_account.clone(), account.mining_account.clone()];
			let mut amount = T::Balance::zero();

			for (index, account_id) in linked_accounts.iter().enumerate() {
				if linked_accounts[..index]
					.iter()
					.any(|prior_account_id| prior_account_id == account_id)
				{
					continue;
				}

				amount.saturating_accrue(
					T::UniswapTransferProvider::account_uniswap_argon_transfers_in_amount(
						account_id,
					),
				);
			}

			amount
		}

		fn vault_amount(account: &OperationalAccount<T>) -> T::Balance {
			T::VaultProvider::get_registration_vault_data(&account.vault_account)
				.map(|vault| vault.securitization)
				.unwrap_or_else(Zero::zero)
		}

		fn mining_seat_count(account: &OperationalAccount<T>) -> u32 {
			account.mining_seat_applied_total.saturating_add(account.mining_seat_accrual)
		}

		fn set_mining_seat_count(account: &mut OperationalAccount<T>, total: u32) {
			let prior_applied_total = account.mining_seat_applied_total;
			account.mining_seat_accrual = total.saturating_sub(prior_applied_total);
		}

		fn recalculate_vault_bitcoin_accrual(
			account: &mut OperationalAccount<T>,
			total_locked: T::Balance,
		) {
			account.vault_bitcoin_accrual =
				total_locked.saturating_sub(account.vault_bitcoin_applied_total);
		}

		fn adjust_account_bitcoin_amount(
			account_id: &T::AccountId,
			amount: T::Balance,
			is_increase: bool,
		) {
			let Some(owner) = Self::operational_owner_for(account_id) else {
				return;
			};
			OperationalAccounts::<T>::mutate(&owner, |maybe_account| {
				let Some(account) = maybe_account else {
					return;
				};
				if account.vault_account != *account_id {
					return;
				}
				let previously_met_minimums = Self::meets_minimums(account);

				if is_increase {
					account.account_bitcoin_amount.saturating_accrue(amount);
				} else {
					account.account_bitcoin_amount.saturating_reduce(amount);
				}
				Self::emit_meets_minimums_event_if_newly_met(
					&owner,
					previously_met_minimums,
					account,
				);
			});
		}

		fn set_account_vault_bond_amount(account_id: &T::AccountId, total_amount: T::Balance) {
			let Some(owner) = Self::operational_owner_for(account_id) else {
				return;
			};
			OperationalAccounts::<T>::mutate(&owner, |maybe_account| {
				let Some(account) = maybe_account else {
					return;
				};
				if account.vault_account != *account_id {
					return;
				}
				let previously_met_minimums = Self::meets_minimums(account);

				account.account_vault_bond_amount = total_amount;
				Self::emit_meets_minimums_event_if_newly_met(
					&owner,
					previously_met_minimums,
					account,
				);
			});
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

		fn vault_bitcoin_lock_funded_weight() -> Weight {
			<T as Config>::WeightInfo::on_vault_bitcoin_lock_funded()
		}

		fn vault_bitcoin_lock_funded(
			vault_operator_account: &T::AccountId,
			total_locked: T::Balance,
		) {
			let Some(owner) = Self::operational_owner_for(vault_operator_account) else {
				return;
			};

			OperationalAccounts::<T>::mutate(&owner, |maybe_account| {
				let Some(account) = maybe_account else {
					return;
				};
				if account.vault_account != *vault_operator_account {
					return;
				}
				Self::recalculate_vault_bitcoin_accrual(account, total_locked);
				Self::materialize_available_access_codes(account);
			});
		}

		fn mining_seat_won_weight() -> Weight {
			<T as Config>::WeightInfo::on_mining_seat_won()
		}

		fn mining_seat_won(miner_account: &T::AccountId) {
			let Some(owner) = Self::operational_owner_for(miner_account) else {
				return;
			};
			OperationalAccounts::<T>::mutate(&owner, |maybe_account| {
				let Some(account) = maybe_account else {
					return;
				};
				account.mining_seat_accrual.saturating_accrue(1);
				Self::materialize_available_access_codes(account);
			});
		}

		fn account_vault_bond_total_updated_weight() -> Weight {
			<T as Config>::WeightInfo::on_account_vault_bond_total_updated()
		}

		fn account_vault_bond_total_updated(account_id: &T::AccountId, total_amount: T::Balance) {
			Self::set_account_vault_bond_amount(account_id, total_amount);
		}

		fn account_uniswap_argon_transfers_in_updated_weight() -> Weight {
			<T as Config>::WeightInfo::on_account_uniswap_argon_transfers_in_updated()
		}

		fn account_uniswap_argon_transfers_in_updated(account_id: &T::AccountId) {
			Self::refresh_account_uniswap_argon_transfers_in_amount(account_id)
		}
	}

	impl<T: Config> UtxoLockEvents<T::AccountId, T::Balance> for Pallet<T> {
		type Weights = weights::ProviderWeightAdapter<T>;

		fn utxo_locked(
			_utxo_id: UtxoId,
			account_id: &T::AccountId,
			amount: T::Balance,
		) -> DispatchResult {
			Self::adjust_account_bitcoin_amount(account_id, amount, true);
			Ok(())
		}

		fn utxo_released(
			_utxo_id: UtxoId,
			account_id: &T::AccountId,
			_remove_pending_mints: bool,
			_burned_argons: T::Balance,
			original_liquidity_promised: T::Balance,
		) -> DispatchResult {
			Self::adjust_account_bitcoin_amount(account_id, original_liquidity_promised, false);
			Ok(())
		}
	}

	impl<T: Config> OperationalAccountProvider<T::AccountId> for Pallet<T> {
		type Weights = weights::ProviderWeightAdapter<T>;

		fn is_eligible(account_id: &T::AccountId) -> bool {
			if !IsOperationalAccountInviteOnly::<T>::get() {
				return true;
			}

			let Some(owner) = Self::operational_owner_for(account_id) else {
				return false;
			};

			OperationalAccounts::<T>::contains_key(owner)
		}
	}
}
