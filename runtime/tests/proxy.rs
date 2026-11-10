use crate::{AccountId, ProxyType, RuntimeCall, SessionKeys};
use codec::{Decode, Encode};
use frame_support::traits::InstanceFilter;
use polkadot_sdk::*;

#[cfg(feature = "try-runtime")]
#[test]
fn upgrade_check_rejects_legacy_proxy_types_and_malformed_rows() {
	use super::mock::{account, externalities};
	use crate::Runtime;
	use argon_runtime_common::try_state::check_proxy_types;
	use pallet_proxy::{Proxies, ProxyDefinition};

	externalities().execute_with(|| {
		let key = Proxies::<Runtime>::hashed_key_for(account(1));
		for proxy_type in [0u8, 4, 1, 7, 8] {
			let value =
				(vec![ProxyDefinition { delegate: account(2), proxy_type, delay: 0u32 }], 0u128)
					.encode();
			sp_io::storage::set(&key, &value);
			assert_eq!(check_proxy_types::<Runtime>().is_ok(), matches!(proxy_type, 0 | 4));
		}

		for value in [vec![0xff], vec![0, 0]] {
			sp_io::storage::set(&key, &value);
			assert!(check_proxy_types::<Runtime>().is_err());
		}
	});
}

#[test]
fn proxy_type_scale_indices_are_stable_and_removed_types_do_not_decode() {
	for (proxy_type, index) in [(ProxyType::Any, 0), (ProxyType::MiningBidRealPaysFee, 4)] {
		assert_eq!(proxy_type.encode(), vec![index]);
		assert_eq!(ProxyType::decode(&mut &[index][..]).unwrap(), proxy_type);
	}

	for index in 0..=u8::MAX {
		if matches!(index, 0 | 4) {
			continue;
		}
		assert!(ProxyType::decode(&mut &[index][..]).is_err(), "decoded proxy type {index}");
	}
}

#[test]
fn full_authority_proxy_types_allow_calls_and_delegate_management() {
	assert_eq!(ProxyType::default(), ProxyType::Any);

	for proxy_type in [ProxyType::Any, ProxyType::MiningBidRealPaysFee] {
		assert!(ProxyType::Any.is_superset(&proxy_type));
		assert_eq!(
			ProxyType::MiningBidRealPaysFee.is_superset(&proxy_type),
			proxy_type == ProxyType::MiningBidRealPaysFee
		);
	}

	for call in [
		RuntimeCall::System(frame_system::Call::remark { remark: vec![] }),
		RuntimeCall::Balances(pallet_balances::Call::transfer_keep_alive {
			dest: AccountId::new([3; 32]).into(),
			value: 1,
		}),
		RuntimeCall::Proxy(pallet_proxy::Call::remove_proxies {}),
	] {
		assert!(ProxyType::Any.filter(&call));
		assert!(!ProxyType::MiningBidRealPaysFee.filter(&call));
	}
}

#[test]
fn mining_bid_proxy_only_allows_bids_and_simple_bid_batches() {
	let bid = RuntimeCall::MiningSlot(pallet_mining_slot::Call::bid {
		bid: 1,
		keys: SessionKeys {
			grandpa: sp_core::ed25519::Public::from_raw([1; 32]).into(),
			block_seal_authority: sp_core::ed25519::Public::from_raw([2; 32]).into(),
		},
		mining_account_id: None,
		target_frame_id: None,
		max_hash: None,
	});
	let remark = RuntimeCall::System(frame_system::Call::remark { remark: vec![] });
	let nested_batch =
		RuntimeCall::Utility(pallet_utility::Call::batch { calls: vec![bid.clone()] });
	assert!(ProxyType::MiningBidRealPaysFee.filter(&bid));
	assert!(!ProxyType::MiningBidRealPaysFee.filter(&remark));

	for (calls, allowed) in [
		(vec![bid.clone(), bid.clone()], true),
		(vec![bid, remark], false),
		(vec![nested_batch], false),
	] {
		for batch in [
			pallet_utility::Call::batch { calls: calls.clone() },
			pallet_utility::Call::batch_all { calls: calls.clone() },
			pallet_utility::Call::force_batch { calls },
		] {
			assert_eq!(
				ProxyType::MiningBidRealPaysFee.filter(&RuntimeCall::Utility(batch)),
				allowed
			);
		}
	}
}
