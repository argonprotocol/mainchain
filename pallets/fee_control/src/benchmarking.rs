use super::*;
use alloc::{boxed::Box, vec::Vec};
use codec::EncodeLike;
use frame_benchmarking::v2::*;
use frame_support::{
	dispatch::{CheckIfFeeless, DispatchInfo, GetDispatchInfo, PostDispatchInfo},
	traits::{Currency, Get},
};
use frame_system::RawOrigin;
use pallet_prelude::argon_primitives::{
	CallFeeRefundProvider, TransactionSponsorProvider, TxSponsor,
};
use pallet_transaction_payment::OnChargeTransaction;
use polkadot_sdk::frame_support::traits::IsSubType;
use sp_runtime::traits::{
	AsTransactionAuthorizedOrigin, DispatchOriginOf, DispatchTransaction, Implication,
	StaticLookup, TransactionExtension, Zero,
};

#[derive(Clone, Eq, PartialEq, Encode, Decode, DecodeWithMemTracking, TypeInfo)]
#[scale_info(skip_type_params(T))]
struct WeightlessPayment<T>(PhantomData<T>);

impl<T> Debug for WeightlessPayment<T> {
	fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
		f.write_str("WeightlessPayment")
	}
}

impl<T> Default for WeightlessPayment<T> {
	fn default() -> Self {
		Self(PhantomData)
	}
}

impl<T> TransactionExtension<T::RuntimeCall> for WeightlessPayment<T>
where
	T: Config + pallet_transaction_payment::Config + Send + Sync,
	T::RuntimeCall: Dispatchable<Info = DispatchInfo, PostInfo = PostDispatchInfo>,
{
	const IDENTIFIER: &'static str = "WeightlessPayment";
	type Implicit = ();
	type Val = pallet_transaction_payment::Val<T>;
	type Pre = pallet_transaction_payment::Pre<T>;

	fn weight(&self, _call: &T::RuntimeCall) -> Weight {
		Weight::zero()
	}

	fn validate(
		&self,
		origin: DispatchOriginOf<T::RuntimeCall>,
		_call: &T::RuntimeCall,
		_info: &DispatchInfo,
		_len: usize,
		_self_implicit: Self::Implicit,
		_inherited_implication: &impl Implication,
		_source: TransactionSource,
	) -> ValidateResult<Self::Val, T::RuntimeCall> {
		Ok((ValidTransaction::default(), pallet_transaction_payment::Val::NoCharge, origin))
	}

	fn prepare(
		self,
		_val: Self::Val,
		_origin: &DispatchOriginOf<T::RuntimeCall>,
		_call: &T::RuntimeCall,
		_info: &DispatchInfo,
		_len: usize,
	) -> Result<Self::Pre, TransactionValidityError> {
		Ok(pallet_transaction_payment::Pre::NoCharge { refund: Weight::zero() })
	}
}

#[benchmarks(
	where
		T: Config
			+ pallet_transaction_payment::Config
			+ pallet_proxy::Config<RuntimeCall = <T as frame_system::Config>::RuntimeCall>
			+ pallet_utility::Config<RuntimeCall = <T as frame_system::Config>::RuntimeCall>
			+ Send
			+ Sync,
		<T as frame_system::Config>::RuntimeCall: From<frame_system::Call<T>>
			+ From<pallet_proxy::Call<T>>
			+ From<pallet_utility::Call<T>>
			+ CheckIfFeeless<Origin = OriginFor<T>>
			+ Dispatchable<Info = DispatchInfo, PostInfo = PostDispatchInfo>
			+ IsSubType<pallet_proxy::Call<T>>
			+ IsSubType<pallet_utility::Call<T>>,
		T::RuntimeOrigin: AsTransactionAuthorizedOrigin + Debug,
		<<T as pallet_transaction_payment::Config>::OnChargeTransaction as OnChargeTransaction<T>>::Balance:
			EncodeLike<T::Balance> + From<T::Balance> + Into<T::Balance>,
)]
mod benchmarks {
	use super::*;

