#[cfg(test)]
mod bitcoin;
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
	use argon_client::{
		api,
		api::{
			runtime_types::{
				argon_primitives::{block_seal, block_seal::RewardDestination},
				argon_runtime::{RuntimeCall, SessionKeys},
				sp_consensus_grandpa as grandpa,
			},
			storage,
			sudo::calls::types,
			tx,
		},
		signer::{Signer, Sr25519Signer},
		ArgonConfig, ArgonOnlineClient,
	};
	use argon_primitives::{prelude::*, BLOCK_SEAL_KEY_TYPE};
	use argon_testing::{ArgonTestNode, ArgonTestNotary};
	use sp_core::{crypto::key_types::GRANDPA, Pair};
	use sp_keyring::{AccountKeyring::Alice, Sr25519Keyring};
	use subxt::tx::TxInBlock;

	#[allow(dead_code)]
	pub(crate) async fn transfer_mainchain(
		test_node: &ArgonTestNode,
		from: &Sr25519Signer,
		to: AccountId,
		amount: Balance,
		wait_for_finalized: bool,
	) -> anyhow::Result<TxInBlock<ArgonConfig, ArgonOnlineClient>> {
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
		let test_notary = ArgonTestNotary::start_with_archive(test_node, archive_bucket).await?;
		activate_notary(test_node, &test_notary).await?;

		Ok(test_notary)
	}

	pub(crate) async fn sudo(
		test_node: &ArgonTestNode,
		call: types::sudo::Call,
	) -> anyhow::Result<TxInBlock<ArgonConfig, ArgonOnlineClient>> {
		let from = Sr25519Signer::new(Alice.pair());
		let client = test_node.client.clone();
		let params = client.params_with_best_nonce(&from.account_id()).await?.build();
		test_node
			.client
			.submit_tx(&tx().sudo().sudo(call), &from, Some(params), false)
			.await
	}

	pub(crate) async fn mining_slot_ownership_needed(
		test_node: &ArgonTestNode,
	) -> anyhow::Result<Balance> {
		Ok(test_node
			.client
			.fetch_storage(&storage().mining_slot().argonots_per_mining_seat(), None)
			.await?
			.unwrap_or_default())
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
	) -> anyhow::Result<()> {
		let client = node.client.clone();

		println!("Registering miners");
		let bids = miner_accounts
			.into_iter()
			.map(|(a, keys)| {
				let account = client.api_account(&a);
				RuntimeCall::MiningSlot(api::runtime_types::pallet_mining_slot::pallet::Call::bid {
					mining_account_id: Some(account),
					keys,
					bid: 0,
					reward_destination: RewardDestination::Owner,
				})
			})
			.collect::<Vec<_>>();
		let params = client.params_with_best_nonce(&funding.account_id()).await?.build();

		let register = client
			.submit_tx(&tx().utility().batch_all(bids), &funding, Some(params), true)
			.await?;
		println!(
			"miner registered. ext hash: {:?}, block {:?}",
			register.extrinsic_hash(),
			register.block_hash()
		);
		Ok(())
	}
}
