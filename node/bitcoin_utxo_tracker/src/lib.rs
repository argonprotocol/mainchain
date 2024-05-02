#![allow(dead_code)]

use std::{
	collections::BTreeMap,
	net::{SocketAddr, TcpStream},
	path::PathBuf,
	sync::Arc,
};

use bitcoin::Script;
use codec::Codec;
pub use nakamoto_client;
use nakamoto_client::{chan::Receiver, traits::Handle as HandleT, Client, Config, Event};
use nakamoto_common::{
	bitcoin_hashes::Hash,
	network::{Network, Services},
};
use sc_service::TaskManager;
use sp_api::ProvideRuntimeApi;
use sp_runtime::traits::{Block as BlockT, NumberFor};

use ulx_primitives::{
	bitcoin::{
		BitcoinNetwork, BitcoinRejectedReason, BitcoinSyncStatus, BitcoinUtxo, H256Le, LockedUtxo,
		UtxoLookup,
	},
	inherents::BitcoinUtxoSync,
	AccountId, Balance, BitcoinApis, BlockNumber, BondId,
};

type Reactor = nakamoto_net_poll::Reactor<TcpStream>;
type Waker = <nakamoto_net_poll::Reactor<TcpStream> as nakamoto_net::Reactor>::Waker;
type Handle = nakamoto_client::Handle<Waker>;

pub type BitcoinHeight = nakamoto_common::block::Height;

pub struct UtxoTracker {
	receiver: Receiver<Event>,
	handle: Handle,
}

pub fn get_bitcoin_inherent<C, B, A>(
	tracker: &Arc<UtxoTracker>,
	client: &Arc<C>,
	block_hash: &B::Hash,
) -> anyhow::Result<Option<BitcoinUtxoSync>>
where
	B: BlockT,
	C: ProvideRuntimeApi<B> + 'static,
	C::Api: BitcoinApis<B, A, BondId, Balance, NumberFor<B>>,
	A: Codec + Clone,
{
	let api = client.runtime_api();
	let Some(sync_status) = api.get_sync_status(*block_hash)? else {
		return Ok(None);
	};

	let utxos = api.active_utxos(*block_hash)?;
	Ok(Some(tracker.sync(sync_status, utxos)?))
}

type DefaultLockedUtxo = LockedUtxo<AccountId, BondId, Balance, BlockNumber>;

impl UtxoTracker {
	pub fn new(
		network: BitcoinNetwork,
		peers: Vec<SocketAddr>,
		storage_dir: PathBuf,
		task_handle: &TaskManager,
	) -> anyhow::Result<Self> {
		let client = Client::<Reactor>::new()?;

		let handle = client.handle();
		let client_recv = handle.events();
		let network: Network = match network {
			BitcoinNetwork::Mainnet => Network::Mainnet,
			BitcoinNetwork::Testnet => Network::Testnet,
			BitcoinNetwork::Signet => Network::Signet,
			BitcoinNetwork::Regtest => Network::Regtest,
		};
		task_handle.spawn_essential_handle().spawn_blocking(
			"bitcoin-utxo-monitor",
			None,
			async move {
				match client.run(Config {
					listen: vec![], // Don't listen for incoming connections.
					connect: peers,
					root: storage_dir,
					network,
					..Config::default()
				}) {
					Err(e) => panic!("Bitcoin synching failed! {:?}", e),
					Ok(_) => {},
				}
			},
		);

		handle.wait_for_peers(1, Services::All)?;

		Ok(Self { receiver: client_recv, handle })
	}

