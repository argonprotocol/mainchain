use crate::pallet::{AssetKind, Config, Pallet, SourceChain};
#[cfg(feature = "try-runtime")]
use codec::{Decode, Encode};
use frame_support::{traits::OnRuntimeUpgrade, weights::Weight, PalletId};
use pallet_prelude::*;
use sp_core::crypto::{AccountId32, Ss58Codec};
use sp_runtime::traits::AccountIdConversion;

const LEGACY_TOKEN_GATEWAY_PALLET_ID: [u8; 8] = [0xa0, 0x9b, 0x1c, 0x60, 0xe8, 0x65, 0x02, 0x45];
const HYPERBRIDGE_ROLLBACK_ARGONOT_RESIDUE: u128 = 7;
const LOG_TARGET: &str = "runtime::crosschain_transfer::migration";

pub struct InitializeCrosschainTransfer<T: Config>(core::marker::PhantomData<T>);

impl<T: Config> OnRuntimeUpgrade for InitializeCrosschainTransfer<T> {
	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<Vec<u8>, sp_runtime::TryRuntimeError> {
		Ok(legacy_balances::<T>().encode())
	}

	fn on_runtime_upgrade() -> Weight {
		let (legacy_argon, legacy_argonot) = legacy_balances::<T>();
		if legacy_argon == 0 && legacy_argonot == 0 {
			log::info!(
				target: LOG_TARGET,
				"no Hyperbridge rollback balances found for CrosschainTransfer initialization",
			);
			return Weight::zero();
		}

		log::info!(
			target: LOG_TARGET,
			"initializing CrosschainTransfer from legacy Hyperbridge balances: argon={legacy_argon}, argonot={legacy_argonot}",
		);

		initialize_crosschain_transfer::<T>()
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade(state: Vec<u8>) -> Result<(), sp_runtime::TryRuntimeError> {
		let (legacy_argon_before, legacy_argonot_before) = <(u128, u128)>::decode(&mut &state[..])
			.map_err(|_| "invalid crosschain migration state")?;
		let (legacy_argon_after, legacy_argonot_after) = legacy_balances::<T>();

		if legacy_argon_before > 0 || legacy_argonot_before > 0 {
			frame_support::ensure!(
				legacy_argon_after == 0 && legacy_argonot_after == 0,
				"legacy Hyperbridge balances were not fully migrated"
			);
		}
		frame_support::ensure!(
			<Pallet<T> as frame_support::traits::GetStorageVersion>::on_chain_storage_version() ==
				StorageVersion::new(1),
			"wrong storage version after crosschain initialization"
		);

		Ok(())
	}
}

pub(crate) fn initialize_crosschain_transfer<T: Config>() -> Weight {
	let legacy_account = legacy_account::<T>();
	let burn_account = Pallet::<T>::burn_account(SourceChain::Ethereum);
	let (legacy_argon_before, legacy_argonot_before) = legacy_balances::<T>();
	let (argon_refund_count, argon_refund_total, argonot_refund_count, argonot_refund_total) =
		ready_refunds()
			.iter()
			.fold((0usize, 0u128, 0usize, 0u128), |totals, refund| match refund.asset_kind {
				AssetKind::Argon => (
					totals.0.saturating_add(1),
					totals.1.saturating_add(refund.amount),
					totals.2,
					totals.3,
				),
				AssetKind::Argonot => (
					totals.0,
					totals.1,
					totals.2.saturating_add(1),
					totals.3.saturating_add(refund.amount),
				),
			});

	log::info!(
		target: LOG_TARGET,
		"running CrosschainTransfer initialization: legacy_argon_before={legacy_argon_before}, legacy_argonot_before={legacy_argonot_before}, argon_refunds={argon_refund_count}/{argon_refund_total}, argonot_refunds={argonot_refund_count}/{argonot_refund_total}, argonot_residue_to_burn={HYPERBRIDGE_ROLLBACK_ARGONOT_RESIDUE}",
	);

	migrate_legacy_balance::<T>(AssetKind::Argon, &legacy_account, &burn_account);
	migrate_legacy_balance::<T>(AssetKind::Argonot, &legacy_account, &burn_account);

	for refund in ready_refunds() {
		let account_id = decode_account_id::<T>(refund.account_id);
		let amount: T::Balance = refund.amount.into();

		match refund.asset_kind {
			AssetKind::Argon => {
				if let Err(error) = T::NativeCurrency::transfer(
					&burn_account,
					&account_id,
					amount,
					Preservation::Expendable,
				) {
					log::error!(
						target: LOG_TARGET,
						"failed to apply ready Argon refund for {}: {:?}",
						refund.account_id,
						error,
					);
				}
			},
			AssetKind::Argonot => {
				if let Err(error) = T::OwnershipCurrency::transfer(
					&burn_account,
					&account_id,
					amount,
					Preservation::Expendable,
				) {
					log::error!(
						target: LOG_TARGET,
						"failed to apply ready Argonot refund for {}: {:?}",
						refund.account_id,
						error,
					);
				}
			},
		}
	}

	if let Err(error) = T::OwnershipCurrency::burn_from(
		&burn_account,
		HYPERBRIDGE_ROLLBACK_ARGONOT_RESIDUE.into(),
		Preservation::Expendable,
		Precision::Exact,
		Fortitude::Force,
	) {
		log::error!(
			target: LOG_TARGET,
			"failed to burn Hyperbridge rollback Argonot residue: {:?}",
			error,
		);
	}

	let (legacy_argon_after, legacy_argonot_after) = legacy_balances::<T>();
	log::info!(
		target: LOG_TARGET,
		"finished CrosschainTransfer initialization: legacy_argon_after={legacy_argon_after}, legacy_argonot_after={legacy_argonot_after}",
	);

	StorageVersion::new(1).put::<Pallet<T>>();

	T::DbWeight::get().reads_writes(9, 9)
}

pub(crate) fn needs_initialization<T: Config>() -> bool {
	let (legacy_argon, legacy_argonot) = legacy_balances::<T>();
	legacy_argon > 0 || legacy_argonot > 0
}

struct RecoveryRefund {
	account_id: &'static str,
	asset_kind: AssetKind,
	amount: u128,
}

fn ready_refunds() -> &'static [RecoveryRefund] {
	&[
		RecoveryRefund {
			account_id: "5C5CdgR7eNjc8HCtR43uWSKsMoWZGZpMYGgCXAPJPMKdoVU2",
			asset_kind: AssetKind::Argon,
			amount: 1_000_001,
		},
		RecoveryRefund {
			account_id: "5F4UiKa1o5LLrLwgZz3pFizXhZnEvEr3mvtoGrEk3fZTXeyd",
			asset_kind: AssetKind::Argon,
			amount: 2_000,
		},
		RecoveryRefund {
			account_id: "5Cz3PZVcLitGyqc1Su4KYcvseoLhn93pUHtXDNBLx5aoKsF5",
			asset_kind: AssetKind::Argon,
			amount: 197_069_590,
		},
		RecoveryRefund {
			account_id: "5EqsqBNe1LfkGLEah9GpSMWTT4XHzGeVEAZ4dGUm5vFHA4t8",
			asset_kind: AssetKind::Argonot,
			amount: 200_000,
		},
	]
}

