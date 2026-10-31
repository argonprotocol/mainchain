use crate::{
	Config, ExecutionHeaderAnchorReferences, FinalizedExecutionHeaderAnchor, Fork,
	ForkVersionSchedule, ForkVersions, Pallet,
};
use alloc::collections::BTreeMap;
use core::marker::PhantomData;
use polkadot_sdk::frame_support::{
	traits::{Get, UncheckedOnRuntimeUpgrade},
	weights::Weight,
};

pub struct AddGloasFork<T>(PhantomData<T>);

impl<T: Config> UncheckedOnRuntimeUpgrade for AddGloasFork<T> {
	fn on_runtime_upgrade() -> Weight {
		// A SCALE struct of seven Fork fields has the same encoding as this fixed-size array.
		ForkVersionSchedule::<T>::translate::<[Fork; 7], _>(|old| {
			old.map(|[genesis, altair, bellatrix, capella, deneb, electra, fulu]| ForkVersions {
				genesis,
				altair,
				bellatrix,
				capella,
				deneb,
				electra,
				fulu,
				gloas: Fork { version: [7, 0, 0, 0], epoch: u64::MAX },
			})
		})
		.expect("stored fork schedule must decode before upgrading");
		let mut references = BTreeMap::<_, u32>::new();
		let mut reads = 1;
		for hash in FinalizedExecutionHeaderAnchor::<T>::iter_values() {
			reads += 1;
			*references.entry(hash).or_default() += 1;
		}
		let writes = references.len() as u64 + 1;
		for (hash, count) in references {
			ExecutionHeaderAnchorReferences::<T>::insert(hash, count);
		}
		T::DbWeight::get().reads_writes(reads, writes)
	}
}

pub type AddGloasForkMigration<T> = polkadot_sdk::frame_support::migrations::VersionedMigration<
	1,
	2,
	AddGloasFork<T>,
	Pallet<T>,
	<T as polkadot_sdk::frame_system::Config>::DbWeight,
>;
