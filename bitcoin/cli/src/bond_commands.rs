use std::str::FromStr;

use anyhow::{anyhow, bail, Context};
use base64::{engine::general_purpose, Engine};
use bitcoin::{
	absolute::LockTime,
	bip32::{ChildNumber, DerivationPath, Fingerprint},
	secp256k1,
	transaction::Version,
	Address, CompressedPublicKey, FeeRate, Network, Psbt, Transaction, TxOut, Txid,
};
use clap::{Subcommand, ValueEnum};
use comfy_table::{modifiers::UTF8_ROUND_CORNERS, presets::UTF8_FULL, ContentArrangement, Table};
use sp_runtime::{testing::H256, FixedPointNumber, FixedU128};

use argon_bitcoin::{Amount, CosignScript, CosignScriptArgs, Error, UnlockStep, UtxoUnlocker};
use argon_client::{
	api,
	api::{apis, runtime_types::pallet_bond::pallet::UtxoCosignRequest, storage, tx},
	conversion::from_api_fixed_u128,
	MainchainClient,
};
use argon_primitives::{
	bitcoin::{
		BitcoinNetwork, BitcoinScriptPubkey, BitcoinSignature, CompressedBitcoinPubkey, H256Le,
		UtxoId, SATOSHIS_PER_BITCOIN,
	},
	BlockNumber, BondId, KeystoreParams, VaultId,
};

use crate::formatters::ArgonFormatter;

#[derive(Subcommand, Debug)]
pub enum BondCommands {
	/// Create a bond application
	Apply {
		/// The vault id
		#[clap(short, long)]
		vault_id: VaultId,

		/// The amount of btc to bond
		#[clap(short, long)]
		btc: f32,

		/// The owner pubkey you will cosign with
		#[clap(short, long)]
		owner_pubkey: String,

		#[clap(flatten)]
		keypair: KeystoreParams,
	},
	/// Create a partially signed bitcoin transaction to fund this bitcoin bond
	CreatePsbt {
		/// The bond id
		#[clap(short, long)]
		bond_id: BondId,
	},
	/// Show the current state of the bond
	Get {
		/// The bond id
		#[clap(short, long)]
		bond_id: BondId,
		/// Retrieve the bond at a specific block
		#[clap(short, long)]
		at_block: Option<BlockNumber>,
	},
	/// Helps create an unlock request
	RequestUnlock {
		/// The bond id
		#[clap(short, long)]
		bond_id: BondId,

		/// The destination to send the bitcoin to
		#[clap(short, long)]
		dest_pubkey: String,

		/// The fee rate per sats to use
		#[clap(short, long, default_value = "5")]
		fee_rate_per_sats: u64,

		#[clap(flatten)]
		keypair: KeystoreParams,
	},
	/// Create the vault side of this unlock request
	VaultCosignPsbt {
		/// The bond id to unlock
		#[clap(short, long)]
		bond_id: BondId,
	},
	/// Submit a cosignature to the blockchain
	VaultCosignSubmit {
		/// The bond id to unlock
		#[clap(short, long)]
		bond_id: BondId,

		/// The psbt to retrieve the signature from
		#[clap(short, long)]
		psbt: String,
	},
	/// Create an unlock psbt to submit to bitcoin
	OwnerCosignPsbt {
		/// The utxo id in Argon. NOTE: bonds are cleaned up on release, so you need this id. You
		/// can use the `bond get` command at a previous block to look this up.
		#[clap(short, long)]
		utxo_id: UtxoId,

		/// Provide the hd path to put as a hint into the psbt (if applicable)
		#[clap(long)]
		hd_path: Option<String>,

		/// Provide the parent fingerprint to put as a hint into the psbt (if applicable)
		#[clap(long)]
		parent_fingerprint: Option<String>,

		/// Wait for the cosignature to be submitted if it's not found right away
		#[clap(long)]
		wait: bool,
	},
	/// Create a psbt to claim back the utxo
	ClaimUtxoPsbt {
		/// The bond id that held this bitcoin utxo in Argon.
		#[clap(short, long)]
		bond_id: BondId,

		/// The block height when this bond existed
		#[clap(short, long)]
		at_block: BlockNumber,

		/// The claimer of the bond
		#[clap(long)]
		claimer: BitcoinClaimer,

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
		fee_rate_per_sats: u64,
	},
}

