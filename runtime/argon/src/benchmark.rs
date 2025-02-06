use super::*;

frame_benchmarking::define_benchmarks!(
	[frame_benchmarking, BaselineBench::<Runtime>]
	[frame_system, SystemBench::<Runtime>]
	[pallet_balances, ArgonTokens]
	[pallet_balances, OwnershipTokens]
	[pallet_timestamp, Timestamp]
	[pallet_ticks, Ticks]
	[pallet_domains, Domain]
	[pallet_block_seal_spec, VoteEligibility]
	[pallet_block_rewards, BlockRewards]
	[pallet_mining_slot, MiningSlot]
	[pallet_bitcoin_locks, BitcoinLocks]
	[pallet_bitcoin_utxos, BitcoinMint]
	[pallet_mint, Mint]
	[pallet_block_seal, BlockSeal]
	[pallet_authorship, Authorship]
	[pallet_sudo, Sudo]
	[pallet_notaries, Notaries]
	[pallet_chain_transfer, ChainTransfer]
);
