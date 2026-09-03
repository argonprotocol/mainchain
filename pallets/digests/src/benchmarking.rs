use super::*;
use argon_primitives::{BlockVoteDigest, NotebookAuditResult, NotebookDigest};
use frame_support::traits::Hooks;
use pallet_prelude::benchmarking::set_all_digests;
use polkadot_sdk::frame_benchmarking::v2::*;

#[benchmarks(where T::NotebookVerifyError: From<argon_notary_audit::VerifyError>)]
mod benchmarks {
	use super::*;

	#[benchmark]
	fn on_initialize(n: Linear<0, 256>) {
		seed_digests::<T>(n);
		assert!(!TempDigests::<T>::exists());

		#[block]
		{
			Pallet::<T>::on_initialize(1u32.into());
		}

		assert_eq!(
			TempDigests::<T>::get()
				.expect("digests should be cached")
				.notebooks
				.notebooks
				.len(),
			n as usize
		);
	}

	#[benchmark]
	fn on_finalize() {
		seed_digests::<T>(0);
		Pallet::<T>::on_initialize(1u32.into());
		assert!(TempDigests::<T>::exists());

		#[block]
		{
			Pallet::<T>::on_finalize(1u32.into());
		}

		assert!(!TempDigests::<T>::exists());
	}

	impl_benchmark_test_suite!(Pallet, crate::mock::new_test_ext(), crate::mock::Test);
}

fn seed_digests<T: Config>(notebook_count: u32)
where
	T::NotebookVerifyError: From<argon_notary_audit::VerifyError>,
{
	let author: T::AccountId = whitelisted_caller();
	let notebooks = (1..=notebook_count)
		.map(|_| NotebookAuditResult::<T::NotebookVerifyError> {
			notary_id: u32::MAX,
			notebook_number: u32::MAX,
			tick: u64::MAX,
			audit_first_failure: Some(
				argon_notary_audit::VerifyError::BalanceChangeError {
					change_index: u16::MAX,
					note_index: u16::MAX,
					message: "x".repeat(100).try_into().expect("maximum bounded error message"),
				}
				.into(),
			),
		})
		.collect::<Vec<_>>();

	set_all_digests::<T, T::NotebookVerifyError>(
		author,
		1,
		BlockVoteDigest { voting_power: 1, votes_count: 1 },
		NotebookDigest { notebooks: BoundedVec::truncate_from(notebooks) },
	);
}
