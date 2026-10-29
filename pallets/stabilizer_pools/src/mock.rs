//! Shared setup for tests and runtime benchmarks; excluded from ordinary runtime builds.
//! Ethereum finality anchors are supplied here, while account/storage proofs use the real verifier.
#[cfg(test)]
use crate as pallet_stabilizer_pools;
use crate::*;
use alloy_primitives::{keccak256, B256, U256 as AlloyU256};
use alloy_trie::{
	proof::{ProofNodes, ProofRetainer},
	HashBuilder, Nibbles, TrieAccount,
};
use argon_primitives::{EthereumExecutionBlockProof, EthereumStorageSlotProof};
#[cfg(test)]
use frame_support::{assert_ok, derive_impl};
use pallet_ethereum_verifier::{types::ExecutionHeaderAnchor, ExecutionHeaderAnchors};
#[cfg(test)]
use pallet_prelude::{new_test_with_genesis, TestState};
#[cfg(test)]
use sp_runtime::{traits::IdentityLookup, AccountId32};

#[cfg(test)]
type Block = frame_system::mocking::MockBlock<Test>;

#[cfg(test)]
frame_support::construct_runtime!(pub enum Test {
	System: frame_system,
	EthereumVerifier: pallet_ethereum_verifier,
	StabilizerPools: pallet_stabilizer_pools,
});

#[cfg(test)]
#[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
impl frame_system::Config for Test {
	type Block = Block;
	type AccountId = AccountId32;
	type Lookup = IdentityLookup<Self::AccountId>;
}

#[cfg(test)]
impl pallet_ethereum_verifier::Config for Test {
	type FreeHeadersInterval = ConstU32<32>;
	type WeightInfo = ();
}

#[cfg(test)]
impl Config for Test {
	type WeightInfo = ();
}

#[cfg(test)]
pub fn new_test_ext() -> TestState {
	let mut ext = new_test_with_genesis::<Test>(|_| {});
	ext.execute_with(|| {
		System::set_block_number(1);
		assert_ok!(StabilizerPools::register_pool(RuntimeOrigin::root(), pool_manager(), config()));
	});
	ext
}

pub(crate) fn pool_manager() -> H160 {
	H160::repeat_byte(0x33)
}

pub(crate) fn config() -> PoolConfig {
	PoolConfig {
		position_manager: H160::repeat_byte(0x22),
		subscriber: H160::repeat_byte(0x11),
		key: PoolKey {
			currency0: H160::repeat_byte(0x44),
			currency1: H160::repeat_byte(0x55),
			fee: 500,
			tick_spacing: 10,
			hooks: H160::zero(),
		},
	}
}

pub(crate) fn pool_id() -> H256 {
	evm::pool_id(&config().key)
}

