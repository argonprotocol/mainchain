use crate::{
	ismp_module::{convert_to_erc20, Asset, Body, EvmChain},
	mock::{ChainTransfer as ChainTransferPallet, *},
	pallet::{ExpiringTransfersOutByNotary, NextTransferId},
	ActiveEvmDestinations, Error, Event, TokenGatewayAddresses, TransferToEvm,
};
use alloy_sol_types::SolValue;
use argon_primitives::{
	notebook::{AccountOrigin, ChainTransfer, NotebookHeader},
	tick::Tick,
	NotebookEventHandler,
};
use frame_support::{assert_err, assert_noop, assert_ok};
use ismp::{
	host::StateMachine,
	module::IsmpModule,
	router::{PostRequest, Request, Timeout},
};
use sp_core::{bounded_vec, ByteArray, H160};
use sp_keyring::AccountKeyring::{Alice, Bob};
use sp_runtime::testing::H256;
use std::time::SystemTime;

#[test]
fn it_can_send_funds_to_localchain() {
	new_test_ext().execute_with(|| {
		let who = Bob.to_account_id();
		// Go past genesis block so events get deposited
		System::set_block_number(1);
		set_argons(&who, 5000);
		assert_ok!(ChainTransferPallet::send_to_localchain(
			RuntimeOrigin::signed(who.clone()),
			1000,
			1,
		));
		assert_eq!(Balances::free_balance(&who), 4000);
		let expires_tick: Tick = 1 + TransferExpirationTicks::get();
		assert_eq!(ExpiringTransfersOutByNotary::<Test>::get(1, expires_tick)[0], 1);
		assert_eq!(NextTransferId::<Test>::get(), Some(2));
	});
}

#[test]
fn it_allows_you_to_transfer_full_balance() {
	new_test_ext().execute_with(|| {
		let who = Bob.to_account_id();
		// Go past genesis block so events get deposited
		System::set_block_number(1);
		System::inc_account_nonce(&who);
		set_argons(&who, 5000);
		assert_ok!(ChainTransferPallet::send_to_localchain(
			RuntimeOrigin::signed(who.clone()),
			5000,
			1,
		));
		assert_eq!(Balances::free_balance(&who), 0);
		assert!(!System::account_exists(&who));
	});
}

#[test]
fn it_expires_transfers_on_notebook_tick() {
	new_test_ext().execute_with(|| {
		let who = Bob.to_account_id();
		// Go past genesis block so events get deposited
		System::set_block_number(1);
		set_argons(&who, 2000);
		assert_ok!(ChainTransferPallet::send_to_localchain(
			RuntimeOrigin::signed(who.clone()),
			2000,
			1,
		));
		assert_eq!(Balances::free_balance(&who), 0);
		assert!(!System::account_exists(&who));
		let expires_tick: Tick = 1 + TransferExpirationTicks::get();
		assert_eq!(ExpiringTransfersOutByNotary::<Test>::get(1, expires_tick)[0], 1);

		ChainTransferPallet::notebook_submitted(&NotebookHeader {
			notary_id: 1,
			notebook_number: 10,
			tick: expires_tick,
			chain_transfers: Default::default(),
			changed_accounts_root: Default::default(),
			changed_account_origins: Default::default(),
			version: 1,
			tax: 0,
			block_voting_power: 0,
			blocks_with_votes: Default::default(),
			block_votes_root: H256::random(),
			secret_hash: H256::random(),

			parent_secret: None,
			block_votes_count: 0,
			domains: Default::default(),
		});
		assert!(System::account_exists(&who));
		assert_eq!(Balances::free_balance(&who), 2000);
	});
}

