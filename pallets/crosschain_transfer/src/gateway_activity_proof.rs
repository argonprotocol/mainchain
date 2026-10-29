use alloc::vec::Vec;
use argon_primitives::{
	EthereumLog, EthereumLogTopics, MAX_ETHEREUM_EXECUTION_HEADER_RLP_BYTES,
	MAX_ETHEREUM_HEADER_CHAIN_LEN, MAX_ETHEREUM_LOG_DATA_BYTES,
};
use codec::{Compact, Decode, DecodeWithMemTracking, Encode, Input, MaxEncodedLen};
use core::ops::Deref;
use polkadot_sdk::{
	frame_support::{BoundedVec, CloneNoBound, DebugNoBound, EqNoBound, PartialEqNoBound},
	sp_runtime::traits::Get,
};
use scale_info::TypeInfo;

/// Aggregate log-data budget, reserving room for the full execution-header ancestry path.
/// Pallet integrity checks also account for log envelopes, SCALE prefixes, and the shared witness.
pub const MAX_GATEWAY_ACTIVITY_LOG_DATA_BYTES: u32 =
	3 * 1024 * 1024 - MAX_ETHEREUM_HEADER_CHAIN_LEN * MAX_ETHEREUM_EXECUTION_HEADER_RLP_BYTES;

/// A SCALE-compatible log vector bounded by both activity count and total data bytes.
/// Mutation goes through checked construction so MaxEncodedLen remains a true upper bound.
#[derive(Encode, CloneNoBound, PartialEqNoBound, EqNoBound, DebugNoBound)]
pub struct GatewayActivityLogs<MaxLogs: Get<u32>>(BoundedVec<EthereumLog, MaxLogs>);

// Preserve the existing client-facing vector shape as well as its SCALE encoding.
impl<MaxLogs: Get<u32> + 'static> TypeInfo for GatewayActivityLogs<MaxLogs> {
	type Identity = <BoundedVec<EthereumLog, MaxLogs> as TypeInfo>::Identity;

	fn type_info() -> scale_info::Type {
		BoundedVec::<EthereumLog, MaxLogs>::type_info()
	}
}

impl<MaxLogs: Get<u32>> TryFrom<Vec<EthereumLog>> for GatewayActivityLogs<MaxLogs> {
	type Error = &'static str;

	fn try_from(logs: Vec<EthereumLog>) -> Result<Self, Self::Error> {
		let logs = BoundedVec::try_from(logs).map_err(|_| "Too many gateway activities")?;
		Self::try_from(logs)
	}
}

impl<MaxLogs: Get<u32>> TryFrom<BoundedVec<EthereumLog, MaxLogs>> for GatewayActivityLogs<MaxLogs> {
	type Error = &'static str;

	fn try_from(logs: BoundedVec<EthereumLog, MaxLogs>) -> Result<Self, Self::Error> {
		let bytes = logs.iter().try_fold(0usize, |total, log| total.checked_add(log.data.len()));
		if bytes.is_none_or(|bytes| bytes > MAX_GATEWAY_ACTIVITY_LOG_DATA_BYTES as usize) {
			return Err("Gateway activity log data exceeds byte budget");
		}
		Ok(Self(logs))
	}
}

impl<MaxLogs: Get<u32>> Deref for GatewayActivityLogs<MaxLogs> {
	type Target = [EthereumLog];

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl<MaxLogs: Get<u32>> Default for GatewayActivityLogs<MaxLogs> {
	fn default() -> Self {
		Self(BoundedVec::default())
	}
}

impl<MaxLogs: Get<u32>> Decode for GatewayActivityLogs<MaxLogs> {
	fn decode<I: Input>(input: &mut I) -> Result<Self, codec::Error> {
		Self::try_from(BoundedVec::<EthereumLog, MaxLogs>::decode(input)?).map_err(Into::into)
	}
}

impl<MaxLogs: Get<u32>> DecodeWithMemTracking for GatewayActivityLogs<MaxLogs> {}

impl<MaxLogs: Get<u32>> MaxEncodedLen for GatewayActivityLogs<MaxLogs> {
	fn max_encoded_len() -> usize {
		let count = MaxLogs::get() as usize;
		let log_envelope =
			20 + EthereumLogTopics::max_encoded_len() + Compact::<u32>::max_encoded_len();
		let data = count
			.saturating_mul(MAX_ETHEREUM_LOG_DATA_BYTES as usize)
			.min(MAX_GATEWAY_ACTIVITY_LOG_DATA_BYTES as usize);
		Compact::<u32>::max_encoded_len() + count.saturating_mul(log_envelope) + data
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use codec::DecodeWithMemLimit;
	use polkadot_sdk::sp_runtime::traits::ConstU32;

	type Logs = GatewayActivityLogs<ConstU32<4096>>;

	fn log(bytes: usize) -> EthereumLog {
		EthereumLog {
			address: Default::default(),
			topics: Default::default(),
			data: vec![0xff; bytes].try_into().unwrap(),
		}
	}

	#[test]
	fn keeps_the_existing_vector_encoding_and_full_activity_count() {
		assert_eq!(
			scale_info::MetaType::new::<Logs>(),
			scale_info::MetaType::new::<BoundedVec<EthereumLog, ConstU32<4096>>>()
		);
		assert_eq!(Logs::type_info(), BoundedVec::<EthereumLog, ConstU32<4096>>::type_info());
		let logs = vec![log(320); 4096];
		let encoded = logs.encode();
		let bounded = Logs::try_from(logs).unwrap();
		assert_eq!(bounded.encode(), encoded);
		assert_eq!(Logs::decode(&mut encoded.as_slice()).unwrap(), bounded);
		assert!(encoded.len() <= Logs::max_encoded_len());
		assert!(GatewayActivityLogs::<ConstU32<1>>::decode(&mut encoded.as_slice()).is_err());
	}

	#[test]
	fn enforces_the_aggregate_budget_on_construction_and_untrusted_decode() {
		let budget = MAX_GATEWAY_ACTIVITY_LOG_DATA_BYTES as usize;
		let max_log = MAX_ETHEREUM_LOG_DATA_BYTES as usize;
		let mut logs = vec![log(max_log); budget / max_log];
		logs.push(log(budget % max_log));
		let bounded = Logs::try_from(logs.clone()).unwrap();
		assert!(bounded.encoded_size() <= Logs::max_encoded_len());
		let encoded = bounded.encode();
		assert_eq!(Logs::decode(&mut encoded.as_slice()).unwrap(), bounded);
		assert!(Logs::decode_with_mem_limit(&mut encoded.as_slice(), budget / 2).is_err());

		logs.push(log(1));
		assert!(Logs::try_from(logs.clone()).is_err());
		assert!(Logs::decode(&mut logs.encode().as_slice()).is_err());
	}
}
