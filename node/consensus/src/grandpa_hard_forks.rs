use hex_literal::hex;
use polkadot_sdk::*;
use sc_consensus_grandpa::AuthoritySetHardFork;
use sp_consensus_grandpa::{AuthorityList, SetId};
use sp_runtime::traits::{Block as BlockT, NumberFor};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct GrandpaHardForkBlock {
	hash: [u8; 32],
	number: u32,
	set_id: SetId,
}

pub fn authority_set_hard_forks<B>(
	chain_id: &str,
	genesis_authorities: &AuthorityList,
) -> Vec<AuthoritySetHardFork<B>>
where
	B: BlockT,
	B::Hash: From<[u8; 32]>,
	NumberFor<B>: From<u32>,
{
	match chain_id {
		"argon" => mainnet_hard_forks(genesis_authorities),
		"argon-testnet" => testnet_hard_forks(genesis_authorities),
		_ => Vec::new(),
	}
}

pub fn is_known_grandpa_hard_fork_block(hash: &[u8], number: u32) -> bool {
	MAINNET_HARD_FORK_BLOCKS
		.iter()
		.chain(TESTNET_HARD_FORK_BLOCKS.iter())
		.any(|block| block.number == number && block.hash.as_slice() == hash)
}

pub fn is_known_grandpa_authority_change_block(hash: &[u8], number: u32) -> bool {
	MAINNET_AUTHORITY_CHANGE_BLOCKS
		.iter()
		.chain(TESTNET_AUTHORITY_CHANGE_BLOCKS.iter())
		.any(|block| block.number == number && block.hash.as_slice() == hash)
}

fn mainnet_hard_forks<B>(genesis_authorities: &AuthorityList) -> Vec<AuthoritySetHardFork<B>>
where
	B: BlockT,
	B::Hash: From<[u8; 32]>,
	NumberFor<B>: From<u32>,
{
	// These historical fixes were set-id bumps without a real authority rotation.
	MAINNET_HARD_FORK_BLOCKS
		.iter()
		.map(|block| hard_fork(*block, genesis_authorities))
		.collect()
}

fn testnet_hard_forks<B>(genesis_authorities: &AuthorityList) -> Vec<AuthoritySetHardFork<B>>
where
	B: BlockT,
	B::Hash: From<[u8; 32]>,
	NumberFor<B>: From<u32>,
{
	// The old runtime state patch exposed the new set ids starting on the block *after*
	// each handoff. GRANDPA hard forks apply on the actual handoff block instead.
	TESTNET_HARD_FORK_BLOCKS
		.iter()
		.map(|block| hard_fork(*block, genesis_authorities))
		.collect()
}

fn hard_fork<B>(block: GrandpaHardForkBlock, authorities: &AuthorityList) -> AuthoritySetHardFork<B>
where
	B: BlockT,
	B::Hash: From<[u8; 32]>,
	NumberFor<B>: From<u32>,
{
	AuthoritySetHardFork {
		block: (B::Hash::from(block.hash), block.number.into()),
		set_id: block.set_id,
		authorities: authorities.clone(),
		last_finalized: Some(block.number.saturating_sub(1).into()),
	}
}

const MAINNET_HARD_FORK_BLOCKS: &[GrandpaHardForkBlock] = &[GrandpaHardForkBlock {
	hash: hex!("7927b62bef2a417d0affc650f9a3cd2e3ef69a27cbd7ba14691774b0ea2cd712"),
	number: 17_573,
	set_id: 1,
}];

const MAINNET_AUTHORITY_CHANGE_BLOCKS: &[GrandpaHardForkBlock] = &[GrandpaHardForkBlock {
	hash: hex!("f3f9fb2a75a34d87a78984decf2a0432dfab8e08f75cd42cdc7f1c4fbb8a568d"),
	number: 17_572,
	set_id: 1,
}];