#[test]
fn it_can_handle_multiple_transfer() {
	new_test_ext().execute_with(|| {
		let who = Bob.to_account_id();
		// Go past genesis block so events get deposited
		MaxPendingTransfersOutPerBlock::set(2);
		System::set_block_number(1);
		set_argons(&who, 5000);
		assert_ok!(ChainTransferPallet::send_to_localchain(
			RuntimeOrigin::signed(who.clone()),
			1000,
			1,
		));
		System::inc_account_nonce(&who);
		assert_ok!(ChainTransferPallet::send_to_localchain(
			RuntimeOrigin::signed(who.clone()),
			700,
			1,
		),);
		assert_eq!(Balances::free_balance(&who), 3300);
		let expires_tick: Tick = 1 + TransferExpirationTicks::get();
		assert_eq!(ExpiringTransfersOutByNotary::<Test>::get(1, expires_tick), vec![1, 2]);

		System::inc_account_nonce(&who);
		// We have a max number of transfers out per block
		assert_noop!(
			ChainTransferPallet::send_to_localchain(RuntimeOrigin::signed(who.clone()), 1200, 1,),
			Error::<Test>::MaxBlockTransfersExceeded
		);
	});
}

#[test]
fn it_can_handle_transfers_in() {
	MaxNotebookBlocksToRemember::set(2);
	new_test_ext().execute_with(|| {
		// Go past genesis block so events get deposited
		System::set_block_number(1);
		let who = Bob.to_account_id();
		set_argons(&who, 5000);
		assert_ok!(ChainTransferPallet::send_to_localchain(
			RuntimeOrigin::signed(who.clone()),
			5000,
			1,
		));
		let expires_tick: Tick = 1 + TransferExpirationTicks::get();
		assert_eq!(ExpiringTransfersOutByNotary::<Test>::get(1, expires_tick)[0], 1);

		let changed_accounts_root = H256::random();
		ChainTransferPallet::notebook_submitted(&NotebookHeader {
			notary_id: 1,
			notebook_number: 1,
			tick: 1,
			chain_transfers: bounded_vec![ChainTransfer::ToLocalchain { transfer_id: 1 }],
			changed_accounts_root,
			changed_account_origins: bounded_vec![AccountOrigin {
				notebook_number: 1,
				account_uid: 1
			}],
			version: 1,
			tax: 0,
			block_voting_power: 0,
			blocks_with_votes: Default::default(),
			block_votes_root: H256::random(),
			secret_hash: H256::random(),

			parent_secret: None,
			block_votes_count: 0,
			domains: Default::default(),
		});

		assert_eq!(ExpiringTransfersOutByNotary::<Test>::get(1, expires_tick).len(), 0);
		assert_eq!(Balances::free_balance(&who), 0);

		let change_root_2 = H256::random();
		ChainTransferPallet::notebook_submitted(&NotebookHeader {
			notary_id: 1,
			notebook_number: 2,
			tick: 2,
			chain_transfers: bounded_vec![ChainTransfer::ToMainchain {
				account_id: who.clone(),
				amount: 5000
			}],
			changed_accounts_root: change_root_2,
			changed_account_origins: bounded_vec![AccountOrigin {
				notebook_number: 1,
				account_uid: 1
			}],
			version: 1,
			tax: 0,
			block_voting_power: 0,
			blocks_with_votes: Default::default(),
			block_votes_root: H256::random(),
			secret_hash: H256::random(),
			parent_secret: None,
			block_votes_count: 0,
			domains: Default::default(),
		});
		assert_eq!(Balances::free_balance(&who), 5000);
	});
}

#[test]
fn it_reduces_circulation_on_tax() {
	MaxNotebookBlocksToRemember::set(2);
	new_test_ext().execute_with(|| {
		// Go past genesis block so events get deposited
		System::set_block_number(1);
		let who = ChainTransferPallet::notary_account_id(1);
		set_argons(&who, 25000);
		assert_eq!(Balances::total_issuance(), 25_000);

		ChainTransferPallet::notebook_submitted(&NotebookHeader {
			notary_id: 1,
			notebook_number: 1,
			tick: 1,
			chain_transfers: bounded_vec![],
			changed_accounts_root: H256::random(),
			changed_account_origins: bounded_vec![],
			version: 1,
			tax: 2000,
			block_voting_power: 0,
			blocks_with_votes: Default::default(),
			block_votes_root: H256::random(),
			secret_hash: H256::random(),
			parent_secret: None,
			block_votes_count: 0,
			domains: Default::default(),
		});

		assert_eq!(Balances::total_issuance(), 23_000);
		assert_eq!(Balances::free_balance(&who), 23_000);
	})
}

