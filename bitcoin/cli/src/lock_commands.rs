use anyhow::{anyhow, bail, Context};
use base64::{engine::general_purpose, Engine};
use bitcoin::{
	bip32::{ChildNumber, DerivationPath, Fingerprint},
	key::Secp256k1,
	secp256k1, Address, CompressedPublicKey, FeeRate, Network, Txid,
};
use clap::{Subcommand, ValueEnum};
use comfy_table::{modifiers::UTF8_ROUND_CORNERS, presets::UTF8_FULL, ContentArrangement, Table};
use polkadot_sdk::*;
use sp_runtime::{testing::H256, FixedPointNumber, FixedU128};
use std::str::FromStr;

use argon_bitcoin::{Amount, CosignReleaser, CosignScript, CosignScriptArgs, ReleaseStep};
use argon_client::{
	api,
	api::{apis, runtime_types::pallet_bitcoin_locks::pallet::LockReleaseRequest, storage, tx},
	conversion::from_api_fixed_u128,
	FetchAt, MainchainClient,
};
use argon_primitives::{
	bitcoin::{
		BitcoinScriptPubkey, BitcoinSignature, CompressedBitcoinPubkey, H256Le, UtxoId,
		SATOSHIS_PER_BITCOIN,
	},
	BlockNumber, KeystoreParams, VaultId,
};

use crate::{formatters::ArgonFormatter, helpers::get_bitcoin_network, xpriv_file::XprivFile};

#[derive(Subcommand, Debug)]
pub enum LockCommands {
	/// Initialize a LockedBitcoin
	Initialize {
		/// The vault id
		#[clap(short, long)]
		vault_id: VaultId,

		/// The amount of btc to lock
		#[clap(short, long)]
		btc: f64,

		/// The owner pubkey you will cosign with
		#[clap(short, long)]
		owner_pubkey: String,

		#[clap(flatten)]
		keypair: KeystoreParams,
	},
	/// Outputs the address that must be funded to activate the LockedBitcoin
	SendToAddress {
		/// The utxo id
		#[clap(short, long)]
		utxo_id: UtxoId,
	},
	/// Show the current state of the lock
	Status {
		/// The utxo id
		#[clap(short, long)]
		utxo_id: UtxoId,
		/// Retrieve the lock at a specific block
		#[clap(short, long)]
		at_block: Option<BlockNumber>,
	},
	/// Starts te process to release a utxo
	RequestRelease {
		/// The bitcoin lock id
		#[clap(short, long)]
		utxo_id: UtxoId,

		/// The destination to send the bitcoin to
		#[clap(short, long)]
		dest_pubkey: String,

		/// The fee rate per sats (sat/vB) to use
		#[clap(short, long, default_value = "5")]
		fee_rate_sats_per_kb: u64,

		#[clap(flatten)]
		keypair: KeystoreParams,
	},
	/// Create the vault side of this release request to submit to Argon
	VaultCosignRelease {
		/// The bitcoin lock id
		#[clap(short, long)]
		utxo_id: UtxoId,

		#[clap(flatten)]
		xpriv_file: XprivFile,

		/// Provide the path of the derived master xpub uploaded to Argon
		#[clap(long)]
		master_xpub_hd_path: String,
	},
	/// Create a release psbt to submit to bitcoin
	OwnerCosignRelease {
		/// The utxo id in Argon. NOTE: locks are cleaned up on release, so you need this id. You
		/// can use the `lock status` command at a previous block to look this up.
		#[clap(short, long)]
		utxo_id: UtxoId,

		/// Provide the hd path to put as a hint into the psbt (if applicable)
		#[clap(long)]
		hd_path: Option<String>,

		/// Provide the parent fingerprint to put as a hint into the psbt (if applicable)
		#[clap(long)]
		parent_fingerprint: Option<String>,

		/// Provide the private key directly to sign the psbt (this should be used for testnet
		/// only).
		#[clap(long)]
		private_key: Option<String>,

		/// Wait for the cosignature to be submitted if it's not found right away
		#[clap(long)]
		wait: bool,
	},
	/// Create a psbt to claim back the utxo
	ClaimUtxoPsbt {
		/// The bitcoin lock id
		#[clap(short, long)]
		utxo_id: UtxoId,

		/// The block height when this lock existed
		#[clap(short, long)]
		at_block: BlockNumber,

		/// The claimer of the lock
		#[clap(long)]
		claimer: BitcoinClaimer,

		#[clap(flatten)]
		xpriv_file: XprivFile,

		/// If claiming as the vault, the master xpub hd path to put as a hint into the psbt (if
		/// applicable)
		#[clap(long)]
		master_xpub_hd_path: Option<String>,

		/// Provide the hd path to put as a hint into the psbt (if applicable)
		#[clap(long)]
		hd_path: Option<String>,

		/// Provide the parent fingerprint to put as a hint into the psbt (if applicable)
		#[clap(long)]
		parent_fingerprint: Option<String>,

		/// The destination you want to send the bitcoin to
		#[clap(short, long)]
		dest_pubkey: String,

		/// The fee rate per sats to use
		#[clap(short, long, default_value = "5")]
		fee_rate_sats_per_kb: u64,
	},
}

