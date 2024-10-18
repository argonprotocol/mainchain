use binary_merkle_tree::merkle_root;
use codec::Encode;
use jsonrpsee::{
	core::{async_trait, SubscriptionResult},
	server::{PendingSubscriptionSink, Server, ServerHandle, SubscriptionMessage},
	types::error::ErrorObjectOwned,
	RpcModule,
};
use sc_utils::notification::NotificationSender;
use sp_core::{ed25519::Signature, Blake2Hasher, H256};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::Mutex;

use argon_notary::server::{pipe_from_stream_and_drop, NotebookHeaderStream};
use argon_notary_apis::{
	localchain::{BalanceChangeResult, BalanceTipResult, LocalchainRpcServer},
	notebook::NotebookRpcServer,
};
use argon_primitives::{
	tick::Tick, AccountId, AccountOrigin, AccountType, BalanceProof, BalanceTip, BlockVote,
	Notarization, NotarizationBalanceChangeset, NotarizationBlockVotes, NotarizationDomains,
	Notebook, NotebookHeader, NotebookMeta, NotebookNumber, SignedNotebookHeader,
};

#[derive(Debug, Default)]
pub struct NotaryState {
	pub headers: HashMap<NotebookNumber, SignedNotebookHeader>,
	pub metadata: Option<NotebookMeta>,
}

#[derive(Clone)]
pub struct MockNotary {
	server_handle: Option<ServerHandle>,
	pub addr: String,
	pub notary_id: u32,
	pub state: Arc<Mutex<NotaryState>>,
	pub header_channel: (NotificationSender<SignedNotebookHeader>, NotebookHeaderStream),
}

impl MockNotary {
	pub fn new(notary_id: u32) -> Self {
		Self {
			notary_id,
			state: Default::default(),
			header_channel: NotebookHeaderStream::channel(),
			server_handle: None,
			addr: String::new(),
		}
	}

	#[allow(dead_code)]
	pub async fn stop(&mut self) {
		if let Some(server_handle) = self.server_handle.take() {
			server_handle.stop().expect("Should be able to stop server");
			server_handle.stopped().await;
		}
	}

	pub async fn start(&mut self) -> anyhow::Result<()> {
		let server = Server::builder().build("127.0.0.1:0").await?;
		self.addr = format!("ws://{:?}", server.local_addr()?);

		let mut module = RpcModule::new(());
		module.merge(NotebookRpcServer::into_rpc(self.clone()))?;
		module.merge(LocalchainRpcServer::into_rpc(self.clone()))?;

		let handle = server.start(module);
		self.server_handle = Some(handle.clone());
		// handle in background
		tokio::spawn(handle.stopped());

		Ok(())
	}

	pub async fn add_notebook_header(&self, header: SignedNotebookHeader) {
		let mut state = self.state.lock().await;
		state.headers.insert(header.header.notebook_number, header.clone());
		state.metadata = Some(NotebookMeta {
			finalized_tick: header.header.tick,
			finalized_notebook_number: header.header.notebook_number,
		});
		drop(state);
		let _ = self.header_channel.0.notify(|| Ok::<_, anyhow::Error>(header));
	}

	pub async fn next_details(&self) -> (NotebookNumber, Tick) {
		let state = self.state.lock().await;
		let mut notebook_number = 0u32;
		let mut last_tick = 0u32;
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

	async fn get_origin(
		&self,
		_account_id: AccountId,
		_account_type: AccountType,
	) -> Result<AccountOrigin, ErrorObjectOwned> {
		todo!()
	}

	async fn get_tip(
		&self,
		_account_id: AccountId,
		_account_type: AccountType,
	) -> Result<BalanceTipResult, ErrorObjectOwned> {
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

	async fn get_header(
		&self,
		notebook_number: NotebookNumber,
	) -> Result<SignedNotebookHeader, ErrorObjectOwned> {
		let state = self.state.lock().await;
		state.headers.get(&notebook_number).cloned().ok_or_else(|| {
			ErrorObjectOwned::owned(-32000, "MockNotary header not set".to_string(), None::<String>)
		})
	}

	async fn get_raw_headers(
		&self,
		since_notebook: Option<NotebookNumber>,
		list: Option<Vec<NotebookNumber>>,
	) -> Result<Vec<(NotebookNumber, Vec<u8>)>, ErrorObjectOwned> {
		let state = self.state.lock().await;

		Ok(state
			.headers
			.iter()
			.filter_map(|(notebook_number, header)| {
				if let Some(since_notebook) = since_notebook {
					if *notebook_number > since_notebook {
						Some((*notebook_number, header.encode()))
					} else {
						None
					}
				} else if let Some(list) = &list {
					if list.contains(notebook_number) {
						Some((*notebook_number, header.encode()))
					} else {
						None
					}
				} else {
					None
				}
			})
			.collect())
	}

	async fn get(&self, notebook_number: NotebookNumber) -> Result<Notebook, ErrorObjectOwned> {
		let state = self.state.lock().await;
		let header = state.headers.get(&notebook_number).cloned().ok_or_else(|| {
			ErrorObjectOwned::owned(-32000, "MockNotary header not set".to_string(), None::<String>)
		})?;
		Ok(Notebook {
			header: header.header,
			signature: header.signature,
			notarizations: Default::default(),
			hash: H256::random(),
			new_account_origins: Default::default(),
		})
	}

	async fn get_raw_body(
		&self,
		notebook_number: NotebookNumber,
	) -> Result<Vec<u8>, ErrorObjectOwned> {
		let result = self.get(notebook_number).await?;
		Ok(result.encode())
	}

	async fn subscribe_headers(
		&self,
		subscription_sink: PendingSubscriptionSink,
	) -> SubscriptionResult {
		let stream = self.header_channel.1.subscribe(1_000);
		pipe_from_stream_and_drop(subscription_sink, stream, |a| {
			SubscriptionMessage::from_json(&a).map_err(Into::into)
		})
		.await
		.map_err(Into::into)
	}

	async fn subscribe_raw_headers(
		&self,
		subscription_sink: PendingSubscriptionSink,
	) -> SubscriptionResult {
		let stream = self.header_channel.1.subscribe(1_000);
		pipe_from_stream_and_drop(subscription_sink, stream, |item| {
			SubscriptionMessage::from_json(&(item.header.notebook_number, item.encode()))
				.map_err(Into::into)
		})
		.await
		.map_err(Into::into)
	}
}
