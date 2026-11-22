#![cfg_attr(not(feature = "std"), no_std)]
//! Maintained account and network quantities used by Treasury frame calculations and bond backing.
//! Source pallets own individual vaults, locks, lots, Fissions and custody. This pallet maintains
//! their aggregates and account earnings. Treasury owns frame terms and reward funding.

extern crate alloc;

pub use argon_primitives::treasury::CollectAmounts;
use argon_primitives::{
	treasury::{
		BitcoinLockPosition, BitcoinLockPositionProvider, CollectSource, EarningsIndexValues,
		EarningsSettlementProvider, IndexedEarningsPosition, NetworkPositionQuantities,
		PositionQuantities, PositionQuantity, TreasuryPositionProvider, UpstreamPosition,
		VaultEarningsProvider,
	},
	MiningFrameTransitionProvider, OperationalAccountProvider, TreasuryPoolProvider, VaultId,
};
use frame_support::{
	traits::{
		fungible::{Inspect, Mutate},
		tokens::Preservation,
	},
	PalletId,
};
pub use pallet::*;
use pallet_prelude::*;
use polkadot_sdk::sp_runtime::{traits::AccountIdConversion, ArithmeticError};
pub use weights::{ProviderWeightAdapter, WeightInfo};
#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
pub mod migrations;
#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;
#[cfg(feature = "try-runtime")]
mod try_state;
pub mod weights;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	pub(super) const STORAGE_VERSION: StorageVersion = StorageVersion::new(4);

	#[pallet::pallet]
	#[pallet::storage_version(STORAGE_VERSION)]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: polkadot_sdk::frame_system::Config {
		/// Argon balance units for bond principal and Bitcoin collateral (microgons in the
		/// runtime).
		type Balance: AtLeast32BitUnsigned
			+ Member
			+ codec::FullCodec
			+ Copy
			+ Default
			+ MaybeSerializeDeserialize
			+ core::fmt::Debug
			+ From<u128>
			+ Into<u128>
			+ codec::HasCompact
			+ DecodeWithMemTracking
			+ TypeInfo
			+ MaxEncodedLen;
		/// Resolves the actual vault account and its single upstream vault from Operational
		/// Accounts.
		type OperationalAccountProvider: OperationalAccountProvider<Self::AccountId>;
		/// Reads canonical owner-indexed Bitcoin locks when initializing a registration snapshot.
		type BitcoinPositionProvider: BitcoinLockPositionProvider<Self::AccountId, Self::Balance>;
		/// Reads canonical upstream bond principal when initializing a registration snapshot.
		type TreasuryPoolProvider: TreasuryPoolProvider<Self::AccountId, Balance = Self::Balance>;
		/// Source for registered stabilizer pool positions and their claim cursors.
		type EarningsSettlementProvider: EarningsSettlementProvider<Self::AccountId>;
		/// Vault-owned collection checks and source revenue bookkeeping.
		type VaultEarningsProvider: VaultEarningsProvider<Self::AccountId, Self::Balance>;
		/// Account that holds issued backing for committed collections.
		#[pallet::constant]
		type PalletId: Get<PalletId>;
		/// Maximum ready quantity changes settled by one Collect.
		#[pallet::constant]
		type MaxEarningChangesPerCall: Get<u32>;
		/// Financial day used for next-frame quantity activation.
		type MiningFrameTransitionProvider: MiningFrameTransitionProvider;
		/// Delivers the account's combined ARGON collection.
		type Currency: Inspect<Self::AccountId, Balance = Self::Balance> + Mutate<Self::AccountId>;
		/// Costs of reading and maintaining account aggregates.
		type WeightInfo: WeightInfo;
	}

	/// Account-wide positions and earnings, with an optional subset in the upstream vault.
	#[derive(
		Encode,
		Decode,
		DecodeWithMemTracking,
		Clone,
		Debug,
		Default,
		PartialEq,
		Eq,
		TypeInfo,
		MaxEncodedLen,
	)]
	pub struct Position<Balance>
	where
		Balance: Codec + MaxEncodedLen,
	{
		/// Live principal for certification/backing, including displaced flexible bonds.
		#[codec(compact)]
		pub bond_principal: Balance,
		/// Live admitted bond/stake quantities and active Fission promises.
		pub quantities: PositionQuantities<Balance>,
		/// Participation in the single vault that onboarded this account. Absent when no upstream
		/// vault is resolvable, including accounts whose upstream has yet to create a vault.
		pub upstream: Option<UpstreamPosition<Balance>>,
		/// Recorded whole microgons, by source. Unsettled indexed earnings are additional.
		pub collect_accrued: CollectAmounts<Balance>,
		/// Lifetime amounts delivered through collection, by source. Counters saturate without
		/// blocking payment.
		pub collected: CollectAmounts<Balance>,
		/// Indexed earning quantities and the oldest/newest unsettled quantity changes.
		pub earning_positions: EarningPositions,
	}

	#[derive(
		Encode,
		Decode,
		DecodeWithMemTracking,
		Clone,
		Debug,
		Default,
		PartialEq,
		Eq,
		TypeInfo,
		MaxEncodedLen,
	)]
	pub struct EarningPositions {
		/// Eligible whole ARGON bonds and their saved earnings progress.
		pub bonds: IndexedEarningsPosition,
		/// Eligible whole ARGONOT stakes and their saved earnings progress.
		pub stakes: IndexedEarningsPosition,
		/// Eligible promised Bitcoin liquidity in microgons and its saved earnings progress.
		pub bitcoin_liquidity: IndexedEarningsPosition,
		/// Oldest unsettled quantity replacement; absent when active quantities are current.
		pub next_change_frame: Option<FrameId>,
		/// Newest replacement, allowing same-frame changes to overwrite one entry.
		pub last_change_frame: Option<FrameId>,
	}

	#[derive(
		Encode, Decode, DecodeWithMemTracking, Clone, Debug, PartialEq, Eq, TypeInfo, MaxEncodedLen,
	)]
	pub struct EarningPositionChange<Balance: Codec + MaxEncodedLen> {
		/// Earning quantities effective at this storage key's frame start.
		pub quantities: PositionQuantities<Balance>,
		/// The same account's next replacement; strictly later than this entry.
		pub next_frame: Option<FrameId>,
	}

	/// Only unsettled next-frame quantity changes survive; same-frame changes coalesce.
	#[pallet::storage]
	pub type EarningPositionChanges<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		T::AccountId,
		Twox64Concat,
		FrameId,
		EarningPositionChange<T::Balance>,
		OptionQuery,
	>;

	#[derive(
		Encode, Decode, DecodeWithMemTracking, Clone, Debug, PartialEq, Eq, TypeInfo, MaxEncodedLen,
	)]
	pub struct EarningsIndexState {
		/// Funded cumulative source earnings at FixedU128 precision.
		pub values: EarningsIndexValues,
		/// First earning frame included at the upgrade cutover.
		pub start_frame: FrameId,
		/// Latest frame whose allocation is funded and included in values.
		pub last_processed_frame: Option<FrameId>,
	}

	impl Default for EarningsIndexState {
		fn default() -> Self {
			Self { values: Default::default(), start_frame: 1, last_processed_frame: None }
		}
	}

	/// Current funded index progress; frame transitions update this single record.
	#[pallet::storage]
	pub type EarningsIndexes<T: Config> = StorageValue<_, EarningsIndexState, ValueQuery>;

	#[derive(
		Encode,
		Decode,
		DecodeWithMemTracking,
		Clone,
		Debug,
		Default,
		PartialEq,
		Eq,
		TypeInfo,
		MaxEncodedLen,
	)]
	pub struct FrameEarningsIndexes {
		/// Cumulative indexes immediately before this frame; None until the prior frame funds.
		pub values: Option<EarningsIndexValues>,
		/// Account changes referencing this boundary; delete it when the count reaches zero.
		pub pending_account_count: u32,
	}

	/// Only boundaries still referenced by unsettled account quantity changes are retained.
	#[pallet::storage]
	pub type EarningsIndexesAtFrameStart<T: Config> =
		StorageMap<_, Twox64Concat, FrameId, FrameEarningsIndexes, OptionQuery>;

	/// Source quantities and earnings keyed by the position owner. Upstream quantities belong to
	/// the linked vault account; primary and mining accounts have separate positions.
	#[pallet::storage]
	pub type PositionsByAccount<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, Position<T::Balance>, OptionQuery>;

	/// Account contribution totals and Vault Bitcoin coverage, maintained as sources change.
	/// Treasury freezes these network quantities at each frame boundary.
	#[pallet::storage]
	pub type NetworkTotals<T: Config> =
		StorageValue<_, NetworkPositionQuantities<T::Balance>, ValueQuery>;

	/// Issued funding and delivered collections since backed Collect was activated. The custody
	/// minimum and historical wallet payments are excluded from these counters.
	#[pallet::storage]
	pub type CollectTotals<T: Config> =
		StorageValue<_, argon_primitives::treasury::CollectTotals<T::Balance>, ValueQuery>;

	/// Pre-upgrade IOUs still awaiting their one-time custody funding. Payments and new funding
	/// finish this aggregate cutover first; retries never walk account records.
	#[pallet::storage]
	pub type PendingCollectBacking<T: Config> =
		StorageValue<_, CollectAmounts<T::Balance>, OptionQuery>;

	#[pallet::error]
	pub enum Error<T> {
		/// The account layout upgrade must succeed before custody can be used.
		CollectMigrationIncomplete,
		/// An account quantity transition still refers to a missing retained boundary.
		MissingEarningsBoundary,
		/// Indexed frames must commit once and in their allocation order.
		EarningsFrameOutOfOrder,
		/// A nonzero indexed allocation requires eligible quantity.
		UnfundedEarnings,
		/// The source tried to expire more vault revenue than its account bucket contains.
		InsufficientVaultEarnings,
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		#[cfg(feature = "try-runtime")]
		fn try_state(_: BlockNumberFor<T>) -> Result<(), sp_runtime::TryRuntimeError> {
			crate::try_state::check::<T>()
		}
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// ARGON delivered to the account, recorded by its source.
		Collected { account: T::AccountId, amounts: CollectAmounts<T::Balance> },
		/// Source earnings backed in custody, before assignment to individual owners.
		CollectFunded { amounts: CollectAmounts<T::Balance> },
		/// Already backed earnings assigned to the beneficiary.
		CollectAccrued { account: T::AccountId, source: CollectSource, amount: T::Balance },
		/// Funded allocations added to the cumulative bond/stake indexes, once per frame.
		FrameEarningsIndexed { frame_id: FrameId, amounts: CollectAmounts<T::Balance> },
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Settle the signer's available source earnings and collect them in one payment.
		#[pallet::call_index(0)]
		#[pallet::weight(weights::collect::<T>())]
		pub fn collect(origin: OriginFor<T>) -> DispatchResult {
			let account = ensure_signed(origin)?;
			Self::complete_backing_cutover()?;
			T::EarningsSettlementProvider::settle_account(&account)?;
			Self::settle_account_indexes(&account)?;

			let Some(mut position) = PositionsByAccount::<T>::get(&account) else { return Ok(()) };
			let mut amounts = position.collect_accrued;
			if !T::VaultEarningsProvider::collect_earnings(&account, amounts.vault_earnings)? {
				amounts.vault_earnings = T::Balance::zero();
			}

			let amount = amounts.total();
			if amount.is_zero() {
				return Ok(())
			}

			T::Currency::transfer(&Self::account_id(), &account, amount, Preservation::Preserve)?;
			CollectTotals::<T>::mutate(|totals| totals.collected.saturating_accrue(&amounts));
			position.collected.saturating_accrue(&amounts);
			position.collect_accrued.saturating_reduce(&amounts);
			PositionsByAccount::<T>::insert(&account, position);

			Self::deposit_event(Event::Collected { account, amounts });
			Ok(())
		}
	}

	impl<T: Config> TreasuryPositionProvider<T::AccountId, T::Balance> for Pallet<T> {
		type Weights = ProviderWeightAdapter<T>;

		fn record_funding(amounts: CollectAmounts<T::Balance>) {
			CollectTotals::<T>::mutate(|totals| totals.funded.saturating_accrue(&amounts));
			Self::deposit_event(Event::CollectFunded { amounts });
		}

		fn expire_vault_earnings(account: &T::AccountId, amount: T::Balance) -> DispatchResult {
			if amount.is_zero() {
				return Ok(())
			}
			Self::complete_backing_cutover()?;
			PositionsByAccount::<T>::try_mutate(account, |record| -> DispatchResult {
				let position = record.as_mut().ok_or(Error::<T>::InsufficientVaultEarnings)?;
				ensure!(
					position.collect_accrued.vault_earnings >= amount,
					Error::<T>::InsufficientVaultEarnings
				);
				T::Currency::burn_from(
					&Self::account_id(),
					amount,
					Preservation::Preserve,
					Precision::Exact,
					Fortitude::Force,
				)?;
				position
					.collect_accrued
					.saturating_reduce(&CollectAmounts::from_source(CollectSource::Vaults, amount));
				CollectTotals::<T>::mutate(|totals| {
					totals.expired_vault_earnings.saturating_accrue(amount)
				});
				Ok(())
			})
		}

		fn ensure_account(account: &T::AccountId) {
			if !PositionsByAccount::<T>::contains_key(account) {
				PositionsByAccount::<T>::insert(account, Self::new_position(account));
			}
		}

		fn collect_account() -> Option<T::AccountId> {
			Some(Self::account_id())
		}

		fn prepare_collect() -> DispatchResult {
			Self::complete_backing_cutover()?;
			Self::seed_custody()
		}

		fn fund(source: CollectSource, amount: T::Balance) -> DispatchResult {
			if amount.is_zero() {
				return Ok(())
			}

			Self::prepare_collect()?;
			T::Currency::mint_into(&Self::account_id(), amount)?;
			Self::record_funding(CollectAmounts::from_source(source, amount));
			Ok(())
		}

		fn accrue(
			account: &T::AccountId,
			source: CollectSource,
			amount: T::Balance,
		) -> DispatchResult {
			if amount.is_zero() {
				return Ok(())
			}

			Self::complete_backing_cutover()?;
			PositionsByAccount::<T>::mutate(account, |record| {
				let position = record.get_or_insert_with(|| Self::new_position(account));
				position
					.collect_accrued
					.saturating_accrue(&CollectAmounts::from_source(source, amount));
			});

			Self::deposit_event(Event::CollectAccrued { account: account.clone(), source, amount });
			Ok(())
		}

		fn migrate_accrued(account: &T::AccountId, amounts: CollectAmounts<T::Balance>) {
			if amounts.total().is_zero() {
				return
			}

			PositionsByAccount::<T>::mutate(account, |record| {
				let position = record.get_or_insert_with(|| Self::new_position(account));
				position.collect_accrued.saturating_accrue(&amounts);
			});
			PendingCollectBacking::<T>::mutate(|pending| {
				pending.get_or_insert_with(CollectAmounts::default).saturating_accrue(&amounts);
			});
		}

		fn allocate_frame_earnings(
			frame: FrameId,
			quantities: &PositionQuantities<T::Balance>,
			amounts: CollectAmounts<T::Balance>,
		) -> DispatchResult {
			let mut state = EarningsIndexes::<T>::get();
			ensure!(frame >= state.start_frame, Error::<T>::EarningsFrameOutOfOrder);
			ensure!(
				state.last_processed_frame.is_none_or(|previous| frame > previous),
				Error::<T>::EarningsFrameOutOfOrder
			);

			for (index, quantity, amount) in [
				(&mut state.values.bonds, quantities.bonds, amounts.bond_earnings),
				(&mut state.values.stakes, quantities.stakes, amounts.stake_earnings),
				(
					&mut state.values.bitcoin_liquidity,
					quantities.fission_liquidity.into(),
					amounts.bitcoin_earnings,
				),
			] {
				if amount.is_zero() {
					continue
				}

				ensure!(quantity > 0, Error::<T>::UnfundedEarnings);
				let increment = FixedU128::saturating_from_rational(amount.into(), quantity);
				*index = index.saturating_add(increment);
			}

			if !amounts.total().is_zero() {
				Self::complete_backing_cutover()?;
				Self::record_funding(amounts);
			}
			state.last_processed_frame = Some(frame);
			EarningsIndexesAtFrameStart::<T>::mutate_exists(frame.saturating_add(1), |boundary| {
				if let Some(boundary) = boundary {
					boundary.values = Some(state.values);
				}
			});
			EarningsIndexes::<T>::put(&state);

			Self::deposit_event(Event::FrameEarningsIndexed { frame_id: frame, amounts });
			Ok(())
		}

		fn indexed_frame() -> Option<FrameId> {
			EarningsIndexes::<T>::get().last_processed_frame
		}

		fn earnings_start_frame() -> FrameId {
			EarningsIndexes::<T>::get().start_frame
		}

		/// Seed the upstream subset after registration records the relationship. Locks and bonds
		/// can predate registration, so query their canonical owner indexes once. Preserve the
		/// account-wide bond principal already maintained by live source updates.
		fn operational_account_registered(account: &T::AccountId) -> DispatchResult {
			let upstream = if let Some((_, vault_id)) =
				T::OperationalAccountProvider::upstream_vault(account)
			{
				let bitcoin = T::BitcoinPositionProvider::account_position(account, vault_id)?;
				Some(UpstreamPosition {
					vault_id,
					bitcoin_securitization: bitcoin.activated_securitization,
					bitcoin_allocated_securitization: bitcoin.allocated_securitization,
					bond_principal: T::TreasuryPoolProvider::active_vault_bond_amount(
						vault_id, account,
					),
				})
			} else {
				None
			};
			Self::set_upstream_position(account, upstream);
			Ok(())
		}

		/// Live principal across all vaults, including displaced flexible bonds and excluding
		/// releases.
		fn bond_principal(account: &T::AccountId) -> T::Balance {
			PositionsByAccount::<T>::get(account)
				.map(|p| p.bond_principal)
				.unwrap_or_default()
		}

		fn account_quantities(account: &T::AccountId) -> PositionQuantities<T::Balance> {
			PositionsByAccount::<T>::get(account).map(|p| p.quantities).unwrap_or_default()
		}

		fn network_totals() -> NetworkPositionQuantities<T::Balance> {
			NetworkTotals::<T>::get()
		}

		/// Replace a source's contribution before or after account registration. Account and
		/// network quantities commit together; Treasury retains the current frame's frozen terms.
		fn account_quantity_updated<Amount: Into<u128>>(
			account: &T::AccountId,
			quantity: PositionQuantity,
			previous: Amount,
			current: Amount,
		) -> DispatchResult {
			let previous = previous.into();
			let current = current.into();
			if previous == current {
				return Ok(());
			}

			PositionsByAccount::<T>::try_mutate(account, |record| -> DispatchResult {
				let account_position = record.get_or_insert_with(|| Self::new_position(account));
				// One boundary handles the usual next-frame refresh. Collect drains older history.
				Self::settle_indexes(account, account_position, 1)?;

				let replace = |quantities: &mut PositionQuantities<T::Balance>| -> DispatchResult {
					let total = match quantity {
						PositionQuantity::Bonds => quantities.bonds,
						PositionQuantity::Stakes => quantities.stakes,
						PositionQuantity::FissionLiquidity => quantities.fission_liquidity.into(),
					};
					let updated = total
						.checked_sub(previous)
						.ok_or(ArithmeticError::Underflow)?
						.checked_add(current)
						.ok_or(ArithmeticError::Overflow)?;
					match quantity {
						PositionQuantity::Bonds => quantities.bonds = updated,
						PositionQuantity::Stakes => quantities.stakes = updated,
						PositionQuantity::FissionLiquidity =>
							quantities.fission_liquidity = updated.into(),
					}
					Ok(())
				};

				replace(&mut account_position.quantities)?;
				Self::schedule_earning_change(account, account_position)?;

				NetworkTotals::<T>::try_mutate(|totals| replace(&mut totals.account_quantities))
			})
		}

		fn network_securitized_satoshis_updated(previous: u128, current: u128) -> DispatchResult {
			if previous == current {
				return Ok(());
			}
			NetworkTotals::<T>::try_mutate(|totals| -> DispatchResult {
				totals.vault_securitized_satoshis = totals
					.vault_securitized_satoshis
					.checked_sub(previous)
					.ok_or(ArithmeticError::Underflow)?
					.checked_add(current)
					.ok_or(ArithmeticError::Overflow)?;
				Ok(())
			})
		}

		/// Read the maintained upstream subset without scanning canonical locks or bond lots.
		fn upstream_position(account: &T::AccountId) -> Option<UpstreamPosition<T::Balance>> {
			PositionsByAccount::<T>::get(account).and_then(|p| p.upstream)
		}

		/// Install a complete upstream snapshot at registration or upgrade, preserving total
		/// principal.
		fn set_upstream_position(
			account: &T::AccountId,
			upstream: Option<UpstreamPosition<T::Balance>>,
		) {
			PositionsByAccount::<T>::mutate(account, |p| {
				p.get_or_insert_with(|| Self::new_position(account)).upstream = upstream
			});
		}

		/// Replace an affected Bitcoin securitization contribution in this owner's upstream.
		/// Checked arithmetic prevents drift; a failed replacement leaves the aggregate unchanged.
		fn bitcoin_position_updated(
			account: &T::AccountId,
			vault: VaultId,
			previous: BitcoinLockPosition<T::Balance>,
			position: BitcoinLockPosition<T::Balance>,
		) -> DispatchResult {
			PositionsByAccount::<T>::try_mutate_exists(account, |record| -> DispatchResult {
				Self::bind_upstream_if_created(account, vault, record);
				let Some(upstream) = record
					.as_mut()
					.and_then(|p| p.upstream.as_mut())
					.filter(|p| p.vault_id == vault)
				else {
					return Ok(());
				};
				upstream.bitcoin_securitization = upstream
					.bitcoin_securitization
					.checked_sub(&previous.activated_securitization)
					.ok_or(ArithmeticError::Underflow)?
					.checked_add(&position.activated_securitization)
					.ok_or(ArithmeticError::Overflow)?;
				upstream.bitcoin_allocated_securitization = upstream
					.bitcoin_allocated_securitization
					.checked_sub(&previous.allocated_securitization)
					.ok_or(ArithmeticError::Underflow)?
					.checked_add(&position.allocated_securitization)
					.ok_or(ArithmeticError::Overflow)?;
				Ok(())
			})
		}

		/// Replace one active vault lot's principal in the account total and, when applicable, its
		/// upstream subset. Releasing lots contribute zero; flexible displacement retains
		/// principal.
		fn bond_position_updated(
			account: &T::AccountId,
			vault: VaultId,
			previous: T::Balance,
			principal: T::Balance,
		) -> DispatchResult {
			PositionsByAccount::<T>::try_mutate(account, |record| -> DispatchResult {
				Self::bind_upstream_if_created(account, vault, record);
				let p = record.get_or_insert_with(|| Self::new_position(account));
				p.bond_principal = p
					.bond_principal
					.checked_sub(&previous)
					.ok_or(ArithmeticError::Underflow)?
					.checked_add(&principal)
					.ok_or(ArithmeticError::Overflow)?;
				if let Some(upstream) = p.upstream.as_mut().filter(|p| p.vault_id == vault) {
					upstream.bond_principal = upstream
						.bond_principal
						.checked_sub(&previous)
						.ok_or(ArithmeticError::Underflow)?
						.checked_add(&principal)
						.ok_or(ArithmeticError::Overflow)?;
				}
				Ok(())
			})
		}
	}

	impl<T: Config> Pallet<T> {
		pub fn account_id() -> T::AccountId {
			T::PalletId::get().into_account_truncating()
		}

		pub(crate) fn settle_account_indexes(account: &T::AccountId) -> DispatchResult {
			PositionsByAccount::<T>::try_mutate_exists(account, |record| {
				if let Some(position) = record {
					Self::settle_indexes(account, position, T::MaxEarningChangesPerCall::get())?;
				}
				Ok(())
			})
		}

		/// Settle only this account on Collect or a source change; frame hooks never call this.
		fn settle_indexes(
			account: &T::AccountId,
			position: &mut Position<T::Balance>,
			max_changes: u32,
		) -> DispatchResult {
			let state = EarningsIndexes::<T>::get();

			for _ in 0..max_changes {
				let Some(frame) = position.earning_positions.next_change_frame else { break };
				if state
					.last_processed_frame
					.is_none_or(|processed| processed < frame.saturating_sub(1))
				{
					break
				}

				let boundary = EarningsIndexesAtFrameStart::<T>::get(frame)
					.and_then(|value| value.values)
					.ok_or(Error::<T>::MissingEarningsBoundary)?;
				Self::settle_to(position, boundary);

				let change = EarningPositionChanges::<T>::take(account, frame)
					.ok_or(Error::<T>::MissingEarningsBoundary)?;
				position.earning_positions.bonds.quantity = change.quantities.bonds;
				position.earning_positions.stakes.quantity = change.quantities.stakes;
				position.earning_positions.bitcoin_liquidity.quantity =
					change.quantities.fission_liquidity.into();
				position.earning_positions.next_change_frame = change.next_frame;
				if change.next_frame.is_none() {
					position.earning_positions.last_change_frame = None;
				}
				Self::release_earnings_boundary(frame)?;
			}

			let has_ready_change =
				position.earning_positions.next_change_frame.is_some_and(|frame| {
					state
						.last_processed_frame
						.is_some_and(|processed| processed >= frame.saturating_sub(1))
				});
			if !has_ready_change {
				Self::settle_to(position, state.values);
			}
			Ok(())
		}

		fn settle_to(position: &mut Position<T::Balance>, values: EarningsIndexValues) {
			let bonds = position.earning_positions.bonds.settle(values.bonds);
			let stakes = position.earning_positions.stakes.settle(values.stakes);
			let bitcoin =
				position.earning_positions.bitcoin_liquidity.settle(values.bitcoin_liquidity);
			position.collect_accrued.saturating_accrue(&CollectAmounts {
				bond_earnings: bonds.into(),
				stake_earnings: stakes.into(),
				bitcoin_earnings: bitcoin.into(),
				..Default::default()
			});
		}

		/// Record this account's next-frame quantity when its source position changes.
		fn schedule_earning_change(
			account: &T::AccountId,
			position: &mut Position<T::Balance>,
		) -> DispatchResult {
			let current = T::MiningFrameTransitionProvider::get_current_frame_id();
			if current < EarningsIndexes::<T>::get().start_frame {
				position.earning_positions.bonds.quantity = position.quantities.bonds;
				position.earning_positions.stakes.quantity = position.quantities.stakes;
				position.earning_positions.bitcoin_liquidity.quantity =
					position.quantities.fission_liquidity.into();
				return Ok(())
			}

			let frame = current.saturating_add(1);
			let tail = position.earning_positions.last_change_frame;
			if tail == Some(frame) {
				EarningPositionChanges::<T>::try_mutate(
					account,
					frame,
					|record| -> DispatchResult {
						let change = record.as_mut().ok_or(Error::<T>::MissingEarningsBoundary)?;
						change.quantities = position.quantities;
						Ok(())
					},
				)?;
				return Ok(())
			}

			if let Some(previous) = tail {
				ensure!(previous < frame, Error::<T>::EarningsFrameOutOfOrder);
				EarningPositionChanges::<T>::try_mutate(
					account,
					previous,
					|record| -> DispatchResult {
						let change = record.as_mut().ok_or(Error::<T>::MissingEarningsBoundary)?;
						change.next_frame = Some(frame);
						Ok(())
					},
				)?;
			} else {
				position.earning_positions.next_change_frame = Some(frame);
			}

			EarningsIndexesAtFrameStart::<T>::mutate(frame, |record| {
				let boundary = record.get_or_insert_with(FrameEarningsIndexes::default);
				boundary.pending_account_count = boundary.pending_account_count.saturating_add(1);
			});

			EarningPositionChanges::<T>::insert(
				account,
				frame,
				EarningPositionChange { quantities: position.quantities, next_frame: None },
			);
			position.earning_positions.last_change_frame = Some(frame);
			Ok(())
		}

		fn release_earnings_boundary(frame: FrameId) -> DispatchResult {
			EarningsIndexesAtFrameStart::<T>::try_mutate_exists(frame, |record| -> DispatchResult {
				let boundary = record.as_mut().ok_or(Error::<T>::MissingEarningsBoundary)?;
				boundary.pending_account_count = boundary.pending_account_count.saturating_sub(1);
				if boundary.pending_account_count == 0 {
					*record = None;
				}
				Ok(())
			})
		}

		pub(crate) fn complete_backing_cutover() -> DispatchResult {
			ensure!(
				StorageVersion::get::<Self>() == STORAGE_VERSION,
				Error::<T>::CollectMigrationIncomplete
			);
			let Some(amounts) = PendingCollectBacking::<T>::get() else { return Ok(()) };

			Self::seed_custody()?;
			T::Currency::mint_into(&Self::account_id(), amounts.total())?;
			CollectTotals::<T>::mutate(|totals| totals.funded.saturating_accrue(&amounts));
			PendingCollectBacking::<T>::kill();
			Ok(())
		}

		/// Endow the currency minimum once as system setup, outside earnings and reserve budgets.
		pub(crate) fn seed_custody() -> DispatchResult {
			let account = Self::account_id();
			let missing =
				T::Currency::minimum_balance().saturating_sub(T::Currency::balance(&account));
			if !missing.is_zero() {
				T::Currency::mint_into(&account, missing)?;
			}
			Ok(())
		}

		fn new_position(account: &T::AccountId) -> Position<T::Balance> {
			frame_system::Pallet::<T>::inc_providers(account);
			let mut position = Position::default();
			let values = EarningsIndexes::<T>::get().values;
			position.earning_positions.bonds.checkpoint.index = values.bonds;
			position.earning_positions.stakes.checkpoint.index = values.stakes;
			position.earning_positions.bitcoin_liquidity.checkpoint.index =
				values.bitcoin_liquidity;
			position
		}

		/// An invite can predate its upstream's vault. Bind when that vault first gets a position.
		/// Vault IDs are never reused, so there are no earlier contributions in this newly created
		/// vault to scan. Only the actual linked vault account may own the upstream subset.
		fn bind_upstream_if_created(
			account: &T::AccountId,
			vault: VaultId,
			record: &mut Option<Position<T::Balance>>,
		) {
			if record.as_ref().is_some_and(|p| p.upstream.is_some()) {
				return;
			}
			if let Some((owner, upstream_vault)) =
				T::OperationalAccountProvider::upstream_vault(account) &&
				owner == *account &&
				upstream_vault == vault
			{
				record.get_or_insert_with(|| Self::new_position(account)).upstream =
					Some(UpstreamPosition { vault_id: vault, ..Default::default() });
			}
		}
	}
}