#[derive(Debug, Clone, ValueEnum, Default)]
pub enum BitcoinClaimer {
	#[default]
	Owner,
	Vault,
}

impl LockCommands {
	pub async fn process(self, rpc_url: String) -> anyhow::Result<()> {
		match self {
			LockCommands::Initialize { vault_id, keypair: _, owner_pubkey, btc } => {
				let client = MainchainClient::from_url(&rpc_url)
					.await
					.context("Failed to connect to argon node")?;

				let owner_pubkey: CompressedBitcoinPubkey =
					CompressedPublicKey::from_str(&owner_pubkey)?.into();

				let vault = client
					.fetch_storage(&storage().vaults().vaults_by_id(vault_id), FetchAt::Best)
					.await?
					.ok_or(anyhow!("Vault not found"))?;
				let satoshis = FixedU128::from_float(btc).saturating_mul_int(SATOSHIS_PER_BITCOIN);
				let Some(argons_minted) =
					client.call(apis().bitcoin_apis().market_rate(satoshis), None).await?
				else {
					println!("No price conversion found in blockchain for bitcoin to argon");
					return Ok(());
				};
				let fee = vault.terms.bitcoin_base_fee +
					from_api_fixed_u128(vault.terms.bitcoin_annual_percent_rate)
						.saturating_mul_int(argons_minted);
				println!("You're locking {} sats in exchange for {}. Your Argon account needs {} for the lock cost",
						 satoshis, ArgonFormatter(argons_minted), ArgonFormatter(fee));

				let call = tx().bitcoin_locks().initialize(vault_id, satoshis, owner_pubkey.into());
				let url = client.create_polkadotjs_deeplink(&call)?;
				println!("Link to complete transaction:\n\t{}", url);
			},
			LockCommands::SendToAddress { utxo_id } => {
				let client = MainchainClient::from_url(&rpc_url)
					.await
					.context("Failed to connect to argon node")?;

				let (_utxo_id, lock) =
					get_bitcoin_lock_from_utxo_id(&client, utxo_id, FetchAt::Finalized).await?;
				let network = get_bitcoin_network(&client, FetchAt::Finalized).await?;

				let cosign_script = get_cosign_script(&lock, network)?;
				let compressed_pubkey: CompressedBitcoinPubkey = lock.owner_pubkey.into();
				let compressed_pubkey: CompressedPublicKey = compressed_pubkey.try_into()?;

				println!(
					"You must send exactly {} satoshis to {}, which is a multisig with your public key {}.",
					lock.satoshis,
					cosign_script.get_script_address(),
					compressed_pubkey
				);
				// bitcoin-cli walletprocesspsbt "psbt_base64"
				// bitcoin-cli finalizepsbt "processed_psbt_base64"
				// bitcoin-cli sendrawtransaction "finalized_hex"
			},
			LockCommands::Status { utxo_id, at_block } => {
				let client = MainchainClient::from_url(&rpc_url)
					.await
					.context("Failed to connect to argon node")?;
				let at_block = if let Some(at_block) = at_block {
					client.block_at_height(at_block).await?.unwrap()
				} else {
					let latest_block = client.latest_finalized_block_hash().await?;
					latest_block.hash()
				};

				let fetch_at = FetchAt::Block(at_block);
				let (utxo_id, lock) =
					get_bitcoin_lock_from_utxo_id(&client, utxo_id, fetch_at).await?;

				let utxo_ref = client
					.fetch_storage(&storage().bitcoin_utxos().utxo_id_to_ref(utxo_id), fetch_at)
					.await?;

				let release_request = find_release_request(&client, fetch_at, utxo_id).await?;

				let status = match lock.is_verified {
					true =>
						if release_request.is_some() {
							"Release Requested"
						} else {
							"Verified"
						},
					false => "Unverified",
				};
				let pending_mint_query = storage().mint().pending_mint_utxos();
				let pending_mint = client
					.fetch_storage(&pending_mint_query, fetch_at)
					.await?
					.map(|a| a.0)
					.unwrap_or_default();
				let remaining = pending_mint
					.into_iter()
					.find_map(|(mint_utxo_id, _account_id, amount)| {
						if mint_utxo_id == utxo_id {
							return Some(amount);
						}
						None
					})
					.unwrap_or_default();

				let redemption_price = if let Some(ref request) = release_request {
					request.redemption_price
				} else {
					let redemption_price_query =
						apis().bitcoin_apis().redemption_rate(lock.satoshis);
					client.call(redemption_price_query, Some(at_block)).await?.unwrap_or_default()
				};

				let minted = if lock.is_verified { lock.lock_price - remaining } else { 0 };
				let utxo_ref_str = utxo_ref
					.map(|a| {
						let utxo_txid: Txid = H256Le(a.txid.0).into();
						format!("{}, vout={}", utxo_txid, a.output_index)
					})
					.unwrap_or("-".to_string());
				let cosign_script = get_cosign_script(&lock, Network::Bitcoin)?;

				let vault_pubkey: CompressedBitcoinPubkey = lock.vault_pubkey.into();
				let vault_bitcoin_pubkey: bitcoin::CompressedPublicKey = vault_pubkey.try_into()?;

				let owner_pubkey: CompressedBitcoinPubkey = lock.owner_pubkey.into();
				let owner_bitcoin_pubkey: bitcoin::CompressedPublicKey = owner_pubkey.try_into()?;

				let mut rows = vec![
					vec!["Bitcoin Utxo".into(), utxo_ref_str],
					vec!["Vault pubkey".into(), format!("{}", vault_bitcoin_pubkey)],
					vec!["Owner pubkey".into(), format!("{}", owner_bitcoin_pubkey)],
					vec!["Output Descriptor".into(), format!("{}", cosign_script.descriptor)],
					vec![
						"Minted Argons".into(),
						format!(
							"{} of {}",
							ArgonFormatter(minted),
							ArgonFormatter(lock.lock_price)
						),
					],
					vec!["Status".into(), status.into()],
					vec![
						format!(
							"Redemption Price{}",
							if release_request.is_some() { " (paid)" } else { "" }
						),
						format!("{}", ArgonFormatter(redemption_price)),
					],
					vec!["Expiration Bitcoin Block".into(), format!("{}", lock.vault_claim_height)],
					vec![
						"Owner Reclaim Bitcoin Block".into(),
						format!("{}", lock.open_claim_height),
					],
				];

				if let Some(ref request) = release_request {
					rows.push(vec![
						"Release Requested".into(),
						format!("due at block {}", request.cosign_due_block),
					])
				}

				let mut table = Table::new();
				table
					.load_preset(UTF8_FULL)
					.apply_modifier(UTF8_ROUND_CORNERS)
					.set_content_arrangement(ContentArrangement::Dynamic);

				table.add_rows(rows);

				println!("{table}");
			},
			LockCommands::RequestRelease {
				utxo_id,
				dest_pubkey,
				fee_rate_sats_per_kb,
				keypair: _,
			} => {
				let client = MainchainClient::from_url(&rpc_url)
					.await
					.context("Failed to connect to argon node")?;
				let network = get_bitcoin_network(&client, FetchAt::Finalized).await?;

				let (_, lock) =
					get_bitcoin_lock_from_utxo_id(&client, utxo_id, FetchAt::Finalized).await?;
				let redemption_price_query = apis().bitcoin_apis().redemption_rate(lock.satoshis);

				let latest_block = client.latest_finalized_block_hash().await?;
				let redemption_price = client
					.live
					.runtime_api()
					.at(latest_block)
					.call(redemption_price_query)
					.await?
					.unwrap_or_default();

				let bitcoin_dest_pubkey = Address::from_str(&dest_pubkey)
					.map_err(|e| anyhow!("Unable to parse bitcoin destination pubkey: {e:?}"))?
					.require_network(network)?
					.script_pubkey();
				let cosign = get_cosign_script(&lock, network)?;

				let network_fee = cosign.calculate_fee(
					true,
					bitcoin_dest_pubkey.clone(),
					FeeRate::from_sat_per_vb(fee_rate_sats_per_kb)
						.ok_or(anyhow!("Invalid fee rate"))?,
				)?;

				println!(
					"The price to release this lock is: {}\nBitcoin fee: {:?}",
					ArgonFormatter(redemption_price),
					network_fee
				);
				let argon_bitcoin_script_pubkey: BitcoinScriptPubkey = bitcoin_dest_pubkey.into();
				let call = tx().bitcoin_locks().request_release(
					utxo_id,
					argon_bitcoin_script_pubkey.into(),
					network_fee.to_sat(),
				);
				let url = client.create_polkadotjs_deeplink(&call)?;
				println!("Link to create transaction:\n\t{}", url);
			},
			LockCommands::VaultCosignRelease { utxo_id, xpriv_file, master_xpub_hd_path } => {
				let client = MainchainClient::from_url(&rpc_url)
					.await
					.context("Failed to connect to argon node")?;
				let at_block = FetchAt::Finalized;
				let child_xpriv = xpriv_file.read()?.derive_priv(
					&Secp256k1::new(),
					&DerivationPath::from_str(&master_xpub_hd_path)?,
				)?;

				let (utxo_id, lock) =
					get_bitcoin_lock_from_utxo_id(&client, utxo_id, at_block).await?;
				let mut releaser = load_cosign_releaser(&client, utxo_id, &lock, at_block).await?;
				let owner_pubkey: CompressedPublicKey =
					releaser.cosign_script.script_args.owner_pubkey.try_into()?;
				let compressed: CompressedPublicKey = owner_pubkey;
				let fingerprint = Fingerprint::from(lock.vault_xpub_sources.0);

				let hd_path =
					DerivationPath::from(vec![ChildNumber::from(lock.vault_xpub_sources.1)]);

				releaser.psbt.inputs[0]
					.bip32_derivation
					.insert(compressed.0, (fingerprint, hd_path));
				let mut psbt = releaser.psbt;
				psbt.sign(&child_xpriv, &Secp256k1::new()).map_err(|e| {
					anyhow!(
						"Unable to sign this bitcoin transaction with the given XPriv -> {:#?}",
						e.1
					)
				})?;
				println!(
					"Your xpriv was used to sign the following psbt:\n\n{}",
					general_purpose::STANDARD.encode(&psbt.serialize()[..])
				);
				let compressed_pubkey: CompressedBitcoinPubkey = lock.vault_pubkey.clone().into();
				let compressed_pubkey: CompressedPublicKey = compressed_pubkey.try_into()?;

				let signature: BitcoinSignature = (*psbt.inputs[0]
					.partial_sigs
					.get(&compressed_pubkey.into())
					.ok_or(anyhow!("No signature found"))?)
				.try_into()
				.map_err(|_| anyhow!("Unable to translate signature to bytes"))?;

				let release_fulfill =
					tx().bitcoin_locks().cosign_release(utxo_id, signature.into());
				let url = client.create_polkadotjs_deeplink(&release_fulfill)?;
				println!("Link to create transaction:\n\t{}", url);
			},
			LockCommands::OwnerCosignRelease {
				utxo_id,
				parent_fingerprint,
				hd_path,
				private_key,
				wait,
			} => {
				let private_key = if let Some(private_key) = private_key {
					Some(bitcoin::PrivateKey::from_str(&private_key)?)
				} else {
					None
				};

				let client = MainchainClient::from_url(&rpc_url)
					.await
					.context("Failed to connect to argon node")?;
				let at_block = FetchAt::Finalized;

				let mut signature: Option<BitcoinSignature> = None;
				let mut active_height: Option<H256> = None;
				if let Some(release_height) = client
					.fetch_storage(
						&storage().bitcoin_locks().lock_release_cosign_height_by_id(utxo_id),
						at_block,
					)
					.await?
				{
					let release_block = client
						.block_at_height(release_height)
						.await?
						.ok_or(anyhow::anyhow!("No block found for the cosign block height"))?;
					let release_events =
						client.live.blocks().at(release_block).await?.events().await?;
					let release_event = release_events
                        .find_first::<api::bitcoin_locks::events::BitcoinUtxoCosigned>()?.ok_or(anyhow!("No corresponding event found for the cosign release height in the blockchain."))?;

					signature = Some(
						release_event
							.signature
							.try_into()
							.map_err(|_| anyhow!("Unable to translate bitcoin signature"))?,
					);

					active_height = client.block_at_height(release_height - 1).await?;
				} else {
					let pending_release =
						find_release_request(&client, at_block, utxo_id).await?.ok_or(anyhow!(
							"This lock isn't pending release and has no pending signatures.\
						\nPossibilities:\
							\n - the cosign period expired\
							\n - the request was already processed"
						))?;

					if wait {
						let mut finalized_sub = client.live.blocks().subscribe_finalized().await?;

						while let Some(block) = finalized_sub.next().await {
							print!(".");
							let block = block?;
							let cosign = block
								.events()
								.await?
								.find_first::<api::bitcoin_locks::events::BitcoinUtxoCosigned>(
							)?;
							if let Some(cosign_release) = cosign {
								if cosign_release.utxo_id == pending_release.utxo_id {
									signature = Some(
										cosign_release
											.signature
											.try_into()
											.map_err(|_| anyhow!("Unable to decode signature"))?,
									);
									active_height = Some(block.hash());
									break;
								}
							}
						}
					} else {
						bail!(
							"This lock release request hasn't been processed yet. It is due by block {} (current={})",
							pending_release.cosign_due_block,
							client.latest_finalized_block().await?
						)
					}
				};

				let (active_height, signature) = match (active_height, signature) {
					(Some(a), Some(b)) => (Some(a), b),
					_ => bail!("No signature found"),
				};

				let fetch_at: FetchAt = active_height.map(Into::into).unwrap_or_default();

				let utxo = client
					.fetch_storage(&storage().bitcoin_locks().locks_by_utxo_id(utxo_id), fetch_at)
					.await?
					.ok_or(anyhow::anyhow!("No utxo found for lock"))?;

				let mut releaser = load_cosign_releaser(&client, utxo_id, &utxo, fetch_at).await?;
				releaser.add_signature(
					releaser
						.cosign_script
						.script_args
						.bitcoin_vault_pubkey()
						.map_err(|e| anyhow!("Could not convert the vault pubkey {:?}", e))?,
					signature.try_into()?,
				);
				let vault_fingerprint = Fingerprint::from(utxo.vault_xpub_sources.0);
				let vault_hd_path =
					DerivationPath::from(vec![ChildNumber::from(utxo.vault_xpub_sources.1)]);
				let vault_pubkey: CompressedBitcoinPubkey = utxo.vault_pubkey.into();
				let vault_pubkey: CompressedPublicKey = vault_pubkey.try_into()?;
				releaser.psbt.inputs[0]
					.bip32_derivation
					.insert(vault_pubkey.0, (vault_fingerprint, vault_hd_path));
				if let (Some(hd_path), Some(parent_fingerprint)) = (hd_path, parent_fingerprint) {
					let fingerprint = Fingerprint::from_str(&parent_fingerprint)?;
					let hd_path = DerivationPath::from_str(&hd_path)?;
					let keysource = (fingerprint, hd_path);
					let owner_pubkey = releaser
						.cosign_script
						.script_args
						.bitcoin_owner_pubkey()
						.map_err(|e| anyhow!("Could not convert owner pubkey {:?}", e))?;
					println!(
						"Adding owner pubkey to psbt bip32 derivation: {:?}, {:?}",
						owner_pubkey, keysource
					);
					releaser.psbt.inputs[0].bip32_derivation.insert(
						secp256k1::PublicKey::from_slice(&owner_pubkey.to_bytes()[..])?,
						keysource,
					);
				}
				if let Some(private_key) = private_key {
					releaser.sign(private_key)?;
				}

				let psbt = releaser.psbt;

				println!(
					"Import this psbt to sign and broadcast the transaction:\n\n{}",
					general_purpose::STANDARD.encode(&psbt.serialize()[..])
				);
				return Ok(());
			},
			LockCommands::ClaimUtxoPsbt {
				utxo_id,
				at_block: block_number,
				claimer,
				dest_pubkey,
				xpriv_file,
				master_xpub_hd_path,
				fee_rate_sats_per_kb,
				hd_path,
				parent_fingerprint,
			} => {
				let client = MainchainClient::from_url(&rpc_url)
					.await
					.context("Failed to connect to argon node")?;
				let block_hash = client
					.fetch_storage(&storage().system().block_hash(block_number), Default::default())
					.await?
					.ok_or(anyhow::anyhow!("No block found for the given block number"))?;

				let at_block = FetchAt::Block(block_hash.into());
				let (utxo_id, lock) =
					get_bitcoin_lock_from_utxo_id(&client, utxo_id, at_block).await?;

				let utxo_ref = client
					.fetch_storage(&storage().bitcoin_utxos().utxo_id_to_ref(utxo_id), at_block)
					.await?
					.ok_or(anyhow::anyhow!("No utxo ref found for lock"))?;
				let network = get_bitcoin_network(&client, at_block).await?;
				let cosign_script = get_cosign_script(&lock, network)?;
				let txid: Txid = H256Le(utxo_ref.txid.0).into();
				let fee_rate = FeeRate::from_sat_per_vb(fee_rate_sats_per_kb)
					.ok_or(anyhow!("Invalid fee rate"))?;

				let pay_scriptpub: BitcoinScriptPubkey = Address::from_str(&dest_pubkey)
					.map_err(|e| anyhow!("Unable to parse bitcoin destination pubkey: {e:?}"))?
					.require_network(network)?
					.script_pubkey()
					.into();

				let fee =
					cosign_script.calculate_fee(true, pay_scriptpub.clone().into(), fee_rate)?;

				let mut releaser = CosignReleaser::from_script(
					cosign_script,
					lock.satoshis,
					txid,
					utxo_ref.output_index,
					match claimer {
						BitcoinClaimer::Owner => ReleaseStep::OwnerClaim,
						BitcoinClaimer::Vault => ReleaseStep::VaultClaim,
					},
					fee,
					pay_scriptpub.into(),
				)?;
				let vault_fingerprint = Fingerprint::from(lock.vault_xpub_sources.0);
				let vault_hd_path =
					DerivationPath::from(vec![ChildNumber::from(lock.vault_xpub_sources.2)]);
				let vault_pubkey: CompressedBitcoinPubkey = lock.vault_claim_pubkey.into();
				let vault_pubkey: CompressedPublicKey = vault_pubkey.try_into()?;
				releaser.psbt.inputs[0]
					.bip32_derivation
					.insert(vault_pubkey.0, (vault_fingerprint, vault_hd_path));

				if let (Some(hd_path), Some(parent_fingerprint)) = (hd_path, parent_fingerprint) {
					let owner_pubkey: CompressedPublicKey =
						releaser.cosign_script.script_args.owner_pubkey.try_into()?;
					let fingerprint = Fingerprint::from_str(&parent_fingerprint)?;
					let hd_path = DerivationPath::from_str(&hd_path)?;
					releaser.psbt.inputs[0]
						.bip32_derivation
						.insert(owner_pubkey.0, (fingerprint, hd_path));
				}
				let mut psbt = releaser.psbt;
				if matches!(claimer, BitcoinClaimer::Vault) {
					let master_xpub_hd_path = master_xpub_hd_path.ok_or(anyhow!(
						"Master xpub hd path is required when claiming as the vault"
					))?;
					let vault_xpriv = xpriv_file.read()?.derive_priv(
						&Secp256k1::new(),
						&DerivationPath::from_str(&master_xpub_hd_path)?,
					)?;
					psbt.sign(&vault_xpriv, &Secp256k1::new()).map_err(|e| {
						anyhow!(
							"Unable to sign this bitcoin transaction with the given XPriv -> {:#?}",
							e.1
						)
					})?;
				}

				println!(
					"Load this psbt to your wallet to claim the bitcoin:\n\n{}",
					general_purpose::STANDARD.encode(&psbt.serialize()[..])
				);
			},
		}
		Ok(())
	}
}

