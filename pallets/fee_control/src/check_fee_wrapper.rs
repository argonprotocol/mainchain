// Error code for InvalidTransaction::Custom when sponsored fee is too high
const INVALID_TX_SPONSORED_FEE_TOO_HIGH: u8 = 1;
use crate::pallet::{Config, Event, Pallet};
use Intermediate::*;
use alloc::vec::Vec;
use codec::EncodeLike;
use frame_support::{dispatch::CheckIfFeeless, traits::InstanceFilter};
use pallet_prelude::{
	Decode, DecodeWithMemTracking, DispatchInfoOf, DispatchResult, Encode, OriginFor, OriginTrait,
	TransactionSource, TransactionValidityError, ValidTransaction, ValidateResult, Weight,
	argon_primitives::{
		CallTxPoolKeyProvider, FeelessCallTxPoolKeyProvider, TransactionSponsorProvider, TxSponsor,
	},
	sp_runtime::traits::{
		DispatchOriginOf, Implication, PostDispatchInfoOf, StaticLookup, TransactionExtension, Zero,
	},
	*,
};
use pallet_transaction_payment::OnChargeTransaction;
use polkadot_sdk::frame_support::traits::IsSubType;
use scale_info::{StaticTypeInfo, TypeInfo};
use sp_runtime::traits;

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
	RequiresFee(T, O, A),
	/// The wrapped extension should be skipped.
	Feeless(O),
}

impl<T, S> CheckFeeWrapper<T, S>
where
	T: Config + pallet_transaction_payment::Config + pallet_proxy::Config,
	// Ensure the proxy pallet's call type is the same RuntimeCall as frame_system.
	T: pallet_proxy::Config<RuntimeCall = RuntimeCallOf<T>>,
	RuntimeCallOf<T>: IsSubType<pallet_proxy::Call<T>>,
{
	// Determine the call to check for sponsorship. If the outer call is a proxy wrapper,
	// unwrap to the inner call (recursing through nested proxy wrappers).
	fn unwrap_proxy(call: &RuntimeCallOf<T>) -> &RuntimeCallOf<T> {
		if let Some(
			pallet_proxy::Call::proxy { call, .. } |
			pallet_proxy::Call::proxy_announced { call, .. },
		) = <RuntimeCallOf<T> as IsSubType<pallet_proxy::Call<T>>>::is_sub_type(call)
		{
			return Self::unwrap_proxy(call.as_ref());
		}
		call
	}

	fn validated_pool_key_context(
		call: &RuntimeCallOf<T>,
		signer: Option<T::AccountId>,
	) -> (&RuntimeCallOf<T>, Option<T::AccountId>) {
		let Some(signer) = signer else {
			return (call, None);
		};

		if let Some(pallet_proxy::Call::proxy { real, force_proxy_type, call: inner_call }) =
			<RuntimeCallOf<T> as IsSubType<pallet_proxy::Call<T>>>::is_sub_type(call)
		{
			let Ok(real) = T::Lookup::lookup(real.clone()) else {
				return (call, Some(signer));
			};
			let Ok(def) =
				pallet_proxy::Pallet::<T>::find_proxy(&real, &signer, force_proxy_type.clone())
			else {
				return (call, Some(signer));
			};
			if !def.delay.is_zero() || !Self::proxy_call_is_allowed(&def, inner_call.as_ref()) {
				return (call, Some(signer));
			}

			return Self::validated_pool_key_context(inner_call.as_ref(), Some(real));
		}

		(call, Some(signer))
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
    T: Config + pallet_transaction_payment::Config + pallet_proxy::Config + Send + Sync,
    T: pallet_proxy::Config<RuntimeCall = RuntimeCallOf<T>>,
    RuntimeCallOf<T>: CheckIfFeeless<Origin = OriginFor<T>> + IsSubType<pallet_proxy::Call<T>>,
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
        let inner_call = Self::unwrap_proxy(call);
        let (pool_key_call, pool_key_signer) =
            Self::validated_pool_key_context(call, origin.as_signer().cloned());
        let general_pool_key =
            T::CallTxPoolKeyProviders::key_for(pool_key_call, pool_key_signer.as_ref());
        if call.is_feeless(&origin) {
            let mut validity = ValidTransaction::default();
            let mut push_provides = |key: Option<Vec<u8>>| {
                if let Some(key) = key {
                    if !validity.provides.contains(&key) {
                        validity.provides.push(key);
                    }
                }
            };

            push_provides(general_pool_key.clone());
            push_provides(T::FeelessCallTxPoolKeyProviders::key_for(call));

            Ok((validity, Feeless(origin.clone()), origin))
        } else {
            let mut delegated_origin = origin.clone();
            let mut tx_sponsor = None;
            if let Some(signer) = origin.as_signer() {
                if let Some(sponsor) =
                    T::TransactionSponsorProviders::get_transaction_sponsor(signer, inner_call)
                {
                    log::debug!("fee sponsor detected: payer={:?}", sponsor.payer);
                    delegated_origin.set_caller_from_signed(sponsor.payer.clone());
                    tx_sponsor = Some(sponsor);
                }
            };

            let (mut validity, inner_val, origin_out) = self.0.validate(
                delegated_origin.clone(),
                call,
                info,
                len,
                self_implicit,
                inherited_implication,
                source,
            )?;
            if let Some(max_fee_with_tip) = tx_sponsor.as_ref().and_then(|sp| sp.max_fee_with_tip)
            {
                if let pallet_transaction_payment::Val::<T>::Charge { fee_with_tip, .. } =
                    &inner_val
                {
                    let total_fee: T::Balance = (*fee_with_tip).into();
                    if total_fee > max_fee_with_tip {
                        return Err(TransactionValidityError::Invalid(
                            InvalidTransaction::Custom(INVALID_TX_SPONSORED_FEE_TOO_HIGH),
                        ));
                    }
                }
            }

            let mut push_provides = |key: Option<Vec<u8>>| {
                if let Some(key) = key {
                    if !validity.provides.contains(&key) {
                        validity.provides.push(key);
                    }
                }
            };

            push_provides(general_pool_key);
            push_provides(tx_sponsor.as_ref().and_then(|sponsor| sponsor.unique_tx_key.clone()));

            Ok((validity, RequiresFee(inner_val, delegated_origin, tx_sponsor), origin_out))
        }
    }

    fn prepare(
        self,
        val: Self::Val,
        _origin: &DispatchOriginOf<RuntimeCallOf<T>>,
        call: &RuntimeCallOf<T>,
        info: &DispatchInfoOf<RuntimeCallOf<T>>,
        len: usize,
    ) -> Result<Self::Pre, TransactionValidityError> {
        match val {
            RequiresFee(inner, delegated_origin, sponsor) => {
                let res = self.0.prepare(inner, &delegated_origin, call, info, len)?;
                Ok(RequiresFee(res, delegated_origin, sponsor))
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
            RequiresFee(pre, origin, tx_sponsor) => {
                let result = S::post_dispatch_details(pre, info, post_info, len, result)?;
                if let (Some(sponsor), Some(from)) = (tx_sponsor, origin.as_signer()) {
                    Pallet::<T>::deposit_event(Event::<T>::FeeDelegated {
                        origin: origin.clone().into_caller(),
                        from: from.clone(),
                        to: sponsor.payer
                    });
                }
                Ok(result)
            },
            Feeless(origin) => {
                Pallet::<T>::deposit_event(Event::<T>::FeeSkipped { origin: origin.into_caller() });
                Ok(Weight::zero())
            },
        }
    }
}
