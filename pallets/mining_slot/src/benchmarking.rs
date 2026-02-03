//! Benchmarking setup for pallet-mining-slot
#![cfg(feature = "runtime-benchmarks")]

use super::*;
use argon_primitives::{AccountId, NotaryId, balance_change::MerkleProof, localchain::BlockVote};
use frame_benchmarking::v2::*;
use frame_system::RawOrigin;
use sp_core::U256;

#[benchmarks(
	where
		T::OwnershipCurrency: frame_support::traits::fungible::Mutate<T::AccountId, Balance = T::Balance>,
)]
mod benchmarks {
	use super::*;

	/// Core extrinsic: Bid (new miner)
	#[benchmark]
	fn bid() -> Result<(), BenchmarkError> {
		let caller: T::AccountId = whitelisted_caller();
		let bid_increment: T::Balance = T::BidIncrements::get();

		// Inline key creation
		use sp_runtime::codec::Decode;
		let key_size = T::Keys::max_encoded_len();
		let key_data = vec![0u8; key_size];
		let keys = T::Keys::decode(&mut &key_data[..])
			.expect("Failed to create test keys for benchmarking");

		// Inline mining environment setup
		let mining_config = MiningSlotConfig {
			slot_bidding_start_after_ticks: 0u64,
			ticks_between_slots: 10u64,
			ticks_before_bid_end_for_vrf_close: 1u64,
		};
		MiningConfig::<T>::put(mining_config);
		IsNextSlotBiddingOpen::<T>::put(true);
		let argonots_per_seat: T::Balance = 100_000u128.into();
		ArgonotsPerMiningSeat::<T>::put(argonots_per_seat);
		NextFrameId::<T>::put(2);
		let next_cohort_size = T::MaxCohortSize::get().max(1);
		NextCohortSize::<T>::put(next_cohort_size);
		let bid_amount: T::Balance = bid_increment.saturating_mul((next_cohort_size + 1).into());

		// Inline account setup
		use frame_support::traits::fungible::Mutate;
		let _ = T::ArgonCurrency::mint_into(&caller, 5_000_000u128.into());
		let _ = T::OwnershipCurrency::mint_into(&caller, 5_000_000u128.into());

		// Inline bid pool setup
		let bid_pool_account = T::BidPoolProvider::get_bid_pool_account();
		let _ = T::ArgonCurrency::mint_into(&bid_pool_account, 10_000_000u128.into());

		// Pre-fill the cohort so the new bid bumps the lowest bidder.
		for i in 0..next_cohort_size {
			let bidder: T::AccountId = account("prefill", i, 0);
			let _ = T::ArgonCurrency::mint_into(&bidder, 5_000_000u128.into());
			let _ = T::OwnershipCurrency::mint_into(&bidder, 5_000_000u128.into());
			let prefill_bid: T::Balance = bid_increment.saturating_mul((i + 1).into());

			use sp_runtime::codec::Decode;
			let key_size = T::Keys::max_encoded_len();
			let key_data = vec![0u8; key_size];
			let keys = T::Keys::decode(&mut &key_data[..])
				.expect("Failed to create test keys for benchmarking");
			let _ = Pallet::<T>::bid(RawOrigin::Signed(bidder).into(), prefill_bid, keys, None);
		}
		let bumped = BidsForNextSlotCohort::<T>::get()
			.iter()
			.last()
			.expect("Bid queue should be populated")
			.account_id
			.clone();

		#[extrinsic_call]
		_(RawOrigin::Signed(caller.clone()), bid_amount, keys, None);

		// Verify the bid was actually placed
		let bids = BidsForNextSlotCohort::<T>::get();
		assert!(!bids.is_empty(), "Bid queue should contain the placed bid");
		let bid_exists = bids.iter().any(|registration| {
			registration.account_id == caller && registration.bid >= bid_amount
		});
		assert!(bid_exists, "Bid should be found in the bid queue with correct amount");
		assert!(
			!bids.iter().any(|registration| registration.account_id == bumped),
			"Lowest bidder should be bumped from the queue"
		);

		Ok(())
	}

	/// Core extrinsic: Configure mining slot delay
	#[benchmark]
	fn configure_mining_slot_delay() -> Result<(), BenchmarkError> {
		// Inline mining environment setup
		let mining_config = MiningSlotConfig {
			slot_bidding_start_after_ticks: 0u64,
			ticks_between_slots: 10u64,
			ticks_before_bid_end_for_vrf_close: 1u64,
		};
		MiningConfig::<T>::put(mining_config);
		IsNextSlotBiddingOpen::<T>::put(true);
		let argonots_per_seat: T::Balance = 100_000u128.into();
		ArgonotsPerMiningSeat::<T>::put(argonots_per_seat);
		NextFrameId::<T>::put(2);
		let m = T::MinCohortSize::get();
		NextCohortSize::<T>::put(m);

		#[extrinsic_call]
		_(RawOrigin::Root, Some(10u64), Some(5u64));

		// Verify the mining slot delay was actually configured
		let config = MiningConfig::<T>::get();
		assert_eq!(
			config.slot_bidding_start_after_ticks, 10u64,
			"Slot bidding start delay should be updated"
		);
		assert_eq!(
			config.ticks_before_bid_end_for_vrf_close, 5u64,
			"VRF close delay should be updated"
		);

		Ok(())
	}

