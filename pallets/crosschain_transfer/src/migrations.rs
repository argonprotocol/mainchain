use crate::pallet::{self, AssetKind, Config, Pallet, SourceChain};
use frame_support::{
	migrations::VersionedMigration, traits::UncheckedOnRuntimeUpgrade, weights::Weight, PalletId,
};
use pallet_prelude::*;
use sp_core::crypto::{AccountId32, Ss58Codec};
use sp_runtime::traits::AccountIdConversion;

const LEGACY_TOKEN_GATEWAY_PALLET_ID: [u8; 8] = [0xa0, 0x9b, 0x1c, 0x60, 0xe8, 0x65, 0x02, 0x45];
const HYPERBRIDGE_ROLLBACK_ARGONOT_RESIDUE: u128 = 7;

pub struct InitializeCrosschainTransfer<T: Config>(core::marker::PhantomData<T>);

impl<T: Config> UncheckedOnRuntimeUpgrade for InitializeCrosschainTransfer<T> {
	fn on_runtime_upgrade() -> Weight {
		let legacy_account = legacy_token_gateway_account::<T>();
		let source_chain = SourceChain::Ethereum;
		let burn_account = Pallet::<T>::burn_account(source_chain);

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

		T::DbWeight::get().reads_writes(9, 9)
	}
}

pub type InitializeCrosschainTransferMigration<T> = VersionedMigration<
	0,
	1,
	InitializeCrosschainTransfer<T>,
	pallet::Pallet<T>,
	<T as frame_system::Config>::DbWeight,
>;

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

fn legacy_token_gateway_account<T: Config>() -> T::AccountId {
	PalletId(LEGACY_TOKEN_GATEWAY_PALLET_ID).into_account_truncating()
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
