#[cfg(test)]
mod bitcoin;
#[cfg(test)]
mod localchain_transfer;

#[cfg(test)]
pub(crate) mod utils {
	use argon_client::{
		api::{
			runtime_types::{
				argon_primitives::{block_seal, block_seal::RewardDestination},
				argon_runtime::SessionKeys,
				sp_consensus_grandpa::app::Public,
			},
			storage, tx,
		},
		signer::{Signer, Sr25519Signer},
		MainchainClient,
	};
	use argon_primitives::{AccountId, Nonce};
	use argon_testing::ArgonTestNode;
	use sp_core::{ed25519, sr25519, Pair};

	#[allow(dead_code)]
	pub(crate) async fn register_miner(
		test_node: &ArgonTestNode,
		miner_mnemonic: String,
		sugar_daddy: &Sr25519Signer,
		nonce: Nonce,
	) -> anyhow::Result<()> {
		let miner_sr25519 = sr25519::Pair::from_string(&miner_mnemonic, None)?;
		let client = test_node.client.clone();
		let grandpa_key =
			ed25519::Pair::from_string(format!("{}//grandpa", miner_mnemonic).as_str(), None)?;
		let mining_key =
			ed25519::Pair::from_string(format!("{}//mining", miner_mnemonic).as_str(), None)?;

		// how much ownership is needed
		let ownership_needed = client
			.fetch_storage(&storage().mining_slot().ownership_bond_amount(), None)
			.await?
			.unwrap();
		println!("ownership needed {:?}", ownership_needed);
		// transfer from alice
		let params = MainchainClient::ext_params_builder().nonce(nonce.into()).build();
		println!("nonce {:?}", nonce);

		let account_id: AccountId = sugar_daddy.account_id();
		let sugar_daddy_account_id = client.api_account(&account_id);
		let alice_balance = client
			.fetch_storage(&storage().ownership().account(sugar_daddy_account_id.clone()), None)
			.await?;
		println!("alice balance {:?}", alice_balance);

		let miner_account_id = client.api_account(&miner_sr25519.public().into());

		let ownership_transfer = client
			.live
			.tx()
			.sign_and_submit_then_watch(
				&tx().ownership().transfer_allow_death(miner_account_id.into(), ownership_needed),
				sugar_daddy,
				params,
			)
			.await?
			.wait_for_finalized_success()
			.await?;
		println!("ownership transfer {:?}", ownership_transfer.extrinsic_hash());

		let register = client
			.live
			.tx()
			.sign_and_submit_then_watch_default(
				&tx().mining_slot().bid(
					None,
					RewardDestination::Owner,
					SessionKeys {
						grandpa: Public(grandpa_key.public().0),
						block_seal_authority: block_seal::app::Public(mining_key.public().0),
					},
				),
				&Sr25519Signer::new(miner_sr25519.clone()),
			)
			.await?
			.wait_for_finalized_success()
			.await?;
		println!("miner registered {:?}", register);
		Ok(())
	}
}
