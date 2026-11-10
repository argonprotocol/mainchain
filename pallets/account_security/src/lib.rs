#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

use alloc::{boxed::Box, vec, vec::Vec};
use argon_primitives::{AccountSecurityProvider, AccountSecurityProviderWeightInfo, Signature};
use core::{iter::once, marker::PhantomData};
use frame_support::{
	dispatch::{
		extract_actual_weight, DispatchResultWithPostInfo, GetDispatchInfo, PostDispatchInfo,
	},
	traits::{IsSubType, OriginTrait, ReservableCurrency},
	EqNoBound, PartialEqNoBound,
};
use frame_system::{Account as SystemAccount, Config as SystemConfig, Pallet as System};
pub use pallet::*;
use pallet_multisig::{Call as MultisigCall, Config as MultisigConfig};
use pallet_prelude::*;
use pallet_proxy::{Call as ProxyCall, Config as ProxyConfig};
use pallet_utility::{Call as UtilityCall, Config as UtilityConfig};
use sp_runtime::{
	traits::{
		DispatchOriginOf, IdentifyAccount, Implication, TransactionExtension, ValidateResult,
		Verify,
	},
	transaction_validity::{
		InvalidTransaction, TransactionSource, TransactionValidityError, ValidTransaction,
	},
};

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;
mod traversal;
mod types;
mod weights;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

pub use weights::WeightInfo;

pub use types::{
	Approval, Authorization, AuthorizationContext, CallRequirement, PendingPolicyUpdate, Policy,
	Registration,
};
use types::{ApprovalStatus, AuthorizationInspection};

pub type RuntimeCallOf<T> = <T as SystemConfig>::RuntimeCall;

/// Business rules are runtime-owned; this pallet owns proofs, policy changes and wrapper traversal.
pub trait AccountSecurityPolicy<T: Config> {
	/// Storage-free upper bound for this call's business rules, excluding native wrappers.
	fn inspection_weight(call: &RuntimeCallOf<T>) -> Weight;
	fn requirements(
		call: &RuntimeCallOf<T>,
		context: &AuthorizationContext<T>,
	) -> Result<Vec<CallRequirement<T::AccountId>>, Error<T>>;
	fn default_account(account: &T::AccountId) -> T::AccountId;

	/// Prepares the runtime's real default-account lookup, including linked-member reads.
	#[cfg(feature = "runtime-benchmarks")]
	fn prepare_benchmark_account(member: &T::AccountId, default_account: &T::AccountId);
}

/// Constructs native wallet signing bytes using the runtime's real transaction-extension tuple.
pub trait ApprovalPayloadProvider<Call> {
	fn signing_payload(
		call: Call,
		with_metadata_hash: bool,
	) -> Result<Vec<u8>, TransactionValidityError>;
}

#[frame_support::pallet]
#[allow(clippy::too_many_arguments)] // Flat policy fields are part of hardware-wallet review.
pub mod pallet {
	use super::*;

	#[pallet::config]
	pub trait Config:
		polkadot_sdk::frame_system::Config<
			RuntimeCall: Dispatchable<PostInfo = PostDispatchInfo>
			                 + From<Call<Self>>
			                 + IsSubType<ProxyCall<Self>>
			                 + IsSubType<MultisigCall<Self>>
			                 + IsSubType<UtilityCall<Self>>
			                 + IsSubType<Call<Self>>,
		> + ProxyConfig<RuntimeCall = RuntimeCallOf<Self>>
		+ MultisigConfig<RuntimeCall = RuntimeCallOf<Self>>
		+ UtilityConfig<RuntimeCall = RuntimeCallOf<Self>>
	{
		type CallAuthorizationPolicy: AccountSecurityPolicy<Self>;
		type SigningPayload: ApprovalPayloadProvider<RuntimeCallOf<Self>>;
		type Signature: Parameter
			+ Member
			+ MaxEncodedLen
			+ Into<Signature>
			+ Verify<Signer: IdentifyAccount<AccountId = Self::AccountId>>;
		type MaxCallDepth: Get<u32>;
		type MaxCalls: Get<u32>;
		type MaxSigners: Get<u32>;
		/// Allows the old quorum, new signers and a relayed main-account proof in one update.
		type MaxApprovals: Get<u32>;
		type MaxApprovalLifetime: Get<BlockNumberFor<Self>>;
		/// Consensus clock for recovery windows; independent of signature-expiry blocks.
		type CurrentTick: Get<Tick>;
		type SecurityCurrency: ReservableCurrency<Self::AccountId, Balance = u128>;
		type DepositBase: Get<u128>;
		type DepositPerSigner: Get<u128>;
		type WeightInfo: WeightInfo;
	}

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	/// One registration at the default account; linked members never duplicate it.
	#[pallet::storage]
	pub type Registrations<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, Registration<T>>;

