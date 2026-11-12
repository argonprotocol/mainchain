use codec::Encode;
use frame_support::{
	traits::{Get, IsSubType},
	weights::Weight,
};
use pallet_multisig::{Multisigs, Pallet as Multisig};
use pallet_utility::derivative_account_id;
use polkadot_sdk::*;
use sp_core::hashing::blake2_256;
use sp_runtime::traits::StaticLookup;

use super::{
	AccountSecurityPolicy, ApprovalStatus, AuthorizationContext, AuthorizationInspection,
	Call as SecurityCall, CallRequirement, Config, Error, MultisigCall, MultisigConfig, Pallet,
	ProxyCall, Registrations, RuntimeCallOf, UtilityCall, WeightInfo,
};

/// Structural dispatch shapes only. Approval decisions belong to rules, not these variants.
enum NestedCalls<'a, T: Config> {
	Leaf,
	NoDispatch,
	Batch(&'a [RuntimeCallOf<T>]),
	Either(&'a RuntimeCallOf<T>, &'a RuntimeCallOf<T>),
	As(&'a RuntimeCallOf<T>, AuthorizationContext<T>),
}

impl<T: Config> Pallet<T> {
	/// Charges call inspection and bounded storage work before dispatch.
	pub fn authorization_weight(call: &RuntimeCallOf<T>) -> Weight {
		// Inspection visits at most one rejecting node beyond the limit. Share that
		// budget across branches so the weight calculation is bounded in the same way.
		let mut remaining = T::MaxCalls::get().saturating_add(1);
		Self::call_inspection_weight(call, 0, &mut remaining)
	}

	pub(super) fn inspect(
		call: &RuntimeCallOf<T>,
		context: &AuthorizationContext<T>,
		inspection: &mut AuthorizationInspection,
		depth: u32,
	) -> Result<(), Error<T>> {
		inspection.visited_calls = inspection.visited_calls.saturating_add(1);
		if depth > T::MaxCallDepth::get() {
			return Err(Error::<T>::CallLimitExceeded)
		}
		if inspection.visited_calls > T::MaxCalls::get() {
			return Err(Error::<T>::CallLimitExceeded)
		}

		let security: Option<&SecurityCall<T>> = call.is_sub_type();
		if let Some(security) = security {
			if context.authorized_group.is_some() {
				return Err(Error::<T>::UnsupportedComposition)
			}
			if depth != 0 {
				// Only recovery origins may come through native Proxy/Multisig. Main-account
				// consent must still be a real detached proof, never an inherited proxy origin.
				let recovery_origin = match security {
					SecurityCall::register { recovery_account, .. } =>
						context.account == *recovery_account,
					SecurityCall::update_policy { account, recovery_account, .. } =>
						context.account == *recovery_account ||
							Registrations::<T>::get(account).is_some_and(|registration| {
								registration.policy.recovery_account == context.account
							}),
					SecurityCall::cancel_policy_update { account, .. } =>
						Registrations::<T>::get(account).is_some_and(|registration| {
							registration.policy.recovery_account == context.account
						}),
					_ => return Err(Error::<T>::UnsupportedComposition),
				};
				if !recovery_origin {
					return Err(Error::<T>::IndirectAuthorization)
				}
				inspection.leaf_calls = inspection.leaf_calls.saturating_add(1);
				inspection.changes_authority = true;
				inspection.touches_protection = true;
			}
			return Ok(())
		}

		let multisig: Option<&MultisigCall<T>> = call.is_sub_type();
		if let Some(
			MultisigCall::as_multi { other_signatories, .. } |
			MultisigCall::as_multi_threshold_1 { other_signatories, .. } |
			MultisigCall::approve_as_multi { other_signatories, .. },
		) = multisig
		{
			// Match the native bound before allocating or sorting the account vector.
			if other_signatories.len() >= <T as MultisigConfig>::MaxSignatories::get() as usize {
				return Err(Error::<T>::CallLimitExceeded)
			}
		}
		if let Some(
			MultisigCall::as_multi { threshold, other_signatories, .. } |
			MultisigCall::approve_as_multi { threshold, other_signatories, .. },
		) = multisig
		{
			// approve_as_multi -> as_multi can make a protected child execute in this batch
			// although pre-dispatch state predicts it will remain pending. Include vote-only
			// calls in the composition guard, not just calls predicted to execute.
			inspection.changes_authority = true;
			if !inspection.touches_protection {
				let account =
					Self::multisig_account(&context.account, *threshold, other_signatories);
				inspection.touches_protection =
					Self::approval_status(&account, context) != ApprovalStatus::Unprotected;
			}
		}

		for requirement in T::CallAuthorizationPolicy::requirements(call, context)? {
			match requirement {
				CallRequirement::RequireAccountLinkApproval(participants) => {
					let authorized_registration =
						context.authorized_group.as_ref().and_then(Registrations::<T>::get);

					for participant in participants {
						let default_account =
							T::CallAuthorizationPolicy::default_account(&participant);
						let Some(registration) = Registrations::<T>::get(default_account) else {
							continue
						};
						inspection.touches_protection = true;

						let Some(authorized_registration) = &authorized_registration else {
							inspection.missing_approval |= context.executes;
							continue
						};

						if authorized_registration.policy != registration.policy {
							return Err(Error::<T>::IncompatibleAccountProtection)
						}
					}
				},
				CallRequirement::RequireApproval(account) => {
					match Self::approval_status(&account, context) {
						ApprovalStatus::Unprotected => {},
						ApprovalStatus::Approved => {
							inspection.touches_protection = true;
						},
						ApprovalStatus::Missing => {
							inspection.touches_protection = true;
							inspection.missing_approval |= context.executes;
						},
					}
				},
				CallRequirement::CheckProtectionForBatch(account) => {
					if !inspection.touches_protection {
						inspection.touches_protection =
							Self::approval_status(&account, context) != ApprovalStatus::Unprotected;
					}
				},
				CallRequirement::ChangesAuthorization => {
					// A verified approval binds the entire batch, including its authority changes.
					if Self::approval_status(&context.account, context) != ApprovalStatus::Approved
					{
						inspection.changes_authority = true;
					}
				},
			}
		}

		match Self::nested_calls(call, context)? {
			NestedCalls::Leaf => inspection.leaf_calls = inspection.leaf_calls.saturating_add(1),
			NestedCalls::NoDispatch => {},
			NestedCalls::Batch(calls) =>
				for call in calls {
					Self::inspect(call, context, inspection, depth + 1)?;
				},
			NestedCalls::Either(main, fallback) => {
				Self::inspect(main, context, inspection, depth + 1)?;
				Self::inspect(fallback, context, inspection, depth + 1)?;
			},
			NestedCalls::As(call, child) => Self::inspect(call, &child, inspection, depth + 1)?,
		}

		Ok(())
	}

	fn nested_calls<'a>(
		call: &'a RuntimeCallOf<T>,
		context: &AuthorizationContext<T>,
	) -> Result<NestedCalls<'a, T>, Error<T>> {
		let utility: Option<&UtilityCall<T>> = call.is_sub_type();
		if let Some(utility) = utility {
			return Ok(match utility {
				UtilityCall::batch { calls } |
				UtilityCall::batch_all { calls } |
				UtilityCall::force_batch { calls } => NestedCalls::Batch(calls),
				UtilityCall::if_else { main, fallback } =>
					NestedCalls::Either(main.as_ref(), fallback.as_ref()),
				UtilityCall::as_derivative { index, call } => {
					let account = &context.account;
					let mut child = context.clone();
					child.account = derivative_account_id(account.clone(), *index);
					NestedCalls::As(call.as_ref(), child)
				},
				UtilityCall::dispatch_as { .. } |
				UtilityCall::dispatch_as_fallible { .. } |
				UtilityCall::with_weight { .. } => NestedCalls::NoDispatch,
				UtilityCall::__Ignore(..) => NestedCalls::Leaf,
			});
		}

		let proxy: Option<&ProxyCall<T>> = call.is_sub_type();
		if let Some(proxy) = proxy {
			return Ok(match proxy {
				ProxyCall::proxy { real, call, .. } |
				ProxyCall::proxy_announced { real, call, .. } => {
					let real =
						T::Lookup::lookup(real.clone()).map_err(|_| Error::<T>::InvalidProxy)?;

					let mut child = context.clone();
					child.account = real;
					NestedCalls::As(call.as_ref(), child)
				},
				_ => NestedCalls::Leaf,
			});
		}

		let multisig: Option<&MultisigCall<T>> = call.is_sub_type();
		if let Some(multisig) = multisig {
			return Ok(match multisig {
				MultisigCall::as_multi_threshold_1 { other_signatories, call } => {
					let account = &context.account;
					let mut child = context.clone();
					child.account = Self::multisig_account(account, 1, other_signatories);
					NestedCalls::As(call.as_ref(), child)
				},
				MultisigCall::as_multi { threshold, other_signatories, call, .. } => {
					let account = &context.account;
					let multisig = Self::multisig_account(account, *threshold, other_signatories);
					let call_hash = blake2_256(&call.encode());
					let executes = Multisigs::<T>::get(&multisig, call_hash)
						.map(|pending| {
							pending.approvals.len() +
								usize::from(!pending.approvals.contains(account)) >=
								usize::from(*threshold)
						})
						.unwrap_or(false);

					let mut child = context.clone();
					child.account = multisig;
					child.executes &= executes;
					NestedCalls::As(call.as_ref(), child)
				},
				_ => NestedCalls::Leaf,
			});
		}

		Ok(NestedCalls::Leaf)
	}

	pub(super) fn validate_composition(
		inspection: &AuthorizationInspection,
	) -> Result<(), Error<T>> {
		if inspection.leaf_calls <= 1 {
			return Ok(())
		}
		if !inspection.changes_authority {
			return Ok(())
		}

		// Pre-dispatch inspection can miss new exit protection or a multisig child made
		// executable by an earlier vote. Reject mixed authority changes conservatively,
		// even when exit approvals are present.
		if inspection.touches_protection {
			return Err(Error::<T>::UnsupportedComposition)
		}
		Ok(())
	}

	pub(super) fn multisig_account(
		account: &T::AccountId,
		threshold: u16,
		others: &[T::AccountId],
	) -> T::AccountId {
		let mut signatories = others.to_vec();
		signatories.push(account.clone());
		signatories.sort();
		Multisig::<T>::multi_account_id(&signatories, threshold)
	}

	fn call_inspection_weight(call: &RuntimeCallOf<T>, depth: u32, remaining: &mut u32) -> Weight {
		if *remaining == 0 {
			return Weight::zero()
		}
		*remaining -= 1;

		let leaf = <T as Config>::WeightInfo::authorization_leaf();
		if depth > T::MaxCallDepth::get() {
			return leaf
		}

		let security: Option<&SecurityCall<T>> = call.is_sub_type();
		if let Some(security) = security {
			// Native extension inspection stops here; execute charges its own child traversal.
			if depth != 0 &&
				matches!(
					security,
					SecurityCall::update_policy { .. } | SecurityCall::cancel_policy_update { .. }
				) {
				return leaf.saturating_add(<T as Config>::WeightInfo::authorization_protection())
			}
			return leaf
		}
		let policy = T::CallAuthorizationPolicy::inspection_weight(call);

		let utility: Option<&UtilityCall<T>> = call.is_sub_type();
		if let Some(utility) = utility {
			match utility {
				UtilityCall::batch { calls } |
				UtilityCall::batch_all { calls } |
				UtilityCall::force_batch { calls } => {
					let mut weight =
						policy.saturating_add(<T as Config>::WeightInfo::authorization_batch());
					for call in calls {
						if *remaining == 0 {
							break
						}
						weight = weight.saturating_add(Self::call_inspection_weight(
							call,
							depth + 1,
							remaining,
						));
					}
					return weight
				},
				UtilityCall::if_else { main, fallback } =>
					return policy
						.saturating_add(<T as Config>::WeightInfo::authorization_batch())
						.saturating_add(Self::call_inspection_weight(main, depth + 1, remaining))
						.saturating_add(Self::call_inspection_weight(
							fallback,
							depth + 1,
							remaining,
						)),
				UtilityCall::as_derivative { call, .. } =>
					return policy
						.saturating_add(<T as Config>::WeightInfo::authorization_derivative())
						.saturating_add(Self::call_inspection_weight(call, depth + 1, remaining)),
				UtilityCall::dispatch_as { .. } |
				UtilityCall::dispatch_as_fallible { .. } |
				UtilityCall::with_weight { .. } => return policy.saturating_add(leaf),
				UtilityCall::__Ignore(..) => return policy.saturating_add(leaf),
			}
		}

		let proxy: Option<&ProxyCall<T>> = call.is_sub_type();
		if let Some(ProxyCall::proxy { call, .. } | ProxyCall::proxy_announced { call, .. }) = proxy
		{
			return policy
				// Derivative inspection also bounds a proxy's simpler account-context switch.
				.saturating_add(<T as Config>::WeightInfo::authorization_derivative())
				.saturating_add(Self::call_inspection_weight(call, depth + 1, remaining))
		}

		let multisig: Option<&MultisigCall<T>> = call.is_sub_type();
		match multisig {
			Some(
				MultisigCall::as_multi { other_signatories, .. } |
				MultisigCall::as_multi_threshold_1 { other_signatories, .. } |
				MultisigCall::approve_as_multi { other_signatories, .. },
			) if other_signatories.len() >=
				<T as MultisigConfig>::MaxSignatories::get() as usize =>
				leaf,
			Some(MultisigCall::as_multi { other_signatories, call, .. }) => {
				// Native inspection hashes every as_multi child, even when an outer
				// multisig is pending. Nested wrappers therefore each charge their bytes.
				let bytes = call.encoded_size().min(u32::MAX as usize) as u32;
				let signatories = other_signatories.len() as u32;
				policy
					.saturating_add(<T as Config>::WeightInfo::authorization_multisig(
						signatories,
						bytes,
					))
					.saturating_add(Self::call_inspection_weight(call, depth + 1, remaining))
			},
			Some(MultisigCall::as_multi_threshold_1 { other_signatories, call }) => policy
				.saturating_add(<T as Config>::WeightInfo::authorization_multisig_derivation(
					other_signatories.len() as u32,
				))
				.saturating_add(Self::call_inspection_weight(call, depth + 1, remaining)),
			Some(MultisigCall::approve_as_multi { other_signatories, .. }) => policy
				.saturating_add(<T as Config>::WeightInfo::authorization_multisig_derivation(
					other_signatories.len() as u32,
				))
				.saturating_add(<T as Config>::WeightInfo::authorization_protection()),
			// Policy benchmarks include their leaf visit; unrelated leaves use the generic cost.
			_ => policy.max(leaf),
		}
	}
}
