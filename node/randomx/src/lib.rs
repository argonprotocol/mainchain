pub use randomx_rs::RandomXError;
use randomx_rs::{RandomXCache, RandomXFlag, RandomXVM};
use sp_core::H256;

pub fn calculate_hash(key_hash: &H256, pre_hash: &[u8]) -> Result<H256, RandomXError> {
	let flags = RandomXFlag::get_recommended_flags();
	let cache = RandomXCache::new(flags, key_hash.as_ref())?;
	let vm = RandomXVM::new(flags, Some(cache), None)?;
	vm.calculate_hash(pre_hash).map(|e| H256::from_slice(e.as_ref()))
}

pub fn calculate_mining_hash(key_hash: &H256, pre_hash: &[u8]) -> Result<H256, RandomXError> {
	full_vm::calculate_hash(key_hash, pre_hash)
}

#[derive(Debug, Clone, Default)]
pub struct Config {
	/// Recommended optimization: decreases the number of pages the system needs to manage, which
	/// in turn reduces TLB (Translation Lookaside Buffer) misses and improves memory access speed
	pub large_pages: bool,
	/// Prevent side channel/timing attacks, but slower; also clears out memory after use
	pub secure: bool,
}

pub mod full_vm {
	use super::Config;
	use lazy_static::lazy_static;
	use lru_cache::LruCache;
	use parking_lot::Mutex;
	pub use randomx_rs::RandomXError;
	use randomx_rs::{RandomXCache, RandomXDataset, RandomXFlag, RandomXVM};
	use sp_core::H256;

	use log::info;
	use std::{
		cell::RefCell,
		sync::{Arc, OnceLock},
		thread::spawn,
	};

	// Caches are shared cross threads
	lazy_static! {
		static ref CACHES: Arc<Mutex<LruCache<H256, Arc<VMData>>>> =
			Arc::new(Mutex::new(LruCache::new(2)));
	}

	// VMs are stored in thread local storage to avoid locking
	thread_local! {
		// FULL uses a dataset (4gb of storage) but solves way faster
		static VM: RefCell<Option<(H256, RandomXVM)>> = const { RefCell::new(None) };
	}

	pub fn calculate_hash(key_hash: &H256, pre_hash: &[u8]) -> Result<H256, RandomXError> {
		alloc_vm_if_needed(key_hash)?;
		VM.with_borrow_mut(|vm| {
			let (_, vm) = vm.as_mut().expect("Local VMS always set to Some above; qed");
			vm.calculate_hash(pre_hash).map(|e| H256::from_slice(e.as_ref()))
		})
	}

	fn set_vm_data(data: &VMData, key_hash: &H256) -> Result<(), RandomXError> {
		VM.with_borrow_mut(|entry| {
			if let Some((_, mut vm)) = entry.take() {
				data.reinit(&mut vm, &key_hash[..])?;

				*entry = Some((*key_hash, vm));
			} else {
				let new_vm = data.new_vm()?;
				info!(target:"argon-randomx", "Created new Randomx VM for key: {:?}", hex::encode(key_hash));
				*entry = Some((*key_hash, new_vm));
			};

			Ok::<_, RandomXError>(())
		})?;
		Ok(())
	}

	fn vm_has_key(key_hash: &H256) -> bool {
		VM.with(|vm| {
			if let Some((key, _)) = vm.borrow().as_ref() {
				return key == key_hash;
			}
			false
		})
	}

	fn alloc_vm_if_needed(key_hash: &H256) -> Result<(), RandomXError> {
		if vm_has_key(key_hash) {
			return Ok(());
		}

		let mut shared_caches = CACHES.lock();
		// caches are static, while vms are per thread, so we this code is creating a new vm
		let data: Arc<VMData> = if let Some(data) = shared_caches.get_mut(key_hash) {
			data.clone()
		} else if shared_caches.len() < shared_caches.capacity() || !global_config().large_pages {
			Arc::new(VMData::new(&key_hash[..], &global_config(), true)?)
		} else {
			// last case is using large pages
			// replace the entry with a single entry
			let key_to_replace = (*shared_caches)
				.iter()
				.find(|&(_, cache)| Arc::strong_count(cache) == 1)
				.map(|(key, _)| *key)
				.ok_or(RandomXError::Other("Cache space not available".to_string()))?;

			// we'll use the previous entry and just update it
			shared_caches.remove(&key_to_replace).expect("key exists; qed")
		};
		shared_caches.insert(*key_hash, data.clone());
		drop(shared_caches);

		set_vm_data(&data, key_hash)
	}

	static GLOBAL_CONFIG: OnceLock<Config> = OnceLock::new();

	pub fn global_config() -> Config {
		GLOBAL_CONFIG.get().cloned().unwrap_or(Config::default())
	}

	pub fn set_global_config(config: Config) -> Result<(), Config> {
		GLOBAL_CONFIG.set(config)
	}
	pub struct VMData {
		cache: RandomXCache,
		dataset: Option<RandomXDataset>,
		flags: RandomXFlag,
	}

	unsafe impl Send for VMData {}
	unsafe impl Sync for VMData {}

	impl VMData {
		pub fn new(key: &[u8], config: &Config, use_dataset: bool) -> Result<Self, RandomXError> {
			let mut flags = RandomXFlag::get_recommended_flags();

			if use_dataset {
				flags |= RandomXFlag::FLAG_FULL_MEM;
				if config.large_pages {
					flags |= RandomXFlag::FLAG_LARGE_PAGES
				}
			}

			if config.secure {
				flags |= RandomXFlag::FLAG_SECURE
			}

			let cache = RandomXCache::new(flags, key)?;
			if use_dataset {
				let dataset = RandomXDataset::alloc(flags, cache.clone())?;
				let instance = Self { cache, dataset: Some(dataset), flags };
				instance.init_dataset()?;
				return Ok(instance);
			}

			Ok(Self { cache, dataset: None, flags })
		}

