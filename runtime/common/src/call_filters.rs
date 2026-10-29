#[macro_export]
macro_rules! call_filters {
	() => {
		pub struct BaseCallFilter;
		impl Contains<RuntimeCall> for BaseCallFilter {
			fn contains(call: &RuntimeCall) -> bool {
				// placeholder for filter
				match call {
					RuntimeCall::System(..) => true,
					_ => true,
				}
			}
		}

		/// Calls that cannot be paused by the tx-pause pallet.
		pub struct TxPauseWhitelistedCalls;

		impl Contains<RuntimeCallNameOf<Runtime>> for TxPauseWhitelistedCalls {
			fn contains(full_name: &RuntimeCallNameOf<Runtime>) -> bool {
				#[allow(clippy::match_like_matches_macro)]
				match (full_name.0.as_slice(), full_name.1.as_slice()) {
					(b"System", _) => true,
					_ => false,
				}
			}
		}

		pub struct VaultAdminCallFilter;

		impl VaultAdminCallFilter {
			fn is_crosschain_vault_admin_call(call: &RuntimeCall) -> bool {
				matches!(
					call,
					RuntimeCall::CrosschainTransfer(
						pallet_crosschain_transfer::Call::register_council_signer { .. } |
							pallet_crosschain_transfer::Call::register_minting_authority { .. } |
							pallet_crosschain_transfer::Call::approve_queue_entries { .. } |
							pallet_crosschain_transfer::Call::collateralize_transfer { .. } |
							pallet_crosschain_transfer::Call::deactivate_minting_authority { .. }
					)
				)
			}

			fn is_single_vault_admin_call(call: &RuntimeCall) -> bool {
				matches!(
					call,
					RuntimeCall::Vaults(..) |
						RuntimeCall::Treasury(pallet_treasury::Call::buy_bonds { .. }) |
						RuntimeCall::Treasury(pallet_treasury::Call::liquidate_bond_lot { .. }) |
						RuntimeCall::BitcoinLocks(
							pallet_bitcoin_locks::Call::create_receive_address { .. }
						) |
						RuntimeCall::BitcoinLocks(
							pallet_bitcoin_locks::Call::cosign_release { .. }
						) | RuntimeCall::BitcoinLocks(
						pallet_bitcoin_locks::Call::cosign_orphaned_utxo_release { .. }
					)
				) || Self::is_crosschain_vault_admin_call(call)
			}
		}

		/// The type used to represent the kinds of proxying allowed.
		#[derive(
			Copy,
			Clone,
			Eq,
			PartialEq,
			Ord,
			PartialOrd,
			TypeInfo,
			Encode,
			Decode,
			DecodeWithMemTracking,
			Debug,
			MaxEncodedLen,
		)]
		pub enum ProxyType {
			Any,
			NonTransfer,
			PriceIndex,
			MiningBid,
			MiningBidRealPaysFee,
			Bitcoin,
			VaultAdmin,
			VaultDelegate,
		}
		impl Default for ProxyType {
			fn default() -> Self {
				Self::Any
			}
		}
		impl InstanceFilter<RuntimeCall> for ProxyType {
			fn filter(&self, c: &RuntimeCall) -> bool {
				match self {
					ProxyType::Any => true,
					ProxyType::NonTransfer => !matches!(
						c,
						RuntimeCall::Balances(..) |
							RuntimeCall::Ownership(..) |
							RuntimeCall::LocalchainTransfer(..)
					),
					ProxyType::MiningBidRealPaysFee | ProxyType::MiningBid => match c {
						RuntimeCall::MiningSlot(pallet_mining_slot::Call::bid { .. } |  pallet_mining_slot::Call::bid_max { .. }) => true,
						RuntimeCall::Utility(pallet_utility::Call::batch { calls }) |
						RuntimeCall::Utility(pallet_utility::Call::batch_all { calls }) |
						RuntimeCall::Utility(pallet_utility::Call::force_batch { calls }) =>
							calls.iter().all(|sc| {
								matches!(
									sc,
									RuntimeCall::MiningSlot(pallet_mining_slot::Call::bid { .. } |  pallet_mining_slot::Call::bid_max { .. })
								)
							}),
						_ => false,
					},
					ProxyType::PriceIndex => matches!(c, RuntimeCall::PriceIndex(..)),
					ProxyType::Bitcoin => match c {
						RuntimeCall::BitcoinLocks(..) => true,
						RuntimeCall::Utility(pallet_utility::Call::batch { calls }) |
						RuntimeCall::Utility(pallet_utility::Call::batch_all { calls }) |
						RuntimeCall::Utility(pallet_utility::Call::force_batch { calls }) =>
							calls.iter().all(|sc| matches!(sc, RuntimeCall::BitcoinLocks(..))),
						_ => false,
					},
					ProxyType::VaultAdmin => match c {
						RuntimeCall::Utility(pallet_utility::Call::batch { calls }) |
						RuntimeCall::Utility(pallet_utility::Call::batch_all { calls }) |
						RuntimeCall::Utility(pallet_utility::Call::force_batch { calls }) =>
							calls.iter().all(VaultAdminCallFilter::is_single_vault_admin_call),
						_ => VaultAdminCallFilter::is_single_vault_admin_call(c),
					},
					ProxyType::VaultDelegate => match c {
						RuntimeCall::CrosschainTransfer(
							pallet_crosschain_transfer::Call::prove_gateway_activity { .. } |
							pallet_crosschain_transfer::Call::prove_gateway_activity_storage { .. }
						) => true,
						RuntimeCall::Utility(pallet_utility::Call::batch { calls }) |
						RuntimeCall::Utility(pallet_utility::Call::batch_all { calls }) |
						RuntimeCall::Utility(pallet_utility::Call::force_batch { calls }) =>
							calls.iter().all(|sc| {
								matches!(
									sc,
									RuntimeCall::CrosschainTransfer(
											pallet_crosschain_transfer::Call::prove_gateway_activity { .. } |
											pallet_crosschain_transfer::Call::prove_gateway_activity_storage { .. }
									)
								)
							}),
						_ => false,
					},
				}
			}
			fn is_superset(&self, o: &Self) -> bool {
				match (self, o) {
					(x, y) if x == y => true,
					(ProxyType::Any, _) => true,
					(_, ProxyType::Any) => false,
					(ProxyType::NonTransfer, _) => true,
					_ => false,
				}
			}
		}

		#[cfg(test)]
		mod gateway_proof_proxy_tests {
			use super::*;

			#[test]
			fn vault_delegate_permits_both_proof_calls_but_not_gateway_admin() {
				use pallet_crosschain_transfer::{Call, SourceChain};
				let receipt = RuntimeCall::CrosschainTransfer(Call::prove_gateway_activity {
					source_chain: SourceChain::Ethereum,
					previous_gateway_activity_nonce: 0,
					proof_batch: argon_primitives::EthereumReceiptLogProofBatch {
						execution_block_proof: argon_primitives::EthereumExecutionBlockProof {
							anchor_block_hash: Default::default(),
							target_to_anchor_header_chain: Default::default(),
						},
						blocks: Default::default(),
					},
				});
				let storage = RuntimeCall::CrosschainTransfer(Call::prove_gateway_activity_storage {
					source_chain: SourceChain::Ethereum,
					previous_gateway_activity_nonce: 0,
					proof: pallet_crosschain_transfer::GatewayActivityStorageProof {
						locator_index: 1,
						activity_root_seed: Default::default(),
						storage_proof: argon_primitives::EthereumAccountStorageProof {
							execution_block_proof: argon_primitives::EthereumExecutionBlockProof {
								anchor_block_hash: Default::default(),
								target_to_anchor_header_chain: Default::default(),
							},
							account_proof: Default::default(),
							storage_proof: Default::default(),
							slots: Default::default(),
						},
						activity_logs: Default::default(),
					},
				});
				let admin = RuntimeCall::CrosschainTransfer(Call::pause_gateway { source_chain: SourceChain::Ethereum });
				assert!(ProxyType::VaultDelegate.filter(&receipt));
				assert!(ProxyType::VaultDelegate.filter(&storage));
				assert!(!ProxyType::VaultDelegate.filter(&admin));

				for batch in [
					pallet_utility::Call::batch { calls: vec![receipt.clone(), storage.clone()] },
					pallet_utility::Call::batch_all { calls: vec![receipt.clone(), storage.clone()] },
					pallet_utility::Call::force_batch { calls: vec![receipt, storage.clone()] },
				] {
					assert!(ProxyType::VaultDelegate.filter(&RuntimeCall::Utility(batch)));
				}
				assert!(!ProxyType::VaultDelegate.filter(&RuntimeCall::Utility(pallet_utility::Call::batch {
					calls: vec![storage, admin],
				})));
			}
		}
	};
}
