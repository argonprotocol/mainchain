use polkadot_sdk::sp_core::H256;
pub use randomx_rs::RandomXError;
use randomx_rs::{RandomXCache, RandomXFlag, RandomXVM};

#[cfg(test)]
static MINING_MEMORY_TEST_LOCK: parking_lot::Mutex<()> = parking_lot::Mutex::new(());

pub fn calculate_hash(key_hash: &H256, pre_hash: &[u8]) -> Result<H256, RandomXError> {
	let flags = RandomXFlag::get_recommended_flags();
	let cache = RandomXCache::new(flags, key_hash.as_ref())?;
	let vm = RandomXVM::new(flags, Some(cache), None)?;
	vm.calculate_hash(pre_hash).map(|e| H256::from_slice(e.as_ref()))
}

pub fn calculate_mining_hash(
	key_hash: &H256,
	pre_hash: &[u8],
	generation: usize,
) -> Result<Option<H256>, RandomXError> {
	full_vm::calculate_hash(key_hash, pre_hash, generation)
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
	use polkadot_sdk::sp_core::H256;
	pub use randomx_rs::RandomXError;
	use randomx_rs::{RandomXCache, RandomXDataset, RandomXFlag, RandomXVM};

	use log::info;
	use std::{
		cell::RefCell,
		sync::{
			atomic::{AtomicUsize, Ordering},
			Arc, OnceLock,
		},
	};

	// Caches are shared cross threads
	lazy_static! {
		static ref CACHES: Arc<Mutex<LruCache<H256, Arc<VMData>>>> =
			Arc::new(Mutex::new(LruCache::new(2)));
	}
	static CACHE_GENERATION: AtomicUsize = AtomicUsize::new(0);

	struct MiningVm {
		generation: usize,
		key_hash: H256,
		vm: RandomXVM,
		// Keep the cache entry marked in use while this VM references its dataset.
		_data: Arc<VMData>,
	}

	// VMs are stored in thread local storage to avoid locking
	thread_local! {
		static VM: RefCell<Option<MiningVm>> = const { RefCell::new(None) };
	}

	pub(crate) fn calculate_hash(
		key_hash: &H256,
		pre_hash: &[u8],
		generation: usize,
	) -> Result<Option<H256>, RandomXError> {
		if !alloc_vm_if_needed(key_hash, generation)? {
			return Ok(None);
		}
		VM.with_borrow_mut(|vm| {
			let vm = vm.as_mut().expect("Local VMS always set to Some above; qed");
			vm.vm.calculate_hash(pre_hash).map(|e| Some(H256::from_slice(e.as_ref())))
		})
	}

	/// Capture when creating work so cancelled solvers cannot allocate after eviction.
	pub fn cache_generation() -> usize {
		CACHE_GENERATION.load(Ordering::SeqCst)
	}

	/// Release shared datasets and ask every mining thread to release its VM.
	pub fn evict_mining_memory() -> bool {
		let Some(mut shared_caches) = CACHES.try_lock() else {
			return false;
		};
		CACHE_GENERATION.fetch_add(1, Ordering::SeqCst);
		shared_caches.clear();
		true
	}

	pub fn has_cached_data() -> bool {
		CACHES.try_lock().is_some_and(|caches| !caches.is_empty())
	}

	/// Call on the mining thread, including while it has no work to solve.
	pub fn release_vm_if_evicted() {
		let generation = CACHE_GENERATION.load(Ordering::SeqCst);
		VM.with_borrow_mut(|entry| {
			if entry.as_ref().is_some_and(|vm| vm.generation != generation) {
				*entry = None;
			}
		});
	}

	fn alloc_vm_if_needed(key_hash: &H256, generation: usize) -> Result<bool, RandomXError> {
		release_vm_if_evicted();
		if generation != cache_generation() {
			return Ok(false);
		}
		if VM.with_borrow(|entry| entry.as_ref().is_some_and(|vm| vm.key_hash == *key_hash)) {
			return Ok(true);
		}

		// Release the previous key's VM before looking for an unused dataset to replace.
		VM.with_borrow_mut(|entry| *entry = None);

		let mut shared_caches = CACHES.lock();
		// Eviction advances the generation under this same lock. Recheck work admitted before
		// eviction so it cannot recreate a dataset after the shared caches have been cleared.
		if generation != cache_generation() {
			return Ok(false);
		}

		// Caches are shared, while VMs are local to each mining thread.
		let data: Arc<VMData> = if let Some(data) = shared_caches.get_mut(key_hash) {
			data.clone()
		} else if shared_caches.len() < shared_caches.capacity() || !global_config().large_pages {
			let vm_data = VMData::new(&key_hash[..], &global_config(), true)?;
			info!(target:"argon-randomx", "Created new Randomx VMData for key: {:?}", hex::encode(key_hash));
			Arc::new(vm_data)
		} else {
			// last case is using large pages
			// replace the entry with a single entry
			let key_to_replace = (*shared_caches)
				.iter()
				.find(|&(_, cache)| Arc::strong_count(cache) == 1)
				.map(|(key, _)| *key)
				.ok_or(RandomXError::Other("Cache space not available".to_string()))?;

			let data = shared_caches.remove(&key_to_replace).expect("key exists; qed");
			data.cache.init(&key_hash[..])?;
			data.init_dataset()?;
			data
		};
		// Only fully initialized data can be reused by another worker or a retry.
		shared_caches.insert(*key_hash, data.clone());
		drop(shared_caches);

		let vm = data.new_vm()?;
		VM.with_borrow_mut(|entry| {
			*entry = Some(MiningVm { generation, key_hash: *key_hash, vm, _data: data });
		});
		Ok(true)
	}

	static GLOBAL_CONFIG: OnceLock<Config> = OnceLock::new();

	pub fn global_config() -> Config {
		GLOBAL_CONFIG.get().cloned().unwrap_or(Config::default())
	}

	pub fn set_global_config(config: Config) -> Result<(), Config> {
		GLOBAL_CONFIG.set(config)
	}
	pub(crate) struct VMData {
		cache: RandomXCache,
		dataset: Option<RandomXDataset>,
		flags: RandomXFlag,
	}

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

		#[cfg(test)]
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

			let mut start_ticker = 0;
			let mut spawned = Vec::with_capacity(cpus_to_use as usize);
			let mut initialization = Ok(());
			for i in 0..cpus_to_use {
				let dataset = dataset.clone();
				let mut count = init_per_thread;
				if i == cpus_to_use - 1 {
					count += remainder;
				}
				let start = start_ticker;
				start_ticker += count;

				match std::thread::Builder::new().spawn(move || dataset.init(start, count)) {
					Ok(thread) => spawned.push(thread),
					Err(err) => {
						initialization = Err(RandomXError::CreationError(format!(
							"Dataset init thread creation failed: {err}"
						)));
						break;
					},
				}
			}

			// Finish every started initializer before returning, including after a spawn failure.
			for handle in spawned {
				let result = handle
					.join()
					.map_err(|e| RandomXError::CreationError(format!("Dataset init error: {e:?}")))
					.and_then(|result| result);
				if initialization.is_ok() {
					initialization = result;
				}
			}
			initialization
		}

		#[cfg(test)]
		pub fn reinit(&self, vm: &mut RandomXVM, key: &[u8]) -> Result<(), RandomXError> {
			self.cache.init(key)?;
			self.init_dataset()?;
			self.attach_to_vm(vm)?;
			Ok(())
		}
	}

	#[cfg(test)]
	mod tests {
		use super::*;
		use std::{sync::mpsc, thread, time::Duration};

		#[test]
		fn eviction_releases_idle_worker_vms_and_allows_mining_to_restart() {
			let _mining_memory_guard = crate::MINING_MEMORY_TEST_LOCK.lock();
			let key = H256::repeat_byte(71);

			// Dataset initialization can exceed a minute on CI. Time only worker coordination.
			let hash =
				calculate_hash(&key, b"fallback block", cache_generation()).unwrap().unwrap();
			VM.with_borrow_mut(|entry| *entry = None);

			thread::scope(|scope| {
				let (ready_tx, ready_rx) = mpsc::channel();
				let mut release_txs = Vec::new();
				for _ in 0..2 {
					let (release_tx, release_rx) = mpsc::channel();
					release_txs.push(release_tx);
					let ready_tx = ready_tx.clone();
					scope.spawn(move || {
						let hash = calculate_hash(&key, b"fallback block", cache_generation())
							.unwrap()
							.unwrap();
						assert!(VM.with_borrow(|entry| entry.is_some()));
						if ready_tx.send(hash).is_err() {
							return;
						}
						if release_rx.recv().is_err() {
							return;
						}
						release_vm_if_evicted();
						assert!(VM.with_borrow(|entry| entry.is_none()));
					});
				}
				drop(ready_tx);

				for _ in 0..2 {
					assert_eq!(hash, ready_rx.recv_timeout(Duration::from_secs(60)).unwrap());
				}
				assert!(CACHES.lock().get_mut(&key).unwrap().dataset.is_some());
				assert!(evict_mining_memory());
				assert!(!has_cached_data());
				for release_tx in release_txs {
					release_tx.send(()).unwrap();
				}
			});

			assert_eq!(
				calculate_hash(&key, b"fallback block", cache_generation()).unwrap(),
				Some(hash)
			);
			assert!(has_cached_data());
			assert!(evict_mining_memory());
			release_vm_if_evicted();
			assert!(VM.with_borrow(|entry| entry.is_none()));
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
		let _mining_memory_guard = crate::MINING_MEMORY_TEST_LOCK.lock();
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
		let _mining_memory_guard = crate::MINING_MEMORY_TEST_LOCK.lock();
		let cache = VMData::new(&b"RandomX example key"[..], &Default::default(), true).unwrap();
		let mut vm = cache.new_vm().unwrap();
		let hash1 = vm.calculate_hash(&b"RandomX example input"[..]).unwrap();

		cache.reinit(&mut vm, &b"RandomX example key 2"[..]).expect("Failed to reinit");

		let hash2 = vm.calculate_hash(&b"RandomX example input"[..]).unwrap();
		assert_ne!(hash1, hash2,);

		Ok(())
	}
}
