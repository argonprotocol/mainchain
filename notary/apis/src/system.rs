use jsonrpsee::{proc_macros::rpc, types::ErrorObjectOwned};

#[rpc(server, client, namespace = "system")]
pub trait SystemRpc {
	#[method(name = "getArchiveBaseUrl")]
	async fn get_archive_base_url(&self) -> Result<String, ErrorObjectOwned>;

	#[method(name = "health")]
	async fn health(&self) -> Result<(), ErrorObjectOwned>;
}
