use crate::{configs::ArgonToken, Balances, Runtime, RuntimeEvent};
use argon_primitives::{AccountId, Balance};
use core::marker::PhantomData;
use frame_support::{
	parameter_types,
	traits::{fungible, fungible::Balanced, ConstU8, Imbalance, OnUnbalanced},
	weights::{
		constants::ExtrinsicBaseWeight, ConstantMultiplier, WeightToFeeCoefficient,
		WeightToFeeCoefficients, WeightToFeePolynomial,
	},
};
use pallet_transaction_payment::{ConstFeeMultiplier, FungibleAdapter, Multiplier};
use smallvec::smallvec;
use sp_arithmetic::{traits::One, Perbill};

pub type AccountIdOf<T> = <T as frame_system::Config>::AccountId;

parameter_types! {
	pub FeeMultiplier: Multiplier = Multiplier::one();
	pub const TransactionByteFee: Balance = 1;
}

impl pallet_transaction_payment::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type OnChargeTransaction = FungibleAdapter<Balances, DealWithFees<Runtime>>;
	type OperationalFeeMultiplier = ConstU8<5>;
	type WeightToFee = WeightToFee;
	type LengthToFee = ConstantMultiplier<Balance, TransactionByteFee>;
	type FeeMultiplierUpdate = ConstFeeMultiplier<FeeMultiplier>;
}

/// Handles converting a weight scalar to a fee value, based on the scale and granularity of the
/// node's balance type.
///
/// This should typically create a mapping between the following ranges:
///   - `[0, MAXIMUM_BLOCK_WEIGHT]`
///   - `[Balance::min, Balance::max]`
///
/// Yet, it can be used for any other sort of change to weight-fee. Some examples being:
///   - Setting it to `0` will essentially disable the weight fee.
///   - Setting it to `1` will cause the literal `#[weight = x]` values to be charged.
pub struct WeightToFee;
impl WeightToFeePolynomial for WeightToFee {
	type Balance = Balance;
	fn polynomial() -> WeightToFeeCoefficients<Self::Balance> {
		let p = 1_000_000; // microgons
		let q = 100 * Balance::from(ExtrinsicBaseWeight::get().ref_time());
		// BAB - disabling wage protector for fees. Makes it hard to keep system stable
		// let cpi = PriceIndex::get_argon_cpi().unwrap_or(ArgonCPI::zero());
		// if cpi.is_positive() {
		// 	let cpi = cpi.into_inner() / ArgonCPI::accuracy();
		// 	let adjustment = (p * (cpi as u128) * 1_000).checked_div(1_000).unwrap_or_default();
		// 	p += adjustment;
		// }
		smallvec![WeightToFeeCoefficient {
			degree: 1,
			negative: false,
			coeff_frac: Perbill::from_rational(p % q, q),
			coeff_integer: p / q,
		}]
	}
}

pub struct DealWithFees<R>(PhantomData<R>);

impl<R> OnUnbalanced<fungible::Credit<R::AccountId, pallet_balances::Pallet<R, ArgonToken>>>
	for DealWithFees<R>
where
	R: pallet_authorship::Config + pallet_balances::Config<ArgonToken>,
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
			if let Some(author) = pallet_authorship::Pallet::<R>::author() {
				let _ =
					<pallet_balances::Pallet<R, ArgonToken>>::resolve(&author, fees).map_err(drop);
			}
		}
	}
}
