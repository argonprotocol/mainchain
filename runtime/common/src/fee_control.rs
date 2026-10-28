#[macro_export]
macro_rules! vault_admin_fee_refund_policy {
	() => {
		use pallet_fee_control::WeightInfo as _;

		pub struct VaultAdminFeeRefundPolicy;
		pub struct VaultAdminFeeRefundPolicyWeights;

		impl CallFeeRefundProviderWeightInfo<RuntimeCall> for VaultAdminFeeRefundPolicyWeights {
			fn refund_fee_on_success(call: &RuntimeCall) -> Weight {
				let RuntimeCall::Utility(pallet_utility::Call::batch_all { calls }) = call else {
					return Weight::zero();
				};

				<Runtime as pallet_fee_control::Config>::WeightInfo::inspect_refund_calls(
					calls.len() as u32,
				)
			}
		}

		impl VaultAdminFeeRefundPolicy {
			fn is_collect_call(call: &RuntimeCall) -> bool {
				matches!(call, RuntimeCall::Vaults(pallet_vaults::Call::collect { .. }))
			}

			fn is_collect_prerequisite_call(call: &RuntimeCall) -> bool {
				matches!(
					call,
					RuntimeCall::CrosschainTransfer(
						pallet_crosschain_transfer::Call::approve_queue_entries { .. }
					) | RuntimeCall::BitcoinLocks(
						pallet_bitcoin_locks::Call::cosign_release { .. }
					) | RuntimeCall::BitcoinLocks(
						pallet_bitcoin_locks::Call::cosign_orphaned_utxo_release { .. }
					)
				)
			}
		}

		impl CallFeeRefundProvider<RuntimeCall> for VaultAdminFeeRefundPolicy {
			type Weights = VaultAdminFeeRefundPolicyWeights;

			#[cfg(feature = "runtime-benchmarks")]
			fn benchmark_call(count: u32) -> Option<RuntimeCall> {
				let calls = (0..count)
					.map(|_| RuntimeCall::Vaults(pallet_vaults::Call::collect { vault_id: 1 }))
					.collect();
				Some(RuntimeCall::Utility(pallet_utility::Call::batch_all { calls }))
			}

			fn refund_fee_on_success(call: &RuntimeCall) -> bool {
				let RuntimeCall::Utility(pallet_utility::Call::batch_all { calls }) = call else {
					return false;
				};

				let mut has_collect = false;
				let mut has_prerequisite = false;

				for call in calls.iter() {
					if Self::is_collect_call(call) {
						has_collect = true;
						continue;
					}
					if Self::is_collect_prerequisite_call(call) {
						has_prerequisite = true;
						continue;
					}
					return false;
				}

				has_collect && has_prerequisite
			}
		}
	};
}

#[cfg(test)]
mod tests {
	use crate::prelude::*;
	use argon_runtime::{
		Balances, Runtime, RuntimeCall, RuntimeGenesisConfig, RuntimeOrigin, System,
	};
	use codec::Encode;
	use pallet_fee_control::{CheckFeeWrapper, WeightInfo};
	use pallet_transaction_payment::ChargeTransactionPayment;
	use polkadot_sdk::{
		frame_support::{assert_noop, assert_ok},
		sp_io::TestExternalities,
		sp_runtime::{
			traits::{DispatchTransaction, ExtensionPostDispatchWeightHandler},
			BuildStorage,
		},
	};

