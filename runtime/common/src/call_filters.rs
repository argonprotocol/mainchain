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
						RuntimeCall::BitcoinLocks(pallet_bitcoin_locks::Call::initialize { .. }) |
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
						RuntimeCall::MiningSlot(pallet_mining_slot::Call::bid { .. }) => true,
						RuntimeCall::Utility(pallet_utility::Call::batch { calls }) |
						RuntimeCall::Utility(pallet_utility::Call::batch_all { calls }) |
						RuntimeCall::Utility(pallet_utility::Call::force_batch { calls }) =>
							calls.iter().all(|sc| {
								matches!(
									sc,
									RuntimeCall::MiningSlot(pallet_mining_slot::Call::bid { .. })
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
						RuntimeCall::BitcoinLocks(pallet_bitcoin_locks::Call::initialize_for {
							..
						}) |
						RuntimeCall::CrosschainTransfer(
							pallet_crosschain_transfer::Call::prove_gateway_activity { .. }
						) => true,
						RuntimeCall::Utility(pallet_utility::Call::batch { calls }) |
						RuntimeCall::Utility(pallet_utility::Call::batch_all { calls }) |
						RuntimeCall::Utility(pallet_utility::Call::force_batch { calls }) =>
							calls.iter().all(|sc| {
								matches!(
									sc,
									RuntimeCall::BitcoinLocks(
										pallet_bitcoin_locks::Call::initialize_for { .. }
									) | RuntimeCall::CrosschainTransfer(
										pallet_crosschain_transfer::Call::prove_gateway_activity { .. }
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
	};
}