	#[benchmark]
	fn regular() {
		let caller: T::AccountId = whitelisted_caller();
		let call =
			<T as frame_system::Config>::RuntimeCall::from(frame_system::Call::<T>::remark {
				remark: vec![],
			});
		let info = call.get_dispatch_info();
		let extension =
			CheckFeeWrapper::<T, WeightlessPayment<T>>::from(WeightlessPayment::<T>::default());

		#[block]
		{
			assert!(extension
				.test_run(
					RawOrigin::Signed(caller).into(),
					&call,
					&info,
					call.encoded_size(),
					0,
					|_| Ok(PostDispatchInfo::default()),
				)
				.unwrap()
				.is_ok());
		}
	}

	#[benchmark]
	fn batch(c: Linear<1, 100>) {
		let caller: T::AccountId = whitelisted_caller();
		let calls = (0..c).map(system_remark::<T>).collect();
		let call =
			<T as frame_system::Config>::RuntimeCall::from(pallet_utility::Call::<T>::batch_all {
				calls,
			});
		let info = call.get_dispatch_info();
		let extension =
			CheckFeeWrapper::<T, WeightlessPayment<T>>::from(WeightlessPayment::<T>::default());

		#[block]
		{
			assert!(extension
				.test_run(
					RawOrigin::Signed(caller).into(),
					&call,
					&info,
					call.encoded_size(),
					0,
					|_| Ok(PostDispatchInfo::default()),
				)
				.unwrap()
				.is_ok());
		}
	}

	#[benchmark]
	fn proxy(c: Linear<1, 10_000>) {
		let (caller, real) = seed_proxy::<T>();
		let call = T::TransactionSponsorProviders::benchmark_call(&caller, &real, c)
			.unwrap_or_else(|| {
				pallet_proxy::Call::<T>::proxy {
					real: T::Lookup::unlookup(real.clone()),
					force_proxy_type: Some(T::ProxyType::default()),
					call: Box::new(system_remark::<T>(0)),
				}
				.into()
			});

		#[block]
		{
			// Authorization runs during freshness validation, pool-key collection, and prepare.
			for _ in 0..3 {
				core::hint::black_box(
					CheckFeeWrapper::<T, WeightlessPayment<T>>::validated_pool_key_context(
						core::hint::black_box(&call),
						Some(caller.clone()),
					),
				);
			}
			core::hint::black_box(T::TransactionSponsorProviders::get_transaction_sponsor(
				&caller,
				core::hint::black_box(&call),
			));
		}
	}

	#[benchmark]
	fn proxy_announced(n: Linear<0, 5_242_880>) {
		let (caller, real) = seed_proxy::<T>();
		let inner_call =
			<T as frame_system::Config>::RuntimeCall::from(frame_system::Call::<T>::remark {
				remark: vec![0xff; n as usize],
			});
		let call_hash = T::CallHasher::hash_of(&inner_call);
		for index in 1..T::MaxPending::get() {
			pallet_proxy::Pallet::<T>::announce(
				RawOrigin::Signed(caller.clone()).into(),
				T::Lookup::unlookup(real.clone()),
				T::CallHasher::hash_of(&system_remark::<T>(index)),
			)
			.expect("benchmark proxy announcement should be created");
		}
		pallet_proxy::Pallet::<T>::announce(
			RawOrigin::Signed(caller.clone()).into(),
			T::Lookup::unlookup(real.clone()),
			call_hash,
		)
		.expect("benchmark proxy announcement should be created");
		let call = <T as frame_system::Config>::RuntimeCall::from(
			pallet_proxy::Call::<T>::proxy_announced {
				delegate: T::Lookup::unlookup(caller.clone()),
				real: T::Lookup::unlookup(real),
				force_proxy_type: Some(T::ProxyType::default()),
				call: Box::new(inner_call),
			},
		);
		#[block]
		{
			for _ in 0..3 {
				core::hint::black_box(
					CheckFeeWrapper::<T, WeightlessPayment<T>>::validated_pool_key_context(
						core::hint::black_box(&call),
						Some(caller.clone()),
					),
				);
			}
			core::hint::black_box(T::TransactionSponsorProviders::get_transaction_sponsor(
				&caller,
				core::hint::black_box(&call),
			));
		}
	}

