// Error code for InvalidTransaction::Custom when sponsored fee is too high
const INVALID_TX_SPONSORED_FEE_TOO_HIGH: u8 = 1;
use crate::pallet::{Config, Event, Pallet};
use alloc::vec::Vec;
use codec::EncodeLike;
use frame_support::{
	dispatch::{CheckIfFeeless, DispatchInfo, PostDispatchInfo},
	traits::InstanceFilter,
};
use pallet_prelude::{
	argon_primitives::{
		CallFeeRefundProvider, CallTxPoolKeyProvider, CallTxValidityProvider,
		FeelessCallTxPoolKeyProvider, TransactionSponsorProvider, TxSponsor,
	},
	sp_runtime::traits::{
		DispatchOriginOf, Implication, PostDispatchInfoOf, StaticLookup, TransactionExtension, Zero,
	},
	Decode, DecodeWithMemTracking, DispatchInfoOf, DispatchResult, Encode, OriginFor, OriginTrait,
	TransactionSource, TransactionValidityError, ValidTransaction, ValidateResult, Weight, *,
};
use pallet_transaction_payment::OnChargeTransaction;
use polkadot_sdk::frame_support::traits::IsSubType;
use scale_info::{StaticTypeInfo, TypeInfo};
use sp_runtime::traits::{self, BlockNumberProvider, Hash};
use Intermediate::*;

/// A [`TransactionExtension`] that checks if a call can be feeless and allows protecting against
/// dos by providing a unique tx pool key
#[derive(Encode, Decode, DecodeWithMemTracking, Clone, Eq, PartialEq)]
pub struct CheckFeeWrapper<T, S>(pub S, core::marker::PhantomData<T>);

// Make this extension "invisible" from the outside (ie metadata type information)
impl<T, S: StaticTypeInfo> TypeInfo for CheckFeeWrapper<T, S> {
	type Identity = S;
	fn type_info() -> scale_info::Type {
		S::type_info()
	}
}

impl<T, S: Encode> core::fmt::Debug for CheckFeeWrapper<T, S> {
	#[cfg(feature = "std")]
	fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
		write!(f, "CheckFeelessCall<{:?}>", self.0.encode())
	}
	#[cfg(not(feature = "std"))]
	fn fmt(&self, _: &mut core::fmt::Formatter) -> core::fmt::Result {
		Ok(())
	}
}

impl<T, S> From<S> for CheckFeeWrapper<T, S> {
	fn from(s: S) -> Self {
		Self(s, core::marker::PhantomData)
	}
}

pub enum Intermediate<T, O, A> {
	/// The wrapped extension should be applied.
	RequiresFee {
		inner: T,
		transaction_origin: O,
		delegated_origin: O,
		tx_sponsor: A,
		refund_fee_on_success: bool,
	},
	/// The wrapped extension should be skipped.
	Feeless(O),
}

#[derive(Decode)]
struct AnnouncementRecord<AccountId, CallHash, BlockNumber> {
	real: AccountId,
	call_hash: CallHash,
	height: BlockNumber,
}