const TESTNET_HARD_FORK_BLOCKS: &[GrandpaHardForkBlock] = &[
	GrandpaHardForkBlock {
		hash: hex!("36ca332782b5f28798135a6ba266c301b5df1f521caba2be0708cb337295228d"),
		number: 30_270,
		set_id: 1,
	},
	GrandpaHardForkBlock {
		hash: hex!("abca5db970894dfc5d7ae04b1575512fdc57f8e5fbb9c3131a1eb64338300a08"),
		number: 34_561,
		set_id: 2,
	},
	GrandpaHardForkBlock {
		hash: hex!("664e696211cb8368df3e9313c467bc81d206e2f97f9acb91c93e0b2a42231fa1"),
		number: 38_880,
		set_id: 3,
	},
	GrandpaHardForkBlock {
		hash: hex!("34d4b371868027be46cb9437b12b5d125d0b739f422f41c96fcb5a3cf23692e3"),
		number: 40_320,
		set_id: 4,
	},
	GrandpaHardForkBlock {
		hash: hex!("06e968f03068bf9c62da2b83279bd4b60f58f95b4b3d8222d260a56cc46b0c85"),
		number: 41_760,
		set_id: 5,
	},
	GrandpaHardForkBlock {
		hash: hex!("35400f2b37a85d6fe3e9841e615236ea9306a974549f5b077bf537cb88e9066e"),
		number: 43_200,
		set_id: 6,
	},
	GrandpaHardForkBlock {
		hash: hex!("b3239c8d41ce46e1c08bd12c167d0ac6ebef02dd2bd4748c0dc65f57e8515a43"),
		number: 44_640,
		set_id: 7,
	},
	GrandpaHardForkBlock {
		hash: hex!("9b677b87ab329807af73f676e702a61c11fa7544a9d81dd6f038d17a89fc9a21"),
		number: 46_080,
		set_id: 8,
	},
	GrandpaHardForkBlock {
		hash: hex!("856a44fdd6d656eecfc4d186aeecbf64a9de7f2cbf79f9d57f68c317c718e4a1"),
		number: 47_520,
		set_id: 9,
	},
	GrandpaHardForkBlock {
		hash: hex!("c356f29a4b348fb4e6d3211adb32a96094df2433229cf22a92e9f490689205da"),
		number: 48_960,
		set_id: 10,
	},
	GrandpaHardForkBlock {
		hash: hex!("14dc783ab6ebb6acad7a318b0424155bec4fd2fc6dd42551246d3003f5adcebb"),
		number: 50_400,
		set_id: 11,
	},
	GrandpaHardForkBlock {
		hash: hex!("547e6ad212585c2d3e4057f8e2e0b6a3d28201b2b904aaf1ddaf3be1b33b23eb"),
		number: 51_840,
		set_id: 12,
	},
	GrandpaHardForkBlock {
		hash: hex!("55f3792183906c2acc201612bafbdfebf8441c1829500a492bd5a0e361f3407c"),
		number: 53_280,
		set_id: 13,
	},
];

