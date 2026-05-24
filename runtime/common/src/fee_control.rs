#[macro_export]
macro_rules! vault_admin_fee_refund_policy {
	() => {
		pub struct VaultAdminFeeRefundPolicy;

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
