// Radicle Registry
// Copyright (C) 2019 Monadic GmbH <radicle@monadic.xyz>
// Copyright 2023 Ulixee
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License version 3 as
// published by the Free Software Foundation.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

use codec::{DecodeAll, Encode, Error};
use sp_runtime::{ConsensusEngineId, Digest, DigestItem};

use crate::Moment;

const CONSENSUS_ID: ConsensusEngineId = *b"time";

pub fn load(digest: &Digest) -> Option<Result<Moment, Error>> {
	digest
		.log(|item| match item {
			DigestItem::Consensus(CONSENSUS_ID, encoded) => Some(encoded),
			_ => None,
		})
		.map(|encoded| DecodeAll::decode_all(&mut encoded.as_slice()))
}

pub fn digest_item(timestamp: Moment) -> DigestItem {
	DigestItem::Consensus(CONSENSUS_ID, timestamp.encode())
}