#[derive(Debug, Clone, ValueEnum, Default)]
pub enum BitcoinClaimer {
	#[default]
	Owner,
	Vault,
}

impl BondCommands {
	pub async fn process(self, rpc_url: String) -> anyhow::Result<()> {
		match self {
			BondCommands::Apply { vault_id, keypair: _, owner_pubkey, btc } => {
				let client = MainchainClient::from_url(&rpc_url)
					.await
					.context("Failed to connect to argon node")?;

				let owner_pubkey: CompressedBitcoinPubkey =
					CompressedPublicKey::from_str(&owner_pubkey)?.into();
				let vault = client
					.fetch_storage(&storage().vaults().vaults_by_id(vault_id), None)
					.await?
					.ok_or(anyhow!("Vault not found"))?;
				let satoshis =
					FixedU128::from_float(btc as f64).saturating_mul_int(SATOSHIS_PER_BITCOIN);
				let Some(argons_minted) =
					client.call(apis().bitcoin_apis().market_rate(satoshis), None).await?
				else {
					println!("No price conversion found in blockchain for bitcoin to argon");
					return Ok(());
				};
				let fee = vault.bitcoin_argons.base_fee +
					from_api_fixed_u128(vault.bitcoin_argons.annual_percent_rate)
						.saturating_mul_int(argons_minted);
				println!("You're bonding {} sats in exchange for {}. Your Argon account needs {} for the bond cost",
						 satoshis, ArgonFormatter(argons_minted), ArgonFormatter(fee));

				let call = tx().bonds().bond_bitcoin(vault_id, satoshis, owner_pubkey.into());
				let url = client.create_polkadotjs_deeplink(&call)?;
				println!("Link to complete transaction:\n\t{}", url);
			},
			BondCommands::CreatePsbt { bond_id } => {
				let client = MainchainClient::from_url(&rpc_url)
					.await
					.context("Failed to connect to argon node")?;

				let (_utxo_id, utxo, _) = get_utxo_from_bond_id(&client, bond_id, None).await?;
				let network = get_bitcoin_network(&client, None).await?;

				let unlocker = get_cosign_script(&utxo, network)?;

				let tx_out = TxOut {
					value: Amount::from_sat(utxo.satoshis),
					script_pubkey: unlocker.get_script_pubkey(),
				};

				// Create an empty transaction with no inputs and one output
				let tx = Transaction {
					version: Version::TWO, // Post BIP-68.
					lock_time: LockTime::ZERO,
					input: vec![],
					output: vec![tx_out],
				};
				let psbt = Psbt::from_unsigned_tx(tx).map_err(Error::PsbtError)?;

				println!(
					"You are sending {} sats to {}.\n\nAdd this psbt to your wallet to fund the bitcoin:\n\n{}",
					utxo.satoshis,
					unlocker.get_script_address(),
					general_purpose::STANDARD.encode(&psbt.serialize()[..])
				);
				// bitcoin-cli walletprocesspsbt "psbt_base64"
				// bitcoin-cli finalizepsbt "processed_psbt_base64"
				// bitcoin-cli sendrawtransaction "finalized_hex"
			},
			BondCommands::Get { bond_id, at_block } => {
				let client = MainchainClient::from_url(&rpc_url)
					.await
					.context("Failed to connect to argon node")?;
				let at_block = if let Some(at_block) = at_block {
					let block_chash = client
						.fetch_storage(&storage().system().block_hash(at_block), None)
						.await?
						.unwrap();
					Some(block_chash)
				} else {
					let latest_block = client.latest_finalized_block_hash().await?;
					Some(latest_block.hash())
				};

				let (utxo_id, utxo, bond) =
					get_utxo_from_bond_id(&client, bond_id, at_block).await?;

				let utxo_ref = client
					.fetch_storage(&storage().bitcoin_utxos().utxo_id_to_ref(utxo_id), at_block)
					.await?;

				let unlock_request = find_unlock_request(&client, at_block, utxo_id).await?;

				let status = match utxo.is_verified {
					true =>
						if unlock_request.is_some() {
							"Unlock Requested"
						} else {
							"Verified"
						},
					false => "Unverified",
				};
				let pending_mint_query = storage().mint().pending_mint_utxos();
				let pending_mint = client
					.fetch_storage(&pending_mint_query, at_block)
					.await?
					.map(|a| a.0)
					.unwrap_or_default();
				let remaining = pending_mint
					.into_iter()
					.find_map(|(mint_utxo_id, _account_id, amount)| {
						if mint_utxo_id == utxo_id {
							return Some(amount)
						}
						None
					})
					.unwrap_or_default();

				let redemption_price = if let Some(ref unlock_request) = unlock_request {
					unlock_request.redemption_price
				} else {
					let redemption_price_query =
						apis().bitcoin_apis().redemption_rate(utxo.satoshis);
					client.call(redemption_price_query, at_block).await?.unwrap_or_default()
				};

				let minted = if utxo.is_verified { bond.amount - remaining } else { 0 };
				let utxo_ref_str = utxo_ref
					.map(|a| {
						let utxo_txid: Txid = H256Le(a.txid.0).into();
						format!("{}, vout={}", utxo_txid, a.output_index)
					})
					.unwrap_or("-".to_string());

				let vault_pubkey: CompressedBitcoinPubkey = utxo.vault_pubkey.into();
				let vault_bitcoin_pubkey: bitcoin::CompressedPublicKey = vault_pubkey.try_into()?;

				let owner_pubkey: CompressedBitcoinPubkey = utxo.owner_pubkey.into();
				let owner_bitcoin_pubkey: bitcoin::CompressedPublicKey = owner_pubkey.try_into()?;

				let mut rows = vec![
					vec!["Bitcoin Utxo".into(), utxo_ref_str],
					vec!["Vault pubkey".into(), format!("{}", vault_bitcoin_pubkey)],
					vec!["Owner pubkey".into(), format!("{}", owner_bitcoin_pubkey)],
					vec![
						"Minted Argons".into(),
						format!("{} of {}", ArgonFormatter(minted), ArgonFormatter(bond.amount)),
					],
					vec!["Status".into(), status.into()],
					vec![
						format!(
							"Redemption Price{}",
							if unlock_request.is_some() { " (paid)" } else { "" }
						),
						format!("{}", ArgonFormatter(redemption_price)),
					],
					vec!["Expiration Bitcoin Block".into(), format!("{}", utxo.vault_claim_height)],
					vec![
						"Owner Reclaim Bitcoin Block".into(),
						format!("{}", utxo.open_claim_height),
					],
				];

				if let Some(ref unlock_request) = unlock_request {
					rows.push(vec![
						"Unlock Requested".into(),
						format!("due at block {}", unlock_request.cosign_due_block),
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
			BondCommands::RequestUnlock { bond_id, dest_pubkey, fee_rate_per_sats, keypair: _ } => {
				let client = MainchainClient::from_url(&rpc_url)
					.await
					.context("Failed to connect to argon node")?;
				let latest_block = client.latest_finalized_block_hash().await?;
				let at_block = Some(latest_block.hash());
				let network = get_bitcoin_network(&client, at_block).await?;

				let (_, utxo, _) = get_utxo_from_bond_id(&client, bond_id, at_block).await?;
				let redemption_price_query = apis().bitcoin_apis().redemption_rate(utxo.satoshis);

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
				let cosign = get_cosign_script(&utxo, network)?;

				let network_fee = cosign.calculate_fee(
					true,
					bitcoin_dest_pubkey.clone(),
					FeeRate::from_sat_per_vb(fee_rate_per_sats)
						.ok_or(anyhow!("Invalid fee rate"))?,
				)?;

				println!(
					"The price to unlock this bond is: {}\nBitcoin fee: {:?}",
					ArgonFormatter(redemption_price),
					network_fee
				);
				let argon_bitcoin_script_pubkey: BitcoinScriptPubkey = bitcoin_dest_pubkey.into();
				let call = tx().bonds().unlock_bitcoin_bond(
					bond_id,
					argon_bitcoin_script_pubkey.into(),
					network_fee.to_sat(),
				);
				let url = client.create_polkadotjs_deeplink(&call)?;
				println!("Link to create transaction:\n\t{}", url);
			},
			BondCommands::VaultCosignPsbt { bond_id } => {
				let client = MainchainClient::from_url(&rpc_url)
					.await
					.context("Failed to connect to argon node")?;
				let latest_block = client.latest_finalized_block_hash().await?;
				let at_block = Some(latest_block.hash());

				let (utxo_id, utxo, _) = get_utxo_from_bond_id(&client, bond_id, at_block).await?;
				let mut unlocker = load_unlocker(&client, utxo_id, &utxo, at_block).await?;
				let owner_pubkey: CompressedPublicKey =
					unlocker.cosign_script.script_args.owner_pubkey.try_into()?;
				let compressed: CompressedPublicKey = owner_pubkey;
				let fingerprint = Fingerprint::from(utxo.vault_xpub_sources.0);
				let hd_path =
					DerivationPath::from(vec![ChildNumber::from(utxo.vault_xpub_sources.1)]);
				unlocker.psbt.inputs[0]
					.bip32_derivation
					.insert(compressed.0, (fingerprint, hd_path));
				let psbt = unlocker.psbt;
				println!(
					"Add your signature to this psbt the bitcoin:\n\n{}",
					general_purpose::STANDARD.encode(&psbt.serialize()[..])
				);
			},
			BondCommands::VaultCosignSubmit { bond_id, psbt } => {
				let client = MainchainClient::from_url(&rpc_url)
					.await
					.context("Failed to connect to argon node")?;
				let latest_block = client.latest_finalized_block_hash().await?;
				let at_block = Some(latest_block.hash());

				let (_, utxo, _) = get_utxo_from_bond_id(&client, bond_id, at_block).await?;

				let psbt = Psbt::from_str(&psbt)?;
				let compressed_pubkey: CompressedBitcoinPubkey = utxo.vault_pubkey.clone().into();
				let compressed_pubkey: CompressedPublicKey = compressed_pubkey.try_into()?;

				let signature: BitcoinSignature = (*psbt.inputs[0]
					.partial_sigs
					.get(&compressed_pubkey.into())
					.ok_or(anyhow!("No signature found"))?)
				.try_into()
				.map_err(|_| anyhow!("Unable to translate signature to bytes"))?;

				let unlock_fulfill = tx().bonds().cosign_bitcoin_unlock(bond_id, signature.into());
				let url = client.create_polkadotjs_deeplink(&unlock_fulfill)?;
				println!("Link to create transaction:\n\t{}", url);
			},
			BondCommands::OwnerCosignPsbt { utxo_id, parent_fingerprint, hd_path, wait } => {
				let client = MainchainClient::from_url(&rpc_url)
					.await
					.context("Failed to connect to argon node")?;
				let latest_block = client.latest_finalized_block_hash().await?;
				let at_block = Some(latest_block.hash());

				let mut signature: Option<BitcoinSignature> = None;
				let mut active_height: Option<H256> = None;
				if let Some(release_height) = client
					.fetch_storage(
						&storage().bonds().utxos_cosign_release_height_by_id(utxo_id),
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
                        .find_first::<api::bonds::events::BitcoinUtxoCosigned>()?.ok_or(anyhow!("No corresponding event found for the cosign release height in the blockchain."))?;

					signature = Some(
						release_event
							.signature
							.try_into()
							.map_err(|_| anyhow!("Unable to translate bitcoin signature"))?,
					);

					active_height = client.block_at_height(release_height - 1).await?;
				} else {
					let pending_unlock =
						find_unlock_request(&client, at_block, utxo_id).await?.ok_or(anyhow!(
							"This utxo isn't pending unlock and has no pending signatures.\
						\nPossibilities:\
							\n - the cosign period expired\
							\n - the request was already processed"
						))?;

					if wait {
						let mut finalized_sub = client.live.blocks().subscribe_finalized().await?;

						while let Some(block) = finalized_sub.next().await {
							print!(".");
							let block = block?;
							let utxo_unlock = block
								.events()
								.await?
								.find_first::<api::bonds::events::BitcoinUtxoCosigned>(
							)?;
							if let Some(utxo_unlock) = utxo_unlock {
								if utxo_unlock.bond_id == pending_unlock.bond_id {
									signature = Some(
										utxo_unlock
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
							"This unlock request hasn't been processed yet. It is due by block {} (current={})",
							pending_unlock.cosign_due_block,
							client.latest_finalized_block().await?
						)
					}
				};

				let (active_height, signature) = match (active_height, signature) {
					(Some(a), Some(b)) => (Some(a), b),
					_ => bail!("No signature found"),
				};

				let utxo = client
					.fetch_storage(&storage().bonds().utxos_by_id(utxo_id), active_height)
					.await?
					.ok_or(anyhow::anyhow!("No utxo found for bond"))?;

				let mut unlocker = load_unlocker(&client, utxo_id, &utxo, active_height).await?;
				unlocker.add_signature(
					unlocker
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
				unlocker.psbt.inputs[0]
					.bip32_derivation
					.insert(vault_pubkey.0, (vault_fingerprint, vault_hd_path));
				if let (Some(hd_path), Some(parent_fingerprint)) = (hd_path, parent_fingerprint) {
					let fingerprint = Fingerprint::from_str(&parent_fingerprint)?;
					let hd_path = DerivationPath::from_str(&hd_path)?;
					let keysource = (fingerprint, hd_path);
					let owner_pubkey = unlocker
						.cosign_script
						.script_args
						.bitcoin_owner_pubkey()
						.map_err(|e| anyhow!("Could not convert owner pubkey {:?}", e))?;
					println!(
						"Adding owner pubkey to psbt bip32 derivation: {:?}, {:?}",
						owner_pubkey, keysource
					);
					unlocker.psbt.inputs[0].bip32_derivation.insert(
						secp256k1::PublicKey::from_slice(&owner_pubkey.to_bytes()[..])?,
						keysource,
					);
				}

				let psbt = unlocker.psbt;

				println!(
					"Add this psbt to your wallet to fund the bitcoin:\n\n{}",
					general_purpose::STANDARD.encode(&psbt.serialize()[..])
				);
				return Ok(())
			},
			BondCommands::ClaimUtxoPsbt {
				bond_id,
				at_block: block_number,
				claimer,
				dest_pubkey,
				fee_rate_per_sats,
				hd_path,
				parent_fingerprint,
			} => {
				let client = MainchainClient::from_url(&rpc_url)
					.await
					.context("Failed to connect to argon node")?;
				let block_hash = client
					.fetch_storage(&storage().system().block_hash(block_number), None)
					.await?
					.ok_or(anyhow::anyhow!("No block found for the given block number"))?;

				let at_block = Some(block_hash);
				let (utxo_id, utxo, _) = get_utxo_from_bond_id(&client, bond_id, at_block).await?;

				let utxo_ref = client
					.fetch_storage(&storage().bitcoin_utxos().utxo_id_to_ref(utxo_id), at_block)
					.await?
					.ok_or(anyhow::anyhow!("No utxo ref found for bond"))?;
				let network = get_bitcoin_network(&client, at_block).await?;
				let cosign_script = get_cosign_script(&utxo, network)?;
				let txid: Txid = H256Le(utxo_ref.txid.0).into();
				let fee_rate = FeeRate::from_sat_per_vb(fee_rate_per_sats)
					.ok_or(anyhow!("Invalid fee rate"))?;

				let pay_scriptpub: BitcoinScriptPubkey = Address::from_str(&dest_pubkey)
					.map_err(|e| anyhow!("Unable to parse bitcoin destination pubkey: {e:?}"))?
					.require_network(network)?
					.script_pubkey()
					.into();

				let fee =
					cosign_script.calculate_fee(true, pay_scriptpub.clone().into(), fee_rate)?;

				let mut unlocker = UtxoUnlocker::from_script(
					cosign_script,
					utxo.satoshis,
					txid,
					utxo_ref.output_index,
					match claimer {
						BitcoinClaimer::Owner => UnlockStep::OwnerClaim,
						BitcoinClaimer::Vault => UnlockStep::VaultClaim,
					},
					fee,
					pay_scriptpub.into(),
				)?;
				let vault_fingerprint = Fingerprint::from(utxo.vault_xpub_sources.0);
				let vault_hd_path =
					DerivationPath::from(vec![ChildNumber::from(utxo.vault_xpub_sources.2)]);
				let vault_pubkey: CompressedBitcoinPubkey = utxo.vault_claim_pubkey.into();
				let vault_pubkey: CompressedPublicKey = vault_pubkey.try_into()?;
				unlocker.psbt.inputs[0]
					.bip32_derivation
					.insert(vault_pubkey.0, (vault_fingerprint, vault_hd_path));

				if let (Some(hd_path), Some(parent_fingerprint)) = (hd_path, parent_fingerprint) {
					let owner_pubkey: CompressedPublicKey =
						unlocker.cosign_script.script_args.owner_pubkey.try_into()?;
					let fingerprint = Fingerprint::from_str(&parent_fingerprint)?;
					let hd_path = DerivationPath::from_str(&hd_path)?;
					unlocker.psbt.inputs[0]
						.bip32_derivation
						.insert(owner_pubkey.0, (fingerprint, hd_path));
				}
				let psbt = unlocker.psbt;

				println!(
					"Add this psbt to your wallet to fund the bitcoin:\n\n{}",
					general_purpose::STANDARD.encode(&psbt.serialize()[..])
				);
			},
		}
		Ok(())
	}
}

async fn get_bitcoin_network(
	client: &MainchainClient,
	at_block: Option<H256>,
) -> anyhow::Result<Network> {
	let network: BitcoinNetwork = client
		.fetch_storage(&storage().bitcoin_utxos().bitcoin_network(), at_block)
		.await?
		.ok_or(anyhow!("No bitcoin network found"))?
		.into();
	Ok(network.into())
}

async fn load_unlocker(
	client: &MainchainClient,
	utxo_id: UtxoId,
	utxo: &api::runtime_types::pallet_bond::pallet::UtxoState,
	at_block: Option<H256>,
) -> anyhow::Result<UtxoUnlocker> {
	let utxo_ref = client
		.fetch_storage(&storage().bitcoin_utxos().utxo_id_to_ref(utxo_id), at_block)
		.await?
		.ok_or(anyhow::anyhow!("No utxo ref found for bond"))?;
	let unlock_info = find_unlock_request(client, at_block, utxo_id)
		.await?
		.ok_or(anyhow!("No unlock request found"))?;
	let network = get_bitcoin_network(client, at_block).await?;

	let txid: Txid = H256Le(utxo_ref.txid.0).into();
	let pay_scriptpub: BitcoinScriptPubkey = unlock_info
		.to_script_pubkey
		.try_into()
		.map_err(|_| anyhow!("Unable to decode the destination pubkey"))?;
	let unlocker = UtxoUnlocker::from_script(
		get_cosign_script(utxo, network)?,
		utxo.satoshis,
		txid,
		utxo_ref.output_index,
		UnlockStep::VaultCosign,
		Amount::from_sat(unlock_info.bitcoin_network_fee),
		pay_scriptpub.into(),
	)?;
	Ok(unlocker)
}

async fn find_unlock_request(
	client: &MainchainClient,
	at_block: Option<H256>,
	utxo_id: UtxoId,
) -> anyhow::Result<Option<UtxoCosignRequest<u128>>> {
	let unlock_request = client
		.fetch_storage(&storage().bonds().utxos_pending_unlock_by_utxo_id(), at_block)
		.await?
		.map(|a| a.0)
		.unwrap_or_default()
		.into_iter()
		.find_map(|(a, unlock)| {
			if a == utxo_id {
				return Some(unlock)
			}
			None
		});
	Ok(unlock_request)
}
async fn get_utxo_from_bond_id(
	client: &MainchainClient,
	bond_id: BondId,
	at_block: Option<H256>,
) -> anyhow::Result<(
	UtxoId,
	api::runtime_types::pallet_bond::pallet::UtxoState,
	api::bonds::storage::types::bonds_by_id::BondsById,
)> {
	let query = storage().bonds().bonds_by_id(bond_id);
	let bond = client
		.fetch_storage(&query, at_block)
		.await?
		.ok_or(anyhow!("No finalized bond found"))?;
	let Some(utxo_id) = bond.utxo_id else {
		bail!("This isn't a bitcoin bond");
	};

	let utxo = client
		.fetch_storage(&storage().bonds().utxos_by_id(utxo_id), at_block)
		.await?
		.ok_or(anyhow!("No utxo found for bond"))?;
	Ok((utxo_id, utxo, bond))
}

fn get_cosign_script(
	utxo: &api::runtime_types::pallet_bond::pallet::UtxoState,
	network: Network,
) -> anyhow::Result<CosignScript> {
	let script_args = CosignScriptArgs {
		vault_pubkey: utxo.vault_pubkey.clone().into(),
		vault_claim_pubkey: utxo.vault_claim_pubkey.clone().into(),
		owner_pubkey: utxo.owner_pubkey.clone().into(),
		vault_claim_height: utxo.vault_claim_height,
		open_claim_height: utxo.open_claim_height,
		created_at_height: utxo.created_at_height,
	};

	Ok(CosignScript::new(script_args, network)?)
}