/// Synthetic Ethereum state, real account/storage tries and the real configured verifier.
/// Slot paths are also checked against deployed Uniswap bytecode in the Solidity package tests.
pub(crate) fn evidence<T: Config>(
	block: u64,
	revision: u64,
	beneficiary: H256,
	liquidity: u128,
	subscribed: bool,
) -> PositionProof {
	let pool_config = config();

	let token_id = U256::one();
	let subscription_slot = evm::mapping_slot(token_id, evm::SUBSCRIPTIONS_SLOT);

	// Pack Uniswap's pool prefix, signed tick bounds, and subscription flag.
	let mut position_info = pool_id().0;
	position_info[25..28].copy_from_slice(&100i32.to_be_bytes()[1..]);
	position_info[28..31].copy_from_slice(&(-100i32).to_be_bytes()[1..]);
	position_info[31] = u8::from(subscribed);

	let mut subscriber_value = [0u8; 32];
	if subscribed {
		subscriber_value[12..].copy_from_slice(pool_config.subscriber.as_bytes());
	}

	// Each account contributes its own storage trie to the shared Ethereum state.
	let slots = [
		vec![
			(subscription_slot, beneficiary),
			(
				evm::offset_slot(subscription_slot, 1),
				H256::from(U256::from(revision).to_big_endian()),
			),
		],
		vec![
			(
				evm::mapping_slot(token_id, evm::POSITION_MANAGER_SUBSCRIBER_SLOT),
				H256::from(subscriber_value),
			),
			(
				evm::mapping_slot(token_id, evm::POSITION_MANAGER_POSITION_INFO_SLOT),
				H256::from(position_info),
			),
		],
		vec![(
			evm::liquidity_slot(pool_id(), pool_config.position_manager, token_id, -100, 100),
			H256::from(U256::from(liquidity).to_big_endian()),
		)],
	];

	let addresses = [pool_config.subscriber, pool_config.position_manager, pool_manager()];
	let storage_tries = slots
		.iter()
		.map(|entries| {
			// Ethereum omits zero storage values; their witnesses prove absence.
			let encoded = entries
				.iter()
				.filter(|(_, value)| *value != H256::zero())
				.map(|(slot, value)| {
					(
						slot.as_bytes().to_vec(),
						alloy_rlp::encode(AlloyU256::from_be_slice(value.as_bytes())),
					)
				})
				.collect::<Vec<_>>();
			trie(&encoded)
		})
		.collect::<Vec<_>>();

	let encoded_accounts = addresses
		.iter()
		.zip(&storage_tries)
		.map(|(address, (root, _))| {
			(
				address.as_bytes().to_vec(),
				alloy_rlp::encode(TrieAccount {
					nonce: 1,
					storage_root: *root,
					..Default::default()
				}),
			)
		})
		.collect::<Vec<_>>();

	let (state_root, account_nodes) = trie(&encoded_accounts);

	// Supply finality at the external-chain boundary; trie verification remains real.
	let anchor = ExecutionHeaderAnchor {
		block_hash: H256::from(U256::from(block).to_big_endian()),
		block_number: block,
		timestamp_millis: block * 12_000,
		parent_hash: H256::zero(),
		state_root: H256::from(state_root.0),
		receipts_root: H256::zero(),
	};
	ExecutionHeaderAnchors::<T>::insert(anchor.block_hash, anchor);

	PositionProof {
		subscriber: account_storage_proof(
			addresses[0],
			&account_nodes,
			&storage_tries[0].1,
			&slots[0],
			block,
		),
		position_manager: account_storage_proof(
			addresses[1],
			&account_nodes,
			&storage_tries[1].1,
			&slots[1],
			block,
		),
		pool_manager: account_storage_proof(
			addresses[2],
			&account_nodes,
			&storage_tries[2].1,
			&slots[2],
			block,
		),
	}
}

fn trie(entries: &[(Vec<u8>, Vec<u8>)]) -> (B256, ProofNodes) {
	let mut entries = entries
		.iter()
		.map(|(key, value)| (Nibbles::unpack(keccak256(key)), value))
		.collect::<Vec<_>>();
	entries.sort_by_key(|(key, _)| *key);

	let mut trie = HashBuilder::default()
		.with_proof_retainer(ProofRetainer::new(entries.iter().map(|(key, _)| *key).collect()));
	for (key, value) in entries {
		trie.add_leaf(key, value);
	}

	let root = trie.root();
	(root, trie.take_proof_nodes())
}

fn account_storage_proof<MaxSlots: Get<u32>>(
	address: H160,
	account_nodes: &ProofNodes,
	storage_nodes: &ProofNodes,
	slots: &[(H256, H256)],
	block: u64,
) -> EthereumAccountStorageProof<MaxSlots> {
	let account_key = Nibbles::unpack(keccak256(address));
	let account_proof = account_nodes
		.matching_nodes_sorted(&account_key)
		.into_iter()
		.map(|(_, node)| node.to_vec().try_into().unwrap())
		.collect::<Vec<_>>()
		.try_into()
		.unwrap();

	let sorted_nodes = storage_nodes.nodes_sorted();
	let storage_proof = sorted_nodes
		.iter()
		.map(|(_, node)| node.to_vec().try_into().unwrap())
		.collect::<Vec<_>>()
		.try_into()
		.unwrap();

	let slots = slots
		.iter()
		.map(|(slot, value)| {
			let storage_key = Nibbles::unpack(keccak256(slot));
			let node_indexes = sorted_nodes
				.iter()
				.enumerate()
				.filter(|(_, (path, _))| storage_key.starts_with(path))
				.map(|(index, _)| index as u16)
				.collect::<Vec<_>>()
				.try_into()
				.unwrap();

			EthereumStorageSlotProof { slot: *slot, value: *value, node_indexes }
		})
		.collect::<Vec<_>>()
		.try_into()
		.unwrap();

	EthereumAccountStorageProof {
		execution_block_proof: EthereumExecutionBlockProof {
			anchor_block_hash: H256::from(U256::from(block).to_big_endian()),
			target_to_anchor_header_chain: Default::default(),
		},
		account_proof,
		storage_proof,
		slots,
	}
}