	#[benchmark]
	fn post_event() {
		let caller: T::AccountId = whitelisted_caller();
		let origin: T::RuntimeOrigin = RawOrigin::Signed(caller.clone()).into();
		let pre = Intermediate::<
			pallet_transaction_payment::Pre<T>,
			T::RuntimeOrigin,
			Option<TxSponsor<T::AccountId, T::Balance>>,
		>::RequiresFee {
			inner: pallet_transaction_payment::Pre::<T>::NoCharge { refund: Weight::zero() },
			transaction_origin: origin.clone(),
			delegated_origin: origin,
			tx_sponsor: Some(TxSponsor {
				payer: caller,
				max_fee_with_tip: None,
				unique_tx_key: None,
			}),
			refund_fee_on_success: false,
		};
		let info = DispatchInfo::default();
		let post_info = PostDispatchInfo::default();

		#[block]
		{
			CheckFeeWrapper::<T, WeightlessPayment<T>>::post_dispatch_details(
				pre,
				&info,
				&post_info,
				0,
				&Ok(()),
			)
			.unwrap();
		}
	}

	#[benchmark]
	fn pool_key_insertion(k: Linear<0, 10_000>) {
		let mut keys = (0..k)
			.map(|index| T::Hashing::hash_of(&index).as_ref().to_vec())
			.collect::<Vec<_>>();
		let mut validity_keys = keys.clone();
		let new_key = T::Hashing::hash_of(&u32::MAX).as_ref().to_vec();

		#[block]
		{
			CheckFeeWrapper::<T, WeightlessPayment<T>>::insert_pool_key(&mut keys, new_key.clone());
			CheckFeeWrapper::<T, WeightlessPayment<T>>::insert_pool_key(
				&mut validity_keys,
				new_key,
			);
		}
		assert_eq!(keys.len(), k as usize + 1);
		assert_eq!(validity_keys.len(), k as usize + 1);
	}

	#[benchmark]
	fn batch_pool_key(k: Linear<0, 10_000>) {
		let keys = (0..k)
			.map(|index| T::Hashing::hash_of(&index).as_ref().to_vec())
			.collect::<Vec<_>>();

		#[block]
		{
			core::hint::black_box(T::Hashing::hash_of(&(b"batch", core::hint::black_box(&keys))));
		}
	}

	#[benchmark]
	fn inspect_refund_calls(c: Linear<0, 10_000>) -> Result<(), BenchmarkError> {
		let call = T::CallFeeRefundProviders::benchmark_call(c).ok_or(BenchmarkError::Skip)?;

		#[block]
		{
			core::hint::black_box(T::CallFeeRefundProviders::refund_fee_on_success(
				core::hint::black_box(&call),
			));
		}
		Ok(())
	}

	impl_benchmark_test_suite!(Pallet, crate::mock::new_test_ext(), crate::mock::Test);
}

fn system_remark<T: Config>(index: u32) -> T::RuntimeCall
where
	T::RuntimeCall: From<frame_system::Call<T>>,
{
	frame_system::Call::<T>::remark { remark: index.encode() }.into()
}

fn seed_proxy<T>() -> (T::AccountId, T::AccountId)
where
	T: Config + pallet_proxy::Config<RuntimeCall = <T as frame_system::Config>::RuntimeCall>,
{
	type ProxyBalanceOf<T> = <<T as pallet_proxy::Config>::Currency as Currency<
		<T as frame_system::Config>::AccountId,
	>>::Balance;

	let real: T::AccountId = account("fee-control-real", 0, 0);
	let mut delegates = (0..T::MaxProxies::get())
		.map(|index| account::<T::AccountId>("fee-control-proxy", index, 0))
		.collect::<Vec<_>>();
	delegates.sort();
	let caller = delegates.pop().expect("MaxProxies must be non-zero");
	T::Currency::make_free_balance_be(&real, ProxyBalanceOf::<T>::max_value());
	T::Currency::make_free_balance_be(&caller, ProxyBalanceOf::<T>::max_value());
	for delegate in delegates.into_iter().chain(core::iter::once(caller.clone())) {
		pallet_proxy::Pallet::<T>::add_proxy_delegate(
			&real,
			delegate,
			T::ProxyType::default(),
			Zero::zero(),
		)
		.expect("benchmark proxy should be created");
	}

	(caller, real)
}
