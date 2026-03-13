use crate::runtime_api::opaque::Block;
use hex_literal::hex;
use polkadot_sdk::*;
use sc_consensus_grandpa::AuthoritySetHardFork;
use sp_consensus_grandpa::{AuthorityList, SetId};
use sp_core::H256;

pub(crate) fn authority_set_hard_forks(
	chain_id: &str,
	genesis_authorities: &AuthorityList,
) -> Vec<AuthoritySetHardFork<Block>> {
	match chain_id {
		"argon" => mainnet_hard_forks(genesis_authorities),
		"argon-testnet" => testnet_hard_forks(genesis_authorities),
		_ => Vec::new(),
	}
}

fn mainnet_hard_forks(genesis_authorities: &AuthorityList) -> Vec<AuthoritySetHardFork<Block>> {
	// These historical fixes were set-id bumps without a real authority rotation.
	vec![hard_fork(
		hex!("7927b62bef2a417d0affc650f9a3cd2e3ef69a27cbd7ba14691774b0ea2cd712"),
		17_573,
		1,
		genesis_authorities,
	)]
}

fn testnet_hard_forks(genesis_authorities: &AuthorityList) -> Vec<AuthoritySetHardFork<Block>> {
	// The old runtime state patch exposed the new set ids starting on the block *after*
	// each handoff. GRANDPA hard forks apply on the actual handoff block instead.
	vec![
		hard_fork(
			hex!("36ca332782b5f28798135a6ba266c301b5df1f521caba2be0708cb337295228d"),
			30_270,
			1,
			genesis_authorities,
		),
		hard_fork(
			hex!("abca5db970894dfc5d7ae04b1575512fdc57f8e5fbb9c3131a1eb64338300a08"),
			34_561,
			2,
			genesis_authorities,
		),
		hard_fork(
			hex!("664e696211cb8368df3e9313c467bc81d206e2f97f9acb91c93e0b2a42231fa1"),
			38_880,
			3,
			genesis_authorities,
		),
		hard_fork(
			hex!("34d4b371868027be46cb9437b12b5d125d0b739f422f41c96fcb5a3cf23692e3"),
			40_320,
			4,
			genesis_authorities,
		),
		hard_fork(
			hex!("06e968f03068bf9c62da2b83279bd4b60f58f95b4b3d8222d260a56cc46b0c85"),
			41_760,
			5,
			genesis_authorities,
		),
		hard_fork(
			hex!("35400f2b37a85d6fe3e9841e615236ea9306a974549f5b077bf537cb88e9066e"),
			43_200,
			6,
			genesis_authorities,
		),
		hard_fork(
			hex!("b3239c8d41ce46e1c08bd12c167d0ac6ebef02dd2bd4748c0dc65f57e8515a43"),
			44_640,
			7,
			genesis_authorities,
		),
		hard_fork(
			hex!("9b677b87ab329807af73f676e702a61c11fa7544a9d81dd6f038d17a89fc9a21"),
			46_080,
			8,
			genesis_authorities,
		),
		hard_fork(
			hex!("856a44fdd6d656eecfc4d186aeecbf64a9de7f2cbf79f9d57f68c317c718e4a1"),
			47_520,
			9,
			genesis_authorities,
		),
		hard_fork(
			hex!("c356f29a4b348fb4e6d3211adb32a96094df2433229cf22a92e9f490689205da"),
			48_960,
			10,
			genesis_authorities,
		),
		hard_fork(
			hex!("14dc783ab6ebb6acad7a318b0424155bec4fd2fc6dd42551246d3003f5adcebb"),
			50_400,
			11,
			genesis_authorities,
		),
		hard_fork(
			hex!("547e6ad212585c2d3e4057f8e2e0b6a3d28201b2b904aaf1ddaf3be1b33b23eb"),
			51_840,
			12,
			genesis_authorities,
		),
		hard_fork(
			hex!("55f3792183906c2acc201612bafbdfebf8441c1829500a492bd5a0e361f3407c"),
			53_280,
			13,
			genesis_authorities,
		),
	]
}

fn hard_fork(
	hash: [u8; 32],
	number: u32,
	set_id: SetId,
	authorities: &AuthorityList,
) -> AuthoritySetHardFork<Block> {
	AuthoritySetHardFork {
		block: (H256::from(hash), number),
		set_id,
		authorities: authorities.clone(),
		last_finalized: None,
	}
}

#[cfg(test)]
mod tests {
	use super::{AuthorityList, authority_set_hard_forks};
	use polkadot_sdk::{sp_consensus_grandpa::AuthorityId, sp_core::ed25519};

	fn test_authorities() -> AuthorityList {
		vec![(AuthorityId::from(ed25519::Public::from_raw([1u8; 32])), 1)]
	}

	#[test]
	fn returns_mainnet_hard_fork_at_effective_block() {
		let hard_forks = authority_set_hard_forks("argon", &test_authorities());
		assert_eq!(hard_forks.len(), 1);
		assert_eq!(hard_forks[0].block.1, 17_573);
		assert_eq!(hard_forks[0].set_id, 1);
	}

	#[test]
	fn returns_testnet_hard_forks_for_all_historical_bumps() {
		let hard_forks = authority_set_hard_forks("argon-testnet", &test_authorities());
		assert_eq!(hard_forks.len(), 13);
		assert_eq!(hard_forks.first().unwrap().block.1, 30_270);
		assert_eq!(hard_forks.last().unwrap().block.1, 53_280);
		assert_eq!(hard_forks.last().unwrap().set_id, 13);
	}
}