	#[test]
	fn proxied_commit_and_reveal_charge_real_account_without_holding_the_ceiling() {
		use argon_runtime::{Ownership, ProxyType, RuntimeEvent, SessionKeys};
		use codec::Decode;
		TestExternalities::new(RuntimeGenesisConfig::default().build_storage().unwrap())
			.execute_with(|| {
				System::set_block_number(1);
				let real = sp_runtime::AccountId32::new([1; 32]);
				let delegate = sp_runtime::AccountId32::new([2; 32]);
				let author = sp_runtime::AccountId32::new([3; 32]);
				Balances::mint_into(&real, 100 * ARGON).unwrap();
				Balances::mint_into(&delegate, ARGON).unwrap();
				Balances::mint_into(&author, ARGON).unwrap();
				Ownership::mint_into(&real, ARGON).unwrap();
				pallet_operational_accounts::IsOperationalAccountInviteOnly::<Runtime>::put(false);
				pallet_mining_slot::IsNextSlotBiddingOpen::<Runtime>::put(true);
				pallet_mining_slot::NextFrameId::<Runtime>::put(2);
				pallet_mining_slot::ArgonotsPerMiningSeat::<Runtime>::put(10_000u128);
				pallet_block_seal::LastBlockSealerInfo::<Runtime>::put(
					argon_primitives::BlockSealerInfo {
						block_author_account_id: author,
						block_vote_rewards_account: None,
						block_seal_authority: None,
					},
				);
				let proxies: frame_support::BoundedVec<
					_,
					<Runtime as pallet_proxy::Config>::MaxProxies,
				> = vec![pallet_proxy::ProxyDefinition {
					delegate: delegate.clone(),
					proxy_type: ProxyType::MiningBidRealPaysFee,
					delay: 0u32,
				}]
				.try_into()
				.unwrap();
				pallet_proxy::Proxies::<Runtime>::insert(&real, (proxies, 0u128));
				let keys =
					SessionKeys::decode(&mut sp_runtime::traits::TrailingZeroInput::zeroes())
						.unwrap();
				let low = ARGON;
				let high = 2 * ARGON;
				let hash = pallet_mining_slot::Pallet::<Runtime>::bid_commitment(
					2, &real, &real, low, high, &keys, [9; 32],
				);
				let commit = RuntimeCall::MiningSlot(pallet_mining_slot::Call::bid {
					target_frame_id: Some(2),
					mining_account_id: None,
					bid: low,
					keys: keys.clone(),
					max_hash: Some(hash),
				});
				let reveal = RuntimeCall::MiningSlot(pallet_mining_slot::Call::bid_max {
					target_frame_id: 2,
					mining_account_id: None,
					max: high,
					salt: [9; 32],
				});
				let before_real = <Balances as frame_support::traits::Currency<sp_runtime::AccountId32>>::total_balance(&real);
				let before_delegate = <Balances as frame_support::traits::Currency<sp_runtime::AccountId32>>::total_balance(&delegate);
				for (block, inner) in [(1, commit), (2, reveal)] {
					System::set_block_number(block);
					let call = RuntimeCall::Proxy(pallet_proxy::Call::proxy {
						real: sp_runtime::MultiAddress::Id(real.clone()),
						force_proxy_type: Some(ProxyType::MiningBidRealPaysFee),
						call: Box::new(inner),
					});
					let extension =
						CheckFeeWrapper::<Runtime, ChargeTransactionPayment<Runtime>>::from(
							ChargeTransactionPayment::from(0),
						);
					let mut info = call.get_dispatch_info();
					info.extension_weight = extension.weight(&call);
					let len = call.encoded_size();
					extension
						.test_run(
							RuntimeOrigin::signed(delegate.clone()),
							&call,
							&info,
							len,
							0,
							|origin| call.clone().dispatch(origin),
						)
						.unwrap()
						.unwrap();
					assert!(System::events().iter().any(|record| matches!(
						&record.event,
						RuntimeEvent::Proxy(pallet_proxy::Event::ProxyExecuted { result: Ok(()) })
					)));
					assert!(!System::events().iter().any(|record| matches!(
						&record.event,
						RuntimeEvent::Proxy(pallet_proxy::Event::ProxyExecuted { result: Err(_) })
					)));
					assert_eq!(
						pallet_mining_slot::HistoricalBidsPerSlot::<Runtime>::get()[0].bids_count,
						1
					);
				}
				let pending = pallet_mining_slot::PendingBids::<Runtime>::get(&real).unwrap();
				assert_eq!(pending.funding, real);
				assert_eq!(pending.max, Some(high));
				assert_eq!(
					Balances::total_balance_on_hold(&real),
					0
				);
				assert_eq!(<Balances as frame_support::traits::Currency<sp_runtime::AccountId32>>::total_balance(&delegate), before_delegate);
				let fees: Vec<_> = System::events()
					.iter()
					.filter_map(|record| match &record.event {
						RuntimeEvent::TransactionPayment(
							pallet_transaction_payment::Event::TransactionFeePaid {
								who,
								actual_fee,
								..
							},
						) => {
							assert_eq!(who, &real);
							Some(*actual_fee)
						},
						_ => None,
					})
					.collect();
				assert_eq!(fees.len(), 2);
				assert!(fees.iter().all(|fee| *fee > 0));
				assert_eq!(before_real - <Balances as frame_support::traits::Currency<sp_runtime::AccountId32>>::total_balance(&real), fees.iter().sum::<u128>());
			});
	}