	/// Next group authorization nonce. Never removed or reset by policy changes or linking.
	#[pallet::storage]
	pub type NextNonce<T: Config> = StorageMap<_, Blake2_128Concat, T::AccountId, u64, ValueQuery>;

	/// At most one delayed policy replacement per default account, separately deposit-backed.
	#[pallet::storage]
	pub type PendingPolicyUpdates<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, PendingPolicyUpdate<T>>;

	#[pallet::error]
	#[derive(EqNoBound, PartialEqNoBound)]
	pub enum Error<T> {
		/// The main account or a required policy/recovery/acceptance signature is missing.
		ApprovalRequired,
		/// A proxy origin or transfer destination could not be resolved to an account.
		InvalidProxy,
		/// Pre-dispatch state cannot safely authorize the nested call combination.
		UnsupportedComposition,
		/// Nested depth, call count, or a native multisig's signatory count exceeds its limit.
		CallLimitExceeded,
		/// Linking would discard a participant's policy or backup authority.
		IncompatibleAccountProtection,
		/// The default account has not enrolled protection.
		NotRegistered,
		/// Initial enrollment cannot overwrite an existing policy.
		AlreadyRegistered,
		/// Only a group's default account may manage its configuration.
		NotDefaultAccount,
		/// Signing roles overlap the protected group or recovery overlaps a policy signer.
		KeysNotIndependent,
		/// A threshold is invalid or signers are empty, duplicated or not sorted.
		InvalidPolicy,
		/// The approval expired or exceeds the allowed future lifetime.
		InvalidExpiry,
		/// The supplied nonce is not the group's next unused authorization nonce.
		InvalidNonce,
		/// The account group or current policy differs from the signed authorization.
		StalePolicy,
		/// A proof does not verify against its account and the exact canonical call payload.
		InvalidSignature,
		/// One account supplied more than one proof.
		DuplicateApproval,
		/// Native wallet signing context could not be constructed for this security call.
		InvalidSigningPayload,
		/// Only recovery-origin management may be submitted through native wrappers.
		IndirectAuthorization,
		/// Authorization nonces cannot wrap or be reused.
		CounterExhausted,
		/// A delayed replacement is already pending; cancel it before proposing another.
		PolicyUpdatePending,
		/// No pending replacement exists with this proposal's consumed authorization nonce.
		UnknownPolicyUpdate,
		/// The current tick has not reached the pending replacement's execution tick.
		RecoveryDelayActive,
		/// Adding the configured delay to the current tick would overflow.
		TickOverflow,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		Registered {
			account: T::AccountId,
		},
		PolicyUpdated {
			account: T::AccountId,
		},
		/// A failed child still consumes its nonce; inspect this result, not outer success.
		Executed {
			account: T::AccountId,
			nonce: u64,
			result: DispatchResult,
		},
		GroupLinked {
			default_account: T::AccountId,
		},
		PolicyUpdateScheduled {
			account: T::AccountId,
			nonce: u64,
			execute_after_tick: Tick,
		},
		PolicyUpdateCancelled {
			account: T::AccountId,
			nonce: u64,
		},
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Enroll the approval policy and backup atomically; every initial authority must accept.
		/// Policy fields stay flat so hardware wallets can label the choices being approved.
		#[pallet::call_index(0)]
		#[pallet::weight(<T as Config>::WeightInfo::register().saturating_add(Pallet::<T>::approval_weight(approvals)))]
		pub fn register(
			origin: OriginFor<T>,
			account: T::AccountId,
			signers: BoundedVec<T::AccountId, T::MaxSigners>,
			quorum: u16,
			recovery_account: T::AccountId,
			recovery_delay: Tick,
			authorization: Authorization<T>,
			approvals: BoundedVec<Approval<T>, T::MaxApprovals>,
		) -> DispatchResult {
			let submitter = ensure_signed(origin)?;
			let policy = Policy { signers, quorum, recovery_account, recovery_delay };

			Self::ensure_default(&account)?;
			ensure!(!Registrations::<T>::contains_key(&account), Error::<T>::AlreadyRegistered);
			Self::check_authorization(&account, &authorization)?;
			Self::validate_policy(&account, &policy)?;

			let call = Call::<T>::register {
				account: account.clone(),
				signers: policy.signers.clone(),
				quorum,
				recovery_account: policy.recovery_account.clone(),
				recovery_delay,
				authorization: authorization.clone(),
				approvals: BoundedVec::default(),
			}
			.into();
			let signers = Self::verify_approvals(&call, &approvals)?;
			Self::ensure_main_consent(&submitter, &account, &signers)?;
			for signer in &policy.signers {
				ensure!(signers.contains(signer), Error::<T>::ApprovalRequired);
			}
			ensure!(
				submitter == policy.recovery_account || signers.contains(&policy.recovery_account),
				Error::<T>::ApprovalRequired
			);

			Self::store_policy(&account, policy)?;
			Self::consume_nonce(&authorization)?;
			Self::deposit_event(Event::Registered { account });
			Ok(())
		}

