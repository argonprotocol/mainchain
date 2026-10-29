#![cfg_attr(not(feature = "std"), no_std)]
//! Claimant-submitted checkpoints for maintained Uniswap v4 positions.
//!
//! Checkpoints establish continuity, not current live network totals. Rewards and collection are
//! deliberately not implemented. A changed revision starts a new baseline without proving the gap.

extern crate alloc;

use alloc::boxed::Box;
use argon_primitives::{EthereumAccountStorageProof, EthereumVerifyProvider};
pub use pallet::*;
use pallet_ethereum_verifier::Pallet as EthereumVerifier;
use pallet_prelude::*;
pub use weights::WeightInfo;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
pub mod evm;
#[cfg(any(test, feature = "runtime-benchmarks"))]
mod mock;
#[cfg(test)]
mod tests;
pub mod weights;

/// Uniswap v4's canonical pool key. The native PoolId is its ABI-encoded keccak256 hash.
#[derive(
	Encode,
	Decode,
	DecodeWithMemTracking,
	Clone,
	Copy,
	Debug,
	PartialEq,
	Eq,
	TypeInfo,
	MaxEncodedLen,
)]
pub struct PoolKey {
	/// Lower-sorted Ethereum currency address; zero represents native ETH.
	pub currency0: H160,
	/// Higher-sorted Ethereum currency address.
	pub currency1: H160,
	/// Uniswap's uint24 fee encoding, including its dynamic-fee flag.
	pub fee: u32,
	/// Positive tick interval to which position boundaries must align.
	pub tick_spacing: i32,
	/// Pool hook contract, or zero for a pool without hooks.
	pub hooks: H160,
}

#[derive(
	Encode,
	Decode,
	DecodeWithMemTracking,
	Clone,
	Copy,
	Debug,
	PartialEq,
	Eq,
	TypeInfo,
	MaxEncodedLen,
)]
pub struct PoolConfig {
	/// Approved Uniswap NFT manager whose token IDs identify checkpointed positions.
	pub position_manager: H160,
	/// Approved subscriber holding each NFT's Argon beneficiary and permanent revision.
	pub subscriber: H160,
	/// Canonical key whose ABI hash identifies this pool within its PoolManager.
	pub key: PoolKey,
}

/// All three account witnesses must prove the same execution block and state root.
#[derive(
	Encode, Decode, DecodeWithMemTracking, Clone, Debug, PartialEq, Eq, TypeInfo, MaxEncodedLen,
)]
pub struct PositionProof {
	/// Subscriber account witness for the NFT's beneficiary and revision slots.
	pub subscriber: EthereumAccountStorageProof<ConstU32<2>>,
	/// NFT manager witness for canonical subscription membership and packed position metadata.
	pub position_manager: EthereumAccountStorageProof<ConstU32<2>>,
	/// PoolManager witness for the NFT's native Uniswap liquidity value.
	pub pool_manager: EthereumAccountStorageProof<ConstU32<1>>,
}

#[derive(
	Encode,
	Decode,
	DecodeWithMemTracking,
	Clone,
	Copy,
	Debug,
	PartialEq,
	Eq,
	TypeInfo,
	MaxEncodedLen,
)]
pub struct ProofBlock {
	/// Authenticated Ethereum execution-block hash, not an Argon block hash.
	pub hash: H256,
	/// Ethereum execution-block height used for the position's replay watermark.
	pub number: u64,
	/// Ethereum block time in Unix milliseconds, authenticated by the execution header.
	pub timestamp_millis: u64,
}

#[derive(
	Encode, Decode, DecodeWithMemTracking, Clone, Debug, PartialEq, Eq, TypeInfo, MaxEncodedLen,
)]
pub struct PositionCheckpoint<AccountId> {
	/// Argon account encoded in the subscription and authorizing this checkpoint.
	pub beneficiary: AccountId,
	/// Ethereum contract owning the pool's underlying Uniswap state.
	pub pool_manager: H160,
	/// Full native PoolId: the ABI hash of the approved canonical pool key.
	pub pool_id: H256,
	/// Subscriber proven to be attached to the NFT at the checkpoint block.
	pub subscriber: H160,
	/// Permanent subscriber counter; any increase starts a new continuity baseline.
	pub revision: U256,
	/// Inclusive lower tick boundary of the NFT's position.
	pub tick_lower: i32,
	/// Exclusive upper tick boundary of the NFT's position.
	pub tick_upper: i32,
	/// Native Uniswap liquidity L, not a token balance or reward amount.
	pub liquidity: u128,
	/// Start of the continuously maintained revision proven by this record.
	pub maintained_since: ProofBlock,
	/// Latest accepted Ethereum endpoint; proves nothing about later blocks.
	pub proven_through: ProofBlock,
}

