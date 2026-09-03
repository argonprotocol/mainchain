use super::*;
use argon_primitives::{
	host::Host,
	notary::{NotaryMeta, NotaryName, NotaryProvider, NotaryPublic, NotaryRecord},
	TickProvider,
};
use frame_system::RawOrigin;
use polkadot_sdk::{frame_benchmarking::v2::*, sp_runtime::BoundedBTreeMap};

#[benchmarks]
mod benchmarks {
	use super::*;

	#[benchmark]
	fn propose() {
		let caller: T::AccountId = whitelisted_caller();
		frame_system::Pallet::<T>::set_block_number(1u32.into());
		let expires = frame_system::Pallet::<T>::block_number() +
			BlockNumberFor::<T>::from(T::MaxProposalHoldBlocks::get());
		let proposals = (0..T::MaxProposalsPerBlock::get().saturating_sub(1))
			.map(|index| account("proposal", index, 0))
			.collect::<Vec<_>>();
		ExpiringProposals::<T>::insert(expires, BoundedVec::truncate_from(proposals));
		let meta = benchmark_meta::<T>(1);

		#[extrinsic_call]
		propose(RawOrigin::Signed(caller.clone()), meta);

		assert!(ProposedNotaries::<T>::contains_key(caller));
		assert_eq!(
			ExpiringProposals::<T>::get(expires).len() as u32,
			T::MaxProposalsPerBlock::get()
		);
	}

	#[benchmark]
	fn activate() {
		let caller: T::AccountId = whitelisted_caller();
		let expires: BlockNumberFor<T> = T::MaxProposalHoldBlocks::get().into();
		let active_count = T::MaxActiveNotaries::get().saturating_sub(1);
		seed_active_notaries::<T>(active_count);
		NextNotaryId::<T>::put(active_count.saturating_add(1));
		ProposedNotaries::<T>::insert(&caller, (benchmark_meta::<T>(1), expires));
		let mut expiring = (0..T::MaxProposalsPerBlock::get().saturating_sub(1))
			.map(|index| account("proposal", index, 0))
			.collect::<Vec<_>>();
		expiring.push(caller.clone());
		ExpiringProposals::<T>::insert(expires, BoundedVec::truncate_from(expiring));

		#[extrinsic_call]
		activate(RawOrigin::Root, caller.clone());

		assert!(!ProposedNotaries::<T>::contains_key(&caller));
		assert_eq!(ActiveNotaries::<T>::get().len() as u32, active_count.saturating_add(1));
	}

	// Both full metadata values must be present in the same proof. The default per-prefix
	// maximum is smaller than the combined proof recorded by this fixture.
	#[benchmark(pov_mode = Measured)]
	fn update() {
		let active_count = T::MaxActiveNotaries::get();
		seed_active_notaries::<T>(active_count);
		let notary = ActiveNotaries::<T>::get()
			.last()
			.expect("benchmark requires an active notary")
			.clone();
		let effective_tick = T::TickProvider::current_tick() + T::MetaChangesTickDelay::get();
		let mut changes = BoundedBTreeMap::new();
		for index in 0..active_count {
			let _ = changes.try_insert(index.saturating_add(1), benchmark_meta::<T>(index as u8));
		}
		QueuedNotaryMetaChanges::<T>::insert(effective_tick, changes);
		let meta = benchmark_meta::<T>(u8::MAX);

		#[extrinsic_call]
		update(
			RawOrigin::Signed(notary.operator_account_id),
			notary.notary_id,
			meta,
			effective_tick,
		);

		assert_eq!(QueuedNotaryMetaChanges::<T>::get(effective_tick).len() as u32, active_count);
	}

	#[benchmark]
	fn on_initialize_without_meta_changes() {
		let current_tick = T::TickProvider::current_tick();

		#[block]
		{
			Pallet::<T>::on_initialize(1u32.into());
		}

		assert!(QueuedNotaryMetaChanges::<T>::get(current_tick).is_empty());
	}

	#[benchmark]
	fn on_initialize_with_meta_changes(m: Linear<1, 25>) {
		let current_tick = T::TickProvider::current_tick();
		let change_count = m.min(T::MaxActiveNotaries::get());
		seed_active_notaries::<T>(T::MaxActiveNotaries::get());
		let mut changes = BoundedBTreeMap::new();
		for index in 0..change_count {
			let notary_id = index.saturating_add(1);
			let _ = changes.try_insert(notary_id, benchmark_meta::<T>(u8::MAX));
			let history = (0..T::MaxTicksForKeyHistory::get().saturating_sub(1))
				.map(|tick| (tick as Tick, NotaryPublic::from_raw([index as u8; 32])))
				.collect::<Vec<_>>();
			NotaryKeyHistory::<T>::insert(notary_id, BoundedVec::truncate_from(history));
		}
		QueuedNotaryMetaChanges::<T>::insert(current_tick, changes);

		#[block]
		{
			Pallet::<T>::on_initialize(1u32.into());
		}

		assert!(QueuedNotaryMetaChanges::<T>::get(current_tick).is_empty());
	}

	#[benchmark]
	fn on_finalize_with_expiring_proposals(p: Linear<0, 10>) {
		let block_number = 1u32.into();
		let proposal_count = p.min(T::MaxProposalsPerBlock::get());
		let mut expiring = Vec::new();
		for index in 0..proposal_count {
			let operator = account("proposal", index, 0);
			ProposedNotaries::<T>::insert(
				&operator,
				(benchmark_meta::<T>(index as u8), block_number),
			);
			expiring.push(operator);
		}
		ExpiringProposals::<T>::insert(block_number, BoundedVec::truncate_from(expiring));

		#[block]
		{
			Pallet::<T>::on_finalize(block_number);
		}

		assert!(!ExpiringProposals::<T>::contains_key(block_number));
		assert!(ProposedNotaries::<T>::iter().next().is_none());
	}

	#[benchmark]
	fn provider_active_notaries() {
		let count = T::MaxActiveNotaries::get();
		seed_active_notaries::<T>(count);

		#[block]
		{
			assert_eq!(
				<Pallet<T> as NotaryProvider<T::Block, T::AccountId>>::active_notaries().len()
					as u32,
				count,
			);
		}
	}

	impl_benchmark_test_suite!(Pallet, crate::mock::new_test_ext(), crate::mock::Test);
}

fn benchmark_meta<T: Config>(seed: u8) -> NotaryMeta<T::MaxNotaryHosts> {
	let hosts = (0..T::MaxNotaryHosts::get())
		.map(|_| Host::from(vec![seed; 253]))
		.collect::<Vec<_>>();
	NotaryMeta {
		name: NotaryName::from(vec![seed; 55]),
		public: NotaryPublic::from_raw([seed; 32]),
		hosts: BoundedVec::truncate_from(hosts),
	}
}

fn benchmark_record<T: Config>(
	index: u32,
) -> NotaryRecord<T::AccountId, BlockNumberFor<T>, T::MaxNotaryHosts> {
	NotaryRecord {
		notary_id: index.saturating_add(1),
		operator_account_id: account("notary", index, 0),
		activated_block: 1u32.into(),
		meta_updated_block: 1u32.into(),
		meta_updated_tick: 1,
		meta: benchmark_meta::<T>(index as u8),
	}
}

fn seed_active_notaries<T: Config>(count: u32) {
	let notaries = (0..count).map(benchmark_record::<T>).collect::<Vec<_>>();
	ActiveNotaries::<T>::put(BoundedVec::truncate_from(notaries));
}