	/// Hook: On finalize GRANDPA rotation (treated as constant-cost; uses `T::MinCohortSize` for
	/// setup)
	#[benchmark]
	fn on_finalize_grandpa_rotation() -> Result<(), BenchmarkError> {
		// m = miners
		// Inline mining environment setup - ensure grandpa rotation path is used (no frame start)
		let mining_config = MiningSlotConfig {
			slot_bidding_start_after_ticks: 0u64,
			ticks_between_slots: 1u64,
			ticks_before_bid_end_for_vrf_close: 1u64,
		};
		MiningConfig::<T>::put(mining_config);
		IsNextSlotBiddingOpen::<T>::put(true);
		let argonots_per_seat: T::Balance = 100_000u128.into();
		ArgonotsPerMiningSeat::<T>::put(argonots_per_seat);
		NextFrameId::<T>::put(2);
		FrameRewardTicksRemaining::<T>::put(2u64);
		ActiveMinersCount::<T>::put(1u16);
		HasAddedGrandpaRotation::<T>::put(false);
		let m = T::MinCohortSize::get();
		NextCohortSize::<T>::put(m);

		// Inline bid pool setup
		use frame_support::traits::fungible::Mutate;
		let bid_pool_account = T::BidPoolProvider::get_bid_pool_account();
		let _ = T::ArgonCurrency::mint_into(&bid_pool_account, 100_000_000u128.into());

		// Create more bidders than slots
		for i in 0..(m * 2) {
			let bidder: T::AccountId = account("bidder", i, 0);
			let _ = T::ArgonCurrency::mint_into(&bidder, 5_000_000u128.into());
			let _ = T::OwnershipCurrency::mint_into(&bidder, 200_000u128.into());

			let bid_amount: T::Balance = (1_000_000u128 + (i as u128 * 10_000)).into();
			use sp_runtime::codec::Decode;
			let key_size = T::Keys::max_encoded_len();
			let key_data = vec![0u8; key_size];
			let keys = T::Keys::decode(&mut &key_data[..])
				.expect("Failed to create test keys for benchmarking");
			let _ = Pallet::<T>::bid(RawOrigin::Signed(bidder).into(), bid_amount, keys, None);
		}

		#[block]
		{
			let _ = Pallet::<T>::on_finalize(1u32.into());
		}

		// Verify rotation path was taken
		assert!(
			HasAddedGrandpaRotation::<T>::get(),
			"Grandpa rotation flag should be set after finalization"
		);

		Ok(())
	}

