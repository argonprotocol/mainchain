#[cfg(test)]
mod bitcoin;
#[cfg(test)]
mod finality;
#[cfg(test)]
mod localchain_transfer;
#[cfg(test)]
mod notary;
#[cfg(test)]
mod sync;
#[cfg(test)]
mod vote_mining;

#[cfg(test)]
pub(crate) mod utils {
	use anyhow::Context;
	use argon_client::{
		FetchAt, MainchainClient, TxInBlockWithEvents, api,
		api::{
			runtime_types::{
				argon_primitives::block_seal,
				argon_runtime::{RuntimeCall, SessionKeys},
				bounded_collections::bounded_vec::BoundedVec,
				sp_consensus_grandpa as grandpa,
			},
			storage,
			sudo::calls::types,
			tx,
		},
		signer::{Signer, Sr25519Signer},
	};
	use argon_primitives::{BLOCK_SEAL_KEY_TYPE, prelude::*};
	use argon_testing::{ArgonTestNode, ArgonTestNotary};
	use sp_core::{DeriveJunction, Pair, crypto::key_types::GRANDPA};
	use sp_keyring::{Sr25519Keyring, Sr25519Keyring::Alice};
	use tokio::join;

	#[allow(dead_code)]
	pub(crate) async fn transfer_mainchain(
		test_node: &ArgonTestNode,
		from: &Sr25519Signer,
		to: AccountId,
		amount: Balance,
		wait_for_finalized: bool,
	) -> anyhow::Result<TxInBlockWithEvents> {
		let to_account_id = test_node.client.api_account(&to);
		let params = test_node.client.params_with_best_nonce(&from.account_id()).await?.build();
		test_node
			.client
			.submit_tx(
				&tx().balances().transfer_keep_alive(to_account_id.into(), amount),
				from,
				Some(params),
				wait_for_finalized,
			)
			.await
	}

	pub(crate) async fn activate_notary(
		test_node: &ArgonTestNode,
		test_notary: &ArgonTestNotary,
	) -> anyhow::Result<()> {
		println!("Registering a notary operator");
		test_notary.register_operator(test_node).await?;

		println!("Sudo approving notary");
		let operator_account = test_node.client.api_account(&test_notary.operator.public().into());
		sudo(
			test_node,
			RuntimeCall::Notaries(
				argon_client::api::runtime_types::pallet_notaries::pallet::Call::activate {
					operator_account,
				},
			),
			true,
		)
		.await?;
		println!("Sudo approved notary");
		Ok(())
	}

	pub(crate) async fn create_active_notary(
		test_node: &ArgonTestNode,
	) -> anyhow::Result<ArgonTestNotary> {
		let test_notary = ArgonTestNotary::start(test_node).await?;
		activate_notary(test_node, &test_notary).await?;

		Ok(test_notary)
	}

	pub(crate) async fn create_active_notary_with_archive_bucket(
		test_node: &ArgonTestNode,
		archive_bucket: String,
	) -> anyhow::Result<ArgonTestNotary> {
		let test_notary =
			ArgonTestNotary::start_with_archive(test_node, archive_bucket, None, None, true)
				.await?;
		activate_notary(test_node, &test_notary).await?;

		Ok(test_notary)
	}

	pub(crate) async fn sudo(
		test_node: &ArgonTestNode,
		call: types::sudo::Call,
		wait_for_finalized: bool,
	) -> anyhow::Result<TxInBlockWithEvents> {
		let from = Sr25519Signer::new(Alice.pair());
		let client = test_node.client.clone();
		let params = client.params_with_best_nonce(&from.account_id()).await?.build();
		test_node
			.client
			.submit_tx(&tx().sudo().sudo(call), &from, Some(params), wait_for_finalized)
			.await
	}

	pub(crate) async fn mining_slot_ownership_needed(
		test_node: &ArgonTestNode,
	) -> anyhow::Result<Balance> {
		Ok(test_node
			.client
			.fetch_storage(&storage().mining_slot().argonots_per_mining_seat(), FetchAt::Finalized)
			.await?
			.unwrap_or_default())
	}