		/// Normal signer changes use the current quorum immediately. Recovery uses the backup
		/// after the current tick delay; replacing the backup or delay uses the current quorum
		/// after that same delay. Main consent and new authorities' acceptance are always required.
		#[pallet::call_index(1)]
		#[pallet::weight(<T as Config>::WeightInfo::update_policy().max(<T as Config>::WeightInfo::schedule_policy_update()).saturating_add(Pallet::<T>::approval_weight(approvals)))]
		pub fn update_policy(
			origin: OriginFor<T>,
			account: T::AccountId,
			signers: BoundedVec<T::AccountId, T::MaxSigners>,
			quorum: u16,
			recovery_account: T::AccountId,
			recovery_delay: Tick,
			authorization: Authorization<T>,
			approvals: BoundedVec<Approval<T>, T::MaxApprovals>,
		) -> DispatchResult {
			let submitter = ensure_signed(origin)?;
			let policy = Policy { signers, quorum, recovery_account, recovery_delay };

			Self::ensure_default(&account)?;
			Self::check_authorization(&account, &authorization)?;
			let current = Registrations::<T>::get(&account).ok_or(Error::<T>::NotRegistered)?;
			Self::validate_policy(&account, &policy)?;

			let call = Call::<T>::update_policy {
				account: account.clone(),
				signers: policy.signers.clone(),
				quorum,
				recovery_account: policy.recovery_account.clone(),
				recovery_delay,
				authorization: authorization.clone(),
				approvals: BoundedVec::default(),
			}
			.into();
			let signers = Self::verify_approvals(&call, &approvals)?;
			Self::ensure_main_consent(&submitter, &account, &signers)?;

			let quorum_approved = Self::has_quorum(&current.policy, &signers);
			let changes_recovery = policy.recovery_account != current.policy.recovery_account ||
				policy.recovery_delay != current.policy.recovery_delay;
			if !quorum_approved {
				ensure!(!changes_recovery, Error::<T>::ApprovalRequired);
				ensure!(
					submitter == current.policy.recovery_account ||
						signers.contains(&current.policy.recovery_account),
					Error::<T>::ApprovalRequired
				);
			}
			if policy.recovery_account != current.policy.recovery_account {
				ensure!(
					submitter == policy.recovery_account ||
						signers.contains(&policy.recovery_account),
					Error::<T>::ApprovalRequired
				);
			}
			for signer in &policy.signers {
				if !current.policy.signers.contains(signer) {
					ensure!(signers.contains(signer), Error::<T>::ApprovalRequired);
				}
			}

			let delayed = !quorum_approved || changes_recovery;
			if delayed && current.policy.recovery_delay > 0 {
				ensure!(
					!PendingPolicyUpdates::<T>::contains_key(&account),
					Error::<T>::PolicyUpdatePending
				);
				let execute_after_tick = T::CurrentTick::get()
					.checked_add(current.policy.recovery_delay)
					.ok_or(Error::<T>::TickOverflow)?;
				let deposit = Self::storage_deposit(policy.signers.len() + 1);
				T::SecurityCurrency::reserve(&account, deposit)?;
				Self::retain_policy_accounts(&policy);
				PendingPolicyUpdates::<T>::insert(
					&account,
					PendingPolicyUpdate {
						policy,
						nonce: authorization.nonce,
						previous_policy_hash: authorization.policy_hash,
						execute_after_tick,
						deposit,
					},
				);
				Self::consume_nonce(&authorization)?;
				Self::deposit_event(Event::PolicyUpdateScheduled {
					account,
					nonce: authorization.nonce,
					execute_after_tick,
				});
				return Ok(())
			}

			Self::store_policy(&account, policy)?;
			Self::consume_nonce(&authorization)?;
			Self::deposit_event(Event::PolicyUpdated { account });
			Ok(())
		}