	/// Frame transitions: start_new_frame with cross-pallet operations
	#[benchmark]
	fn start_new_frame(m: Linear<1, 256>) -> Result<(), BenchmarkError> {
		let max_cohort = T::MaxCohortSize::get().max(1);
		let m = m.min(max_cohort);
		// Inline mining environment setup
		let mining_config = MiningSlotConfig {
			slot_bidding_start_after_ticks: 0u64,
			ticks_between_slots: 10u64,
			ticks_before_bid_end_for_vrf_close: 1u64,
		};
		MiningConfig::<T>::put(mining_config);
		IsNextSlotBiddingOpen::<T>::put(true);
		let argonots_per_seat: T::Balance = 100_000u128.into();
		ArgonotsPerMiningSeat::<T>::put(argonots_per_seat);
		let frames_per_term = T::FramesPerMiningTerm::get() as FrameId;
		let target_frame = frames_per_term;
		let retiring_frame_id = target_frame.saturating_sub(frames_per_term);
		NextFrameId::<T>::put(target_frame);
		NextCohortSize::<T>::put(m);
		ActiveMinersCount::<T>::put(m as u16);

		// Inline bid pool setup
		use frame_support::traits::fungible::Mutate;
		let bid_pool_account = T::BidPoolProvider::get_bid_pool_account();
		let _ = T::ArgonCurrency::mint_into(&bid_pool_account, 100_000_000u128.into());

		// Seed a retiring cohort to exercise rotation logic.
		let hold_reason: T::RuntimeHoldReason = HoldReason::RegisterAsMiner.into();
		let mut retiring_miners: BoundedVec<Registration<T>, T::MaxCohortSize> = BoundedVec::new();
		for i in 0..m {
			let account_id: T::AccountId = account("retire", i, 0);
			let _ = T::OwnershipCurrency::mint_into(&account_id, 200_000u128.into());
			if T::OwnershipCurrency::balance_on_hold(&hold_reason, &account_id) == 0u32.into() {
				frame_system::Pallet::<T>::inc_providers(&account_id);
			}
			let _ = T::OwnershipCurrency::hold(&hold_reason, &account_id, argonots_per_seat);

			let bid_amount: T::Balance = (1_000_000u128 + (i as u128 * 10_000)).into();
			use sp_runtime::codec::Decode;
			let key_size = T::Keys::max_encoded_len();
			let key_data = vec![0u8; key_size];
			let keys = T::Keys::decode(&mut &key_data[..])
				.expect("Failed to create test keys for benchmarking");

			let registration = Registration::<T> {
				account_id: account_id.clone(),
				external_funding_account: None,
				bid: bid_amount,
				argonots: argonots_per_seat,
				authority_keys: keys,
				starting_frame_id: retiring_frame_id,
				bid_at_tick: 0u64,
			};
			let _ = retiring_miners.try_push(registration);
			AccountIndexLookup::<T>::insert(&account_id, (retiring_frame_id, i as u32));
		}
		MinersByCohort::<T>::insert(retiring_frame_id, retiring_miners);

		// Create m bidders for new cohort
		for i in 0..m {
			let bidder: T::AccountId = account("bidder", i, 0);
			let _ = T::ArgonCurrency::mint_into(&bidder, 5_000_000u128.into());
			let _ = T::OwnershipCurrency::mint_into(&bidder, 200_000u128.into());

			let bid_amount: T::Balance = (1_000_000u128 + (i as u128 * 10_000)).into();
			use sp_runtime::codec::Decode;
			let key_size = T::Keys::max_encoded_len();
			let key_data = vec![0u8; key_size];
			let keys = T::Keys::decode(&mut &key_data[..])
				.expect("Failed to create test keys for benchmarking");
			let _ = Pallet::<T>::bid(RawOrigin::Signed(bidder).into(), bid_amount, keys, None);
		}

		let config = MiningSlotConfig {
			slot_bidding_start_after_ticks: 0u64,
			ticks_between_slots: 1u64,
			ticks_before_bid_end_for_vrf_close: 1u64,
		};
		MiningConfig::<T>::put(config);

		#[block]
		{
			Pallet::<T>::start_new_frame(target_frame);
		}

		let new_frame_id = NextFrameId::<T>::get();
		assert_eq!(
			new_frame_id,
			target_frame + 1,
			"Frame ID should be updated to target frame + 1"
		);
		let bid_queue = BidsForNextSlotCohort::<T>::get();
		assert!(bid_queue.is_empty(), "Bid queue should be cleared after frame transition");

		Ok(())
	}

	/// Benchmark the adjust operations that happen in on_finalize during frame start
	#[benchmark]
	fn on_finalize_frame_adjustments() -> Result<(), BenchmarkError> {
		let mining_config = MiningSlotConfig {
			slot_bidding_start_after_ticks: 0u64,
			ticks_between_slots: 10u64,
			ticks_before_bid_end_for_vrf_close: 1u64,
		};
		MiningConfig::<T>::put(mining_config);

		let mut bid_stats_vec = BoundedVec::new();
		for i in 0..10 {
			let bid_stat = MiningBidStats {
				bids_count: 10 + i,
				bid_amount_min: 1_000_000u128,
				bid_amount_max: 5_000_000u128,
				bid_amount_sum: 30_000_000u128,
			};
			let _ = bid_stats_vec.try_push(bid_stat);
		}
		HistoricalBidsPerSlot::<T>::put(bid_stats_vec);

		let argonots_amount: T::Balance = 100_000u128.into();
		ArgonotsPerMiningSeat::<T>::put(argonots_amount);
		NextCohortSize::<T>::put(T::MinCohortSize::get());
		ActiveMinersCount::<T>::put(10u16);

		#[block]
		{
			Pallet::<T>::adjust_argonots_per_seat();
			Pallet::<T>::adjust_number_of_seats();
		}

		Ok(())
	}

	/// Event handler cost for a vote seal
	#[benchmark]
	fn block_seal_read_vote() -> Result<(), BenchmarkError> {
		let mining_config = MiningSlotConfig {
			slot_bidding_start_after_ticks: 0u64,
			ticks_between_slots: 10u64,
			ticks_before_bid_end_for_vrf_close: 2u64,
		};
		MiningConfig::<T>::put(mining_config);
		IsNextSlotBiddingOpen::<T>::put(true);
		FrameRewardTicksRemaining::<T>::put(1u64);
		NextFrameId::<T>::put(2u64);

		let notary_id: NotaryId = 1u32;
		let tick: Tick = 1u64;
		let notary_account = AccountId::new([0u8; 32]);
		let block_vote = BlockVote::create_default_vote(notary_account, tick);
		let seal = BlockSealInherent::Vote {
			seal_strength: U256::zero(),
			notary_id,
			source_notebook_number: 1u32,
			source_notebook_proof: MerkleProof::default(),
			block_vote,
			miner_nonce_score: None,
		};

		#[block]
		{
			<Pallet<T> as BlockSealEventHandler>::block_seal_read(&seal, Some(U256::zero()));
		}

		assert!(!IsNextSlotBiddingOpen::<T>::get());
		Ok(())
	}
}
