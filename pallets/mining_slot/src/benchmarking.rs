//! Benchmarking setup for pallet-mining-slot
#![cfg(feature = "runtime-benchmarks")]

use super::*;
use argon_primitives::{
	balance_change::MerkleProof, localchain::BlockVote, AccountId, BlockRewardAccountsProvider,
	MiningSlotProvider, NotaryId,
};
use frame_system::RawOrigin;
use polkadot_sdk::{frame_benchmarking::v2::*, sp_runtime::BoundedBTreeMap};
use sp_core::U256;

#[benchmarks(
	where
		T::OwnershipCurrency: frame_support::traits::fungible::Mutate<T::AccountId, Balance = T::Balance>,
)]
mod benchmarks {
	use super::*;

	fn prepare_bid<T: Config>(caller: &T::AccountId) -> T::Keys
	where
		T::OwnershipCurrency:
			frame_support::traits::fungible::Mutate<T::AccountId, Balance = T::Balance>,
	{
		use frame_support::traits::fungible::Mutate;
		IsNextSlotBiddingOpen::<T>::put(true);
		AuctionRulesStartAt::<T>::put(0);
		NextFrameId::<T>::put(T::FramesPerMiningTerm::get() as FrameId + 1);
		HistoricalBidsPerSlot::<T>::put(BoundedVec::truncate_from(vec![
			MiningBidStats::default();
			10
		]));
		HasAcceptedBidPrice::<T>::kill();
		FrameRewardTicksRemaining::<T>::put(100u64);
		ArgonotsPerMiningSeat::<T>::put(T::Balance::from(100_000u128));
		let _ = T::ArgonCurrency::mint_into(caller, 1_000_000_000_000u128.into());
		let _ = T::OwnershipCurrency::mint_into(caller, 1_000_000_000u128.into());
		T::Keys::decode(&mut &vec![0u8; T::Keys::max_encoded_len()][..]).expect("benchmark keys")
	}

	fn fill_cohort<T: Config>() {
		let keys = T::Keys::decode(&mut &vec![0u8; T::Keys::max_encoded_len()][..])
			.expect("benchmark keys");
		let entries = (0..T::MaxCohortSize::get())
			.map(|i| Registration::<T> {
				account_id: account("fixed", i, 0),
				external_funding_account: None,
				bid: T::BidIncrements::get(),
				argonots: T::Balance::from(100_000u128),
				authority_keys: keys.clone(),
				starting_frame_id: NextFrameId::<T>::get(),
				bid_at_tick: 0,
			})
			.collect::<Vec<_>>();
		for (position, registration) in entries.iter().enumerate() {
			T::ArgonCurrency::mint_into(&registration.account_id, 1_000_000_000_000u128.into())
				.expect("funded standing benchmark");
			StandingBids::<T>::insert(
				&registration.account_id,
				StandingBid::<T> {
					bid: registration.bid,
					max: registration.bid.saturating_mul(2u32.into()),
					fixed: false,
					submitted_at: Zero::zero(),
					position: position as u32,
				},
			);
		}
		BidsForNextSlotCohort::<T>::put(
			BoundedVec::<Registration<T>, T::MaxCohortSize>::try_from(entries).ok().unwrap(),
		);
	}

	fn fill_active_cohort<T: Config>(caller: &T::AccountId, keys: &T::Keys) {
		let entries = (0..T::MaxCohortSize::get())
			.map(|i| Registration::<T> {
				account_id: if i + 1 == T::MaxCohortSize::get() {
					caller.clone()
				} else {
					account("active", i, 0)
				},
				external_funding_account: None,
				bid: T::BidIncrements::get(),
				argonots: 1u128.into(),
				authority_keys: keys.clone(),
				starting_frame_id: 1,
				bid_at_tick: 0,
			})
			.collect::<Vec<_>>();
		MinersByCohort::<T>::insert(
			1,
			BoundedVec::<Registration<T>, T::MaxCohortSize>::try_from(entries).ok().unwrap(),
		);
		AccountIndexLookup::<T>::insert(caller, (1, T::MaxCohortSize::get() - 1));
		frame_system::Pallet::<T>::inc_providers(caller);
		T::OwnershipCurrency::hold(&HoldReason::RegisterAsMiner.into(), caller, 1u128.into())
			.expect("funded benchmark collateral");
	}