#[derive(
	Encode, Decode, DecodeWithMemTracking, Clone, Debug, PartialEq, Eq, TypeInfo, MaxEncodedLen,
)]
pub enum CheckpointOutcome {
	/// The first proof establishes a baseline without proving earlier activity.
	Registered,
	Maintained {
		/// Previous verified endpoint from which the unchanged revision is extended.
		from: ProofBlock,
	},
	Reset {
		/// Revision replaced by the new baseline.
		previous_revision: U256,
		/// Last verified endpoint before the unproven gap.
		unproven_from: ProofBlock,
	},
}

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	const STORAGE_VERSION: StorageVersion = StorageVersion::new(1);

	#[pallet::pallet]
	#[pallet::storage_version(STORAGE_VERSION)]
	pub struct Pallet<T>(_);

	#[pallet::config]
	// FRAME requires this explicit path when checking the inherited system configuration.
	pub trait Config:
		polkadot_sdk::frame_system::Config + pallet_ethereum_verifier::Config
	{
		type WeightInfo: weights::WeightInfo;
	}

	/// Approved deployments on the Ethereum network authenticated by EthereumVerifier.
	/// Configuration is immutable once registered, preserving checkpoint identity and layout.
	#[pallet::storage]
	pub type Pools<T> =
		StorageDoubleMap<_, Blake2_128Concat, H160, Blake2_128Concat, H256, PoolConfig>;

	/// Last accepted checkpoint for an NFT. Keep its source-block watermark across re-enrollment.
	#[pallet::storage]
	pub type PositionCheckpoints<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		H160,
		Blake2_128Concat,
		U256,
		PositionCheckpoint<T::AccountId>,
		OptionQuery,
	>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		PoolRegistered {
			/// Ethereum PoolManager used to scope the native PoolId.
			pool_manager: H160,
			/// ABI hash of the newly approved pool key.
			pool_id: H256,
			/// Immutable deployment bindings and canonical pool key.
			config: PoolConfig,
		},
		PositionCheckpointed {
			/// Ethereum NFT manager used to scope the token ID.
			position_manager: H160,
			/// NFT identifier within the approved PositionManager.
			token_id: U256,
			/// Authenticated position state now stored as the latest checkpoint.
			checkpoint: Box<PositionCheckpoint<T::AccountId>>,
			/// Whether this proof registered, extended, or reset continuity.
			outcome: CheckpointOutcome,
		},
	}

	#[pallet::error]
	pub enum Error<T> {
		/// A deployment address is zero or reused, or the pool key fails ordering or encoding
		/// bounds.
		InvalidPool,
		/// This PoolManager/PoolId pair already has an immutable registration.
		PoolAlreadyRegistered,
		/// No approved configuration exists for the supplied PoolManager/PoolId pair.
		PoolNotRegistered,
		/// The Ethereum verifier is unavailable, or header ancestry/account/storage authentication
		/// fails.
		InvalidProof,
		/// The three account witnesses do not contain identical execution-block proofs.
		InconsistentProofBlocks,
		/// A witness has the wrong slot count or slot addresses for the requested NFT position.
		InvalidStorageSlots,
		/// The NFT's subscriber or subscription flag is invalid, or its permanent revision is
		/// zero.
		InvalidSubscription,
		/// The signer does not match the beneficiary encoded in the subscription witness.
		NotBeneficiary,
		/// The packed PoolId mismatches, ticks are invalid/misaligned, or liquidity exceeds
		/// uint128.
		InvalidPosition,
		/// Ethereum block height or time does not strictly advance past the stored checkpoint.
		StaleCheckpoint,
		/// The subscriber revision is lower than the NFT's last accepted revision.
		RevisionRegressed,
		/// Beneficiary, subscriber, pool, tick range, or liquidity changed without a revision
		/// increase.
		PositionChangedWithoutRevision,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Approve immutable deployment bindings and the canonical pool key.
		/// Root must verify the pinned implementations and their immutable bindings first.
		#[pallet::call_index(0)]
		#[pallet::weight(<T as Config>::WeightInfo::register_pool())]
		pub fn register_pool(
			origin: OriginFor<T>,
			pool_manager: H160,
			config: PoolConfig,
		) -> DispatchResult {
			ensure_root(origin)?;

			ensure!(pool_manager != H160::zero(), Error::<T>::InvalidPool);
			ensure!(config.position_manager != H160::zero(), Error::<T>::InvalidPool);
			ensure!(config.subscriber != H160::zero(), Error::<T>::InvalidPool);

			ensure!(pool_manager != config.position_manager, Error::<T>::InvalidPool);
			ensure!(config.subscriber != config.position_manager, Error::<T>::InvalidPool);
			ensure!(config.subscriber != pool_manager, Error::<T>::InvalidPool);

			ensure!(config.key.currency0 < config.key.currency1, Error::<T>::InvalidPool);
			ensure!(config.key.fee <= 0xff_ffff, Error::<T>::InvalidPool);
			ensure!((1..=32_767).contains(&config.key.tick_spacing), Error::<T>::InvalidPool);

			let pool_id = evm::pool_id(&config.key);
			ensure!(
				!Pools::<T>::contains_key(pool_manager, pool_id),
				Error::<T>::PoolAlreadyRegistered
			);

			Pools::<T>::insert(pool_manager, pool_id, config);
			Self::deposit_event(Event::PoolRegistered { pool_manager, pool_id, config });
			Ok(())
		}

		/// Register a baseline, extend a maintained revision, or reset after an unproven change.
		#[pallet::call_index(1)]
		#[pallet::weight(weights::checkpoint_position::<T>(proof))]
		pub fn checkpoint_position(
			origin: OriginFor<T>,
			pool_manager: H160,
			pool_id: H256,
			token_id: U256,
			proof: Box<PositionProof>,
		) -> DispatchResult {
			let beneficiary = ensure_signed(origin)?;
			let config =
				Pools::<T>::get(pool_manager, pool_id).ok_or(Error::<T>::PoolNotRegistered)?;
			let execution_proof = &proof.subscriber.execution_block_proof;
			ensure!(
				execution_proof == &proof.position_manager.execution_block_proof,
				Error::<T>::InconsistentProofBlocks
			);
			ensure!(
				execution_proof == &proof.pool_manager.execution_block_proof,
				Error::<T>::InconsistentProofBlocks
			);

			// Decode only the expected slots; callers cannot choose which position the witness
			// describes.
			let position = evm::decode_position::<T>(&config, pool_id, token_id, &proof)?;
			ensure!(
				beneficiary.encode().as_slice() == position.argon_account_id.as_bytes(),
				Error::<T>::NotBeneficiary
			);

			let header = EthereumVerifier::<T>::verified_execution_header(execution_proof)
				.map_err(|_| Error::<T>::InvalidProof)?;
			EthereumVerifier::<T>::verify_account_storage_proof(
				config.subscriber,
				&proof.subscriber,
			)
			.map_err(|_| Error::<T>::InvalidProof)?;
			EthereumVerifier::<T>::verify_account_storage_proof(
				config.position_manager,
				&proof.position_manager,
			)
			.map_err(|_| Error::<T>::InvalidProof)?;
			EthereumVerifier::<T>::verify_account_storage_proof(pool_manager, &proof.pool_manager)
				.map_err(|_| Error::<T>::InvalidProof)?;

			let block = ProofBlock {
				hash: header.block_hash,
				number: header.block_number,
				timestamp_millis: header.timestamp_millis,
			};
			let checkpoint = PositionCheckpoint {
				beneficiary,
				pool_manager,
				pool_id,
				subscriber: config.subscriber,
				revision: position.revision,
				tick_lower: position.tick_lower,
				tick_upper: position.tick_upper,
				liquidity: position.liquidity,
				maintained_since: block,
				proven_through: block,
			};
			Self::record_checkpoint(config.position_manager, token_id, checkpoint)
		}
	}

	impl<T: Config> Pallet<T> {
		pub(crate) fn record_checkpoint(
			position_manager: H160,
			token_id: U256,
			mut checkpoint: PositionCheckpoint<T::AccountId>,
		) -> DispatchResult {
			let block = checkpoint.proven_through;
			let outcome;
			if let Some(previous) = PositionCheckpoints::<T>::get(position_manager, token_id) {
				ensure!(block.number > previous.proven_through.number, Error::<T>::StaleCheckpoint);
				ensure!(
					block.timestamp_millis > previous.proven_through.timestamp_millis,
					Error::<T>::StaleCheckpoint
				);
				ensure!(checkpoint.revision >= previous.revision, Error::<T>::RevisionRegressed);

				if checkpoint.revision == previous.revision {
					ensure!(
						checkpoint.beneficiary == previous.beneficiary,
						Error::<T>::PositionChangedWithoutRevision
					);
					ensure!(
						checkpoint.subscriber == previous.subscriber,
						Error::<T>::PositionChangedWithoutRevision
					);
					ensure!(
						checkpoint.pool_manager == previous.pool_manager,
						Error::<T>::PositionChangedWithoutRevision
					);
					ensure!(
						checkpoint.pool_id == previous.pool_id,
						Error::<T>::PositionChangedWithoutRevision
					);
					ensure!(
						checkpoint.tick_lower == previous.tick_lower,
						Error::<T>::PositionChangedWithoutRevision
					);
					ensure!(
						checkpoint.tick_upper == previous.tick_upper,
						Error::<T>::PositionChangedWithoutRevision
					);
					ensure!(
						checkpoint.liquidity == previous.liquidity,
						Error::<T>::PositionChangedWithoutRevision
					);

					checkpoint.maintained_since = previous.maintained_since;
					outcome = CheckpointOutcome::Maintained { from: previous.proven_through };
				} else {
					outcome = CheckpointOutcome::Reset {
						previous_revision: previous.revision,
						unproven_from: previous.proven_through,
					};
				}
			} else {
				outcome = CheckpointOutcome::Registered;
			}

			// Keep the only position write after authentication and continuity checks.
			PositionCheckpoints::<T>::insert(position_manager, token_id, &checkpoint);
			Self::deposit_event(Event::PositionCheckpointed {
				position_manager,
				token_id,
				checkpoint: Box::new(checkpoint),
				outcome,
			});
			Ok(())
		}
	}
}
