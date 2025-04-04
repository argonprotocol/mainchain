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
			RuntimeDebug,
			MaxEncodedLen,
		)]
		pub enum ProxyType {
			Any,
			NonTransfer,
			PriceIndex,
			MiningBid,
			BitcoinCosign,
			VaultAdmin,
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
							RuntimeCall::ChainTransfer(..) |
							RuntimeCall::TokenGateway(..)
					),
					ProxyType::MiningBid => match c {
						RuntimeCall::MiningSlot(pallet_mining_slot::Call::bid { .. }) => true,
						RuntimeCall::Utility(pallet_utility::Call::batch { calls }) |
						RuntimeCall::Utility(pallet_utility::Call::batch_all { calls }) =>
							calls.iter().all(|sc| {
								matches!(
									sc,
									RuntimeCall::MiningSlot(pallet_mining_slot::Call::bid { .. })
								)
							}),
						_ => false,
					},
					ProxyType::PriceIndex => matches!(c, RuntimeCall::PriceIndex(..)),
					ProxyType::BitcoinCosign => matches!(
						c,
						RuntimeCall::BitcoinLocks(
							pallet_bitcoin_locks::Call::cosign_release { .. }
						)
					),
					ProxyType::VaultAdmin => matches!(
						c,
						RuntimeCall::Vaults(..) |
							RuntimeCall::BitcoinLocks(
								pallet_bitcoin_locks::Call::cosign_release { .. }
							)
					),
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
