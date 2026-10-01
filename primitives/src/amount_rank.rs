use codec::{Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use sp_runtime::AccountId32;

/// A fixed-width ID representation whose encoded bytes preserve ID order.
pub trait AmountRankId: Sized {
	type Ordered: Clone
		+ core::fmt::Debug
		+ PartialEq
		+ Eq
		+ Encode
		+ Decode
		+ DecodeWithMemTracking
		+ MaxEncodedLen
		+ TypeInfo
		+ 'static;

	fn to_ordered(&self) -> Self::Ordered;
	fn from_ordered(ordered: &Self::Ordered) -> Self;
}

impl AmountRankId for u32 {
	type Ordered = [u8; 4];

	fn to_ordered(&self) -> Self::Ordered {
		self.to_be_bytes()
	}

	fn from_ordered(ordered: &Self::Ordered) -> Self {
		Self::from_be_bytes(*ordered)
	}
}

impl AmountRankId for u64 {
	type Ordered = [u8; 8];

	fn to_ordered(&self) -> Self::Ordered {
		self.to_be_bytes()
	}

	fn from_ordered(ordered: &Self::Ordered) -> Self {
		Self::from_be_bytes(*ordered)
	}
}

impl AmountRankId for AccountId32 {
	type Ordered = [u8; 32];

	fn to_ordered(&self) -> Self::Ordered {
		let mut bytes = [0u8; 32];
		bytes.copy_from_slice(self.as_ref());
		bytes
	}

	fn from_ordered(ordered: &Self::Ordered) -> Self {
		(*ordered).into()
	}
}

/// An amount and holder ID whose SCALE bytes sort by amount descending, then ID ascending.
/// Use only with an `Identity`-hashed storage key when iteration order matters.
///
/// The stored fields are fixed-width ordered bytes so SCALE metadata and generated clients
/// describe the same encoding used by the runtime.
#[derive(
	Clone, Debug, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, MaxEncodedLen, TypeInfo,
)]
pub struct AmountRankKey<Id: AmountRankId> {
	descending_amount: [u8; 16],
	ordered_id: Id::Ordered,
}

impl<Id: AmountRankId> AmountRankKey<Id> {
	pub fn new(amount: u128, holder_id: Id) -> Self {
		Self {
			descending_amount: (u128::MAX - amount).to_be_bytes(),
			ordered_id: holder_id.to_ordered(),
		}
	}

	pub fn amount(&self) -> u128 {
		u128::MAX - u128::from_be_bytes(self.descending_amount)
	}

	pub fn holder_id(&self) -> Id {
		Id::from_ordered(&self.ordered_id)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn encoded_rank_sorts_larger_amounts_first_and_breaks_ties_by_id() {
		let higher = AmountRankKey::new(1_000, 257u32);
		let lower = AmountRankKey::new(999, 1u32);
		let tied_with_lower_id = AmountRankKey::new(1_000, 256u32);
		assert!(higher.encode() < lower.encode());
		assert!(tied_with_lower_id.encode() < higher.encode());
		assert_eq!(higher.amount(), 1_000);
		assert_eq!(higher.holder_id(), 257);
		assert_eq!(AmountRankKey::<u32>::decode(&mut &higher.encode()[..]), Ok(higher));
	}
}