async fn load_cosign_releaser(
	client: &MainchainClient,
	utxo_id: UtxoId,
	lock: &api::runtime_types::pallet_bitcoin_locks::pallet::LockedBitcoin,
	at_block: FetchAt,
) -> anyhow::Result<CosignReleaser> {
	let utxo_ref = client
		.fetch_storage(&storage().bitcoin_utxos().utxo_id_to_ref(utxo_id), at_block)
		.await?
		.ok_or(anyhow::anyhow!("No utxo ref found for lock"))?;
	let release_info = find_release_request(client, at_block, utxo_id)
		.await?
		.ok_or(anyhow!("No lock release request found"))?;
	let network = get_bitcoin_network(client, at_block).await?;

	let txid: Txid = H256Le(utxo_ref.txid.0).into();
	let pay_scriptpub: BitcoinScriptPubkey = release_info
		.to_script_pubkey
		.try_into()
		.map_err(|_| anyhow!("Unable to decode the destination pubkey"))?;
	let releaser = CosignReleaser::from_script(
		get_cosign_script(lock, network)?,
		lock.satoshis,
		txid,
		utxo_ref.output_index,
		ReleaseStep::VaultCosign,
		Amount::from_sat(release_info.bitcoin_network_fee),
		pay_scriptpub.into(),
	)?;
	Ok(releaser)
}

