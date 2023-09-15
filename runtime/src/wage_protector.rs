use crate::Perbill;
use frame_support::weights::{WeightToFeeCoefficient, WeightToFeeCoefficients};
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

use smallvec::smallvec;

use crate::Balance;

pub struct WageProtectorFee;

impl WeightToFeePolynomial for WageProtectorFee {
	type Balance = Balance;

	fn polynomial() -> WeightToFeeCoefficients<Self::Balance> {
		// TODO: lookup current cpi to multiply!
		let p = 1_000; // milligons
		let q = 10 * Self::Balance::from(ExtrinsicBaseWeight::get().ref_time());
		smallvec![WeightToFeeCoefficient::<Self::Balance> {
			degree: 1,
			negative: false,
			coeff_frac: Perbill::from_rational(p % q, q),
			coeff_integer: p / q,
		}]
	}
}
