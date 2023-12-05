use std::{
	collections::BTreeMap,
	env,
	mem::take,
	path::PathBuf,
	sync::Arc,
	time::{Duration, Instant},
};

use anyhow::anyhow;
use clap::{crate_version, Parser};
use sc_cli::KeystoreParams;
use sc_keystore::LocalKeystore;
use sc_service::config::KeystoreConfig;
use serde::{Deserialize, Serialize};
use sp_core::{bounded_vec, crypto::Ss58Codec, H256};
use sp_keystore::KeystorePtr;
use sp_runtime::BoundedVec;
use subxt::{backend::rpc::RpcClient, rpc_params, utils::AccountId32};
use tokio::time;

use ulixee_client::{
	api::{
		apis,
		runtime_types::ulx_primitives::block_seal::{app::Public, Host, MiningAuthority},
	},
	try_until_connected, UlxClient,
};
use ulx_notary::apis::LocalchainRpcClient;
use ulx_notary_primitives::{AccountId, BlockVote, NotaryId, MAX_BLOCK_VOTES_PER_NOTARIZATION};

#[derive(Parser, Debug)]
#[clap(version = crate_version!())]
#[command(author, version, about, arg_required_else_help = true, long_about = None)]
struct Cli {
	#[allow(missing_docs)]
	#[clap(flatten)]
	keystore_params: KeystoreParams,

	#[clap(long, env = "ULX_NOTARY_BASE_PATH")]
	base_path: Option<PathBuf>,

	/// What mainchain RPC websocket url do you want to reach out use to sync blocks and submit
	/// notebook?
	#[clap(short, long, env, default_value = "ws://127.0.0.1:9944")]
	trusted_rpc_url: String,

	/// What mainchain RPC websocket url do you want to reach out use to sync blocks and submit
	/// notebook?
	#[clap(short, long, env, default_value = "ws://127.0.0.1:9944")]
	notary_url: String,

	/// What notary id will be used to create votes
	#[clap(short, long, env, default_value = "1")]
	notary_id: NotaryId,

	/// What account id will be used to create votes
	#[clap(long, value_name = "SS58_ADDRESS")]
	account_id: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
	let cli = Cli::parse();
	tracing_subscriber::FmtSubscriber::builder()
		.with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
		.try_init()
		.expect("setting default subscriber failed");

	let mainchain_client = UlxClient::from_url(cli.trusted_rpc_url.clone()).await?;
	let notary_client = ulx_notary::create_client(cli.notary_url.as_str()).await?;
	let notary_client = Arc::new(notary_client);
	// let keystore = read_keystore(cli.base_path.clone(), cli.keystore_params)?;

	let account_id: AccountId = Ss58Codec::from_ss58check(&cli.account_id)?;
	let account_id_32: [u8; 32] = account_id.clone().into();

	let notary_id = cli.notary_id;
	let mut vote_publisher =
		VotePublisher::new(AccountId32::from(account_id_32), mainchain_client.clone());

	let mut blocks_sub = mainchain_client.blocks().subscribe_best().await?;
	let mut interval = time::interval(Duration::from_secs(1));
	loop {
		tokio::select! {biased;
			block_next = blocks_sub.next() => {
				match block_next {
					Some(Ok(ref block)) => {
						// if let Some(next_eligibility) = get_eligibility_digest(&block).ok() {
						// 	worker.on_build(block.hash(), next_eligibility);
						let _ = vote_publisher.on_block(block.hash().clone()).await.map_err(|e| {
							tracing::error!("Error processing block: {:?}", e);
						});
					},
					Some(Err(ref e)) => {
						tracing::error!("Error polling best blocks: {:?}", e);
						if let Some(client) = try_until_connected(cli.trusted_rpc_url.clone(), 2500).await.ok() {
							tracing::error!("Reconnected to mainchain block polling");
							blocks_sub = client.blocks().subscribe_best().await?;
							vote_publisher.mainchain_client = client;
						}
					},
					None => break,
				}
			},
			// voter = vote_receiver.next() => {
			// 	match voter {
			// 		Some(puzzle) => {
			// 			vote_publisher.append_vote(puzzle.into());
			// 		},
			// 		None => continue,
			// 	}
			// },
			_ = interval.tick() => {
				let mut pending_votes = Vec::new();
				vote_publisher.votes_by_block_hash.retain(|_, meta|  {
					if meta.votes.is_empty() {
						return true
					}
					if meta.expiration_time > Instant::now() {
						return false
					}

					let mut should_publish = false;
					if Instant::now() - meta.expiration_time < Duration::from_secs(1) {
						should_publish = true;
					}

					if Instant::now() - meta.last_publish_time < Duration::from_secs(10) {
						should_publish = true;
					}

					if !should_publish {
						return true
					}

					meta.last_publish_time = Instant::now();

					let host = meta.peer.rpc_hosts.0.first().expect("Must have at least one host");
					let rpc_url = get_rpc_url(&host);
					let votes = take(&mut meta.votes);
					let (publish, retain) = votes.split_at(MAX_BLOCK_VOTES_PER_NOTARIZATION as usize);
					meta.votes.append(&mut retain.to_vec());
					pending_votes.push(( publish.to_vec(), rpc_url));

					true
				});

				for (votes, rpc_url) in pending_votes {
					let task = VotePublisher::publish_pending_votes(votes, rpc_url, notary_id, notary_client.clone());
					tokio::spawn(task);
				}
			},
		}
	}