async fn find_release_request(
	client: &MainchainClient,
	at_block: FetchAt,
	utxo_id: UtxoId,
) -> anyhow::Result<Option<LockReleaseRequest<u128>>> {
	let release_request = client
		.fetch_storage(&storage().bitcoin_locks().locks_pending_release_by_utxo_id(), at_block)
		.await?
		.map(|a| a.0)
		.unwrap_or_default()
		.into_iter()
		.find_map(|(a, unlock)| {
			if a == utxo_id {
				return Some(unlock);
			}
			None
		});
	Ok(release_request)
}
async fn get_bitcoin_lock_from_utxo_id(
	client: &MainchainClient,
	utxo_id: UtxoId,
	at_block: FetchAt,
) -> anyhow::Result<(UtxoId, api::runtime_types::pallet_bitcoin_locks::pallet::LockedBitcoin)> {
	let query = storage().bitcoin_locks().locks_by_utxo_id(utxo_id);
	let lock = client
		.fetch_storage(&query, at_block)
		.await?
		.ok_or(anyhow!("No finalized LockedBitcoin found"))?;

	Ok((utxo_id, lock))
}

fn get_cosign_script(
	lock: &api::runtime_types::pallet_bitcoin_locks::pallet::LockedBitcoin,
	network: Network,
) -> anyhow::Result<CosignScript> {
	let script_args = CosignScriptArgs {
		vault_pubkey: lock.vault_pubkey.clone().into(),
		vault_claim_pubkey: lock.vault_claim_pubkey.clone().into(),
		owner_pubkey: lock.owner_pubkey.clone().into(),
		vault_claim_height: lock.vault_claim_height,
		open_claim_height: lock.open_claim_height,
		created_at_height: lock.created_at_height,
	};

	Ok(CosignScript::new(script_args, network)?)
}