const TESTNET_AUTHORITY_CHANGE_BLOCKS: &[GrandpaHardForkBlock] = &[
	GrandpaHardForkBlock {
		hash: hex!("1f1f857295b01455051c70c2e3f8c31aa9bead6f8384d26f94b5555d6f3aa62c"),
		number: 30_269,
		set_id: 1,
	},
	GrandpaHardForkBlock {
		hash: hex!("9f5fbb1480503a1dec9b0c71b500220524cb15c4fa70a91fe39a2526acf58606"),
		number: 34_560,
		set_id: 2,
	},
	GrandpaHardForkBlock {
		hash: hex!("3c6f31698c1fc9104270a12f1bffbebfa35c68095fcaf224f0829afb52aa44c2"),
		number: 38_879,
		set_id: 3,
	},
	GrandpaHardForkBlock {
		hash: hex!("56b3b5431bebd6f51a18c609e482887a25bee5f4f6b2ba3cf86f20e2ee86d1e2"),
		number: 40_319,
		set_id: 4,
	},
	GrandpaHardForkBlock {
		hash: hex!("77fe4a7eb752197c9526f3497dc383835aa81385b6073ec7e341cb7fbc03b788"),
		number: 41_759,
		set_id: 5,
	},
	GrandpaHardForkBlock {
		hash: hex!("03df5ce96ed574eb93cf7f2f9d14a46428cd8ca74cc330fd03f84d19b393a232"),
		number: 43_199,
		set_id: 6,
	},
	GrandpaHardForkBlock {
		hash: hex!("636631dc471531eb82ceea84e9b5cf15b8e844355bb4edfa4a17eeba06598494"),
		number: 44_639,
		set_id: 7,
	},
	GrandpaHardForkBlock {
		hash: hex!("32504ce3f81a76e18f356e778d3f52d59d3fd996f182683c095a1a57744490a8"),
		number: 46_079,
		set_id: 8,
	},
	GrandpaHardForkBlock {
		hash: hex!("52e907d8b9a925a19de2c7c2eb42f72991c2a0a617e45f4b43b7ae1dbc567029"),
		number: 47_519,
		set_id: 9,
	},
	GrandpaHardForkBlock {
		hash: hex!("b60e5028ae812174509f9f149ad70244613a9ea64031721109b4ac6863c2cf13"),
		number: 48_959,
		set_id: 10,
	},
	GrandpaHardForkBlock {
		hash: hex!("886f812c2882c1ea9a236283efefb5ec112d0a74da9676903c6e2b6f6799da7b"),
		number: 50_399,
		set_id: 11,
	},
	GrandpaHardForkBlock {
		hash: hex!("4c12a91257bcab7dfb38b58b68df72c9f06873b0880f104ff4abc7a418b68d54"),
		number: 51_839,
		set_id: 12,
	},
	GrandpaHardForkBlock {
		hash: hex!("5a9ab83bc1f7408106421b904e4a24791d4261c8d1c5343b562f0f73f46bf011"),
		number: 53_279,
		set_id: 13,
	},
];

#[cfg(test)]
mod tests {
	use super::{
		AuthorityList, authority_set_hard_forks, is_known_grandpa_authority_change_block,
		is_known_grandpa_hard_fork_block,
	};
	use argon_primitives::BlockNumber;
	use hex_literal::hex;
	use polkadot_sdk::{
		sp_consensus_grandpa::AuthorityId,
		sp_core::ed25519,
		sp_runtime::{OpaqueExtrinsic, generic, traits::BlakeTwo256},
	};

	type Header = generic::Header<BlockNumber, BlakeTwo256>;
	type Block = generic::Block<Header, OpaqueExtrinsic>;

	fn test_authorities() -> AuthorityList {
		vec![(AuthorityId::from(ed25519::Public::from_raw([1u8; 32])), 1)]
	}

	#[test]
	fn returns_mainnet_hard_fork_at_effective_block() {
		let hard_forks = authority_set_hard_forks::<Block>("argon", &test_authorities());
		assert_eq!(hard_forks.len(), 1);
		assert_eq!(hard_forks[0].block.1, 17_573);
		assert_eq!(hard_forks[0].set_id, 1);
		assert_eq!(hard_forks[0].last_finalized, Some(17_572));
	}

	#[test]
	fn returns_testnet_hard_forks_for_all_historical_bumps() {
		let hard_forks = authority_set_hard_forks::<Block>("argon-testnet", &test_authorities());
		assert_eq!(hard_forks.len(), 13);
		assert_eq!(hard_forks.first().unwrap().block.1, 30_270);
		assert_eq!(hard_forks.first().unwrap().last_finalized, Some(30_269));
		assert_eq!(hard_forks.last().unwrap().block.1, 53_280);
		assert_eq!(hard_forks.last().unwrap().set_id, 13);
		assert_eq!(hard_forks.last().unwrap().last_finalized, Some(53_279));
	}

	#[test]
	fn separates_hard_fork_effective_blocks_from_authority_change_import_blocks() {
		assert!(is_known_grandpa_hard_fork_block(
			&hex!("36ca332782b5f28798135a6ba266c301b5df1f521caba2be0708cb337295228d"),
			30_270,
		));
		assert!(is_known_grandpa_authority_change_block(
			&hex!("1f1f857295b01455051c70c2e3f8c31aa9bead6f8384d26f94b5555d6f3aa62c"),
			30_269,
		));
		assert!(!is_known_grandpa_authority_change_block(
			&hex!("36ca332782b5f28798135a6ba266c301b5df1f521caba2be0708cb337295228d"),
			30_270,
		));
	}
}