#[test]
fn it_doesnt_allow_a_notary_balance_to_go_negative() {
	new_test_ext().execute_with(|| {
		// Go past genesis block so events get deposited
		System::set_block_number(2);

		ChainTransferPallet::notebook_submitted(&NotebookHeader {
			notary_id: 1,
			notebook_number: 1,
			tick: 0,
			chain_transfers: bounded_vec![ChainTransfer::ToMainchain {
				account_id: Bob.to_account_id(),
				amount: 5000
			}],
			changed_accounts_root: H256::random(),
			changed_account_origins: bounded_vec![],
			version: 1,
			tax: 0,
			block_voting_power: 0,
			blocks_with_votes: Default::default(),
			block_votes_root: H256::random(),
			secret_hash: H256::random(),
			parent_secret: None,
			block_votes_count: 0,
			domains: Default::default(),
		});
		System::assert_last_event(
			Event::<Test>::TransferFromLocalchainError {
				notary_id: 1,
				notebook_number: 1,
				amount: 5000,
				account_id: Bob.to_account_id(),
				error: Error::<Test>::InsufficientNotarizedFunds.into(),
			}
			.into(),
		);
	});
}

#[test]
fn it_allows_you_to_transfer_to_evm_chains() {
	new_test_ext().execute_with(|| {
		let who = Bob.to_account_id();
		// Go past genesis block so events get deposited
		System::set_block_number(1);
		set_argons(&who, 5000);
		let account = H160::random();
		let evm_chain = EvmChain::Ethereum;
		let transfer = TransferToEvm {
			evm_chain: evm_chain.clone(),
			relayer_fee: 0,
			timeout: 60,
			amount: 1009,
			asset: Asset::Argon,
			recipient: account,
		};
		assert_err!(
			ChainTransferPallet::send_to_evm_chain(
				RuntimeOrigin::signed(who.clone()),
				transfer.clone()
			),
			Error::<Test>::EvmChainNotSupported,
			// if not in active evm destinations, not supported
		);
		assert_eq!(Balances::free_balance(&who), 5000);

		ActiveEvmDestinations::<Test>::try_mutate(|destinations| {
			destinations.try_push(evm_chain.clone())
		})
		.expect("Failed to insert destination");

		assert_err!(
			ChainTransferPallet::send_to_evm_chain(
				RuntimeOrigin::signed(who.clone()),
				transfer.clone()
			),
			Error::<Test>::EvmChainNotConfigured,
			// "if nothing in the token gateways, not configured yet"
		);
		assert_eq!(Balances::free_balance(&who), 5000);

		TokenGatewayAddresses::<Test>::insert(
			evm_chain.get_state_machine(true),
			[0u8; 32].to_vec(),
		);

		assert_ok!(ChainTransferPallet::send_to_evm_chain(
			RuntimeOrigin::signed(who.clone()),
			transfer
		));
		assert_eq!(Balances::free_balance(&who), 3991);
		assert_eq!(Balances::free_balance(ChainTransferPallet::pallet_account()), 1009);
		let events = System::events();
		events
			.iter()
			.find_map(|event| match event.event {
				RuntimeEvent::Ismp(pallet_ismp::Event::<Test>::Request {
					source_chain,
					dest_chain,
					..
				}) => {
					assert_eq!(source_chain, StateMachine::Substrate(*b"argn"));
					assert_eq!(dest_chain, evm_chain.get_state_machine(true));
					Some(())
				},
				_ => None,
			})
			.expect("Expected Ismp PostRequest event");

		assert!(events.iter().any(|event| {
			match &event.event {
				RuntimeEvent::ChainTransfer(ev) => {
					if let Event::TransferToEvm::<Test> {
						evm_chain, to, asset, from, amount, ..
					} = ev
					{
						assert_eq!(evm_chain, &EvmChain::Ethereum);
						assert_eq!(to, &account);
						assert_eq!(asset, &Asset::Argon);
						assert_eq!(from, &who);
						assert_eq!(amount, &1009);
						return true
					}
					false
				},
				_ => false,
			}
		}));
	})
}