	#[test]
	fn mining_bid_activity_rolls_back_with_an_outer_batch() {
		use argon_runtime::{Ownership, SessionKeys};
		use codec::Decode;
		TestExternalities::new(RuntimeGenesisConfig::default().build_storage().unwrap())
			.execute_with(|| {
				System::set_block_number(1);
				let real = sp_runtime::AccountId32::new([1; 32]);
				let first = sp_runtime::AccountId32::new([3; 32]);
				let second = sp_runtime::AccountId32::new([4; 32]);
				let tail = sp_runtime::AccountId32::new([5; 32]);
				Balances::mint_into(&real, 100 * ARGON).unwrap();
				Ownership::mint_into(&real, ARGON).unwrap();
				pallet_operational_accounts::IsOperationalAccountInviteOnly::<Runtime>::put(false);
				pallet_mining_slot::IsNextSlotBiddingOpen::<Runtime>::put(true);
				pallet_mining_slot::NextFrameId::<Runtime>::put(2);
				pallet_mining_slot::ArgonotsPerMiningSeat::<Runtime>::put(10_000u128);
				let keys =
					SessionKeys::decode(&mut sp_runtime::traits::TrailingZeroInput::zeroes())
						.unwrap();
				let bid = |miner, amount| {
					RuntimeCall::MiningSlot(pallet_mining_slot::Call::bid {
						bid: amount,
						keys: keys.clone(),
						mining_account_id: Some(miner),
						target_frame_id: Some(2),
						max_hash: None,
					})
				};
				assert_ok!(bid(first.clone(), ARGON).dispatch(RuntimeOrigin::signed(real.clone())));
				let history = pallet_mining_slot::HistoricalBidsPerSlot::<Runtime>::get();
				assert_eq!(history[0].bids_count, 1);
				let retry = bid(second.clone(), ARGON);
				let batch = RuntimeCall::Utility(pallet_utility::Call::batch_all {
					calls: vec![retry.clone(), bid(second.clone(), 1), bid(tail.clone(), ARGON)],
				});
				assert_noop!(
					batch
						.dispatch(RuntimeOrigin::signed(real.clone()))
						.map_err(|error| error.error),
					pallet_mining_slot::Error::<Runtime>::InvalidBidAmount
				);
				assert_eq!(pallet_mining_slot::HistoricalBidsPerSlot::<Runtime>::get(), history);
				assert!(pallet_mining_slot::PendingBids::<Runtime>::contains_key(first));
				assert!(!pallet_mining_slot::PendingBids::<Runtime>::contains_key(&second));
				assert!(!pallet_mining_slot::PendingBids::<Runtime>::contains_key(tail));
				assert_ok!(retry.dispatch(RuntimeOrigin::signed(real)));
				assert_eq!(
					pallet_mining_slot::HistoricalBidsPerSlot::<Runtime>::get()[0].bids_count,
					2
				);
				assert!(pallet_mining_slot::PendingBids::<Runtime>::contains_key(second));
			});
	}