		pub fn new_vm(&self) -> Result<RandomXVM, RandomXError> {
			RandomXVM::new(self.flags, Some(self.cache.clone()), self.dataset.clone())
		}

		pub fn attach_to_vm(&self, vm: &mut RandomXVM) -> Result<(), RandomXError> {
			if let Some(dataset) = self.dataset.clone() {
				vm.reinit_dataset(dataset)?;
			} else {
				vm.reinit_cache(self.cache.clone())?;
			}
			Ok(())
		}

		pub fn init_dataset(&self) -> Result<(), RandomXError> {
			let Some(dataset) = self.dataset.clone() else {
				return Ok(());
			};

			let cpus_to_use = num_cpus::get().saturating_sub(2).max(1) as u32;
			let dataset_count = RandomXDataset::count()?;
			let init_per_thread = dataset_count / cpus_to_use;
			let remainder = dataset_count % cpus_to_use;

			// dataset.init(0, dataset_count)?;
			let mut start_ticker = 0;
			let dataset_arc = Arc::new(dataset);
			let spawned = (0..cpus_to_use)
				.map(|i| {
					let dataset = dataset_arc.clone();

					let mut count = init_per_thread;
					if i == cpus_to_use - 1 {
						count += remainder;
					}
					let start = start_ticker;
					start_ticker += count;
					spawn(move || dataset.init(start, count))
				})
				.collect::<Vec<_>>();

			for handle in spawned {
				handle.join().map_err(|e| {
					RandomXError::CreationError(format!("Dataset init error: {:?}", e))
				})??;
			}
			Ok(())
		}

		pub fn reinit(&self, vm: &mut RandomXVM, key: &[u8]) -> Result<(), RandomXError> {
			self.cache.init(key)?;
			self.init_dataset()?;
			self.attach_to_vm(vm)?;
			Ok(())
		}
	}
}

#[cfg(test)]
mod tests {
	use crate::full_vm::VMData;

	#[test]
	fn should_match_randomx_tests() {
		// test that hashes from randomx source work
		let cache = VMData::new(&b"test key 000"[..], &Default::default(), false).unwrap();
		let mut vm = cache.new_vm().expect("Failed to create VM");
		{
			// test_a
			let hash = vm.calculate_hash(&b"This is a test"[..]).unwrap();
			assert_eq!(
				hex::encode(hash),
				"639183aae1bf4c9a35884cb46b09cad9175f04efd7684e7262a0ac1c2f0b4e3f"
			);
		}
		{
			// test_c
			let hash = vm
				.calculate_hash(
					&b"sed do eiusmod tempor incididunt ut labore et dolore magna aliqua"[..],
				)
				.unwrap();
			assert_eq!(
				hex::encode(hash),
				"c36d4ed4191e617309867ed66a443be4075014e2b061bcdaf9ce7b721d2b77a8"
			);
		}
		cache.reinit(&mut vm, &b"test key 001"[..]).expect("Failed to reinit");
		{
			//test_d
			let hash = vm
				.calculate_hash(
					&b"sed do eiusmod tempor incididunt ut labore et dolore magna aliqua"[..],
				)
				.unwrap();
			assert_eq!(
				hex::encode(hash),
				"e9ff4503201c0c2cca26d285c93ae883f9b1d30c9eb240b820756f2d5a7905fc"
			);
		}
		{
			// test_e
			let hash = vm
				.calculate_hash(
					&hex::decode(
						"0b0b98bea7e805e0010a2126d287a2a0cc833d312cb786385a7c2f9de69d25537f584a9bc9977b00000000666fd8753bf61a8631f12984e3fd44f4014eca629276817b56f32e9b68bd82f416",
					)
					.unwrap(),
				)
				.unwrap();
			assert_eq!(
				hex::encode(hash),
				"c56414121acda1713c2f2a819d8ae38aed7c80c35c2a769298d34f03833cd5f1"
			);
		}
	}

	#[test]
	fn should_work_with_vm() {
		let light_cache =
			VMData::new(&b"RandomX example key"[..], &Default::default(), false).unwrap();
		let light_vm = light_cache.new_vm().expect("Failed to create VM");
		let hash = light_vm.calculate_hash(&b"RandomX example input"[..]).unwrap();
		let full_cache =
			VMData::new(&b"RandomX example key"[..], &Default::default(), true).unwrap();
		let vm = full_cache.new_vm().expect("Failed to create VM");
		let full_hash = vm.calculate_hash(&b"RandomX example input"[..]).unwrap();
		assert_eq!(hash, full_hash);
	}

	#[test]
	fn reinit_should_work() -> Result<(), String> {
		let cache = VMData::new(&b"RandomX example key"[..], &Default::default(), true).unwrap();
		let mut vm = cache.new_vm().unwrap();
		let hash1 = vm.calculate_hash(&b"RandomX example input"[..]).unwrap();

		cache.reinit(&mut vm, &b"RandomX example key 2"[..]).expect("Failed to reinit");

		let hash2 = vm.calculate_hash(&b"RandomX example input"[..]).unwrap();
		assert_ne!(hash1, hash2,);

		Ok(())
	}
}
