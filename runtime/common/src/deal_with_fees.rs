#[macro_export]
macro_rules! deal_with_fees {
	() => {
		pub struct DealWithFees<R>(PhantomData<R>);

		impl<R> OnUnbalanced<fungible::Credit<R::AccountId, pallet_balances::Pallet<R, ArgonToken>>>
			for DealWithFees<R>
		where
			R: pallet_block_rewards::Config<Balance = Balance>
				+ pallet_balances::Config<ArgonToken, Balance = Balance>,
			AccountIdOf<R>: From<AccountId> + Into<AccountId>,
			<R as frame_system::Config>::RuntimeEvent: From<pallet_balances::Event<R, ArgonToken>>,
		{
			fn on_unbalanceds(
				mut fees_then_tips: impl Iterator<
					Item = fungible::Credit<R::AccountId, pallet_balances::Pallet<R, ArgonToken>>,
				>,
			) {
				if let Some(mut fees) = fees_then_tips.next() {
					if let Some(tips) = fees_then_tips.next() {
						tips.merge_into(&mut fees);
					}
					let amount: Balance = fees.peek();
					if amount.is_zero() {
						drop(fees);
						return;
					}
					let author = pallet_block_rewards::Pallet::<R>::fees_account();
					match <pallet_balances::Pallet<R, ArgonToken>>::resolve(&author, fees) {
						Ok(()) => pallet_block_rewards::Pallet::<R>::track_fee(amount),
						Err(x) => drop(x),
					}
				}
			}
		}

		#[derive(
			Encode, Decode, DecodeWithMemTracking, scale_info::TypeInfo, Clone, Eq, PartialEq, Default, RuntimeDebugNoBound,
		)]
        #[scale_info(skip_type_params(T))]
		pub struct ProxyFeeRefund<T>(PhantomData<T>);

		impl<T: frame_system::Config<AccountId = AccountId, RuntimeCall = RuntimeCall> + Send + Sync> TransactionExtension<T::RuntimeCall> for ProxyFeeRefund<T> where
			T::RuntimeCall: Dispatchable<Info = DispatchInfo, PostInfo = PostDispatchInfo>,
		{
			// no implicit data, no value passed between validate/prepare
			type Implicit = ();
			type Val = ();
			// what we want to carry into post_dispatch: (real, proxy)
			type Pre = Option<(AccountId, AccountId)>;

			const IDENTIFIER: &'static str = "ProxyFeeRefund";

			// we don’t care about custom weight / validation; pull in the defaults:
			impl_tx_ext_default!(T::RuntimeCall; weight validate);

			fn prepare(
				self,
				_val: Self::Val,
				origin: &DispatchOriginOf<T::RuntimeCall>,
				call: &T::RuntimeCall,
				_info: &DispatchInfoOf<T::RuntimeCall>,
				_len: usize,
			) -> Result<Self::Pre, TransactionValidityError> {
				// signer = proxy
				let Some(proxy) = origin.clone().into_signer() else {
					return Ok(None);
                };

				// detect our special proxy type
				if let RuntimeCall::Proxy(pallet_proxy::Call::proxy { real, force_proxy_type, .. }) = call {
					if matches!(force_proxy_type, Some(ProxyType::MiningBidRealPaysFee)) {
						if let MultiAddress::Id(real) = real {
							// (real, proxy)
							return Ok(Some((real.clone(), proxy)));
						}
					}
				}

				Ok(None)
			}

			fn post_dispatch_details(
				pre: Self::Pre,
				info: &DispatchInfoOf<T::RuntimeCall>,
				post_info: &PostDispatchInfoOf<T::RuntimeCall>,
				len: usize,
				_result: &DispatchResult,
			) -> Result<Weight, TransactionValidityError> {
				let Some((real, proxy)) = pre else {
					return Ok(Weight::zero());
				};

				let len_u32 = len.min(u32::MAX as usize) as u32;

				// for now, assume `tip = 0` – if you want to refund tips as well,
				// you’ll need to align with whatever extension owns the tip.
				let tip = Zero::zero();

				let actual_fee = pallet_transaction_payment::Pallet::<Runtime>::compute_actual_fee(
					len_u32, info, post_info, tip,
				);

				if actual_fee.is_zero() {
					return Ok(Weight::zero());
				}


				// pay the proxy back from the real account
				if let Err(err) = <Balances as fungible::Mutate<AccountId>>::transfer(&real, &proxy, actual_fee, Preservation::Preserve) {
					log::error!(
						"ProxyFeeRefund: failed to transfer {:?} from real {:?} to proxy {:?}: {:?}",
						actual_fee,
						real,
						proxy,
						err
					);
				}

				// we don't change effective weight
				Ok(Weight::zero())
			}
		}
	};
}
