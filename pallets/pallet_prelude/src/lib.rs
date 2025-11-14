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
	BoundedVec, CloneNoBound, DebugNoBound, DefaultNoBound, EqNoBound, OrdNoBound, PalletId,
	PartialEqNoBound, PartialOrdNoBound, RuntimeDebugNoBound,
	pallet_prelude::*,
	storage::{bounded_vec, with_storage_layer},
	traits::{
		Incrementable,
		fungible::{Inspect, InspectHold, Mutate, MutateHold},
		tokens::{Fortitude, Precision, Preservation, Restriction},
	},
};
pub use frame_system::{WeightInfo as SystemWeightInfo, pallet_prelude::*};
pub use sp_arithmetic::{FixedI128, FixedU128, Perbill, Percent, Permill, traits::*};
pub use sp_core::{ConstU32, ConstU64, H160, H256, U256, hashing::blake2_256};
pub use sp_runtime::{
	Digest, DigestItem, DispatchError,
	DispatchError::Token,
	FixedPointNumber, RuntimeDebug, TokenError,
	traits::{
		AtLeast32BitUnsigned, BlakeTwo256, Block as BlockT, Bounded, CheckedDiv, DispatchInfoOf,
		Dispatchable, Hash, HashingFor, Header as HeaderT, Member, NumberFor, One,
		SaturatedConversion, Saturating, StaticLookup, TrailingZeroInput, UniqueSaturatedInto,
		Zero,
	},
};

#[cfg(feature = "test")]
pub mod test;

#[cfg(feature = "test")]
pub use test::*;
