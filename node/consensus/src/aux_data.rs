use std::sync::Arc;

use codec::{Decode, Encode};
use parking_lot::RwLock;
use sc_client_api::AuxStore;

use crate::aux_client::AuxKey;

#[derive(Clone)]
pub struct AuxData<T, C> {
	client: Arc<C>,
	data: Arc<RwLock<T>>,
	key: Vec<u8>,
}

impl<T, C: AuxStore> AuxData<T, C>
where
	T: Encode + Decode + Sync + Send + Clone + Default,
{
	pub fn new(client: Arc<C>, key: AuxKey) -> Self {
		let key = key.encode();
		let start_data = Self::get_static(&key, &client);
		AuxData { client, key, data: Arc::new(RwLock::new(start_data)) }
	}

	pub fn get_static(key: &[u8], client: &Arc<C>) -> T {
		let key = key.encode();
		if let Ok(Some(bytes)) = client.get_aux(&key) {
			T::decode(&mut &bytes[..]).ok().unwrap_or_default()
		} else {
			Default::default()
		}
	}

	pub fn mutate<F, R>(&self, f: F) -> Result<R, sp_blockchain::Error>
	where
		F: FnOnce(&mut T) -> R,
	{
		let (result, encoded, start_data) = {
			let mut data = self.data.write();
			let start_data = data.clone();
			let result = f(&mut data);
			(result, data.encode(), start_data)
		};

		self.client
			.insert_aux(&[(self.key.as_slice(), encoded.as_slice())], &[])
			.inspect_err(|_| {
				// roll back the data and throw
				*self.data.write() = start_data;
			})?;

		Ok(result)
	}

	pub fn hold_lock(&self) -> Arc<RwLock<T>> {
		self.data.clone()
	}

	pub fn get(&self) -> T {
		self.data.read().clone()
	}
}