		/// Submit one full call/batch and its collected off-chain approvals. The main account
		/// signs the outer transaction or supplies its own proof when a relayer submits.
		/// Every execution consumes the group's approval nonce and therefore requires quorum,
		/// even when the child could also be submitted directly without approval.
		#[pallet::call_index(2)]
		#[pallet::weight(<T as Config>::WeightInfo::execute(call.encoded_size() as u32).saturating_add(Pallet::<T>::approval_weight(approvals)).saturating_add(Pallet::<T>::authorization_weight(call)).saturating_add(call.get_dispatch_info().call_weight))]
		pub fn execute(
			origin: OriginFor<T>,
			account: T::AccountId,
			call: Box<RuntimeCallOf<T>>,
			authorization: Authorization<T>,
			approvals: BoundedVec<Approval<T>, T::MaxApprovals>,
		) -> DispatchResultWithPostInfo {
			let submitter = ensure_signed(origin)?;
			Self::check_authorization(&account, &authorization)?;
			let current =
				Registrations::<T>::get(&authorization.group).ok_or(Error::<T>::NotRegistered)?;
			let context = AuthorizationContext {
				account: account.clone(),
				authorized_group: Some(authorization.group.clone()),
				executes: true,
			};
			Self::ensure_call_authorized(&call, &context)?;

			let security_call = Call::<T>::execute {
				account: account.clone(),
				call: call.clone(),
				authorization: authorization.clone(),
				approvals: BoundedVec::default(),
			}
			.into();
			let signers = Self::verify_approvals(&security_call, &approvals)?;
			Self::ensure_main_consent(&submitter, &account, &signers)?;
			ensure!(Self::has_quorum(&current.policy, &signers), Error::<T>::ApprovalRequired);

			Self::consume_nonce(&authorization)?;
			let inspection_weight = Self::authorization_weight(&call);
			let execution_weight = <T as Config>::WeightInfo::execute(call.encoded_size() as u32);
			let info = call.get_dispatch_info();
			let result = call.dispatch(frame_system::RawOrigin::Signed(account.clone()).into());
			let actual_weight = extract_actual_weight(&result, &info);
			Self::deposit_event(Event::Executed {
				account,
				nonce: authorization.nonce,
				result: result.map(|_| ()).map_err(|error| error.error),
			});
			Ok(Some(
				execution_weight
					.saturating_add(Self::approval_weight(&approvals))
					.saturating_add(inspection_weight)
					.saturating_add(actual_weight),
			)
			.into())
		}