	#[benchmark]
	fn bid() -> Result<(), BenchmarkError> {
		let caller: T::AccountId = whitelisted_caller();
		let keys = prepare_bid::<T>(&caller);
		frame_system::Pallet::<T>::set_block_number(1u32.into());
		fill_cohort::<T>();
		// Exercise full-size incumbent scans and a nearly full commitment vector.
		NextCohortSize::<T>::put(T::MaxCohortSize::get());
		for i in 0..T::MaxBidsPerBlock::get().saturating_sub(1) {
			let other = account("queued", i, 0);
			let other_keys = prepare_bid::<T>(&other);
			Pallet::<T>::bid(
				RawOrigin::Signed(other).into(),
				T::BidIncrements::get().saturating_mul(3u32.into()),
				other_keys,
				None,
				Some(NextFrameId::<T>::get()),
				None,
			)?;
		}
		#[extrinsic_call]
		_(
			RawOrigin::Signed(caller.clone()),
			T::BidIncrements::get().saturating_mul(3u32.into()),
			keys,
			None,
			Some(NextFrameId::<T>::get()),
			None,
		);
		assert!(PendingBids::<T>::get(caller).unwrap().max.is_none());
		Ok(())
	}
	#[benchmark]
	fn bid_renewal() -> Result<(), BenchmarkError> {
		let caller: T::AccountId = whitelisted_caller();
		let keys = prepare_bid::<T>(&caller);
		fill_active_cohort::<T>(
			&caller,
			&T::Keys::decode(&mut &vec![0u8; T::Keys::max_encoded_len()][..]).unwrap(),
		);
		frame_system::Pallet::<T>::set_block_number(1u32.into());
		fill_cohort::<T>();
		// Exercise full-size incumbent scans and a nearly full commitment vector.
		NextCohortSize::<T>::put(T::MaxCohortSize::get());
		for i in 0..T::MaxBidsPerBlock::get().saturating_sub(1) {
			let other = account("queued", i, 0);
			let other_keys = prepare_bid::<T>(&other);
			Pallet::<T>::bid(
				RawOrigin::Signed(other).into(),
				T::BidIncrements::get().saturating_mul(3u32.into()),
				other_keys,
				None,
				Some(NextFrameId::<T>::get()),
				None,
			)?;
		}
		#[extrinsic_call]
		bid(
			RawOrigin::Signed(caller.clone()),
			T::BidIncrements::get().saturating_mul(3u32.into()),
			keys,
			None,
			Some(NextFrameId::<T>::get()),
			None,
		);
		assert!(PendingBids::<T>::get(caller).unwrap().max.is_none());
		Ok(())
	}

	#[benchmark]
	fn bid_legacy() -> Result<(), BenchmarkError> {
		let _ = funded_cohort::<T>(T::MaxCohortSize::get())?;
		let who: T::AccountId = whitelisted_caller();
		let keys = prepare_bid::<T>(&who);
		#[block]
		{
			Pallet::<T>::legacy_bid(
				who.clone(),
				T::BidIncrements::get().saturating_mul(2u32.into()),
				keys,
				who,
			)?;
		}
		Ok(())
	}

	#[benchmark]
	fn auction_bookkeeping(f: Linear<0, 1>) -> Result<(), BenchmarkError> {
		AuctionRulesStartAt::<T>::put(0);
		NextFrameId::<T>::put(1 + f as u64);
		FrameRewardTicksRemaining::<T>::put(0);
		IsNextSlotBiddingOpen::<T>::put(true);
		let block: BlockNumberFor<T> = 20u32.into();
		LastBidBlock::<T>::put(block);
		BidBatches::<T>::insert(
			BlockNumberFor::<T>::from(10u32),
			BoundedVec::<T::AccountId, T::MaxBidsPerBlock>::default(),
		);
		HasAddedGrandpaRotation::<T>::put(true);
		frame_system::Pallet::<T>::set_block_number(block - One::one());
		frame_system::Pallet::<T>::initialize(
			&block,
			&frame_system::Pallet::<T>::parent_hash(),
			&sp_runtime::Digest {
				logs: vec![sp_runtime::DigestItem::Consensus(
					argon_primitives::FRAME_INFO_DIGEST,
					argon_primitives::FrameInfo {
						frame_id: f as u64,
						frame_reward_ticks_remaining: 0,
						is_new_frame: false,
					}
					.encode(),
				)],
			},
		);
		#[block]
		{
			let _ = Pallet::<T>::on_initialize(block);
			Pallet::<T>::on_finalize(block);
		}
		Ok(())
	}

