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
		sp_io::TestExternalities,
		sp_runtime::{
			traits::{DispatchTransaction, ExtensionPostDispatchWeightHandler},
			BuildStorage,
		},
	};

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