impl<T, S> CheckFeeWrapper<T, S>
where
	T: Config + pallet_transaction_payment::Config + pallet_proxy::Config + pallet_utility::Config,
	// Ensure the proxy pallet's call type is the same RuntimeCall as frame_system.
	T: pallet_proxy::Config<RuntimeCall = RuntimeCallOf<T>>,
	T: pallet_utility::Config<RuntimeCall = RuntimeCallOf<T>>,
	RuntimeCallOf<T>: IsSubType<pallet_proxy::Call<T>> + IsSubType<pallet_utility::Call<T>>,
{
	// If the outer call is a proxy wrapper, unwrap to the effective inner call (recursing through
	// nested proxy wrappers). Non-proxy calls return `None`.
	fn unwrap_proxy(call: &RuntimeCallOf<T>) -> Option<&RuntimeCallOf<T>> {
		if let Some(
			pallet_proxy::Call::proxy { call, .. } |
			pallet_proxy::Call::proxy_announced { call, .. },
		) = <RuntimeCallOf<T> as IsSubType<pallet_proxy::Call<T>>>::is_sub_type(call)
		{
			return Some(Self::unwrap_proxy(call.as_ref()).unwrap_or(call.as_ref()));
		}
		None
	}

	fn validated_pool_key_context(
		call: &RuntimeCallOf<T>,
		signer: Option<T::AccountId>,
	) -> (&RuntimeCallOf<T>, Option<T::AccountId>) {
		let Some(signer) = signer else {
			return (call, None);
		};

		if let Some(proxy_call) =
			<RuntimeCallOf<T> as IsSubType<pallet_proxy::Call<T>>>::is_sub_type(call)
		{
			match proxy_call {
				pallet_proxy::Call::proxy { real, force_proxy_type, call: inner_call } => {
					let Ok(real) = T::Lookup::lookup(real.clone()) else {
						return (call, Some(signer));
					};

					return Self::validated_proxy_pool_key_context(
						call,
						Some(signer.clone()),
						real,
						signer,
						false,
						force_proxy_type.clone(),
						inner_call.as_ref(),
					);
				},
				pallet_proxy::Call::proxy_announced {
					delegate,
					real,
					force_proxy_type,
					call: inner_call,
				} => {
					let Ok(delegate) = T::Lookup::lookup(delegate.clone()) else {
						return (call, Some(signer));
					};
					let Ok(real) = T::Lookup::lookup(real.clone()) else {
						return (call, Some(signer));
					};

					return Self::validated_proxy_pool_key_context(
						call,
						Some(signer),
						real,
						delegate,
						true,
						force_proxy_type.clone(),
						inner_call.as_ref(),
					);
				},
				_ => {},
			}
		}

		(call, Some(signer))
	}

	fn push_pool_key(validity: &mut ValidTransaction, key: Vec<u8>) {
		if !validity.provides.contains(&key) {
			validity.provides.push(key);
		}
	}

	fn collect_pool_keys(call: &RuntimeCallOf<T>, signer: Option<T::AccountId>) -> Vec<Vec<u8>> {
		let (call, signer) = Self::validated_pool_key_context(call, signer);

		if let Some(utility_call) =
			<RuntimeCallOf<T> as IsSubType<pallet_utility::Call<T>>>::is_sub_type(call)
		{
			let calls = match utility_call {
				pallet_utility::Call::batch { calls } |
				pallet_utility::Call::batch_all { calls } |
				pallet_utility::Call::force_batch { calls } => Some(calls),
				_ => None,
			};

			if let Some(calls) = calls {
				let mut keys = Vec::new();
				for inner_call in calls.iter() {
					for key in Self::collect_pool_keys(inner_call, signer.clone()) {
						if !keys.contains(&key) {
							keys.push(key);
						}
					}
				}
				if !keys.is_empty() {
					keys.push(T::Hashing::hash_of(&(b"batch", &keys)).as_ref().to_vec());
				}
				return keys;
			}
		}

		T::CallTxPoolKeyProviders::key_for(call, signer.as_ref()).into_iter().collect()
	}

	fn validate_freshness(
		call: &RuntimeCallOf<T>,
		signer: Option<T::AccountId>,
	) -> Result<(), TransactionValidityError> {
		let (call, signer) = Self::validated_pool_key_context(call, signer);

		if let Some(utility_call) =
			<RuntimeCallOf<T> as IsSubType<pallet_utility::Call<T>>>::is_sub_type(call)
		{
			let calls = match utility_call {
				pallet_utility::Call::batch { calls } |
				pallet_utility::Call::batch_all { calls } |
				pallet_utility::Call::force_batch { calls } => Some(calls),
				_ => None,
			};

			if let Some(calls) = calls {
				for inner_call in calls.iter() {
					Self::validate_freshness(inner_call, signer.clone())?;
				}
				return Ok(());
			}
		}

		T::CallTxValidityProviders::validate(call, signer.as_ref())
	}

	fn validated_proxy_pool_key_context<'a>(
		call: &'a RuntimeCallOf<T>,
		fallback_signer: Option<T::AccountId>,
		real: <T as frame_system::Config>::AccountId,
		delegate: <T as frame_system::Config>::AccountId,
		require_mature_announcement: bool,
		force_proxy_type: Option<T::ProxyType>,
		inner_call: &'a RuntimeCallOf<T>,
	) -> (&'a RuntimeCallOf<T>, Option<T::AccountId>) {
		let Ok(def) = pallet_proxy::Pallet::<T>::find_proxy(&real, &delegate, force_proxy_type)
		else {
			return (call, fallback_signer);
		};
		if require_mature_announcement &&
			!Self::has_mature_announcement(&delegate, &real, inner_call, def.delay)
		{
			return (call, fallback_signer);
		}
		if (!require_mature_announcement && !def.delay.is_zero()) ||
			!Self::proxy_call_is_allowed(&def, inner_call)
		{
			return (call, fallback_signer);
		}

		Self::validated_pool_key_context(inner_call, Some(real))
	}

	fn has_mature_announcement(
		delegate: &T::AccountId,
		real: &T::AccountId,
		inner_call: &RuntimeCallOf<T>,
		delay: <<T as pallet_proxy::Config>::BlockNumberProvider as sp_runtime::traits::BlockNumberProvider>::BlockNumber,
	) -> bool {
		let now = T::BlockNumberProvider::current_block_number();
		let call_hash = T::CallHasher::hash_of(inner_call);
		let (announcements, _) = pallet_proxy::Pallet::<T>::announcements(delegate.clone());

		announcements.iter().any(|announcement| {
			let Ok(announcement) = AnnouncementRecord::<
				T::AccountId,
				<T::CallHasher as Hash>::Output,
				<<T as pallet_proxy::Config>::BlockNumberProvider as BlockNumberProvider>::BlockNumber,
			>::decode(&mut &announcement.encode()[..]) else {
				return false;
			};

			announcement.real == *real &&
				announcement.call_hash == call_hash &&
				now.saturating_sub(announcement.height) >= delay
		})
	}

	fn proxy_call_is_allowed(
		def: &pallet_proxy::ProxyDefinition<
			T::AccountId,
			T::ProxyType,
			<<T as pallet_proxy::Config>::BlockNumberProvider as sp_runtime::traits::BlockNumberProvider>::BlockNumber,
		>,
		call: &RuntimeCallOf<T>,
	) -> bool {
		// Mirror pallet_proxy::do_proxy authorization so tx-pool key derivation only treats a
		// wrapped call as coming from `real` when pallet_proxy would actually allow it to dispatch.
		match <RuntimeCallOf<T> as IsSubType<pallet_proxy::Call<T>>>::is_sub_type(call) {
			Some(pallet_proxy::Call::add_proxy { proxy_type, .. }) |
			Some(pallet_proxy::Call::remove_proxy { proxy_type, .. })
				if !def.proxy_type.is_superset(proxy_type) =>
				false,
			Some(pallet_proxy::Call::remove_proxies { .. }) |
			Some(pallet_proxy::Call::kill_pure { .. })
				if def.proxy_type != T::ProxyType::default() =>
				false,
			_ => def.proxy_type.filter(call),
		}
	}
}

