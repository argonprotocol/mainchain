#![feature(slice_take)]
#![cfg_attr(not(feature = "std"), no_std)]

use sp_std::{collections::btree_map::BTreeMap, vec::Vec};

pub use pallet::*;
pub use weights::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
pub mod weights;

#[frame_support::pallet(dev_mode)]
pub mod pallet {
	use frame_support::{pallet_prelude::*, traits::FindAuthor};
	use frame_system::pallet_prelude::*;
	use sp_core::{crypto::AccountId32, U256};
	use sp_io::hashing::blake2_256;
	use sp_runtime::{
		traits::UniqueSaturatedInto, ConsensusEngineId, RuntimeAppPublic, Saturating,
	};
	use sp_std::cmp::min;

	use ulx_primitives::{
		block_seal::{
			AuthorityDistance, AuthorityProvider, BlockProof, HistoricalBlockSealersLookup,
			SealNonceHashMessage, SealStamper, SealerSignatureMessage, SEALER_SIGNATURE_PREFIX,
			SEAL_NONCE_PREFIX,
		},
		inherents::{InherentError, UlxBlockSealInherent, UlxBlockSealInherentData},
		ProofOfWorkType, AUTHOR_ID,
	};

	use super::*;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// The identifier type for an authority.
		type AuthorityId: Member
			+ Parameter
			+ RuntimeAppPublic
			+ MaybeSerializeDeserialize
			+ MaxEncodedLen;
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		/// Type representing the weight of this pallet
		type WeightInfo: WeightInfo;
		/// Type that provides authorities
		type AuthorityProvider: AuthorityProvider<Self::AuthorityId, Self::AccountId>;
		/// How many historical block sealers to keep in storage
		#[pallet::constant]
		type HistoricalBlockSealersToKeep: Get<u32>;
	}

	/// Pruned with the max block seal history parameter
	#[pallet::storage]
	pub(super) type HistoricalBlockSealAuthorities<T: Config> = StorageMap<
		_,
		Twox64Concat,
		BlockNumberFor<T>,
		BoundedVec<T::AuthorityId, T::HistoricalBlockSealersToKeep>,
		ValueQuery,
	>;

	/// How many of the xor authorities must sign a block seal to be valid
	#[pallet::storage]
	#[pallet::getter(fn min_seal_signers)]
	pub(super) type MinSealSigners<T: Config> = StorageValue<_, u32, ValueQuery>;

	/// How many closest xor authorities must be included in a block seal to be accepted
	#[pallet::storage]
	#[pallet::getter(fn closest_x_authorities_required)]
	pub(super) type ClosestXAuthoritiesRequired<T: Config> = StorageValue<_, u32, ValueQuery>;

	/// How many authorities need to be registered before we activate the Proof of Tax era.
	#[pallet::storage]
	pub(super) type AuthorityCountInitiatingTaxProof<T: Config> = StorageValue<_, u32, ValueQuery>;

	/// Author of current block (temporary storage).
	#[pallet::storage]
	#[pallet::getter(fn author)]
	pub(super) type Author<T: Config> = StorageValue<_, T::AccountId, OptionQuery>;

	/// Did the seal run exactly once (temporary storage)
	#[pallet::storage]
	pub(super) type DidSeal<T: Config> = StorageValue<_, bool, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn work_type)]
	pub(super) type CurrentWorkType<T: Config> = StorageValue<_, ProofOfWorkType, ValueQuery>;

	pub type AuthoritySignature<T> = <<T as Config>::AuthorityId as RuntimeAppPublic>::Signature;

	#[pallet::genesis_config]
	#[derive(frame_support::DefaultNoBound)]
	pub struct GenesisConfig<T: Config> {
		pub min_seal_signers: u32,
		pub closest_xor_authorities_required: u32,
		pub authority_count_starting_tax_seal: u32,
		pub _phantom: PhantomData<T>,
	}

	#[pallet::genesis_build]
	impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
		fn build(&self) {
			<MinSealSigners<T>>::put(self.min_seal_signers);
			<ClosestXAuthoritiesRequired<T>>::put(self.closest_xor_authorities_required);
			<AuthorityCountInitiatingTaxProof<T>>::put(self.authority_count_starting_tax_seal);
		}
	}

	#[pallet::event]
	pub enum Event<T: Config> {}

	#[pallet::error]
	pub enum Error<T> {
		InvalidBlockSealType,
		InvalidBlockSealNonce,
		InvalidSealValidatorsProvided,
		InsufficientValidatorsIncluded,
		InvalidSealSignature,
		InsufficientSealSigners,
		InvalidXorClosestAuthoritiesOrder,
		UnregisteredBlockAuthor,
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(_: BlockNumberFor<T>) -> Weight {
			let digest = <frame_system::Pallet<T>>::digest();
			let pre_runtime_digests = digest.logs.iter().filter_map(|d| d.as_pre_runtime());
			for (id, mut data) in pre_runtime_digests.into_iter() {
				if id == AUTHOR_ID {
					let decoded = T::AccountId::decode(&mut data);
					if let Some(account_id) = decoded.ok() {
						<Author<T>>::put(&account_id);
					}
				}
			}
			// check if we have enough authorities to begin tax proof
			let current_work_type = <CurrentWorkType<T>>::get();
			if current_work_type == ProofOfWorkType::Compute {
				let validators = T::AuthorityProvider::authority_count();
				if validators >=
					<AuthorityCountInitiatingTaxProof<T>>::get().unique_saturated_into()
				{
					<CurrentWorkType<T>>::put(ProofOfWorkType::Tax);
				}
			}

			if <Author<T>>::get() == None {
				panic!("No valid account id provided for block author.");
			}

			T::DbWeight::get().reads_writes(2, 1)
		}

		fn on_finalize(n: BlockNumberFor<T>) {
			// ensure we never go to trie with these values.
			<Author<T>>::kill();

			<HistoricalBlockSealAuthorities<T>>::take(
				n.saturating_sub(T::HistoricalBlockSealersToKeep::get().into()),
			);
			if CurrentWorkType::<T>::get() == ProofOfWorkType::Tax {
				assert!(
					DidSeal::<T>::take(),
					"Block seal must be processed as an inherent for a proof of tax block"
				);
			}
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::call_index(0)]
		#[pallet::weight((0, DispatchClass::Mandatory))]
		pub fn create(origin: OriginFor<T>, seal: UlxBlockSealInherent) -> DispatchResult {
			ensure_none(origin)?;
			assert!(!DidSeal::<T>::exists(), "A Seal must be updated only once in the block");

			let authorities = T::AuthorityProvider::authorities_by_index();
			let authority_count = authorities.len();
			let min_signatures = min(<MinSealSigners<T>>::get() as usize, authority_count);
			let required_validators =
				min(<ClosestXAuthoritiesRequired<T>>::get() as usize, authority_count);

			let proof = match seal.tax_block_proof {
				Some(proof) => proof,
				None => return Err(Error::<T>::InvalidBlockSealType.into()),
			};

			let nonce = seal.tax_nonce.ok_or(Error::<T>::InvalidBlockSealNonce)?;

			let seal_validators =
				Self::load_seal_authorities(proof.seal_stampers.clone(), &authorities);

			// 1. Did they provide all valid authorities?
			if &seal_validators.len() < &proof.seal_stampers.len() {
				return Err(Error::<T>::InvalidSealValidatorsProvided.into())
			}

			// 2. Did they provide the closest 10?
			if seal_validators.len() != required_validators {
				return Err(Error::<T>::InsufficientValidatorsIncluded.into())
			}

			// 3. Did they get enough signatures?
			let signers = proof.seal_stampers.iter().filter(|x| x.signature.is_some()).count();
			if signers < min_signatures {
				return Err(Error::<T>::InsufficientSealSigners.into())
			}

			// 3. check that the block author is one of the validators
			let block_author = <Author<T>>::get().unwrap();
			let authority_id = T::AuthorityProvider::get_authority(block_author.clone())
				.ok_or(Error::<T>::UnregisteredBlockAuthor)?;
			let validator_0 = seal_validators
				.iter()
				.find(|x| x.0 == authority_id)
				.ok_or(Error::<T>::UnregisteredBlockAuthor)?;
			ensure!(validator_0.0 == authority_id, Error::<T>::UnregisteredBlockAuthor);

			// 4. Did the nonce match our calculation?
			let parent_hash = <frame_system::Pallet<T>>::parent_hash();
			Self::check_nonce(nonce, &proof, parent_hash)?;

			let tax_author_id = proof.author_id.clone();
			let author_bytes = tax_author_id.encode();
			let block_peer_hash =
				U256::from(blake2_256(&[parent_hash.as_ref(), &author_bytes].concat()));

			let authority_xor_distances = T::AuthorityProvider::find_xor_closest_authorities(
				block_peer_hash,
				required_validators.unique_saturated_into(),
			);

			// 6. Check that these are the closest validators to block author hash with block hash
			Self::check_xor_closest_authorities_chosen(&seal_validators, authority_xor_distances)?;

			// 7. Check signatures of all validators
			Self::check_seal_signatures(&seal_validators, &proof, parent_hash, tax_author_id)?;

			DidSeal::<T>::put(true);
			let block_number = <frame_system::Pallet<T>>::block_number();
			let sealers = seal_validators.iter().map(|v| v.0.clone()).collect::<Vec<_>>();
			<HistoricalBlockSealAuthorities<T>>::insert(
				block_number,
				BoundedVec::truncate_from(sealers),
			);

			Ok(())
		}

		#[pallet::call_index(1)]
		#[pallet::weight(0)]
		pub fn configure(
			origin: OriginFor<T>,
			min_seal_signers: Option<u32>,
			required_xor_closest_validators: Option<u32>,
			authority_count_starting_tax_seal: Option<u32>,
		) -> DispatchResult {
			ensure_root(origin)?;
			if let Some(min_seal_signers) = min_seal_signers {
				<MinSealSigners<T>>::put(min_seal_signers);
			}
			if let Some(required_xor_closest_validators) = required_xor_closest_validators {
				<ClosestXAuthoritiesRequired<T>>::put(required_xor_closest_validators);
			}
			if let Some(authority_count_starting_tax_seal) = authority_count_starting_tax_seal {
				<AuthorityCountInitiatingTaxProof<T>>::put(authority_count_starting_tax_seal);
			}
			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		fn load_seal_authorities(
			seal_stampers: Vec<SealStamper>,
			authorities_by_index: &BTreeMap<u16, T::AuthorityId>,
		) -> Vec<(T::AuthorityId, Option<AuthoritySignature<T>>)> {
			seal_stampers
				.clone()
				.into_iter()
				.filter_map(|v| {
					if let Some(id) = authorities_by_index.get(&v.authority_idx) {
						if let Some(signature) = v.signature {
							if let Ok(signature) =
								AuthoritySignature::<T>::decode(&mut signature.as_slice())
							{
								return Some((id.clone(), Some(signature)))
							}
						}
						return Some((id.clone(), None))
					}
					None
				})
				.collect()
		}

		fn check_seal_signatures(
			seal_validators: &Vec<(T::AuthorityId, Option<AuthoritySignature<T>>)>,
			proof: &BlockProof,
			parent_hash: T::Hash,
			author_id: AccountId32,
		) -> DispatchResult {
			let peer_signature_message = blake2_256(
				SealerSignatureMessage {
					prefix: SEALER_SIGNATURE_PREFIX,
					tax_proof_id: proof.tax_proof_id,
					parent_hash,
					author_id,
					tax_amount: proof.tax_amount,
					seal_stampers: seal_validators.iter().map(|v| v.0.clone()).collect(),
				}
				.encode()
				.as_slice(),
			);
			for (id, signature) in seal_validators.iter() {
				if let Some(signature) = signature {
					let is_valid = id.verify(&peer_signature_message, signature);
					if !is_valid {
						return Err(Error::<T>::InvalidSealSignature.into())
					}
				}
			}
			Ok(())
		}

		fn check_nonce(nonce: U256, proof: &BlockProof, parent_hash: T::Hash) -> DispatchResult {
			let calculated_nonce = SealNonceHashMessage::<T::Hash> {
				prefix: SEAL_NONCE_PREFIX,
				tax_proof_id: proof.tax_proof_id,
				tax_amount: proof.tax_amount,
				parent_hash,
				author_id: proof.author_id.clone(),
				seal_stampers: proof.seal_stampers.clone(),
			}
			.using_encoded(blake2_256);

			if nonce != calculated_nonce.into() {
				return Err(Error::<T>::InvalidBlockSealNonce.into())
			}
			Ok(())
		}

		fn check_xor_closest_authorities_chosen(
			seal_validators: &Vec<(T::AuthorityId, Option<AuthoritySignature<T>>)>,
			calculated_block_peers: Vec<AuthorityDistance<T::AuthorityId>>,
		) -> DispatchResult {
			let seal_authorities = seal_validators.iter().map(|v| v.0.clone()).collect::<Vec<_>>();
			let required_authorities = calculated_block_peers
				.iter()
				.map(|v| v.authority_id.clone())
				.collect::<Vec<_>>();
			if required_authorities != seal_authorities {
				return Err(Error::<T>::InvalidXorClosestAuthoritiesOrder.into())
			}
			Ok(())
		}
	}

	#[pallet::inherent]
	impl<T: Config> ProvideInherent for Pallet<T> {
		type Call = Call<T>;
		type Error = InherentError;
		const INHERENT_IDENTIFIER: InherentIdentifier =
			ulx_primitives::inherents::INHERENT_IDENTIFIER;

		fn create_inherent(data: &InherentData) -> Option<Self::Call>
		where
			InherentData: UlxBlockSealInherentData,
		{
			let seal = data
				.block_seal()
				.expect("Could not decode Block seal inherent data")
				.expect("Block seal inherent data must be provided");

			match seal.work_type {
				ProofOfWorkType::Tax => Some(Call::create { seal }),
				_ => None,
			}
		}

		fn check_inherent(call: &Self::Call, data: &InherentData) -> Result<(), Self::Error> {
			let calculated_seal = data
				.block_seal()
				.expect("Could not decode Block seal inherent data")
				.expect("Block seal inherent data must be provided");

			if let Call::create { seal } = call {
				if seal.work_type != calculated_seal.work_type {
					return Err(InherentError::WrongProofOfWork)
				}
				if seal.work_type == ProofOfWorkType::Tax {
					if seal.tax_nonce != calculated_seal.tax_nonce {
						return Err(InherentError::WrongNonce)
					}
				}
			}
			Ok(())
		}

		fn is_inherent(call: &Self::Call) -> bool {
			matches!(call, Call::create { .. })
		}

		fn is_inherent_required(data: &InherentData) -> Result<Option<Self::Error>, Self::Error> {
			let seal = data
				.block_seal()
				.expect("Could not decode Block seal inherent data")
				.expect("Block seal inherent data must be provided");

			if seal.work_type == ProofOfWorkType::Tax {
				// return error if this is missing
				return Ok(Some(InherentError::MissingProofOfTaxInherent))
			}

			return Ok(None)
		}
	}

	impl<T: Config> HistoricalBlockSealersLookup<BlockNumberFor<T>, T::AuthorityId> for Pallet<T> {
		fn get_active_block_sealers_of(block_number: BlockNumberFor<T>) -> Vec<T::AuthorityId> {
			let block_sealers = <HistoricalBlockSealAuthorities<T>>::get(block_number);
			block_sealers
				.into_inner()
				.into_iter()
				.filter(|a| T::AuthorityProvider::is_active(a))
				.collect::<Vec<_>>()
		}
	}

	impl<T: Config> FindAuthor<T::AccountId> for Pallet<T> {
		fn find_author<'a, I>(digests: I) -> Option<T::AccountId>
		where
			I: 'a + IntoIterator<Item = (ConsensusEngineId, &'a [u8])>,
		{
			if let Some(account_id) = <Author<T>>::get() {
				return Some(account_id)
			}

			for (id, mut data) in digests.into_iter() {
				if id == AUTHOR_ID {
					let decoded = T::AccountId::decode(&mut data);
					if let Some(account_id) = decoded.ok() {
						<Author<T>>::put(&account_id);
						return Some(account_id)
					}
				}
			}

			None
		}
	}
}