	/// Synchronize with the latest blocks on the network.
	pub fn sync(
		&self,
		sync_status: BitcoinSyncStatus,
		utxos: BTreeMap<BitcoinUtxo, UtxoLookup>,
	) -> anyhow::Result<BitcoinUtxoSync> {
		let mut scripts = vec![];

		let confirmed_block = self
			.handle
			.get_block_by_height(sync_status.confirmed_block.block_height)?
			.ok_or(anyhow::anyhow!(
				"Failed to get block by height: {:?}",
				sync_status.confirmed_block.block_height
			))?;
		if confirmed_block.block_hash().into_inner() != sync_status.confirmed_block.block_hash.0 {
			return Err(anyhow::anyhow!(
				"Latest confirmed block hash mismatch: {:?} != {:?}. Could be on a different chain, so aborting.",
				confirmed_block.block_hash().into_inner(),
				sync_status.confirmed_block.block_hash.0
			));
		}

		let mut start_block = sync_status.oldest_allowed_block_height;

		if let Some(synched_block) = &sync_status.synched_block {
			let start_block_header = self
				.handle
				.get_block_by_height(synched_block.block_height)?
				.ok_or(anyhow::anyhow!(
				"Failed to get block by height: {:?}",
				synched_block.block_height
			))?;
			// if this block is a different hash, we've re-orged, so go back to the oldest allowed
			// block height to check for moves
			if start_block_header.block_hash().into_inner() != synched_block.block_hash.0 {
				start_block = sync_status.oldest_allowed_block_height;
			} else {
				start_block = synched_block.block_height;
			}
		}

		let mut pending_confirmation_txids = BTreeMap::new();
		for (id, looukup) in &utxos {
			let script: Script = looukup.script_pubkey.to_vec().into();
			if let Some((satoshis, height)) = looukup.pending_confirmation {
				pending_confirmation_txids.entry(id.txid).or_insert(Vec::new()).push((
					id.clone(),
					satoshis,
					script.clone(),
				));
				start_block = start_block.min(height as BitcoinHeight);
			}
			scripts.push(script);
		}

		let mut result = BitcoinUtxoSync {
			sync_to_block: sync_status.confirmed_block.clone(),
			verified: BTreeMap::new(),
			invalid: BTreeMap::new(),
			spent: BTreeMap::new(),
		};

		self.handle.rescan(
			(start_block as u64)..sync_status.confirmed_block.block_height,
			scripts.into_iter(),
		)?;

		loop {
			match self.receiver.recv()? {
				Event::BlockMatched { height, transactions, .. } => {
					for tx in transactions {
						let tx_id = H256Le(tx.txid().into_inner());

						for input in tx.input {
							let utxo_id = BitcoinUtxo {
								txid: H256Le(input.previous_output.txid.into_inner()),
								output_index: input.previous_output.vout,
							};
							// If we're tracking the UTXO, it has been spent
							if utxos.contains_key(&utxo_id) {
								// TODO: should we figure out who spent it here?
								result.spent.insert(utxo_id, height as u64);
							}
						}

						// only take a look at the outputs for the transactions we're waiting for
						let Some(pending) = pending_confirmation_txids.get(&tx_id) else {
							continue;
						};

						for (idx, output) in tx.output.iter().enumerate() {
							let utxo_id = BitcoinUtxo { txid: tx_id, output_index: idx as u32 };
							if let Some((_id, satoshis, script)) =
								pending.iter().find(|(id, _, _)| id == &utxo_id)
							{
								let is_satoshi_match = output.value == *satoshis;
								let is_script_pubkey_match = output.script_pubkey == *script;
								let is_age_appropriate =
									height as u64 > sync_status.oldest_allowed_block_height;

								if is_satoshi_match && is_script_pubkey_match && is_age_appropriate
								{
									result.verified.insert(utxo_id, height as u64);
								} else {
									let reason = if !is_satoshi_match {
										BitcoinRejectedReason::SatoshisMismatch
									} else if !is_age_appropriate {
										BitcoinRejectedReason::TooOld
									} else {
										BitcoinRejectedReason::ScriptPubkeyMismatch
									};
									result.invalid.insert(utxo_id, reason);
								}
							}
						}
					}
				},

				Event::Synced { height, .. } =>
					if height == sync_status.confirmed_block.block_height {
						break;
					},
				_ => { /* ignore */ },
			}
		}

		Ok(result)
	}

	// Destroys self
	pub fn shutdown(self) -> anyhow::Result<()> {
		Ok(self.handle.shutdown()?)
	}
}