	pub(crate) async fn force_set_ownership_balance(
		test_node: &ArgonTestNode,
		account_id: &AccountId,
		amount: Balance,
	) -> anyhow::Result<()> {
		let account = test_node.client.api_account(account_id);
		sudo(
			test_node,
			RuntimeCall::Ownership(
				api::runtime_types::pallet_balances::pallet::Call::force_set_balance {
					who: account.into(),
					new_free: amount,
				},
			),
			true,
		)
		.await?;
		Ok(())
	}

	pub(crate) async fn wait_for_finalized_catchup(
		source: &ArgonTestNode,
		node: &ArgonTestNode,
	) -> anyhow::Result<()> {
		let finalized = source.client.latest_finalized_block_hash().await?;
		let block_number = source.client.block_number(finalized.hash()).await?;
		let latest_node_finalized = node.client.latest_finalized_block_hash().await?;
		let latest_node_number = node.client.block_number(latest_node_finalized.hash()).await?;
		if latest_node_finalized.hash() == finalized.hash() || latest_node_number >= block_number {
			return Ok(());
		}

		let mut catchup_sub = node.client.live.blocks().subscribe_finalized().await?;
		while let Some(next) = catchup_sub.next().await {
			let next = next?;
			println!(
				"Waiting for node catchup to finalized block {:?}. At {:?}",
				block_number,
				next.header().number
			);
			if next.hash().as_ref() == finalized.hash().as_ref() || next.number() >= block_number {
				break;
			}
		}
		Ok(())
	}

	pub(crate) async fn activate_vote_mining(
		source: &ArgonTestNode,
		miner_1: &ArgonTestNode,
		miner_2: &ArgonTestNode,
	) -> anyhow::Result<()> {
		let ownership_needed = mining_slot_ownership_needed(source).await?;
		let seeded_ownership = ownership_needed.saturating_mul(2);
		force_set_ownership_balance(source, &miner_1.account_id, seeded_ownership).await?;
		force_set_ownership_balance(source, &miner_2.account_id, seeded_ownership).await?;
		wait_for_finalized_catchup(source, miner_1).await?;
		wait_for_finalized_catchup(source, miner_2).await?;

		let miner_1_keyring = miner_1.keyring();
		let miner_1_second_account = miner_1
			.keyring()
			.pair()
			.clone()
			.derive(vec![DeriveJunction::hard(1)].into_iter(), None)
			.unwrap()
			.0;
		let miner_1_second_signer = Sr25519Signer::new(miner_1_second_account);
		let miner_1_second_account = miner_1_second_signer.account_id();
		let miner_2_keyring = miner_2.keyring();

		let (keys1, keys_1_2, keys2) = join!(
			register_miner_keys(miner_1, miner_1_keyring, 1),
			register_miner_keys(miner_1, miner_1_keyring, 2),
			register_miner_keys(miner_2, miner_2_keyring, 1)
		);
		let keys1 = keys1.context("failed to register miner_1 primary session keys")?;
		let keys_1_2 = keys_1_2.context("failed to register miner_1 secondary session keys")?;
		let keys2 = keys2.context("failed to register miner_2 session keys")?;
		let source_signer = Sr25519Signer::new(Alice.pair());
		let source_nonce = source.client.get_account_nonce(&source_signer.account_id()).await?;
		register_miners(
			source,
			Sr25519Signer::new(Alice.pair()),
			vec![(miner_2.account_id.clone(), keys2)],
			Some(source_nonce),
		)
		.await
		.context("failed to register miner_2 bids")?;
		register_miners(
			source,
			Sr25519Signer::new(Alice.pair()),
			vec![(miner_1.account_id.clone(), keys1), (miner_1_second_account, keys_1_2)],
			Some(source_nonce + 1),
		)
		.await
		.context("failed to register miner_1 bids")?;
		Ok(())
	}