		/// Complete an accepted policy replacement after its tick window. Anyone may pay the
		/// submission fee; all authority and new-key acceptance was captured at scheduling.
		#[pallet::call_index(3)]
		#[pallet::weight(<T as Config>::WeightInfo::complete_policy_update())]
		pub fn complete_policy_update(
			origin: OriginFor<T>,
			account: T::AccountId,
			update_nonce: u64,
		) -> DispatchResult {
			ensure_signed(origin)?;
			Self::ensure_default(&account)?;
			let pending =
				PendingPolicyUpdates::<T>::get(&account).ok_or(Error::<T>::UnknownPolicyUpdate)?;
			ensure!(pending.nonce == update_nonce, Error::<T>::UnknownPolicyUpdate);
			ensure!(
				T::CurrentTick::get() >= pending.execute_after_tick,
				Error::<T>::RecoveryDelayActive
			);
			let current = Registrations::<T>::get(&account).ok_or(Error::<T>::NotRegistered)?;
			ensure!(
				Some(current.policy).using_encoded(blake2_256) == pending.previous_policy_hash,
				Error::<T>::StalePolicy
			);
			Self::validate_policy(&account, &pending.policy)?;

			Self::store_policy(&account, pending.policy)?;
			// Invalidate signatures collected during the waiting period, even if the policy
			// later returns to an older configuration. Unrelated operations need not stop.
			let nonce = NextNonce::<T>::get(&account)
				.checked_add(1)
				.ok_or(Error::<T>::CounterExhausted)?;
			NextNonce::<T>::insert(&account, nonce);
			Self::deposit_event(Event::PolicyUpdated { account });
			Ok(())
		}

		/// Main plus either current authority can veto or withdraw the exact pending update.
		#[pallet::call_index(4)]
		#[pallet::weight(<T as Config>::WeightInfo::cancel_policy_update().saturating_add(Pallet::<T>::approval_weight(approvals)))]
		pub fn cancel_policy_update(
			origin: OriginFor<T>,
			account: T::AccountId,
			update_nonce: u64,
			authorization: Authorization<T>,
			approvals: BoundedVec<Approval<T>, T::MaxApprovals>,
		) -> DispatchResult {
			let submitter = ensure_signed(origin)?;
			Self::ensure_default(&account)?;
			Self::check_authorization(&account, &authorization)?;
			let pending =
				PendingPolicyUpdates::<T>::get(&account).ok_or(Error::<T>::UnknownPolicyUpdate)?;
			ensure!(pending.nonce == update_nonce, Error::<T>::UnknownPolicyUpdate);
			let current = Registrations::<T>::get(&account).ok_or(Error::<T>::NotRegistered)?;
			let call = Call::<T>::cancel_policy_update {
				account: account.clone(),
				update_nonce,
				authorization: authorization.clone(),
				approvals: BoundedVec::default(),
			}
			.into();
			let signers = Self::verify_approvals(&call, &approvals)?;
			Self::ensure_main_consent(&submitter, &account, &signers)?;
			let recovery_approved = submitter == current.policy.recovery_account ||
				signers.contains(&current.policy.recovery_account);
			ensure!(
				recovery_approved || Self::has_quorum(&current.policy, &signers),
				Error::<T>::ApprovalRequired
			);

			Self::clear_pending_policy(&account);
			Self::consume_nonce(&authorization)?;
			Self::deposit_event(Event::PolicyUpdateCancelled { account, nonce: update_nonce });
			Ok(())
		}
	}
}

impl<T: Config> From<Error<T>> for TransactionValidityError {
	fn from(error: Error<T>) -> Self {
		let encoded = error.encode();
		InvalidTransaction::Custom(100u8.saturating_add(encoded[0])).into()
	}
}

/// Mandatory enforcement for ordinary signed native calls, including their wrapper origins.
#[derive(Encode, Decode, DecodeWithMemTracking, DefaultNoBound, Clone, Eq, PartialEq, TypeInfo)]
#[scale_info(skip_type_params(T))]
pub struct CheckAccountSecurity<T> {
	#[codec(skip)]
	native_signature_weight: Weight,
	_marker: PhantomData<T>,
}

impl<T: Config + Send + Sync> core::fmt::Debug for CheckAccountSecurity<T> {
	fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
		f.write_str("CheckAccountSecurity")
	}
}

impl<T: Config + Send + Sync> CheckAccountSecurity<T> {
	pub fn new() -> Self {
		Self::default()
	}

	/// Set only by the runtime's outer-extrinsic adapter, from the actual signature variant.
	/// This field is absent from both the signed bytes and extension metadata.
	pub fn set_native_signature_weight(&mut self, weight: Weight) {
		self.native_signature_weight = weight;
	}
}

impl<T: Config + Send + Sync> TransactionExtension<RuntimeCallOf<T>> for CheckAccountSecurity<T> {
	const IDENTIFIER: &'static str = "CheckAccountSecurity";
	type Implicit = ();
	type Val = ();
	type Pre = ();

