use binary_merkle_tree::merkle_root;
use codec::Encode;
use jsonrpsee::{
	RpcModule,
	core::{SubscriptionResult, async_trait},
	server::{PendingSubscriptionSink, Server, ServerHandle, SubscriptionMessage},
	types::error::ErrorObjectOwned,
};
use polkadot_sdk::*;
use sc_utils::notification::NotificationSender;
use sp_core::{Blake2Hasher, H256, ed25519::Signature};
use std::{collections::HashMap, str::FromStr, sync::Arc};
use tokio::sync::Mutex;

use argon_notary::server::{NotebookHeaderInfo, NotebookHeaderStream, pipe_from_stream_and_drop};
use argon_notary_apis::{
	get_header_url, get_notebook_url,
	localchain::{BalanceChangeResult, BalanceTipResult, LocalchainRpcServer},
	notebook::{NotebookRpcServer, NotebookSubscriptionBroadcast},
	system::SystemRpcServer,
};
use argon_primitives::{
	AccountId, AccountOrigin, AccountType, BalanceProof, BalanceTip, BlockVote, Notarization,
	NotarizationBalanceChangeset, NotarizationBlockVotes, NotarizationDomains, Notebook,
	NotebookHeader, NotebookMeta, NotebookNumber, SignedNotebookHeader, tick::Tick,
};
use axum::{
	Router,
	body::Bytes,
	extract::{Path, State},
	http::StatusCode,
	response::IntoResponse,
	routing::get,
};
use env_logger::{Builder, Env};
use tokio::task::JoinHandle;

pub fn setup_logs() {
	let env = Env::new().default_filter_or("node=debug"); //info,sync=debug,sc_=debug,sub-libp2p=debug,node=debug,runtime=debug");
	let _ = Builder::from_env(env).is_test(true).try_init();
	sp_tracing::try_init_simple();
}

#[derive(Debug, Default)]
pub struct NotaryState {
	pub headers: HashMap<NotebookNumber, SignedNotebookHeader>,
	pub metadata: Option<NotebookMeta>,
}

#[derive(Clone)]
pub struct MockNotary {
	server_handle: Option<ServerHandle>,
	archive_server_handle: Arc<Mutex<Option<JoinHandle<()>>>>,
	pub addr: String,
	pub archive_host: String,
	pub notary_id: u32,
	pub state: Arc<Mutex<NotaryState>>,
	pub header_channel: (NotificationSender<NotebookHeaderInfo>, NotebookHeaderStream),
}

impl MockNotary {
	pub fn new(notary_id: u32) -> Self {
		Self {
			notary_id,
			state: Default::default(),
			header_channel: NotebookHeaderStream::channel(),
			server_handle: None,
			addr: String::new(),
			archive_host: String::new(),
			archive_server_handle: Default::default(),
		}
	}

	#[allow(dead_code)]
	pub async fn stop(&mut self) {
		if let Some(server_handle) = self.server_handle.take() {
			server_handle.stop().expect("Should be able to stop server");
			server_handle.stopped().await;
		}
		if let Some(handle) = self.archive_server_handle.lock().await.take() {
			handle.abort();
		}
	}

	pub async fn start(&mut self) -> anyhow::Result<()> {
		let notary_id = self.notary_id;
		// build our application with a route
		let archive_server = Router::new()
			// `GET /` goes to `root`
			.route(
				&format!("/notary/{notary_id}/notebook/{{notebook_number}}"),
				get(Self::get_notebook),
			)
			.route(
				&format!("/notary/{notary_id}/header/{{notebook_number}}"),
				get(Self::get_header),
			)
			.with_state(Arc::new(self.clone()));

		let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
		self.archive_host = format!("http://127.0.0.1:{}", listener.local_addr()?.port());
		let archive_server_handle = tokio::spawn(async move {
			axum::serve(listener, archive_server).await.expect("Should be able to serve");
		});

		self.archive_server_handle = Arc::new(Mutex::new(Some(archive_server_handle)));

		let server = Server::builder().build("127.0.0.1:0").await?;
		self.addr = format!("ws://{:?}", server.local_addr()?);

		let mut module = RpcModule::new(());
		module.merge(NotebookRpcServer::into_rpc(self.clone()))?;
		module.merge(LocalchainRpcServer::into_rpc(self.clone()))?;
		module.merge(SystemRpcServer::into_rpc(self.clone()))?;

		let handle = server.start(module);
		self.server_handle = Some(handle.clone());
		// handle in background
		tokio::spawn(handle.stopped());

		Ok(())
	}

	pub async fn add_notebook_header(&self, header: SignedNotebookHeader) {
		let hash = header.header.hash();
		{
			let mut state = self.state.lock().await;
			state.headers.insert(header.header.notebook_number, header.clone());
			state.metadata = Some(NotebookMeta {
				last_closed_notebook_tick: header.header.tick,
				last_closed_notebook_number: header.header.notebook_number,
			});
		}
		let _ = self
			.header_channel
			.0
			.notify(|| Ok::<NotebookHeaderInfo, anyhow::Error>((header, hash)))
			.inspect_err(|e| println!("Failed to notify header: {:?}", e));
	}

	pub async fn next_details(&self) -> (NotebookNumber, Tick) {
		let state = self.state.lock().await;
		let mut notebook_number = 0u32;
		let mut last_tick = 0;
		for (num, header) in state.headers.iter() {
			if num > &notebook_number {
				notebook_number = *num;
				last_tick = header.header.tick;
			}
		}
		notebook_number += 1u32;

		(notebook_number, last_tick + 1)
	}