	#[test]
	fn mining_proxies_and_fee_sponsors_support_both_bid_steps() {
		use argon_primitives::TransactionSponsorProvider;
		use argon_runtime::{MiningBidProxyFeeSponsor, ProxyType, SessionKeys};
		use codec::Decode;
		use polkadot_sdk::frame_support::traits::InstanceFilter;
		TestExternalities::new(RuntimeGenesisConfig::default().build_storage().unwrap())
			.execute_with(|| {
				let real = sp_runtime::AccountId32::new([1; 32]);
				let delegate = sp_runtime::AccountId32::new([2; 32]);
				let keys =
					SessionKeys::decode(&mut sp_runtime::traits::TrailingZeroInput::zeroes())
						.unwrap();
				let calls = vec![
					RuntimeCall::MiningSlot(pallet_mining_slot::Call::bid {
						bid: 10_000,
						keys: keys.clone(),
						mining_account_id: None,
						target_frame_id: Some(2),
						max_hash: None,
					}),
					RuntimeCall::MiningSlot(pallet_mining_slot::Call::bid {
						target_frame_id: Some(2),
						mining_account_id: None,
						bid: 10_000,
						keys: keys.clone(),
						max_hash: Some([1; 32]),
					}),
					RuntimeCall::MiningSlot(pallet_mining_slot::Call::bid_max {
						target_frame_id: 2,
						mining_account_id: None,
						max: 20_000,
						salt: [1; 32],
					}),
				];
				let unrelated = RuntimeCall::System(frame_system::Call::remark { remark: vec![] });
				for proxy_type in [ProxyType::MiningBid, ProxyType::MiningBidRealPaysFee] {
					for call in &calls {
						assert!(proxy_type.filter(call));
					}
					assert!(!proxy_type.filter(&unrelated));
				}
				let proxies: frame_support::BoundedVec<
					_,
					<Runtime as pallet_proxy::Config>::MaxProxies,
				> = vec![pallet_proxy::ProxyDefinition {
					delegate: delegate.clone(),
					proxy_type: ProxyType::MiningBidRealPaysFee,
					delay: 0u32,
				}]
				.try_into()
				.unwrap();
				pallet_proxy::Proxies::<Runtime>::insert(&real, (proxies, 0u128));
				let mut sponsored = calls.clone();
				sponsored.push(RuntimeCall::Utility(pallet_utility::Call::batch_all { calls }));
				for call in sponsored {
					let outer = RuntimeCall::Proxy(pallet_proxy::Call::proxy {
						real: sp_runtime::MultiAddress::Id(real.clone()),
						force_proxy_type: Some(ProxyType::MiningBidRealPaysFee),
						call: Box::new(call),
					});
					let sponsor = MiningBidProxyFeeSponsor::<Runtime>::get_transaction_sponsor(
						&delegate, &outer,
					)
					.expect("mining call sponsored");
					assert_eq!(sponsor.payer, real);
				}
				let outer = RuntimeCall::Proxy(pallet_proxy::Call::proxy {
					real: sp_runtime::MultiAddress::Id(real),
					force_proxy_type: Some(ProxyType::MiningBidRealPaysFee),
					call: Box::new(unrelated),
				});
				assert!(MiningBidProxyFeeSponsor::<Runtime>::get_transaction_sponsor(
					&delegate, &outer
				)
				.is_none());
			});
	}

	#[test]
	fn unused_fee_control_event_weight_is_refunded_to_the_payer() {
		TestExternalities::new(RuntimeGenesisConfig::default().build_storage().unwrap())
			.execute_with(|| {
				System::set_block_number(1);
				let payer = sp_runtime::AccountId32::new([1; 32]);
				Balances::mint_into(&payer, 100 * ARGON).unwrap();
				let author = sp_runtime::AccountId32::new([2; 32]);
				Balances::mint_into(&author, ARGON).unwrap();
				pallet_block_seal::LastBlockSealerInfo::<Runtime>::put(
					argon_primitives::BlockSealerInfo {
						block_author_account_id: author,
						block_vote_rewards_account: None,
						block_seal_authority: None,
					},
				);
				let call = RuntimeCall::System(frame_system::Call::remark { remark: vec![] });
				let extension = CheckFeeWrapper::<Runtime, ChargeTransactionPayment<Runtime>>::from(
					ChargeTransactionPayment::from(0),
				);
				let mut info = call.get_dispatch_info();
				info.extension_weight = extension.weight(&call);
				let event_weight =
					<Runtime as pallet_fee_control::Config>::WeightInfo::post_event();
				assert!(event_weight.ref_time() > 0);
				let len = call.encoded_size();
				let balance_before = Balances::balance(&payer);
				let (pre, _) = extension
					.validate_and_prepare(
						RuntimeOrigin::signed(payer.clone()),
						&call,
						&info,
						len,
						0,
					)
					.unwrap();
				let expected_post = PostDispatchInfo {
					actual_weight: Some(info.total_weight().saturating_sub(event_weight)),
					pays_fee: Pays::Yes,
				};
				let expected_fee =
					pallet_transaction_payment::Pallet::<Runtime>::compute_actual_fee(
						len as u32,
						&info,
						&expected_post,
						0,
					);
				let mut post_info = PostDispatchInfo::default();
				post_info.set_extension_weight(&info);

				CheckFeeWrapper::<Runtime, ChargeTransactionPayment<Runtime>>::post_dispatch(
					pre,
					&info,
					&mut post_info,
					len,
					&Ok(()),
				)
				.unwrap();

				assert_eq!(Balances::balance(&payer), balance_before - expected_fee);
				assert_eq!(post_info.actual_weight, expected_post.actual_weight);
			});
	}
}