	pub(crate) async fn register_miner_keys(
		node: &ArgonTestNode,
		miner: Sr25519Keyring,
		counter: u16,
	) -> anyhow::Result<SessionKeys> {
		let grandpa_seed = format!("{}//grandpa//{counter}", miner.to_seed());
		let grandpa_public = node.insert_ed25519_keystore_key(GRANDPA, grandpa_seed).await?;
		let mining_seed = format!("{}//seal//{counter}", miner.to_seed());
		let seal_public =
			node.insert_ed25519_keystore_key(BLOCK_SEAL_KEY_TYPE, mining_seed).await?;
		Ok(SessionKeys {
			grandpa: grandpa::app::Public(grandpa_public),
			block_seal_authority: block_seal::app::Public(seal_public),
		})
	}

	pub(crate) async fn register_miners(
		node: &ArgonTestNode,
		funding: Sr25519Signer,
		miner_accounts: Vec<(AccountId, SessionKeys)>,
		nonce: Option<Nonce>,
	) -> anyhow::Result<()> {
		let client = node.client.clone();

		let first_account = client.api_account(&miner_accounts[0].0);
		let miner_count = miner_accounts.len() as u16;
		println!("Registering {miner_count} miners");
		let bids = miner_accounts
			.into_iter()
			.map(|(a, keys)| {
				let account = client.api_account(&a);
				RuntimeCall::MiningSlot(api::runtime_types::pallet_mining_slot::pallet::Call::bid {
					mining_account_id: Some(account),
					keys,
					bid: 0,
				})
			})
			.collect::<Vec<_>>();
		let params = if let Some(nonce) = nonce {
			MainchainClient::ext_params_builder().nonce(nonce.into()).build()
		} else {
			client.params_with_best_nonce(&funding.account_id()).await?.build()
		};

		let register = client
			.submit_tx(&tx().utility().batch_all(bids), &funding, Some(params), true)
			.await?;
		println!("miner registered. {register:?}");

		let wait_til_past_frame_id = client
			.fetch_storage(&storage().mining_slot().next_frame_id(), FetchAt::Best)
			.await?
			.unwrap_or_default();
		// wait for next cohort to start
		let lookup = storage().mining_slot().account_index_lookup(first_account);
		let mut block_sub = client.live.blocks().subscribe_best().await?;
		while let Some(Ok(block)) = block_sub.next().await {
			let fetch_at = FetchAt::Block(block.hash());
			let account_index = client.fetch_storage(&lookup, fetch_at).await?;
			if let Some((frame_id, index)) = account_index {
				println!("Miner 1 registered at frame {frame_id}, index {index}");
				break;
			}
			let registered_miners = client
				.fetch_storage(&storage().mining_slot().active_miners_count(), fetch_at)
				.await?
				.unwrap_or_default();
			let bids_for_next_cohort = client
				.fetch_storage(&storage().mining_slot().bids_for_next_slot_cohort(), fetch_at)
				.await?
				.unwrap_or(BoundedVec(vec![]));
			let next_frame_id = client
				.fetch_storage(&storage().mining_slot().next_frame_id(), fetch_at)
				.await?
				.unwrap_or_default();
			println!(
				"Waiting for cohort account to be registered. Currently registered {registered_miners}. Pending cohort: {:?}",
				bids_for_next_cohort
					.0
					.iter()
					.map(|a| a.account_id.to_address())
					.collect::<Vec<_>>()
			);
			let block_confirm = client.block_number(register.block_hash()).await;
			if block_confirm.is_err() {
				println!("Block no longer finalized! {block_confirm:?}");
			}
			if next_frame_id > wait_til_past_frame_id {
				panic!("next frameId changed while waiting for registration");
			}
		}
		let registered_miners = client
			.fetch_storage(&storage().mining_slot().active_miners_count(), FetchAt::Best)
			.await?
			.unwrap_or_default();
		println!("Registered miners: {miner_count} of {registered_miners}");
		assert!(registered_miners >= miner_count);
		Ok(())
	}
}
