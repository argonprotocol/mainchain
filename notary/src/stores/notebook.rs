use std::{cmp::max, collections::BTreeMap};

use binary_merkle_tree::{merkle_proof, merkle_root, verify_proof, Leaf};
use codec::{Decode, Encode};
use scale_info::TypeInfo;
use serde::{Deserialize, Serialize};
use serde_json::{from_value, json};
use sp_core::{bounded::BoundedVec, Blake2Hasher, RuntimeDebug};

use ulx_notary_primitives::{
	ensure, note::Chain, AccountId, AccountOrigin, BalanceProof, BalanceTip, MaxBalanceChanges,
	NotaryId, NotebookAccountOrigin, NotebookNumber, PINNED_BLOCKS_OFFSET,
};

use crate::{
	stores::{
		balance_change::BalanceChangeStore, block_meta::BlockMetaStore,
		chain_transfer::ChainTransferStore, notebook_header::NotebookHeaderStore,
		notebook_new_accounts::NotebookNewAccountsStore, BoxFutureResult,
	},
	Error,
};

pub struct NotebookStore;
impl NotebookStore {
	/// Get proofs for a set of balance tips. This fn should retrieve from the database, not
	/// calculate.
	pub fn get_balance_proof<'a>(
		pool: &'a sqlx::PgPool,
		notary_id: NotaryId,
		notebook_number: NotebookNumber,
		balance_tip: &'a BalanceTip,
	) -> BoxFutureResult<'a, BalanceProof> {
		Box::pin(async move {
			let rows = sqlx::query!(
				"SELECT change_merkle_leafs FROM notebooks WHERE notebook_number = $1 LIMIT 1",
				notebook_number as i32
			)
			.fetch_one(pool)
			.await?;

			let merkle_leafs = rows.change_merkle_leafs;

			let record = balance_tip.encode();

			let index = merkle_leafs
				.iter()
				.position(|x| *x == record)
				.ok_or(Error::InvalidBalanceProofRequested)?;

			let proof = merkle_proof::<Blake2Hasher, _, _>(&merkle_leafs, index);

			Ok(BalanceProof {
				notary_id,
				notebook_number,
				account_origin: balance_tip.account_origin.clone(),
				proof: BoundedVec::truncate_from(
					proof.proof.into_iter().map(|p| p.into()).collect(),
				),
				leaf_index: proof.leaf_index as u32,
				number_of_leaves: proof.number_of_leaves as u32,
			})
		})
	}

	pub fn get_account_origins<'a>(
		db: impl sqlx::PgExecutor<'a> + 'a,
		notebook_number: NotebookNumber,
	) -> BoxFutureResult<'a, BoundedVec<NotebookAccountOrigin, MaxBalanceChanges>> {
		Box::pin(async move {
			let rows = sqlx::query!(
				"SELECT new_account_origins FROM notebooks WHERE notebook_number = $1 LIMIT 1",
				notebook_number as i32
			)
			.fetch_one(db)
			.await?;

			let new_account_origins: Vec<NotebookAccountOrigin> =
				from_value(rows.new_account_origins)?;

			Ok(BoundedVec::truncate_from(new_account_origins))
		})
	}

	pub async fn is_valid_proof<'a>(
		db: impl sqlx::PgExecutor<'a> + 'a,
		balance_tip: &'a BalanceTip,
		balance_proof: &'a BalanceProof,
	) -> anyhow::Result<bool, Error> {
		let root =
			NotebookHeaderStore::get_changed_accounts_root(db, balance_proof.notebook_number)
				.await?;

		let is_valid = verify_proof::<Blake2Hasher, _, _>(
			&root,
			balance_proof.proof.clone().into_inner(),
			balance_proof.number_of_leaves as usize,
			balance_proof.leaf_index as usize,
			Leaf::Value(&balance_tip.encode()),
		);

		Ok(is_valid)
	}

	pub async fn close_notebook(
		db: &mut sqlx::PgConnection,
		notebook_number: NotebookNumber,
	) -> anyhow::Result<(), Error> {
		let meta = BlockMetaStore::load(&mut *db).await?;
		let mut pinned_to_block_number =
			meta.best_block_number.saturating_sub(PINNED_BLOCKS_OFFSET);

		pinned_to_block_number = max(pinned_to_block_number, meta.finalized_block_number);

		if notebook_number > 1 {
			let previous_pin =
				NotebookHeaderStore::get_pinned_block_number(&mut *db, notebook_number - 1).await?;
			if pinned_to_block_number < previous_pin {
				pinned_to_block_number = previous_pin;
			}
		}

		let changesets = BalanceChangeStore::get_for_notebook(&mut *db, notebook_number).await?;

		let mut changed_accounts =
			BTreeMap::<(AccountId, Chain), (u32, u128, AccountOrigin)>::new();
		let new_account_origins =
			NotebookNewAccountsStore::take_notebook_origins(&mut *db, notebook_number).await?;

		let new_account_origin_map =
			BTreeMap::from_iter(new_account_origins.iter().map(|origin| {
				(
					(origin.account_id.clone(), origin.chain.clone()),
					AccountOrigin { notebook_number, account_uid: origin.account_uid },
				)
			}));

		for change in changesets {
			for change in change {
				let key = (change.account_id, change.chain);
				let origin = change
					.previous_balance_proof
					.map(|a| a.account_origin)
					.or_else(|| new_account_origin_map.get(&key).cloned())
					.ok_or(|| {
						Error::InternalError(format!(
							"Could not find origin for account {} {:?}",
							key.0, key.1
						))
					})
					.map_err(|e| Error::InternalError(e().to_string()))?;

				if !changed_accounts.contains_key(&key) ||
					changed_accounts.get(&key).is_some_and(|a| a.0 < change.nonce)
				{
					changed_accounts.insert(key.clone(), (change.nonce, change.balance, origin));
				}
			}
		}

		let mut account_changelist = vec![];
		let merkle_leafs = changed_accounts
			.into_iter()
			.map(|((account_id, chain), (nonce, balance, account_origin))| {
				account_changelist.push(account_origin.clone());
				BalanceTip { account_id, chain, nonce, balance, account_origin }.encode()
			})
			.collect::<Vec<_>>();

		let changes_root = merkle_root::<Blake2Hasher, _>(&merkle_leafs);
		let transfers = ChainTransferStore::take_for_notebook(&mut *db, notebook_number).await?;

		NotebookHeaderStore::complete_notebook(
			&mut *db,
			notebook_number,
			transfers,
			pinned_to_block_number,
			changes_root,
			account_changelist,
		)
		.await?;

		let notebook_origins = new_account_origins
			.iter()
			.map(|a| (a.account_id.clone(), a.chain.clone(), a.account_uid.clone()))
			.collect::<Vec<_>>();

		let origins_json = json!(notebook_origins);
		let res = sqlx::query!(
			r#"
				INSERT INTO notebooks (notebook_number, change_merkle_leafs, new_account_origins) VALUES ($1, $2, $3)
			"#,
			notebook_number as i32,
			merkle_leafs.as_slice(),
			origins_json,
		)
		.execute(db)
		.await?;
		ensure!(
			res.rows_affected() == 1,
			Error::InternalError("Unable to insert notebook".to_string())
		);

		Ok(())
	}
}