	#[benchmark]
	fn bid_with_max() -> Result<(), BenchmarkError> {
		let caller: T::AccountId = whitelisted_caller();
		prepare_bid::<T>(&caller);
		frame_system::Pallet::<T>::set_block_number(1u32.into());
		fill_cohort::<T>();
		for i in 0..T::MaxBidsPerBlock::get().saturating_sub(1) {
			let other = account("queued", i, 0);
			prepare_bid::<T>(&other);
			Pallet::<T>::bid(
				RawOrigin::Signed(other).into(),
				T::BidIncrements::get().saturating_mul(3u32.into()),
				T::Keys::decode(&mut &vec![0; T::Keys::max_encoded_len()][..]).unwrap(),
				None,
				Some(NextFrameId::<T>::get()),
				Some([0; 32]),
			)?;
		}
		#[extrinsic_call]
		bid(
			RawOrigin::Signed(caller.clone()),
			T::BidIncrements::get().saturating_mul(3u32.into()),
			T::Keys::decode(&mut &vec![0; T::Keys::max_encoded_len()][..]).unwrap(),
			None,
			Some(NextFrameId::<T>::get()),
			Some([1; 32]),
		);
		assert!(PendingBids::<T>::contains_key(caller));
		Ok(())
	}
	#[benchmark]
	fn bid_with_max_renewal() -> Result<(), BenchmarkError> {
		let caller: T::AccountId = whitelisted_caller();
		prepare_bid::<T>(&caller);
		fill_active_cohort::<T>(
			&caller,
			&T::Keys::decode(&mut &vec![0u8; T::Keys::max_encoded_len()][..]).unwrap(),
		);
		frame_system::Pallet::<T>::set_block_number(1u32.into());
		fill_cohort::<T>();
		for i in 0..T::MaxBidsPerBlock::get().saturating_sub(1) {
			let other = account("queued", i, 0);
			prepare_bid::<T>(&other);
			Pallet::<T>::bid(
				RawOrigin::Signed(other).into(),
				T::BidIncrements::get().saturating_mul(3u32.into()),
				T::Keys::decode(&mut &vec![0; T::Keys::max_encoded_len()][..]).unwrap(),
				None,
				Some(NextFrameId::<T>::get()),
				Some([0; 32]),
			)?;
		}
		#[extrinsic_call]
		bid(
			RawOrigin::Signed(caller.clone()),
			T::BidIncrements::get().saturating_mul(3u32.into()),
			T::Keys::decode(&mut &vec![0; T::Keys::max_encoded_len()][..]).unwrap(),
			None,
			Some(NextFrameId::<T>::get()),
			Some([1; 32]),
		);
		assert!(PendingBids::<T>::contains_key(caller));
		Ok(())
	}

