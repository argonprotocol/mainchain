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
			DecodeWithMemTracking,
			RuntimeDebug,
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
			BitcoinInitializeFor,
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
						RuntimeCall::Vaults(..) |
						RuntimeCall::Treasury(pallet_treasury::Call::set_allocation { .. }) |
						RuntimeCall::BitcoinLocks(pallet_bitcoin_locks::Call::initialize {
							..
						}) => true,
						RuntimeCall::Utility(pallet_utility::Call::batch { calls }) |
						RuntimeCall::Utility(pallet_utility::Call::batch_all { calls }) |
						RuntimeCall::Utility(pallet_utility::Call::force_batch { calls }) =>
							calls.iter().all(|sc| {
								matches!(
									sc,
									RuntimeCall::Vaults(..) |
										RuntimeCall::Treasury(
											pallet_treasury::Call::set_allocation { .. }
										) | RuntimeCall::BitcoinLocks(
										pallet_bitcoin_locks::Call::initialize { .. }
									)
								)
							}),
						_ => false,
					},
					ProxyType::BitcoinInitializeFor => match c {
						RuntimeCall::BitcoinLocks(pallet_bitcoin_locks::Call::initialize_for {
							..
						}) => true,
						RuntimeCall::Utility(pallet_utility::Call::batch { calls }) |
						RuntimeCall::Utility(pallet_utility::Call::batch_all { calls }) |
						RuntimeCall::Utility(pallet_utility::Call::force_batch { calls }) =>
							calls.iter().all(|sc| {
								matches!(
									sc,
									RuntimeCall::BitcoinLocks(
										pallet_bitcoin_locks::Call::initialize_for { .. }
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