#[derive(Encode, Decode, RuntimeDebug, TypeInfo, Serialize, Deserialize)]
struct AccountIdAndOrigin {
	pub key: [u8; 32],
	pub origin: AccountOrigin,
}
#[cfg(test)]
mod tests {
	use sp_core::{bounded_vec, H256};
	use sp_keyring::{
		AccountKeyring::{Alice, Dave},
		Sr25519Keyring::Bob,
	};
	use sqlx::PgPool;

	use ulx_notary_primitives::{AccountOrigin, BalanceChange, BalanceTip, Chain::Argon};

	use crate::stores::{
		balance_change::BalanceChangeStore, block_meta::BlockMetaStore,
		chain_transfer::ChainTransferStore, notebook::NotebookStore,
		notebook_header::NotebookHeaderStore, notebook_new_accounts::NotebookNewAccountsStore,
	};

	#[sqlx::test]
	async fn test_close_notebook(pool: PgPool) -> anyhow::Result<()> {
		// Initialize the logger
		let _ = tracing_subscriber::fmt::try_init();

		BlockMetaStore::start(&pool, H256::from_slice(&[1u8; 32])).await?;

		let mut tx = pool.begin().await?;
		BlockMetaStore::store_best_block(&mut *tx, 101, [2u8; 32]).await?;
		NotebookHeaderStore::create(&mut *tx, 1, 1, 101).await?;
		ChainTransferStore::record_transfer_to_local_from_block(
			&mut *tx,
			100,
			&Bob.to_account_id(),
			1,
			1000,
		)
		.await?;
		ChainTransferStore::take_and_record_transfer_local(
			&mut *tx,
			1,
			&Bob.to_account_id(),
			1,
			1000,
			0,
			0,
			100,
		)
		.await?;
		BalanceChangeStore::append_notebook_changeset(
			&mut *tx,
			1,
			vec![
				BalanceChange {
					account_id: Bob.to_account_id(),
					chain: Argon,
					nonce: 1,
					balance: 1000,
					previous_balance_proof: None,
					previous_balance: 0,
					notes: bounded_vec![],
				},
				BalanceChange {
					account_id: Alice.to_account_id(),
					chain: Argon,
					nonce: 1,
					balance: 2500,
					previous_balance_proof: None,
					previous_balance: 0,
					notes: bounded_vec![],
				},
				BalanceChange {
					account_id: Dave.to_account_id(),
					chain: Argon,
					nonce: 1,
					balance: 500,
					previous_balance_proof: None,
					previous_balance: 0,
					notes: bounded_vec![],
				},
			],
		)
		.await?;
		NotebookNewAccountsStore::insert_origin(&mut *tx, 1, &Bob.to_account_id(), &Argon).await?;
		NotebookNewAccountsStore::insert_origin(&mut *tx, 1, &Alice.to_account_id(), &Argon)
			.await?;
		NotebookNewAccountsStore::insert_origin(&mut *tx, 1, &Dave.to_account_id(), &Argon).await?;
		tx.commit().await?;

		let mut tx = pool.begin().await?;
		NotebookStore::close_notebook(&mut *tx, 1, 1).await?;
		tx.commit().await?;

		let balance_tip = BalanceTip {
			account_id: Bob.to_account_id(),
			chain: Argon,
			nonce: 1,
			balance: 1000,
			account_origin: AccountOrigin { notebook_number: 1, account_uid: 1 },
		};
		let proof = NotebookStore::get_balance_proof(&pool, 1, 1, &balance_tip).await?;

		assert_eq!(proof.notebook_number, 1);
		assert_eq!(proof.notary_id, 1);
		assert_eq!(proof.number_of_leaves, 3);

		assert_eq!(NotebookStore::is_valid_proof(&pool, &balance_tip, &proof).await?, true);

		assert_eq!(
			NotebookStore::get_account_origins(&pool, 1).await?.into_inner(),
			vec![
				(Bob.to_account_id(), Argon, 1),
				(Alice.to_account_id(), Argon, 2),
				(Dave.to_account_id(), Argon, 3),
			]
		);

		Ok(())
	}
}