#[test]
fn it_allows_you_to_transfer_ownership_tokens_to_evm_chains() {
	new_test_ext().execute_with(|| {
		let who = Bob.to_account_id();
		// Go past genesis block so events get deposited
		System::set_block_number(1);
		set_ownership(&who, 5000);
		let account = H160::random();
		let evm_chain = EvmChain::Base;
		let transfer = TransferToEvm {
			evm_chain: evm_chain.clone(),
			relayer_fee: 0,
			timeout: 60,
			amount: 1009,
			asset: Asset::OwnershipToken,
			recipient: account,
		};
		assert_err!(
			ChainTransferPallet::send_to_evm_chain(
				RuntimeOrigin::signed(who.clone()),
				transfer.clone()
			),
			Error::<Test>::EvmChainNotSupported,
			// if not in active evm destinations, not supported
		);
		assert_eq!(Ownership::free_balance(&who), 5000);

		ActiveEvmDestinations::<Test>::try_mutate(|destinations| {
			destinations.try_push(evm_chain.clone())
		})
		.expect("Failed to insert destination");
		TokenGatewayAddresses::<Test>::insert(
			evm_chain.get_state_machine(true),
			[0u8; 32].to_vec(),
		);

		assert_ok!(ChainTransferPallet::send_to_evm_chain(
			RuntimeOrigin::signed(who.clone()),
			transfer
		));
		assert_eq!(Ownership::free_balance(&who), 3991);
		assert_eq!(Ownership::free_balance(ChainTransferPallet::pallet_account()), 1009);
	})
}

#[test]
fn it_refunds_timed_out_requests() {
	new_test_ext().execute_with(|| {
		let who = Bob.to_account_id();
		// Go past genesis block so events get deposited
		System::set_block_number(1);
		set_argons(&who, 5000);
		let account = H160::random();
		let evm_chain = EvmChain::Ethereum;
		let gateway_address = [3u8; 32].to_vec();
		let transfer = TransferToEvm {
			evm_chain: evm_chain.clone(),
			relayer_fee: 0,
			timeout: 60,
			amount: 1009,
			asset: Asset::Argon,
			recipient: account,
		};
		ActiveEvmDestinations::<Test>::try_mutate(|destinations| {
			destinations.try_push(evm_chain.clone())
		})
		.expect("Failed to insert destination");
		TokenGatewayAddresses::<Test>::insert(
			evm_chain.get_state_machine(true),
			gateway_address.clone(),
		);

		assert_ok!(ChainTransferPallet::send_to_evm_chain(
			RuntimeOrigin::signed(who.clone()),
			transfer
		));
		assert_eq!(Balances::free_balance(&who), 3991);
		assert_eq!(Balances::free_balance(ChainTransferPallet::pallet_account()), 1009);

		let timeout_request = Timeout::Request(Request::Post(PostRequest {
			body: {
				let mut encoded = vec![0];
				let body =
					Body::send_to_evm(1009u128, Asset::Argon.asset_id(), who.clone(), account);
				encoded.extend_from_slice(&Body::abi_encode(&body));
				encoded
			},
			to: [&[0u8; 12][..], &account[..]].concat(),
			from: gateway_address.clone(),
			source: evm_chain.get_state_machine(true),
			dest: StateMachine::Substrate(*b"argn"),
			nonce: 0,
			timeout_timestamp: SystemTime::now().elapsed().unwrap().as_secs(),
		}));
		ChainTransferPallet::default()
			.on_timeout(timeout_request)
			.expect("should handle");

		assert_eq!(Balances::free_balance(&who), 5000);
		assert_eq!(Balances::free_balance(ChainTransferPallet::pallet_account()), 0);
		System::assert_last_event(
			Event::<Test>::TransferToEvmExpired {
				evm_chain: EvmChain::Ethereum,
				amount: 1009,
				asset: Asset::Argon,
				from: who,
				to: account,
			}
			.into(),
		);
	})
}

