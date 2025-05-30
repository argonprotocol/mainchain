#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;
extern crate core;

pub use alloc::{
	format,
	string::{String, ToString},
	vec,
	vec::Vec,
};
pub use core::{convert::TryInto, fmt::Debug, marker::PhantomData, str::FromStr};
pub use log::{self};

pub use argon_primitives::{self, prelude::*};
pub use codec::{Codec, DecodeWithMemTracking};
pub use frame_support::{
	pallet_prelude::*,
	storage::{bounded_vec, with_storage_layer},
	traits::{
		fungible::{Inspect, InspectHold, Mutate, MutateHold},
		tokens::{Fortitude, Precision, Preservation, Restriction},
		Incrementable,
	},
	BoundedVec, CloneNoBound, DebugNoBound, DefaultNoBound, EqNoBound, OrdNoBound, PalletId,
	PartialEqNoBound, PartialOrdNoBound, RuntimeDebugNoBound,
};
pub use frame_system::{pallet_prelude::*, WeightInfo as SystemWeightInfo};
pub use sp_arithmetic::{traits::*, FixedI128, FixedU128, Perbill, Percent, Permill};
pub use sp_core::{hashing::blake2_256, ConstU32, ConstU64, H160, H256, U256};
pub use sp_runtime::{
	traits::{
		AtLeast32BitUnsigned, BlakeTwo256, Block as BlockT, Bounded, CheckedDiv, DispatchInfoOf,
		Dispatchable, Hash, Header as HeaderT, Member, One, SaturatedConversion, Saturating,
		StaticLookup, TrailingZeroInput, UniqueSaturatedInto, Zero,
	},
	Digest, DigestItem, DispatchError,
	DispatchError::Token,
	FixedPointNumber, RuntimeDebug, TokenError,
};

#[cfg(feature = "test")]
pub mod test;

#[cfg(feature = "test")]
pub use test::*;