	fn weight(&self, call: &RuntimeCallOf<T>) -> Weight {
		Pallet::<T>::authorization_weight(call).saturating_add(self.native_signature_weight)
	}

	fn validate(
		&self,
		origin: DispatchOriginOf<RuntimeCallOf<T>>,
		call: &RuntimeCallOf<T>,
		_info: &DispatchInfoOf<RuntimeCallOf<T>>,
		_len: usize,
		_self_implicit: (),
		_inherited_implication: &impl Implication,
		_source: TransactionSource,
	) -> ValidateResult<(), RuntimeCallOf<T>> {
		if let Some(signer) = origin.as_signer() {
			Pallet::<T>::ensure_authorized(signer, call)?;
		}
		Ok((ValidTransaction::default(), (), origin))
	}

	// Inclusion runs validate again immediately before prepare; pool validation is not reused.
	sp_runtime::impl_tx_ext_default!(RuntimeCallOf<T>; prepare);
}

impl<T: Config> Pallet<T> {
	pub fn ensure_authorized(
		signer: &T::AccountId,
		call: &RuntimeCallOf<T>,
	) -> Result<(), Error<T>> {
		let context = AuthorizationContext {
			account: signer.clone(),
			authorized_group: None,
			executes: true,
		};
		Self::ensure_call_authorized(call, &context)
	}

	/// Native transaction signing bytes for this call with approvals removed. Use the runtime's
	/// fixed nonce-zero, immortal, zero-tip signing context, not the relayer's transaction context.
	/// Runtime/genesis binding and the native short-payload hashing rule are preserved.
	pub fn approval_payload(
		call: &RuntimeCallOf<T>,
		with_metadata_hash: bool,
	) -> Result<Vec<u8>, Error<T>> {
		let security: Option<&Call<T>> = call.is_sub_type();
		let mut canonical = security.ok_or(Error::<T>::InvalidSigningPayload)?.clone();
		match &mut canonical {
			Call::register { approvals, .. } |
			Call::update_policy { approvals, .. } |
			Call::cancel_policy_update { approvals, .. } |
			Call::execute { approvals, .. } => approvals.clear(),
			_ => return Err(Error::<T>::InvalidSigningPayload),
		}
		T::SigningPayload::signing_payload(canonical.into(), with_metadata_hash)
			.map_err(|_| Error::<T>::InvalidSigningPayload)
	}

	/// Charge each proof's actual scheme; native proofs never inherit the P256 surcharge.
	/// Management measurements still retain their native proofs as a conservative base.
	pub fn approval_weight(approvals: &[Approval<T>]) -> Weight {
		let mut counts = [0u32; 5];
		for approval in approvals {
			let index = match approval.signature.clone().into() {
				Signature::Ed25519(_) => 0,
				Signature::Sr25519(_) => 1,
				Signature::Ecdsa(_) => 2,
				Signature::Eth(_) => 3,
				Signature::P256(_) => 4,
			};
			counts[index] += 1;
		}

		let verifiers = [
			<T as Config>::WeightInfo::verify_ed25519,
			<T as Config>::WeightInfo::verify_sr25519,
			<T as Config>::WeightInfo::verify_ecdsa,
			<T as Config>::WeightInfo::verify_eth,
			<T as Config>::WeightInfo::verify_p256,
		];
		counts
			.into_iter()
			.zip(verifiers)
			.filter(|(count, _)| *count != 0)
			.fold(Weight::zero(), |weight, (count, verify)| weight.saturating_add(verify(count)))
	}

	fn check_authorization(
		account: &T::AccountId,
		authorization: &Authorization<T>,
	) -> Result<(), Error<T>> {
		ensure!(
			T::CallAuthorizationPolicy::default_account(account) == authorization.group,
			Error::<T>::StalePolicy
		);
		ensure!(
			NextNonce::<T>::get(&authorization.group) == authorization.nonce,
			Error::<T>::InvalidNonce
		);
		ensure!(authorization.nonce < u64::MAX, Error::<T>::CounterExhausted);

		let now = System::<T>::block_number();
		ensure!(now <= authorization.expires_at, Error::<T>::InvalidExpiry);
		ensure!(
			authorization.expires_at <= now.saturating_add(T::MaxApprovalLifetime::get()),
			Error::<T>::InvalidExpiry
		);

		let policy =
			Registrations::<T>::get(&authorization.group).map(|registration| registration.policy);
		ensure!(
			policy.using_encoded(blake2_256) == authorization.policy_hash,
			Error::<T>::StalePolicy
		);
		Ok(())
	}