#[test]
fn it_transfers_in_from_evm() {
	new_test_ext().execute_with(|| {
		let who = Bob.to_account_id();
		// Go past genesis block so events get deposited
		System::set_block_number(1);
		set_argons(&who, 5000);
		let evm_account = H160::random();
		let evm_chain = EvmChain::Ethereum;
		let gateway_address = [3u8; 32].to_vec();
		let transfer = TransferToEvm {
			evm_chain: evm_chain.clone(),
			relayer_fee: 0,
			timeout: 60,
			amount: 1009,
			asset: Asset::Argon,
			recipient: evm_account,
		};
		ActiveEvmDestinations::<Test>::try_mutate(|destinations| {
			destinations.try_push(evm_chain.clone())
		})
		.expect("Failed to insert destination");
		TokenGatewayAddresses::<Test>::insert(
			evm_chain.get_state_machine(true),
			gateway_address.clone(),
		);

		assert_ok!(ChainTransferPallet::send_to_evm_chain(
			RuntimeOrigin::signed(who.clone()),
			transfer
		));
		assert_eq!(Balances::free_balance(&who), 3991);
		assert_eq!(Balances::free_balance(ChainTransferPallet::pallet_account()), 1009);

		let to = Alice.to_account_id();
		let to_bytes: [u8; 32] = to.clone().into();

		let post_request = PostRequest {
			body: {
				let mut encoded = vec![0];
				let body = Body {
					to: to_bytes.into(),
					from: {
						let mut from = [0u8; 32];
						from[12..].copy_from_slice(&evm_account[..]);
						from.into()
					},
					amount: {
						let mut bytes = [0u8; 32];
						convert_to_erc20(1009u128).to_big_endian(&mut bytes);
						alloy_primitives::U256::from_be_bytes(bytes)
					},
					asset_id: Asset::Argon.asset_id().0.into(),
					redeem: false,
				};
				encoded.extend_from_slice(&Body::abi_encode(&body));
				encoded
			},
			to: to.as_slice().to_vec(),
			from: gateway_address.clone(),
			source: evm_chain.get_state_machine(true),
			dest: StateMachine::Substrate(*b"argn"),
			nonce: 0,
			timeout_timestamp: SystemTime::now().elapsed().unwrap().as_secs(),
		};
		ChainTransferPallet::default().on_accept(post_request).expect("should handle");

		assert_eq!(Balances::free_balance(&who), 3991);
		assert_eq!(Balances::free_balance(ChainTransferPallet::pallet_account()), 0);
		assert_eq!(Balances::free_balance(Alice.to_account_id()), 1009);
	})
}

