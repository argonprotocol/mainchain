use std::net::SocketAddr;

use futures::{Stream, StreamExt};
use jsonrpsee::{
	core::{async_trait, SubscriptionResult},
	server::{PendingSubscriptionSink, Server, SubscriptionMessage},
	types::ErrorObjectOwned,
	RpcModule, TrySendError,
};
use serde::Serialize;
use sp_core::{bounded::BoundedVec, ConstU32};
use tokio::net::ToSocketAddrs;

use ulx_notary_primitives::{
	BalanceChange, BalanceProof, BalanceTip, NotebookNumber, MAX_BALANCESET_CHANGES,
};

use crate::{
	apis::{
		localchain::{BalanceChangeResult, LocalchainRpcServer},
		notebook::NotebookRpcServer,
	},
	notary::Notary,
	stores::notebook::NotebookStore,
};

struct NotebookRpcServerImpl {
	notary: Notary,
}
fn from_crate_error(e: crate::Error) -> ErrorObjectOwned {
	let msg = e.to_string();
	let code: i32 = Into::<i32>::into(e);
	ErrorObjectOwned::owned(code, msg, None::<String>)
}

#[async_trait]
impl NotebookRpcServer for NotebookRpcServerImpl {
	async fn get_balance_proof(
		&self,
		notebook_number: NotebookNumber,
		balance_tip: BalanceTip,
	) -> Result<BalanceProof, ErrorObjectOwned> {
		let pool = &self.notary.pool;
		let notary_id = self.notary.notary_id;
		NotebookStore::get_balance_proof(pool, notary_id, notebook_number, &balance_tip)
			.await
			.map_err(from_crate_error)
	}
	async fn subscribe_headers(&self, pending: PendingSubscriptionSink) -> SubscriptionResult {
		let stream = self.notary.completed_notebook_stream.subscribe(1_000);

		pipe_from_stream_and_drop(pending, stream).await.map_err(Into::into)
	}
}
struct LocalchainRpcServerImpl {
	notary: Notary,
}

#[async_trait]
impl LocalchainRpcServer for LocalchainRpcServerImpl {
	async fn notarize(
		&self,
		balance_changeset: BoundedVec<BalanceChange, ConstU32<MAX_BALANCESET_CHANGES>>,
	) -> Result<BalanceChangeResult, ErrorObjectOwned> {
		self.notary
			.apply_balance_changes(balance_changeset.into_inner())
			.await
			.map_err(from_crate_error)
	}
}

pub async fn run_server(notary: &Notary, addrs: impl ToSocketAddrs) -> anyhow::Result<SocketAddr> {
	let server = Server::builder().build(addrs).await?;

	let addr = server.local_addr()?;
	let mut module = RpcModule::new(());
	module.merge(LocalchainRpcServerImpl { notary: notary.clone() }.into_rpc())?;
	module.merge(NotebookRpcServerImpl { notary: notary.clone() }.into_rpc())?;

	let handle = server.start(module);

	// In this example we don't care about doing shutdown so let's it run forever.
	// You may use the `ServerHandle` to shut it down or manage it yourself.
	tokio::spawn(handle.stopped());

	Ok(addr)
}

async fn pipe_from_stream_and_drop<T: Serialize>(
	pending: PendingSubscriptionSink,
	mut stream: impl Stream<Item = T> + Unpin,
) -> Result<(), anyhow::Error> {
	let mut sink = pending.accept().await?;

	loop {
		tokio::select! {
			_ = sink.closed() => break Err(anyhow::anyhow!("Subscription was closed")),
			maybe_item = stream.next() => {
				let item = match maybe_item {
					Some(item) => item,
					None => break Err(anyhow::anyhow!("Subscription was closed")),
				};
				let msg = SubscriptionMessage::from_json(&item)?;
				match sink.try_send(msg) {
					Ok(_) => (),
					Err(TrySendError::Closed(_)) => break Err(anyhow::anyhow!("Subscription was closed")),
					// BAB - copied this message.. don't know better option. "channel is full, let's be naive an just drop the message."
					Err(TrySendError::Full(_)) => (),
				}
			}
		}
	}
}
