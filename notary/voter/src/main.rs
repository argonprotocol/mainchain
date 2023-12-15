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
use sp_core::{bounded_vec, crypto::Ss58Codec, H256};
use sp_keystore::KeystorePtr;
use sp_runtime::BoundedVec;
use subxt::utils::AccountId32;
use tokio::time;

use ulixee_client::{try_until_connected, UlxClient};
use ulx_notary::apis::LocalchainRpcClient;
use ulx_primitives::{AccountId, BlockVote, NotaryId, MAX_BLOCK_VOTES_PER_NOTARIZATION};

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

	let mut vote_publisher =
		VotePublisher::new(AccountId32::from(account_id_32), mainchain_client.clone());

	let mut blocks_sub = mainchain_client.blocks().subscribe_best().await?;
	let mut interval = time::interval(Duration::from_secs(1));
	loop {
		tokio::select! {biased;
			block_next = blocks_sub.next() => {
				match block_next {
					Some(Ok(ref block)) => {
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

					let votes = take(&mut meta.votes);
					let (publish, retain) = votes.split_at(MAX_BLOCK_VOTES_PER_NOTARIZATION as usize);
					meta.votes.append(&mut retain.to_vec());
					pending_votes.push( publish.to_vec());

					true
				});

				for votes in pending_votes {
					let task = VotePublisher::publish_pending_votes(votes, notary_client.clone());
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
#[allow(dead_code)]
pub struct VotePublisher {
	account_id: AccountId32,
	mainchain_client: UlxClient,
	votes_by_block_hash: BTreeMap<H256, BlockVoteMeta>,
}

pub struct BlockVoteMeta {
	votes: Vec<BlockVote>,
	expiration_time: Instant,
	last_publish_time: Instant,
}

impl VotePublisher {
	pub fn new(account_id: AccountId32, mainchain_client: UlxClient) -> Self {
		Self { votes_by_block_hash: BTreeMap::new(), mainchain_client, account_id }
	}

	pub async fn on_block(&mut self, block_hash: H256) -> anyhow::Result<()> {
		self.votes_by_block_hash.insert(
			block_hash.clone(),
			BlockVoteMeta {
				votes: Vec::new(),
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
		notary_client: Arc<ulx_notary::Client>,
	) -> anyhow::Result<()> {
		if votes.is_empty() {
			return Ok(())
		}

		tracing::info!("Publishing {} votes for {}", votes.len(), votes[0].grandparent_block_hash,);

		notary_client
			.notarize(bounded_vec!(), BoundedVec::truncate_from(votes.clone()))
			.await?;
		Ok(())
	}
}