	pub async fn create_notebook_header(&self, votes: Vec<BlockVote>) -> NotebookHeader {
		let merkle_leafs = votes.iter().map(|x| x.encode()).collect::<Vec<_>>();
		let block_votes_root = merkle_root::<Blake2Hasher, _>(&merkle_leafs.clone());
		let (notebook_number, tick) = self.next_details().await;

		let notebook_header = NotebookHeader {
			version: 1,
			notary_id: self.notary_id,
			notebook_number,
			tick,
			tax: 0,
			domains: Default::default(),
			block_votes_count: votes.len() as u32,
			block_voting_power: votes.iter().map(|x| x.power).sum(),
			parent_secret: None,
			block_votes_root,
			changed_account_origins: Default::default(),
			blocks_with_votes: Default::default(),
			secret_hash: H256::random(),
			chain_transfers: Default::default(),
			changed_accounts_root: H256::random(),
		};
		self.add_notebook_header(SignedNotebookHeader {
			header: notebook_header.clone(),
			signature: Signature::from_raw([0u8; 64]),
		})
		.await;
		notebook_header
	}

	async fn get_header(
		State(state): State<Arc<Self>>,
		Path(param): Path<String>,
	) -> impl IntoResponse {
		let notebook_number = NotebookNumber::from_str(param.replace(".scale", "").as_str())
			.expect("Failed to parse notebook number");
		let state = state.state.lock().await;
		if let Some(body) = state.headers.get(&notebook_number).map(|x| x.encode()) {
			return (StatusCode::OK, Bytes::from(body));
		}
		(StatusCode::NOT_FOUND, Bytes::from("Not found"))
	}

	async fn get_notebook(
		State(state): State<Arc<Self>>,
		Path(param): Path<String>,
	) -> impl IntoResponse {
		let notebook_number = NotebookNumber::from_str(param.replace(".scale", "").as_str())
			.expect("Failed to parse notebook number");
		let state = state.state.lock().await;
		if let Some(header) = state.headers.get(&notebook_number).cloned() {
			let notebook = Notebook {
				header: header.header,
				signature: header.signature,
				notarizations: Default::default(),
				hash: H256::random(),
				new_account_origins: Default::default(),
			};
			let body = notebook.encode();
			return (StatusCode::OK, Bytes::from(body));
		}
		(StatusCode::NOT_FOUND, Bytes::from("Not found"))
	}
}

#[async_trait]
impl LocalchainRpcServer for MockNotary {
	async fn notarize(
		&self,
		_balance_changeset: NotarizationBalanceChangeset,
		_block_votes: NotarizationBlockVotes,
		_domains: NotarizationDomains,
	) -> Result<BalanceChangeResult, ErrorObjectOwned> {
		todo!()
	}

	async fn get_tip(
		&self,
		_account_id: AccountId,
		_account_type: AccountType,
	) -> Result<BalanceTipResult, ErrorObjectOwned> {
		todo!()
	}

	async fn get_origin(
		&self,
		_account_id: AccountId,
		_account_type: AccountType,
	) -> Result<AccountOrigin, ErrorObjectOwned> {
		todo!()
	}
}

#[async_trait]
impl NotebookRpcServer for MockNotary {
	async fn get_balance_proof(
		&self,
		_notebook_number: NotebookNumber,
		_balance_tip: BalanceTip,
	) -> Result<BalanceProof, ErrorObjectOwned> {
		todo!()
	}

	async fn get_notarization(
		&self,
		_account_id: AccountId,
		_account_type: AccountType,
		_notebook_number: NotebookNumber,
		_change_number: u32,
	) -> Result<Notarization, ErrorObjectOwned> {
		todo!()
	}

	async fn metadata(&self) -> Result<NotebookMeta, ErrorObjectOwned> {
		let state = self.state.lock().await;
		state.metadata.clone().ok_or_else(|| {
			ErrorObjectOwned::owned(
				-32000,
				"MockNotary metadata not set".to_string(),
				None::<String>,
			)
		})
	}

	async fn get_header_download_url(
		&self,
		notebook_number: NotebookNumber,
	) -> Result<String, ErrorObjectOwned> {
		Ok(get_header_url(&self.archive_host, self.notary_id, notebook_number))
	}

	async fn get_notebook_download_url(
		&self,
		notebook_number: NotebookNumber,
	) -> Result<String, ErrorObjectOwned> {
		Ok(get_notebook_url(&self.archive_host, self.notary_id, notebook_number))
	}

	async fn subscribe_headers(
		&self,
		subscription_sink: PendingSubscriptionSink,
	) -> SubscriptionResult {
		let stream = self.header_channel.1.subscribe(1_000);

		pipe_from_stream_and_drop(subscription_sink, stream, |a| {
			let a = NotebookSubscriptionBroadcast::build(a, &self.archive_host);
			SubscriptionMessage::from_json(&a).map_err(Into::into)
		})
		.await
		.map_err(Into::into)
	}
}

#[async_trait]
impl SystemRpcServer for MockNotary {
	async fn get_archive_base_url(&self) -> Result<String, ErrorObjectOwned> {
		Ok(self.archive_host.clone())
	}
	async fn health(&self) -> Result<(), ErrorObjectOwned> {
		Ok(())
	}
}
