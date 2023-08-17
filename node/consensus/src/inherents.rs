pub use sc_executor::NativeElseWasmExecutor;
use sp_api::BlockT;
use std::marker::PhantomData;
use ulx_primitives::inherents::UlxBlockSealInherent;

#[derive(Clone)]
pub struct UlxCreateInherentDataProviders<B> {
	_block: PhantomData<B>,
}

impl<B> UlxCreateInherentDataProviders<B> {
	pub fn new() -> Self {
		Self { _block: PhantomData }
	}
}

#[async_trait::async_trait]
impl<B> sp_inherents::CreateInherentDataProviders<B, UlxBlockSealInherent>
	for UlxCreateInherentDataProviders<B>
where
	B: BlockT,
{
	type InherentDataProviders =
		(sp_timestamp::InherentDataProvider, ulx_primitives::inherents::InherentDataProvider);

	async fn create_inherent_data_providers(
		&self,
		_parent: B::Hash,
		extra_args: UlxBlockSealInherent,
	) -> Result<Self::InherentDataProviders, Box<dyn std::error::Error + Send + Sync>> {
		let timestamp = sp_timestamp::InherentDataProvider::from_system_time();
		let seal = ulx_primitives::inherents::InherentDataProvider::new(extra_args.clone());
		Ok((timestamp, seal))
	}
}