	fn verify_approvals(
		call: &RuntimeCallOf<T>,
		approvals: &[Approval<T>],
	) -> Result<Vec<T::AccountId>, Error<T>> {
		let plain_payload = if approvals.iter().any(|approval| !approval.with_metadata_hash) {
			Self::approval_payload(call, false)?
		} else {
			vec![]
		};
		let metadata_payload = if approvals.iter().any(|approval| approval.with_metadata_hash) {
			Self::approval_payload(call, true)?
		} else {
			vec![]
		};

		let mut signers = vec![];
		for approval in approvals {
			ensure!(!signers.contains(&approval.signer), Error::<T>::DuplicateApproval);
			let payload =
				if approval.with_metadata_hash { &metadata_payload } else { &plain_payload };
			ensure!(
				approval.signature.verify(payload.as_slice(), &approval.signer),
				Error::<T>::InvalidSignature
			);
			signers.push(approval.signer.clone());
		}
		Ok(signers)
	}

	fn ensure_main_consent(
		submitter: &T::AccountId,
		account: &T::AccountId,
		signers: &[T::AccountId],
	) -> Result<(), Error<T>> {
		ensure!(submitter == account || signers.contains(account), Error::<T>::ApprovalRequired);
		Ok(())
	}

	fn consume_nonce(authorization: &Authorization<T>) -> Result<(), Error<T>> {
		let next = authorization.nonce.checked_add(1).ok_or(Error::<T>::CounterExhausted)?;
		NextNonce::<T>::insert(&authorization.group, next);
		Ok(())
	}

	/// Inspect one complete call tree in its effective authorization context. The executing
	/// entry point must separately verify the authorized group's quorum.
	pub fn ensure_call_authorized(
		call: &RuntimeCallOf<T>,
		context: &AuthorizationContext<T>,
	) -> Result<(), Error<T>> {
		let mut inspection = AuthorizationInspection::default();
		Self::inspect(call, context, &mut inspection, 0)?;
		Self::validate_composition(&inspection)?;
		ensure!(!inspection.missing_approval, Error::<T>::ApprovalRequired);
		Ok(())
	}

	fn has_quorum(policy: &Policy<T>, signers: &[T::AccountId]) -> bool {
		policy.signers.iter().filter(|signer| signers.contains(signer)).count() >=
			usize::from(policy.quorum)
	}

	fn store_policy(account: &T::AccountId, policy: Policy<T>) -> DispatchResult {
		let previous = Registrations::<T>::get(account);

		// Acquire replacement ownership first: a shared unfunded signer must never be
		// reaped between its pending/old policy and its new active policy.
		Self::retain_policy_accounts(&policy);
		Self::clear_pending_policy(account);

		let old_deposit =
			previous.as_ref().map(|registration| registration.deposit).unwrap_or_default();
		let deposit = Self::storage_deposit(policy.signers.len() + 1);
		if deposit > old_deposit {
			T::SecurityCurrency::reserve(account, deposit - old_deposit)?;
		} else {
			T::SecurityCurrency::unreserve(account, old_deposit - deposit);
		}

		if let Some(previous) = previous {
			Self::release_policy_accounts(&previous.policy);
		}
		Registrations::<T>::insert(account, Registration { policy, deposit });
		Ok(())
	}

	fn clear_pending_policy(account: &T::AccountId) {
		if let Some(pending) = PendingPolicyUpdates::<T>::take(account) {
			T::SecurityCurrency::unreserve(account, pending.deposit);
			Self::release_policy_accounts(&pending.policy);
		}
	}

	/// Each active or pending policy owns one sufficient reference per independent authority.
	/// This pays for a balance-free System record without granting provider-dependent consumers.
	fn retain_policy_accounts(policy: &Policy<T>) {
		for account in policy.signers.iter().chain(once(&policy.recovery_account)) {
			System::<T>::inc_sufficients(account);
			SystemAccount::<T>::mutate(account, |record| {
				if record.nonce.is_zero() {
					record.nonce = One::one();
				}
			});
		}
	}