	#[benchmark]
	fn bid_max() -> Result<(), BenchmarkError> {
		let caller: T::AccountId = whitelisted_caller();
		let keys = prepare_bid::<T>(&caller);
		frame_system::Pallet::<T>::set_block_number(1u32.into());
		fill_cohort::<T>();
		let low = T::BidIncrements::get().saturating_mul(3u32.into());
		let high = low.saturating_mul(3u32.into());
		let hash = Pallet::<T>::bid_commitment(
			NextFrameId::<T>::get(),
			&caller,
			&caller,
			low,
			high,
			&keys,
			[1; 32],
		);
		Pallet::<T>::bid(
			RawOrigin::Signed(caller.clone()).into(),
			low,
			T::Keys::decode(&mut &vec![0; T::Keys::max_encoded_len()][..]).unwrap(),
			None,
			Some(NextFrameId::<T>::get()),
			Some(hash),
		)?;
		frame_system::Pallet::<T>::set_block_number(2u32.into());
		let target_frame_id = NextFrameId::<T>::get();
		#[extrinsic_call]
		_(RawOrigin::Signed(caller.clone()), high, [1; 32], None, target_frame_id);
		assert!(PendingBids::<T>::get(caller).unwrap().max.is_some());
		Ok(())
	}
	#[benchmark]
	fn bid_max_renewal() -> Result<(), BenchmarkError> {
		let caller: T::AccountId = whitelisted_caller();
		let keys = prepare_bid::<T>(&caller);
		fill_active_cohort::<T>(
			&caller,
			&T::Keys::decode(&mut &vec![0u8; T::Keys::max_encoded_len()][..]).unwrap(),
		);
		frame_system::Pallet::<T>::set_block_number(1u32.into());
		fill_cohort::<T>();
		// Exercise a miner that is both active and already a next-cohort winner.
		BidsForNextSlotCohort::<T>::mutate(|cohort| {
			cohort.last_mut().expect("nonempty benchmark cohort").account_id = caller.clone();
		});
		StandingBids::<T>::insert(
			&caller,
			StandingBid::<T> {
				bid: T::BidIncrements::get(),
				max: T::BidIncrements::get().saturating_mul(2u32.into()),
				fixed: false,
				submitted_at: Zero::zero(),
				position: T::MaxCohortSize::get() - 1,
			},
		);
		T::OwnershipCurrency::hold(
			&HoldReason::RegisterAsMiner.into(),
			&caller,
			ArgonotsPerMiningSeat::<T>::get().saturating_sub(1u128.into()),
		)?;
		let low = T::BidIncrements::get().saturating_mul(3u32.into());
		let high = low.saturating_mul(3u32.into());
		let hash = Pallet::<T>::bid_commitment(
			NextFrameId::<T>::get(),
			&caller,
			&caller,
			low,
			high,
			&keys,
			[1; 32],
		);
		Pallet::<T>::bid(
			RawOrigin::Signed(caller.clone()).into(),
			low,
			T::Keys::decode(&mut &vec![0; T::Keys::max_encoded_len()][..]).unwrap(),
			None,
			Some(NextFrameId::<T>::get()),
			Some(hash),
		)?;
		frame_system::Pallet::<T>::set_block_number(2u32.into());
		let target_frame_id = NextFrameId::<T>::get();
		#[extrinsic_call]
		bid_max(RawOrigin::Signed(caller.clone()), high, [1; 32], None, target_frame_id);
		assert!(PendingBids::<T>::get(caller).unwrap().max.is_some());
		Ok(())
	}

	fn funded_cohort<T: Config>(m: u32) -> Result<BlockNumberFor<T>, BenchmarkError>
	where
		T::OwnershipCurrency:
			frame_support::traits::fungible::Mutate<T::AccountId, Balance = T::Balance>,
	{
		NextCohortSize::<T>::put(m);
		let mut block = 1u32;
		for start in (0..m).step_by(T::MaxBidsPerBlock::get() as usize) {
			frame_system::Pallet::<T>::set_block_number(block.into());
			for i in start..m.min(start + T::MaxBidsPerBlock::get()) {
				let who = account("incumbent", i, 0);
				let keys = prepare_bid::<T>(&who);
				let low = T::BidIncrements::get();
				let high = low.saturating_mul(2u32.into());
				let hash = Pallet::<T>::bid_commitment(
					NextFrameId::<T>::get(),
					&who,
					&who,
					low,
					high,
					&keys,
					[2; 32],
				);
				Pallet::<T>::bid(
					RawOrigin::Signed(who.clone()).into(),
					low,
					T::Keys::decode(&mut &vec![0; T::Keys::max_encoded_len()][..]).unwrap(),
					None,
					Some(NextFrameId::<T>::get()),
					Some(hash),
				)?;
			}
			frame_system::Pallet::<T>::set_block_number((block + 1).into());
			for i in start..(start + T::MaxBidsPerBlock::get()).min(m) {
				let who = account("incumbent", i, 0);
				Pallet::<T>::bid_max(
					RawOrigin::Signed(who).into(),
					T::BidIncrements::get().saturating_mul(2u32.into()),
					[2; 32],
					None,
					NextFrameId::<T>::get(),
				)?;
			}
			frame_system::Pallet::<T>::set_block_number((block + T::BidRevealBlocks::get()).into());
			Pallet::<T>::settle_bids(block.into())?;
			block += T::BidRevealBlocks::get() + 1;
		}
		// Renewing incumbents also require decoding their full active cohort on eviction.
		// Their existing collateral covers one active unit plus the renewal's remainder.
		let next = BidsForNextSlotCohort::<T>::get();
		let mut active = Vec::new();
		for i in 0..T::MaxCohortSize::get() {
			let mut registration = if i < m {
				next[i as usize].clone()
			} else {
				let who = account("active-padding", i, 0);
				let keys = prepare_bid::<T>(&who);
				frame_system::Pallet::<T>::inc_providers(&who);
				T::OwnershipCurrency::hold(
					&HoldReason::RegisterAsMiner.into(),
					&who,
					1u128.into(),
				)?;
				Registration::<T> {
					account_id: who,
					external_funding_account: None,
					bid: T::BidIncrements::get(),
					argonots: 1u128.into(),
					authority_keys: keys,
					starting_frame_id: 1,
					bid_at_tick: 0,
				}
			};
			registration.starting_frame_id = 1;
			registration.argonots = 1u128.into();
			AccountIndexLookup::<T>::insert(&registration.account_id, (1, i));
			active.push(registration);
		}
		MinersByCohort::<T>::insert(
			1,
			BoundedVec::<Registration<T>, T::MaxCohortSize>::try_from(active).ok().unwrap(),
		);
		Ok(block.into())
	}

