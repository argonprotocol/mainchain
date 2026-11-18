use crate::{Config, Pallet, Position, PositionsByAccount};
use argon_primitives::treasury::{PositionQuantities, UpstreamPosition};
use frame_support::{migrations::VersionedMigration, traits::UncheckedOnRuntimeUpgrade};
use pallet_prelude::*;

#[derive(Encode, Decode)]
pub(crate) struct PositionV2<Balance: codec::Codec + MaxEncodedLen> {
	#[codec(compact)]
	pub bond_principal: Balance,
	pub quantities: PositionQuantities<Balance>,
	pub upstream: Option<UpstreamPosition<Balance>>,
}

pub struct AddCollectAmounts<T>(PhantomData<T>);

impl<T: Config> UncheckedOnRuntimeUpgrade for AddCollectAmounts<T> {
	fn on_runtime_upgrade() -> Weight {
		let mut positions = 0u64;
		PositionsByAccount::<T>::translate_values::<PositionV2<T::Balance>, _>(|previous| {
			positions.saturating_accrue(1);
			Some(Position {
				bond_principal: previous.bond_principal,
				quantities: previous.quantities,
				upstream: previous.upstream,
				..Default::default()
			})
		});
		T::DbWeight::get().reads_writes(positions, positions)
	}
}

pub type AddCollectAmountsMigration<T> = VersionedMigration<
	2,
	3,
	AddCollectAmounts<T>,
	Pallet<T>,
	<T as frame_system::Config>::DbWeight,
>;
