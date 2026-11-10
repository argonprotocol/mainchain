use crate::{
	ArgonApprovalsNonce, Config, GatewayActivityNonce, GatewayState, GatewayStateBySourceChain,
	Pallet, TransferOutById,
};
#[cfg(feature = "try-runtime")]
use crate::{PendingTransferOutQuoteCountsByDestinationChain, SourceChain};
use codec::Compact;
#[cfg(feature = "try-runtime")]
use codec::DecodeAll;
#[cfg(feature = "try-runtime")]
use frame_support::storage::unhashed;
use frame_support::{migrations::VersionedMigration, traits::UncheckedOnRuntimeUpgrade};
use pallet_prelude::*;

// Version 3 stored the two compact nonces followed by the two circulation amounts.
type GatewayStateV3<Balance> =
	(Compact<GatewayActivityNonce>, Compact<ArgonApprovalsNonce>, Balance, Balance);

pub struct AddGatewaySyncTime<T>(PhantomData<T>);

impl<T: Config> UncheckedOnRuntimeUpgrade for AddGatewaySyncTime<T> {
	fn on_runtime_upgrade() -> Weight {
		let mut entries = 0u64;
		GatewayStateBySourceChain::<T>::translate_values::<GatewayStateV3<T::Balance>, _>(
			|(gateway_nonce, approvals_nonce, argon_circulation, argonot_circulation)| {
				entries.saturating_accrue(1);
				Some(GatewayState {
					// Earlier proofs did not retain their source block time. Wait for a new proof
					// rather than treating Argon's upgrade time as an authenticated checkpoint.
					last_synced_source_block_time: None,
					gateway_activity_nonce: gateway_nonce.0,
					argon_approvals_nonce: approvals_nonce.0,
					argon_circulation,
					argonot_circulation,
				})
			},
		);
		T::DbWeight::get().reads_writes(entries.saturating_add(1), entries)
	}

	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<Vec<u8>, sp_runtime::TryRuntimeError> {
		let mut states = Vec::new();
		for chain in GatewayStateBySourceChain::<T>::iter_keys() {
			let bytes = unhashed::get_raw(&GatewayStateBySourceChain::<T>::hashed_key_for(chain))
				.ok_or("Gateway state disappeared during migration preflight")?;
			let state = GatewayStateV3::<T::Balance>::decode_all(&mut &bytes[..])
				.map_err(|_| "Cannot decode version 3 gateway state")?;
			states.push((chain, state));
		}
		Ok(states.encode())
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade(state: Vec<u8>) -> Result<(), sp_runtime::TryRuntimeError> {
		let states = Vec::<(SourceChain, GatewayStateV3<T::Balance>)>::decode_all(&mut &state[..])
			.map_err(|_| "Invalid gateway migration preflight state")?;
		ensure!(
			GatewayStateBySourceChain::<T>::iter_keys().count() == states.len(),
			"Gateway state count changed during migration"
		);
		for (chain, (gateway_nonce, approvals_nonce, argon_circulation, argonot_circulation)) in
			states
		{
			ensure!(
				GatewayStateBySourceChain::<T>::get(chain) ==
					Some(GatewayState {
						last_synced_source_block_time: None,
						gateway_activity_nonce: gateway_nonce.0,
						argon_approvals_nonce: approvals_nonce.0,
						argon_circulation,
						argonot_circulation,
					}),
				"Gateway migration changed the accepted snapshot"
			);
		}
		Ok(())
	}
}

pub type AddGatewaySyncTimeMigration<T> = VersionedMigration<
	3,
	4,
	AddGatewaySyncTime<T>,
	Pallet<T>,
	<T as frame_system::Config>::DbWeight,
>;

pub struct InitializePendingQuoteCounts<T>(PhantomData<T>);

impl<T: Config> UncheckedOnRuntimeUpgrade for InitializePendingQuoteCounts<T> {
	fn on_runtime_upgrade() -> Weight {
		let mut transfers = 0u64;
		for transfer in TransferOutById::<T>::iter_values() {
			Pallet::<T>::track_transfer_out_quote(&transfer)
				.expect("distinct pending quotes obey the existing transfer bound");
			transfers.saturating_accrue(1);
		}
		T::DbWeight::get().reads_writes(transfers.saturating_mul(2).saturating_add(1), transfers)
	}

	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<Vec<u8>, sp_runtime::TryRuntimeError> {
		Ok(TransferOutById::<T>::iter_values()
			.map(|transfer| (transfer.destination_chain, transfer.microgons_per_argonot))
			.collect::<Vec<_>>()
			.encode())
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade(state: Vec<u8>) -> Result<(), sp_runtime::TryRuntimeError> {
		let quotes = Vec::<(SourceChain, T::Balance)>::decode_all(&mut &state[..])
			.map_err(|_| "Invalid pending-quote preflight")?;
		let mut indexed = 0u64;
		for (chain, counts) in PendingTransferOutQuoteCountsByDestinationChain::<T>::iter() {
			for (rate, count) in counts {
				let expected = quotes
					.iter()
					.filter(|(source, quote)| *source == chain && *quote == rate)
					.count() as u32;
				ensure!(count == expected, "Incorrect migrated pending-quote count");
				indexed.saturating_accrue(u64::from(count));
			}
		}
		ensure!(indexed == quotes.len() as u64, "Pending quote missing from migration");
		Ok(())
	}
}

pub type InitializePendingQuoteCountsMigration<T> = VersionedMigration<
	4,
	5,
	InitializePendingQuoteCounts<T>,
	Pallet<T>,
	<T as frame_system::Config>::DbWeight,
>;
