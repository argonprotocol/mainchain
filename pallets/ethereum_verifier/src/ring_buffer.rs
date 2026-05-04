// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2023 Snowfork <hello@snowfork.com>
use codec::FullCodec;
use core::{cmp::Ord, marker::PhantomData, ops::Add};
use frame_support::storage::{StorageMap, StorageValue, types::QueryKindTrait};
use sp_core::{Get, GetDefault};
use sp_runtime::traits::{One, Zero};

pub trait RingBufferMap<Key, Value, QueryKind>
where
	Key: FullCodec,
	Value: FullCodec,
	QueryKind: QueryKindTrait<Value, GetDefault>,
{
	fn insert(k: Key, v: Value);

	fn contains_key(k: Key) -> bool;

	fn get(k: Key) -> QueryKind::Query;
}

pub struct RingBufferMapImpl<Index, B, CurrentIndex, Intermediate, M, QueryKind>(
	PhantomData<(Index, B, CurrentIndex, Intermediate, M, QueryKind)>,
);

impl<Key, Value, Index, B, CurrentIndex, Intermediate, M, QueryKind>
	RingBufferMap<Key, Value, QueryKind>
	for RingBufferMapImpl<Index, B, CurrentIndex, Intermediate, M, QueryKind>
where
	Key: FullCodec + Clone,
	Value: FullCodec,
	Index: Ord + One + Zero + Add<Output = Index> + Copy + FullCodec + Eq,
	B: Get<Index>,
	CurrentIndex: StorageValue<Index, Query = Index>,
	Intermediate: StorageMap<Index, Key, Query = Key>,
	M: StorageMap<Key, Value, Query = QueryKind::Query>,
	QueryKind: QueryKindTrait<Value, GetDefault>,
{
	fn insert(k: Key, v: Value) {
		let bound = B::get();
		let mut current_index = CurrentIndex::get();

		if (current_index + Index::one()) >= bound {
			current_index = Index::zero();
		} else {
			current_index = current_index + Index::one();
		}

		if Intermediate::contains_key(current_index) {
			let older_key = Intermediate::get(current_index);
			M::remove(older_key);
		}

		Intermediate::insert(current_index, k.clone());
		CurrentIndex::set(current_index);
		M::insert(k, v);
	}

	fn contains_key(k: Key) -> bool {
		M::contains_key(k)
	}

	fn get(k: Key) -> M::Query {
		M::get(k)
	}
}