	fn settlement_setup<T: Config>(
		b: u32,
		m: u32,
		replacements: bool,
	) -> Result<BlockNumberFor<T>, BenchmarkError>
	where
		T::OwnershipCurrency:
			frame_support::traits::fungible::Mutate<T::AccountId, Balance = T::Balance>,
	{
		let block = funded_cohort::<T>(m)?;
		frame_system::Pallet::<T>::set_block_number(block);
		for i in 0..b {
			let who = account("challenger", i, 0);
			let keys = prepare_bid::<T>(&who);
			let low = T::BidIncrements::get().saturating_mul(3u32.into());
			let high = T::BidIncrements::get().saturating_mul((3 + i * 17 % 32).into());
			let hash = Pallet::<T>::bid_commitment(
				NextFrameId::<T>::get(),
				&who,
				&who,
				low,
				high,
				&keys,
				[1; 32],
			);
			Pallet::<T>::bid(
				RawOrigin::Signed(who.clone()).into(),
				low,
				T::Keys::decode(&mut &vec![0; T::Keys::max_encoded_len()][..]).unwrap(),
				None,
				Some(NextFrameId::<T>::get()),
				Some(hash),
			)?;
		}
		frame_system::Pallet::<T>::set_block_number(block + One::one());
		for i in 0..b {
			let who = account("challenger", i, 0);
			Pallet::<T>::bid_max(
				RawOrigin::Signed(who).into(),
				T::BidIncrements::get().saturating_mul((3 + i * 17 % 32).into()),
				[1; 32],
				None,
				NextFrameId::<T>::get(),
			)?;
		}
		if replacements {
			// The displaced incumbents have later, already revealed replacements.
			for i in m.saturating_sub(b)..m {
				let who = account("incumbent", i, 0);
				let keys = prepare_bid::<T>(&who);
				Pallet::<T>::bid(
					RawOrigin::Signed(who).into(),
					T::BidIncrements::get().saturating_mul(40u32.into()),
					keys,
					None,
					Some(NextFrameId::<T>::get()),
					None,
				)?;
			}
		}
		frame_system::Pallet::<T>::set_block_number(block + T::BidRevealBlocks::get().into());
		Ok(block)
	}

	#[benchmark]
	fn settle_bids(b: Linear<1, 32>, m: Linear<1, 144>) -> Result<(), BenchmarkError> {
		let m = m.min(T::MaxCohortSize::get());
		let block = settlement_setup::<T>(b, m, false)?;
		#[block]
		{
			Pallet::<T>::settle_bids(block)?;
		}
		assert!(BidBatches::<T>::get(block).is_empty());
		assert_eq!(BidsForNextSlotCohort::<T>::get().len(), m as usize);
		Ok(())
	}

