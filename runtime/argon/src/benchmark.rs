use super::*;

frame_benchmarking::define_benchmarks!(
	[frame_benchmarking, BaselineBench::<Runtime>]
	[frame_system, SystemBench::<Runtime>]
	[pallet_balances, Balances]
	[pallet_balances, Ownership]
	[pallet_timestamp, Timestamp]
	[pallet_inbound_transfer_log, InboundTransferLog]
	[pallet_mining_slot, MiningSlot]
	[pallet_block_seal_spec, BlockSealSpec]
	[pallet_block_seal, BlockSeal]
	[pallet_chain_transfer, ChainTransfer]
	[pallet_notebook, Notebook]
	[pallet_vaults, Vaults]
	[pallet_operational_accounts, OperationalAccounts]
);