	Ok(())
}

#[allow(dead_code)]
fn read_keystore(
	base_path: Option<PathBuf>,
	keystore_params: KeystoreParams,
) -> anyhow::Result<KeystorePtr> {
	let keystore: KeystorePtr = match &base_path {
		Some(r) => {
			let base_path = r.clone();
			match keystore_params.keystore_config(&base_path)? {
				KeystoreConfig::Path { path, password } =>
					Ok(LocalKeystore::open(path, password)?.into()),
				_ => unreachable!("keystore_config always returns path and password; qed"),
			}
		},
		None => Err("No base path provided"),
	}
	.map_err(|e| anyhow!("Error reading keystore details {:?}", e))?;
	Ok(keystore)
}

pub struct VotePublisher {
	account_id: AccountId32,
	mainchain_client: UlxClient,
	votes_by_block_hash: BTreeMap<H256, BlockVoteMeta>,
}

pub struct BlockVoteMeta {
	votes: Vec<BlockVote>,
	peer: MiningAuthority<Public>,
	expiration_time: Instant,
	last_publish_time: Instant,
}

impl VotePublisher {
	pub fn new(account_id: AccountId32, mainchain_client: UlxClient) -> Self {
		Self { votes_by_block_hash: BTreeMap::new(), mainchain_client, account_id }
	}

	pub async fn on_block(&mut self, block_hash: H256) -> anyhow::Result<()> {
		let peer = self
			.mainchain_client
			.runtime_api()
			.at(block_hash.clone())
			.call(apis().mining_authority_apis().block_peer(self.account_id.clone()))
			.await?
			.ok_or(anyhow!("Could not find block peer for block {:?}", block_hash))?;

		self.votes_by_block_hash.insert(
			block_hash.clone(),
			BlockVoteMeta {
				votes: Vec::new(),
				peer,
				expiration_time: Instant::now() + Duration::from_secs(55),
				last_publish_time: Instant::now() - Duration::from_secs(5),
			},
		);

		Ok(())
	}

	pub fn append_vote(&mut self, vote: BlockVote) -> usize {
		let block_hash = vote.grandparent_block_hash;
		if let Some(votes) = self.votes_by_block_hash.get_mut(&block_hash) {
			votes.votes.push(vote);
			return votes.votes.len()
		}
		return 0
	}

	pub async fn publish_pending_votes(
		votes: Vec<BlockVote>,
		rpc_url: String,
		notary_id: NotaryId,
		notary_client: Arc<ulx_notary::Client>,
	) -> anyhow::Result<()> {
		if votes.is_empty() {
			return Ok(())
		}

		tracing::info!(
			"Publishing {} votes for {} to {}",
			votes.len(),
			votes[0].grandparent_block_hash,
			rpc_url
		);

		let _ = {
			notary_client
				.notarize(bounded_vec!(), BoundedVec::truncate_from(votes.clone()))
				.await?;
			let peer_client = RpcClient::from_url(rpc_url).await?;
			// must submit to first client
			let response = peer_client
				.request::<Response>("blockVotes_submit", rpc_params![notary_id, votes])
				.await?;

			if response.accepted {
				Ok(())
			} else {
				Err(anyhow!("Peer did not accept votes"))
			}
		}
		.map_err(|e| {
			tracing::error!("Error publishing votes: {:?}", e);
			e
		});

		Ok(())
	}
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct Response {
	/// hash of the created block.
	pub accepted: bool,
}

fn get_rpc_url(host: &Host) -> String {
	format!(
		"{}://{}:{}",
		if host.is_secure { "wss" } else { "ws" },
		std::net::Ipv4Addr::from(host.ip),
		host.port
	)
}

// fn get_eligibility_digest(
// 	block: &subxt::blocks::Block<UlxConfig, UlxClient>,
// ) -> anyhow::Result<ulx_primi::BlockS> {
// 	let next = block
// 		.header()
// 		.digest
// 		.logs
// 		.iter()
// 		.find_map(|log| {
// 			if let DigestItem::PreRuntime(ulx_notary_primitives::NEXT_SEAL_MINIMUMS_DIGEST_ID, v) =
// 				log
// 			{
// 				return ulx_notary_primitives::BlockVoteEligibility::decode(&mut &v[..]).ok()
// 			}
// 			None
// 		})
// 		.ok_or(anyhow!("Could not local the next block eligibility!!"))?;
// 	Ok(next)
// }