	#[benchmark]
	fn settle_bids_with_replacements(
		b: Linear<1, 32>,
		m: Linear<1, 144>,
	) -> Result<(), BenchmarkError> {
		let m = m.min(T::MaxCohortSize::get());
		let block = settlement_setup::<T>(b, m, true)?;
		#[block]
		{
			Pallet::<T>::settle_bids(block)?;
		}
		assert!(BidBatches::<T>::get(block).is_empty());
		assert_eq!(BidsForNextSlotCohort::<T>::get().len(), m as usize);
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
		let bid_pool_account = T::MiningBidPoolAccount::get();
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
			Pallet::<T>::legacy_bid(bidder.clone(), bid_amount, keys, bidder)?;
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

	/// Hook: On finalize vote-seal author scoring update
	#[benchmark]
	fn on_finalize_record_block_author() -> Result<(), BenchmarkError> {
		let frame_id: FrameId = 2;
		let author: T::AccountId = account("author", 0, 0);
		let cohort = BoundedVec::<MinerNonceScoring<T>, T::MaxCohortSize>::truncate_from(vec![
			MinerNonceScoring {
				nonce: U256::from(1u64),
				last_win_block: None,
				blocks_won_in_frame: 0,
				frame_start_blocks_won_surplus: 0,
			},
		]);
		AccountIndexLookup::<T>::insert(&author, (frame_id, 0));
		MinerNonceScoringByCohort::<T>::mutate(|scores| {
			let _ = scores.try_insert(frame_id, cohort);
		});

		#[block]
		{
			Pallet::<T>::record_block_author(author.clone());
		}

		let updated = MinerNonceScoringByCohort::<T>::get();
		let blocks_won = updated
			.get(&frame_id)
			.and_then(|entry| entry.first())
			.map(|entry| entry.blocks_won_in_frame)
			.unwrap_or_default();
		assert_eq!(blocks_won, 1u16, "author score should increment");

		Ok(())
	}

	/// Frame transitions: start_new_frame with cross-pallet operations
	#[benchmark]
	fn start_new_frame(m: Linear<1, 144>) -> Result<(), BenchmarkError> {
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
		let bid_pool_account = T::MiningBidPoolAccount::get();
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
			Pallet::<T>::legacy_bid(bidder.clone(), bid_amount, keys, bidder)?;
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

		let argonots_amount: T::Balance = 100_000u128.into();
		ArgonotsPerMiningSeat::<T>::put(argonots_amount);
		let frames_per_term = T::FramesPerMiningTerm::get() as FrameId;
		let price_history =
			BoundedVec::<SeatPriceStats<T::Balance>, ConstU32<10>>::truncate_from(vec![
			SeatPriceStats {
				seats: T::MaxCohortSize::get(),
				total_bids: T::TargetPricePerSeat::get().saturating_mul(T::MaxCohortSize::get().into()),
			}; 10
		]);
		let next_frame_id = price_history.len() as FrameId + 1;
		AveragePricePerSeat::<T>::put(price_history);
		NextFrameId::<T>::put(next_frame_id);
		NextCohortSize::<T>::put(T::MinCohortSize::get());

		// Fill the upcoming cohort for the collateral median calculation.
		use sp_runtime::codec::Decode;
		let key_data = vec![0u8; T::Keys::max_encoded_len()];
		let keys = T::Keys::decode(&mut &key_data[..])
			.expect("Failed to create test keys for benchmarking");
		let mut bids = BoundedVec::new();
		for i in 0..T::MaxCohortSize::get() {
			bids.try_push(Registration::<T> {
				account_id: account("median", i, 0),
				external_funding_account: Some(account("funding", i, 0)),
				bid: T::TargetPricePerSeat::get(),
				argonots: argonots_amount,
				authority_keys: keys.clone(),
				starting_frame_id: 1,
				bid_at_tick: 1,
			})
			.expect("Maximum cohort should fit");
		}
		BidsForNextSlotCohort::<T>::put(bids);
		let mut schedule: BoundedBTreeMap<FrameId, u32, ConstU32<11>> = BoundedBTreeMap::new();
		let scheduled_frame_id = next_frame_id + frames_per_term;
		for frame_id in next_frame_id..scheduled_frame_id {
			schedule.try_insert(frame_id, T::MinCohortSize::get()).unwrap();
		}
		ScheduledCohortSizeChangeByFrame::<T>::put(schedule);

		#[block]
		{
			Pallet::<T>::update_argonots_per_seat(next_frame_id);
			Pallet::<T>::adjust_number_of_seats();
		}

		assert_eq!(
			ScheduledCohortSizeChangeByFrame::<T>::get().get(&scheduled_frame_id),
			Some(&T::MaxCohortSize::get()),
		);

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

	#[benchmark]
	fn provider_has_active_rewards_account_seat() -> Result<(), BenchmarkError> {
		use frame_support::traits::fungible::{InspectHold, Mutate, MutateHold};
		use sp_runtime::codec::Decode;

		let rewards_account: T::AccountId = account("provider_rewards", 0, 0);
		let hold_reason: T::RuntimeHoldReason = HoldReason::RegisterAsMiner.into();
		let argonots_per_seat: T::Balance = 100_000u128.into();
		let frames_per_term = T::FramesPerMiningTerm::get() as FrameId;
		let max_cohort_size = T::MaxCohortSize::get().max(1);
		let _ = T::OwnershipCurrency::mint_into(&rewards_account, 1_000_000u128.into());
		if T::OwnershipCurrency::balance_on_hold(&hold_reason, &rewards_account) == 0u32.into() {
			frame_system::Pallet::<T>::inc_providers(&rewards_account);
		}
		let _ = T::OwnershipCurrency::hold(&hold_reason, &rewards_account, argonots_per_seat);

		let key_size = T::Keys::max_encoded_len();
		let key_data = vec![0u8; key_size];
		let keys =
			T::Keys::decode(&mut &key_data[..]).expect("failed to create benchmark mining keys");
		// Benchmark the held-account worst case: all cohorts exist, all are full, and the
		// rewards account is not present so the scan walks every registration.
		for frame_id in 1..=frames_per_term {
			let mut cohort = Vec::with_capacity(max_cohort_size as usize);
			for miner_index in 0..max_cohort_size {
				let account_index =
					(frame_id as u32).saturating_mul(max_cohort_size).saturating_add(miner_index);
				cohort.push(Registration::<T> {
					account_id: account("provider_miner", account_index, 0),
					external_funding_account: Some(account("provider_funding", account_index, 0)),
					bid: 10_000u128.into(),
					argonots: argonots_per_seat,
					authority_keys: keys.clone(),
					starting_frame_id: frame_id,
					bid_at_tick: 0,
				});
			}
			MinersByCohort::<T>::insert(
				frame_id,
				BoundedVec::<Registration<T>, T::MaxCohortSize>::truncate_from(cohort),
			);
		}

		#[block]
		{
			assert!(
				!<Pallet<T> as MiningSlotProvider<T::AccountId>>::has_active_rewards_account_seat(
					&rewards_account,
				)
			);
		}

		Ok(())
	}

	#[benchmark]
	fn provider_get_block_rewards_account() -> Result<(), BenchmarkError> {
		use sp_runtime::codec::Decode;

		let author: T::AccountId = account("provider_author", 0, 0);
		let argonots_per_seat: T::Balance = 100_000u128.into();
		let max_cohort_size = T::MaxCohortSize::get().max(1);
		let key_size = T::Keys::max_encoded_len();
		let key_data = vec![0u8; key_size];
		let keys =
			T::Keys::decode(&mut &key_data[..]).expect("failed to create benchmark mining keys");

		let mut released_miners =
			BoundedBTreeMap::<T::AccountId, Registration<T>, T::MaxCohortSize>::new();
		for i in 0..max_cohort_size {
			let released_account: T::AccountId = account("released_provider_miner", i, 0);
			released_miners
				.try_insert(
					released_account.clone(),
					Registration::<T> {
						account_id: released_account,
						external_funding_account: None,
						bid: 10_000u128.into(),
						argonots: argonots_per_seat,
						authority_keys: keys.clone(),
						starting_frame_id: u64::from(i).saturating_add(1),
						bid_at_tick: 0,
					},
				)
				.map_err(|_| BenchmarkError::Stop("released miners overflow"))?;
		}
		ReleasedMinersByAccountId::<T>::put(released_miners);

		MinersByCohort::<T>::insert(
			1,
			BoundedVec::<Registration<T>, T::MaxCohortSize>::truncate_from(vec![
				Registration::<T> {
					account_id: author.clone(),
					external_funding_account: None,
					bid: 10_000u128.into(),
					argonots: argonots_per_seat,
					authority_keys: keys,
					starting_frame_id: 1u64,
					bid_at_tick: 0,
				},
			]),
		);
		AccountIndexLookup::<T>::insert(&author, (1u64, 0u32));

		#[block]
		{
			assert_eq!(
				<Pallet<T> as BlockRewardAccountsProvider<T::AccountId>>::get_block_rewards_account(
					&author
				),
				Some((author.clone(), 1u64)),
			);
		}

		Ok(())
	}

	#[benchmark]
	fn provider_is_compute_block_eligible_for_rewards() {
		NextFrameId::<T>::put(2);
		ActiveMinersCount::<T>::put(1u16);

		#[block]
		{
			assert!(
				!<Pallet<T> as BlockRewardAccountsProvider<T::AccountId>>::is_compute_block_eligible_for_rewards()
			);
		}
	}
	impl_benchmark_test_suite!(Pallet, crate::mock::new_test_ext(), crate::mock::Test);
}