	fn release_policy_accounts(policy: &Policy<T>) {
		for account in policy.signers.iter().chain(once(&policy.recovery_account)) {
			System::<T>::dec_sufficients(account);
		}
	}

	fn validate_policy(group: &T::AccountId, policy: &Policy<T>) -> Result<(), Error<T>> {
		let recovery = &policy.recovery_account;
		ensure!(
			T::CallAuthorizationPolicy::default_account(recovery) != *group,
			Error::<T>::KeysNotIndependent
		);
		ensure!(policy.quorum > 0, Error::<T>::InvalidPolicy);
		ensure!(usize::from(policy.quorum) <= policy.signers.len(), Error::<T>::InvalidPolicy);
		ensure!(policy.signers.windows(2).all(|pair| pair[0] < pair[1]), Error::<T>::InvalidPolicy);

		for signer in &policy.signers {
			ensure!(signer != recovery, Error::<T>::KeysNotIndependent);
			ensure!(
				T::CallAuthorizationPolicy::default_account(signer) != *group,
				Error::<T>::KeysNotIndependent
			);
		}
		Ok(())
	}

	fn ensure_default(account: &T::AccountId) -> Result<(), Error<T>> {
		ensure!(
			T::CallAuthorizationPolicy::default_account(account) == *account,
			Error::<T>::NotDefaultAccount
		);
		Ok(())
	}

	fn storage_deposit(signers: usize) -> u128 {
		<T as Config>::DepositBase::get()
			.saturating_add(T::DepositPerSigner::get().saturating_mul(signers as u128))
	}

	fn approval_status(
		account: &T::AccountId,
		context: &AuthorizationContext<T>,
	) -> ApprovalStatus {
		let group = T::CallAuthorizationPolicy::default_account(account);
		if !Registrations::<T>::contains_key(&group) {
			return ApprovalStatus::Unprotected
		}
		if context.authorized_group.as_ref() == Some(&group) {
			ApprovalStatus::Approved
		} else {
			ApprovalStatus::Missing
		}
	}
}

pub struct ProviderWeights<T>(PhantomData<T>);
impl<T: Config> AccountSecurityProviderWeightInfo for ProviderWeights<T> {
	fn on_operational_registration() -> Weight {
		<T as Config>::WeightInfo::on_operational_registration()
			.max(<T as Config>::WeightInfo::on_operational_registration_existing())
	}
}

impl<T: Config> AccountSecurityProvider<T::AccountId> for Pallet<T> {
	type Weights = ProviderWeights<T>;

	fn on_operational_registration(
		default_account: &T::AccountId,
		participants: &[T::AccountId],
	) -> DispatchResult {
		let mut accounts = participants.to_vec();
		accounts.push(default_account.clone());
		accounts.sort();
		accounts.dedup();

		let mut registrations: Vec<(T::AccountId, Registration<T>)> = vec![];
		let mut next_nonces = vec![];
		for account in &accounts {
			let nonce = NextNonce::<T>::get(account)
				.checked_add(1)
				.ok_or(Error::<T>::CounterExhausted)?;
			next_nonces.push((account.clone(), nonce));

			let Some(registration) = Registrations::<T>::get(account) else { continue };
			if let Some((_, existing)) = registrations.first() {
				ensure!(
					existing.policy == registration.policy,
					Error::<T>::IncompatibleAccountProtection
				);
			}
			registrations.push((account.clone(), registration));
		}

		if let Some((_, registration)) = registrations.first() {
			let policy = &registration.policy;
			for account in &accounts {
				Self::validate_policy(account, policy)?;
			}

			Self::store_policy(default_account, policy.clone())?;

			for (account, registration) in &registrations {
				if account == default_account {
					continue
				}
				Registrations::<T>::remove(account);
				T::SecurityCurrency::unreserve(account, registration.deposit);
				Self::release_policy_accounts(&registration.policy);
			}

			Self::deposit_event(Event::GroupLinked { default_account: default_account.clone() });
		}

		for (account, nonce) in next_nonces {
			Self::clear_pending_policy(&account);
			NextNonce::<T>::insert(account, nonce);
		}
		Ok(())
	}
}