type RuntimeCallOf<T> = <T as frame_system::Config>::RuntimeCall;

impl<T,S> TransactionExtension<RuntimeCallOf<T>> for CheckFeeWrapper<T, S>
where
    T: Config + pallet_transaction_payment::Config + pallet_proxy::Config + pallet_utility::Config + Send + Sync,
    T: pallet_proxy::Config<RuntimeCall = RuntimeCallOf<T>>,
    T: pallet_utility::Config<RuntimeCall = RuntimeCallOf<T>>,
    RuntimeCallOf<T>: CheckIfFeeless<Origin = OriginFor<T>>
        + Dispatchable<Info = DispatchInfo, PostInfo = PostDispatchInfo>
        + IsSubType<pallet_proxy::Call<T>>
        + IsSubType<pallet_utility::Call<T>>,
    <<T as pallet_transaction_payment::Config>::OnChargeTransaction as OnChargeTransaction<T>>::Balance:
        EncodeLike<T::Balance> + From<T::Balance> + Into<T::Balance>,
    S: TransactionExtension<RuntimeCallOf<T>, Pre = pallet_transaction_payment::Pre<T>, Val = pallet_transaction_payment::Val<T>>,
    <T as pallet_prelude::frame_system::Config>::RuntimeOrigin: pallet_prelude::Debug {

    // From the outside this extension should be "invisible", because it just extends the wrapped
    // extension with an extra check in `pre_dispatch` and `post_dispatch`. Thus, we should forward
    // the identifier of the wrapped extension to let wallets see this extension as it would only be
    // the wrapped extension itself.
    const IDENTIFIER: &'static str = S::IDENTIFIER;
    type Implicit = S::Implicit;

    fn implicit(&self) -> Result<Self::Implicit, TransactionValidityError> {
        self.0.implicit()
    }

    fn metadata() -> Vec<traits::TransactionExtensionMetadata> {
        S::metadata()
    }
    type Val = Intermediate<S::Val, DispatchOriginOf<RuntimeCallOf<T>>, Option<TxSponsor<<T as frame_system::Config>::AccountId, T::Balance>>>;
    type Pre = Intermediate<S::Pre, DispatchOriginOf<RuntimeCallOf<T>>, Option<TxSponsor<<T as frame_system::Config>::AccountId, T::Balance>>>;

    fn weight(&self, call: &RuntimeCallOf<T>) -> Weight {
		self.0.weight(call)
    }

    fn validate(
        &self,
        origin: DispatchOriginOf<RuntimeCallOf<T>>,
        call: &RuntimeCallOf<T>,
        info: &DispatchInfoOf<RuntimeCallOf<T>>,
        len: usize,
        self_implicit: S::Implicit,
        inherited_implication: &impl Implication,
        source: TransactionSource,
	) -> ValidateResult<Self::Val, RuntimeCallOf<T>> {
		let unwrapped_proxy_call = Self::unwrap_proxy(call);
		let inner_call = unwrapped_proxy_call.unwrap_or(call);
		let signer = origin.as_signer().cloned();
		Self::validate_freshness(call, signer.clone())?;
		let general_pool_keys = Self::collect_pool_keys(call, signer);
		if call.is_feeless(&origin) {
			let synthetic_fee =
				pallet_transaction_payment::Pallet::<T>::compute_fee(len as u32, info, Zero::zero());
			let mut validity = ValidTransaction {
				priority: pallet_transaction_payment::ChargeTransactionPayment::<T>::get_priority(
					info,
					len,
					Zero::zero(),
					synthetic_fee,
				),
				..Default::default()
			};

			for key in general_pool_keys.iter().cloned() {
				Self::push_pool_key(&mut validity, key);
			}
			if let Some(key) = T::FeelessCallTxPoolKeyProviders::key_for(call) {
				Self::push_pool_key(&mut validity, key);
			}

			Ok((validity, Feeless(origin.clone()), origin))
		} else {
			let mut delegated_origin = origin.clone();
			let mut tx_sponsor = None;
			// Runtime refund policies are evaluated on the effective call after proxy unwrapping.
			let refund_fee_on_success =
				T::CallFeeRefundProviders::refund_fee_on_success(inner_call);

			if let Some(signer) = origin.as_signer() {
				let sponsor_for_outer_call =
					T::TransactionSponsorProviders::get_transaction_sponsor(signer, call);
				let sponsor = if let Some(sponsor) = sponsor_for_outer_call {
					Some(sponsor)
				} else {
					unwrapped_proxy_call.and_then(|inner_call| {
						T::TransactionSponsorProviders::get_transaction_sponsor(signer, inner_call)
					})
				};

				if let Some(sponsor) = sponsor {
					log::debug!("fee sponsor detected: payer={:?}", sponsor.payer);
					delegated_origin.set_caller_from_signed(sponsor.payer.clone());
					tx_sponsor = Some(sponsor);
				}
			}

			let (mut validity, inner_val, _) = self.0.validate(
				delegated_origin.clone(),
				call,
				info,
				len,
				self_implicit,
				inherited_implication,
				source,
			)?;
			if let Some(max_fee_with_tip) = tx_sponsor.as_ref().and_then(|sp| sp.max_fee_with_tip)
				&& let pallet_transaction_payment::Val::<T>::Charge { fee_with_tip, .. } =
					&inner_val
			{
				let total_fee: T::Balance = (*fee_with_tip).into();
				if total_fee > max_fee_with_tip {
					return Err(TransactionValidityError::Invalid(
						InvalidTransaction::Custom(INVALID_TX_SPONSORED_FEE_TOO_HIGH),
					));
				}
			}

			for key in general_pool_keys {
				Self::push_pool_key(&mut validity, key);
			}
			if let Some(key) = tx_sponsor.as_ref().and_then(|sponsor| sponsor.unique_tx_key.clone())
			{
				Self::push_pool_key(&mut validity, key);
			}

			Ok((
				validity,
				RequiresFee {
					inner: inner_val,
					transaction_origin: origin.clone(),
					delegated_origin,
					tx_sponsor,
					refund_fee_on_success,
				},
				origin,
			))
		}
	}

    fn prepare(
        self,
        val: Self::Val,
        origin: &DispatchOriginOf<RuntimeCallOf<T>>,
        call: &RuntimeCallOf<T>,
        info: &DispatchInfoOf<RuntimeCallOf<T>>,
        len: usize,
    ) -> Result<Self::Pre, TransactionValidityError> {
		Self::validate_freshness(call, origin.as_signer().cloned())?;

		match val {
			RequiresFee {
				inner,
				transaction_origin,
				delegated_origin,
				tx_sponsor,
				refund_fee_on_success,
			} => {
				let res = self.0.prepare(inner, &delegated_origin, call, info, len)?;
				Ok(RequiresFee {
					inner: res,
					transaction_origin,
					delegated_origin,
					tx_sponsor,
					refund_fee_on_success,
				})
			},
			Feeless(origin) => Ok(Feeless(origin)),
		}
    }

    fn post_dispatch_details(
        pre: Self::Pre,
        info: &DispatchInfoOf<RuntimeCallOf<T>>,
        post_info: &PostDispatchInfoOf<RuntimeCallOf<T>>,
        len: usize,
		result: &DispatchResult,
	) -> Result<Weight, TransactionValidityError> {
		match pre {
			RequiresFee {
				inner: pre,
				transaction_origin,
				delegated_origin: _,
				tx_sponsor,
				refund_fee_on_success,
			} => {
				let adjusted_post_info;
				let post_info = if refund_fee_on_success && result.is_ok() && post_info.pays_fee == Pays::Yes
				{
					adjusted_post_info = PostDispatchInfo {
						actual_weight: post_info.actual_weight,
						pays_fee: Pays::No,
					};
					&adjusted_post_info
				} else {
					post_info
				};
				let result = S::post_dispatch_details(pre, info, post_info, len, result)?;
				if let (Some(sponsor), Some(from)) = (tx_sponsor, transaction_origin.as_signer()) {
					Pallet::<T>::deposit_event(Event::<T>::FeeDelegated {
						origin: transaction_origin.clone().into_caller(),
						from: from.clone(),
						to: sponsor.payer,
					});
				}
				Ok(result)
			},
			Feeless(origin) => {
				Pallet::<T>::deposit_event(Event::<T>::FeeSkipped {
					origin: origin.into_caller(),
				});
				Ok(Weight::zero())
			},
		}
	}
}
