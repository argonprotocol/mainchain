use std::{fmt::Debug, sync::Arc};

use codec::{Decode, Encode};
use sc_client_api::{self, backend::AuxStore};
use sc_consensus::BlockImportParams;
use sp_runtime::traits::{Block as BlockT, Header};

use crate::error::Error;

/// Auxiliary storage prefix for Ulx engine.
pub const ULX_AUX_PREFIX: [u8; 4] = *b"ulx:";

/// Get the auxiliary storage key used by engine to store total difficulty.
pub fn aux_key<T: AsRef<[u8]>>(hash: &T) -> Vec<u8> {
	ULX_AUX_PREFIX.iter().chain(hash.as_ref()).copied().collect()
}

/// Define methods that total difficulty should implement.
pub trait TotalDifficulty {
	fn increment(&mut self, other: Self);
}

impl TotalDifficulty for u128 {
	fn increment(&mut self, other: Self) {
		let ret = self.saturating_add(other);
		*self = ret;
	}
}

/// Auxiliary storage data for Ulx.
#[derive(Encode, Decode, Clone, Debug, Default)]
pub struct UlxAux<Difficulty>
where
	Difficulty: Into<u128> + TryFrom<u128> + Copy,
{
	/// Difficulty of the current block.
	pub difficulty: Difficulty,
	/// Total difficulty up to current block.
	pub total_difficulty: Difficulty,
}

impl<Difficulty> UlxAux<Difficulty>
where
	Difficulty: Decode + Default + Into<u128> + TryFrom<u128> + TotalDifficulty + Encode + Copy,
{
	/// Read the auxiliary from client.
	pub fn read<C: AuxStore, B: BlockT>(client: &C, hash: &B::Hash) -> Result<Self, Error<B>> {
		let key = aux_key(&hash);

		match client.get_aux(&key).map_err(Error::Client)? {
			Some(bytes) => Self::decode(&mut &bytes[..]).map_err(Error::Codec),
			None => Ok(Self::default()),
		}
	}

	pub fn record<C: AuxStore, B: BlockT>(
		client: &Arc<C>,
		best_header: B::Header,
		block: &mut BlockImportParams<B>,
		difficulty: Difficulty,
	) -> Result<(Self, Self), Error<B>> {
		let best_hash = best_header.hash();
		let best_aux = Self::read::<C, B>(client.as_ref(), &best_hash)?;

		let parent_hash = block.header.parent_hash();
		let mut aux = Self::read::<C, B>(client.as_ref(), &parent_hash)?;

		aux.difficulty = difficulty.clone();
		aux.total_difficulty.increment(difficulty.try_into().unwrap());
		// TODO: put tax id into aux. We're only going to accept the first block from a
		//	 tax record we see.. TBD: how do we handle if the longest chain used a different version
		// 	of the block? We would stall

		let key = aux_key(&block.post_hash());
		block.auxiliary.push((key, Some(aux.encode())));
		Ok((aux, best_aux))
	}
}
