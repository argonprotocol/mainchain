use crate::{configs::Get, Grandpa};
use core::marker::PhantomData;
use log::warn;
use pallet_grandpa::Config;

/// The grandpa authorities need a rotation for proofs to work
pub struct AddGrandpaRotation<T>(PhantomData<T>);
impl<T: Config> frame_support::traits::OnRuntimeUpgrade for AddGrandpaRotation<T> {
	fn on_runtime_upgrade() -> frame_support::weights::Weight {
		warn!("Adding grandpa rotation");
		Grandpa::schedule_change(Grandpa::grandpa_authorities(), 1, None)
			.expect("Needs to be accepted");
		T::DbWeight::get().reads_writes(1, 2)
	}
}
