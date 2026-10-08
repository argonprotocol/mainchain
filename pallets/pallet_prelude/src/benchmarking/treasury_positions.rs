//! In-memory contributions for consumers benchmarking without Positions storage access.
use super::*;
use argon_primitives::treasury::{
	BitcoinLockPosition, PositionQuantities, PositionQuantity, TreasuryPositionProvider,
	UpstreamPosition,
};

#[derive(Default)]
struct State {
	quantities: BTreeMap<Vec<u8>, PositionQuantities<u128>>,
	principal: BTreeMap<Vec<u8>, u128>,
	upstream: BTreeMap<Vec<u8>, Vec<u8>>,
	totals: PositionQuantities<u128>,
}

pub fn reset_benchmark_treasury_positions() {
	backend::with(|state| *state = State::default());
}

pub struct BenchmarkTreasuryPositionProvider<AccountId, Balance>(PhantomData<(AccountId, Balance)>);
impl<AccountId, Balance> TreasuryPositionProvider<AccountId, Balance>
	for BenchmarkTreasuryPositionProvider<AccountId, Balance>
where
	AccountId: Codec,
	Balance: Codec + MaxEncodedLen + HasCompact + Copy + Default + From<u128> + Into<u128>,
{
	type Weights = ();

	fn operational_account_registered(_: &AccountId) -> DispatchResult {
		Ok(())
	}

	fn bond_principal(account: &AccountId) -> Balance {
		backend::with(|state| {
			state.principal.get(&account.encode()).copied().unwrap_or_default().into()
		})
	}

	fn account_quantities(account: &AccountId) -> PositionQuantities<Balance> {
		let quantities = backend::with(|state| {
			state.quantities.get(&account.encode()).copied().unwrap_or_default()
		});
		PositionQuantities {
			bonds: quantities.bonds,
			stakes: quantities.stakes,
			fission_liquidity: quantities.fission_liquidity.into(),
		}
	}

	fn network_totals() -> PositionQuantities<Balance> {
		let totals = backend::with(|state| state.totals);
		PositionQuantities {
			bonds: totals.bonds,
			stakes: totals.stakes,
			fission_liquidity: totals.fission_liquidity.into(),
		}
	}

	fn account_quantity_updated<Amount: Into<u128>>(
		account: &AccountId,
		quantity: PositionQuantity,
		previous: Amount,
		current: Amount,
	) -> DispatchResult {
		let previous = previous.into();
		let current = current.into();
		backend::with(|state| {
			let account = state.quantities.entry(account.encode()).or_default();
			let (account_total, network_total) = match quantity {
				PositionQuantity::Bonds => (&mut account.bonds, &mut state.totals.bonds),
				PositionQuantity::Stakes => (&mut account.stakes, &mut state.totals.stakes),
				PositionQuantity::FissionLiquidity =>
					(&mut account.fission_liquidity, &mut state.totals.fission_liquidity),
			};
			*account_total = account_total
				.checked_sub(previous)
				.expect("seeded account contribution")
				.checked_add(current)
				.expect("benchmark quantity fits");
			*network_total = network_total
				.checked_sub(previous)
				.expect("seeded network contribution")
				.checked_add(current)
				.expect("benchmark quantity fits");
		});
		Ok(())
	}

	fn upstream_position(account: &AccountId) -> Option<UpstreamPosition<Balance>> {
		backend::with(|state| {
			state.upstream.get(&account.encode()).map(|bytes| {
				UpstreamPosition::<Balance>::decode(&mut bytes.as_slice())
					.expect("seeded upstream position")
			})
		})
	}

	fn set_upstream_position(account: &AccountId, upstream: Option<UpstreamPosition<Balance>>) {
		backend::with(|state| {
			if let Some(upstream) = upstream {
				state.upstream.insert(account.encode(), upstream.encode());
			} else {
				state.upstream.remove(&account.encode());
			}
		});
	}

	fn bitcoin_position_updated(
		_: &AccountId,
		_: VaultId,
		_: BitcoinLockPosition<Balance>,
		_: BitcoinLockPosition<Balance>,
	) -> DispatchResult {
		Ok(())
	}

	fn bond_position_updated(
		account: &AccountId,
		_: VaultId,
		previous: Balance,
		current: Balance,
	) -> DispatchResult {
		backend::with(|state| {
			let total = state.principal.entry(account.encode()).or_default();
			*total = total
				.checked_sub(previous.into())
				.expect("seeded bond principal")
				.checked_add(current.into())
				.expect("benchmark principal fits");
		});
		Ok(())
	}
}

#[cfg(feature = "std")]
mod backend {
	use super::State;
	use core::cell::RefCell;
	std::thread_local! { static STATE: RefCell<State> = RefCell::new(State::default()); }
	pub(super) fn with<R>(f: impl FnOnce(&mut State) -> R) -> R {
		STATE.with(|state| f(&mut state.borrow_mut()))
	}
}

#[cfg(not(feature = "std"))]
mod backend {
	use super::State;
	use core::cell::UnsafeCell;
	struct StateCell(UnsafeCell<Option<State>>);
	// Benchmark Wasm runs on one thread. No state or synchronization is used in production.
	unsafe impl Sync for StateCell {}
	static STATE: StateCell = StateCell(UnsafeCell::new(None));
	pub(super) fn with<R>(f: impl FnOnce(&mut State) -> R) -> R {
		unsafe { f((*STATE.0.get()).get_or_insert_with(State::default)) }
	}
}