#[test]
fn it_registers_tokens_correctly() {
	new_test_ext().execute_with(|| {
		// Go past genesis block so events get deposited
		System::set_block_number(1);

		let gateway_address = H160::random().as_bytes().to_vec();

		assert_ok!(ChainTransferPallet::register_hyperbridge_assets(
			RuntimeOrigin::signed(Alice.to_account_id()),
			bounded_vec![(EvmChain::Ethereum.get_state_machine(true), gateway_address.clone())],
		));

		assert_eq!(
			TokenGatewayAddresses::<Test>::get(EvmChain::Ethereum.get_state_machine(true)),
			Some(gateway_address.clone())
		);
		assert_eq!(ActiveEvmDestinations::<Test>::get().to_vec(), vec![EvmChain::Ethereum]);

		let commitments = System::events()
			.iter()
			.filter_map(|event| match &event.event {
				RuntimeEvent::Ismp(pallet_ismp::Event::<Test>::Request { commitment, .. }) =>
					Some(*commitment),
				_ => None,
			})
			.collect::<Vec<_>>();
		assert_eq!(commitments.len(), 2);
		let new_chain_events = System::events()
			.iter()
			.filter_map(|event| match &event.event {
				RuntimeEvent::ChainTransfer(ev) =>
					if let Event::ERC6160AssetRegistrationDispatched::<Test> {
						added_chains, ..
					} = ev
					{
						assert_eq!(added_chains.len(), 1);
						assert!(added_chains[0].is_evm());
						let StateMachine::Evm(evm_chain) = added_chains[0] else { return None };
						let evm_chain = EvmChain::try_from(evm_chain, true).unwrap();
						assert_eq!(evm_chain, EvmChain::Ethereum);
						Some(())
					} else {
						None
					},
				_ => None,
			})
			.collect::<Vec<_>>();
		assert_eq!(new_chain_events.len(), 2);

		System::reset_events();

		assert_err!(
			ChainTransferPallet::register_hyperbridge_assets(
				RuntimeOrigin::signed(Alice.to_account_id()),
				bounded_vec![(EvmChain::Base.get_state_machine(true), gateway_address.clone())],
			),
			Error::<Test>::Erc6160AlreadyRegistered
		);

		// can update the chains
		assert_ok!(ChainTransferPallet::update_hyperbridge_assets(
			RuntimeOrigin::signed(Alice.to_account_id()),
			bounded_vec![
				(EvmChain::Ethereum.get_state_machine(true), gateway_address.clone()),
				(EvmChain::Base.get_state_machine(true), gateway_address.clone())
			],
		),);
		assert_eq!(
			TokenGatewayAddresses::<Test>::get(EvmChain::Base.get_state_machine(true)),
			Some(gateway_address.clone())
		);
		assert_eq!(
			ActiveEvmDestinations::<Test>::get().to_vec(),
			vec![EvmChain::Ethereum, EvmChain::Base]
		);

		let new_chain_events = System::events()
			.iter()
			.filter_map(|event| match &event.event {
				RuntimeEvent::ChainTransfer(ev) =>
					if let Event::ERC6160AssetRegistrationDispatched::<Test> {
						added_chains,
						removed_chains,
						..
					} = ev
					{
						assert_eq!(removed_chains.len(), 0);
						assert_eq!(added_chains.len(), 1);
						assert!(added_chains[0].is_evm());
						let StateMachine::Evm(evm_chain) = added_chains[0] else { return None };
						let evm_chain = EvmChain::try_from(evm_chain, true).unwrap();
						assert_eq!(evm_chain, EvmChain::Base);
						Some(())
					} else {
						None
					},
				_ => None,
			})
			.collect::<Vec<_>>();
		assert_eq!(new_chain_events.len(), 2);

		// can remove chains
		System::reset_events();
		assert_ok!(ChainTransferPallet::update_hyperbridge_assets(
			RuntimeOrigin::signed(Alice.to_account_id()),
			bounded_vec![(EvmChain::Ethereum.get_state_machine(true), gateway_address.clone())],
		),);
		assert_eq!(
			TokenGatewayAddresses::<Test>::get(EvmChain::Ethereum.get_state_machine(true)),
			Some(gateway_address.clone())
		);
		assert_eq!(
			TokenGatewayAddresses::<Test>::get(EvmChain::Base.get_state_machine(true)),
			None
		);
		assert_eq!(ActiveEvmDestinations::<Test>::get().to_vec(), vec![EvmChain::Ethereum]);
		let new_chain_events = System::events()
			.iter()
			.filter_map(|event| match &event.event {
				RuntimeEvent::ChainTransfer(ev) =>
					if let Event::ERC6160AssetRegistrationDispatched::<Test> {
						added_chains,
						removed_chains,
						..
					} = ev
					{
						assert_eq!(added_chains.len(), 0);
						assert_eq!(removed_chains.len(), 1);
						assert!(removed_chains[0].is_evm());
						let StateMachine::Evm(evm_chain) = removed_chains[0] else { return None };
						let evm_chain = EvmChain::try_from(evm_chain, true).unwrap();
						assert_eq!(evm_chain, EvmChain::Base);
						Some(())
					} else {
						None
					},
				_ => None,
			})
			.collect::<Vec<_>>();
		assert_eq!(new_chain_events.len(), 2);
	});
}
