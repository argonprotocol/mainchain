use frame_support::weights::WeightToFee;
pub use frame_support::{
	construct_runtime, parameter_types,
	traits::{
		ConstBool, ConstU128, ConstU32, ConstU64, ConstU8, KeyOwnerProofSystem, Randomness,
		StorageInfo,
	},
	weights::{
		constants::{
			BlockExecutionWeight, ExtrinsicBaseWeight, RocksDbWeight, WEIGHT_REF_TIME_PER_SECOND,
		},
		IdentityFee, Weight, WeightToFeePolynomial,
	},
	StorageValue,
};
pub use frame_system::Call as SystemCall;
use sp_arithmetic::traits::{BaseArithmetic, SaturatedConversion, Unsigned};

pub struct WageProtectorFee<T>(sp_std::marker::PhantomData<T>);

impl<T> WeightToFee for WageProtectorFee<T>
where
	T: BaseArithmetic + From<u32> + Copy + Unsigned,
{
	type Balance = T;

	fn weight_to_fee(weight: &Weight) -> Self::Balance {
		// TODO: lookup current cpi to multiply!
		Self::Balance::saturated_from::<u64>(weight.ref_time())
	}
}
