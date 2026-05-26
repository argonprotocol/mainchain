use crate::pallet::{AssetKind, Config, Pallet, SourceChain};
use frame_support::{weights::Weight, PalletId};
use pallet_prelude::*;
use sp_core::crypto::{AccountId32, Ss58Codec};
use sp_runtime::traits::AccountIdConversion;

const LEGACY_TOKEN_GATEWAY_PALLET_ID: [u8; 8] = [0xa0, 0x9b, 0x1c, 0x60, 0xe8, 0x65, 0x02, 0x45];
const HYPERBRIDGE_ROLLBACK_ARGONOT_RESIDUE: u128 = 7;

pub(crate) fn initialize_crosschain_transfer<T: Config>() -> Weight {
	let legacy_account: T::AccountId =
		PalletId(LEGACY_TOKEN_GATEWAY_PALLET_ID).into_account_truncating();
	let burn_account = Pallet::<T>::burn_account(SourceChain::Ethereum);

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
						target: "runtime::crosschain_transfer::migration",
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
						target: "runtime::crosschain_transfer::migration",
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
			target: "runtime::crosschain_transfer::migration",
			"failed to burn Hyperbridge rollback Argonot residue: {:?}",
			error,
		);
	}

	StorageVersion::new(1).put::<Pallet<T>>();

	T::DbWeight::get().reads_writes(9, 9)
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
			target: "runtime::crosschain_transfer::migration",
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