fn migrate_legacy_balance<T: Config>(
	asset_kind: AssetKind,
	legacy_account: &T::AccountId,
	burn_account: &T::AccountId,
) {
	let balance = match asset_kind {
		AssetKind::Argon => T::NativeCurrency::reducible_balance(
			legacy_account,
			Preservation::Expendable,
			Fortitude::Force,
		),
		AssetKind::Argonot => T::OwnershipCurrency::reducible_balance(
			legacy_account,
			Preservation::Expendable,
			Fortitude::Force,
		),
	};

	if balance == Zero::zero() || legacy_account == burn_account {
		return;
	}

	let result = match asset_kind {
		AssetKind::Argon => T::NativeCurrency::transfer(
			legacy_account,
			burn_account,
			balance,
			Preservation::Expendable,
		),
		AssetKind::Argonot => T::OwnershipCurrency::transfer(
			legacy_account,
			burn_account,
			balance,
			Preservation::Expendable,
		),
	};

	if let Err(error) = result {
		log::error!(
			target: LOG_TARGET,
			"failed to migrate legacy {:?} balance: {:?}",
			asset_kind,
			error,
		);
	}
}

fn decode_account_id<T: Config>(address: &'static str) -> T::AccountId {
	let account_id = AccountId32::from_ss58check(address)
		.expect("migration recovery accounts must be valid ss58 ids");
	let bytes: &[u8] = account_id.as_ref();
	T::AccountId::decode(&mut &bytes[..]).expect("migration recovery accounts are 32-byte ids")
}

fn legacy_account<T: Config>() -> T::AccountId {
	PalletId(LEGACY_TOKEN_GATEWAY_PALLET_ID).into_account_truncating()
}

fn legacy_balances<T: Config>() -> (u128, u128) {
	let legacy_account = legacy_account::<T>();
	let argon = T::NativeCurrency::reducible_balance(
		&legacy_account,
		Preservation::Expendable,
		Fortitude::Force,
	)
	.into();
	let argonot = T::OwnershipCurrency::reducible_balance(
		&legacy_account,
		Preservation::Expendable,
		Fortitude::Force,
	)
	.into();

	(argon, argonot)
}
