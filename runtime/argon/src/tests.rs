use super::*;
use alloy_sol_types::SolEvent;
use argon_ethereum_contracts::minting_gateway::TransferToArgonFromSwapStarted;
use argon_primitives::{
	amount_rank::AmountRankId,
	treasury::{NetworkPositionQuantities, PositionQuantities},
	vault::{BitcoinSecuritizationBasis, CosignerGuarantee, Vault, VaultTerms},
	EthereumCombinedReceiptProof, EthereumExecutionBlockProof, EthereumLog, EthereumReceiptLog,
	EthereumReceiptLogProofBlock, EthereumReceiptProofReceipt, LiquidRepaymentProvider, PriceIndex,
	PriceIndexUpdated, SourceChain, TickProvider, UniswapPurchase, UniswapPurchaseProvider,
	MICROGONS_PER_ARGON,
};
use frame_support::{assert_noop, assert_ok, storage::with_storage_layer, traits::Hooks};
use pallet_bitcoin_fissions::{LiquidTicks, LiquidTicksByOwnerAndId};
use pallet_crosschain_transfer::{
	ChainConfig, GatewayActivityReceiptProofBatch, GatewayStateBySourceChain,
};
use pallet_ethereum_verifier::{types::ExecutionHeaderAnchor, ExecutionHeaderAnchors};
use pallet_stabilizer_buys::{
	ApprovedPool, BuysByAccountAndFrame, HoldReason as PurchaseHoldReason,
	LiquidRepaysByAccountAndGatewayNonce, PayoutStateByAccount, PurchasePool,
};
use pallet_treasury::{OutstandingPurchaseRewards, RewardReserves};
use sp_core::H160;
use sp_runtime::BuildStorage;

#[test]
fn proven_purchase_delivery_and_principal_hold_commit_together() {
	let fixture: serde_json::Value = serde_json::from_str(include_str!(
		"../../../chains/ethereum/fixtures/uniswap-v4-purchase-refund.json"
	))
	.unwrap();
	let native_log: alloy_primitives::Log =
		serde_json::from_value(fixture["eventLog"].clone()).unwrap();
	let event = TransferToArgonFromSwapStarted::decode_raw_log_validate(
		native_log.topics().iter().copied(),
		&native_log.data.data,
	)
	.unwrap();
	let owner = AccountId::from(event.purchase.argonAccountId.0);
	let principal = event.amount;
	let nonce = event.gatewayState.gatewayActivityNonce;
	let gateway = H160::from_slice(native_log.address.as_slice());
	let chain_id = fixture["chainId"].as_str().unwrap().parse().unwrap();
	let transaction_index = fixture["transactionIndex"].as_u64().unwrap();
	let anchor = ExecutionHeaderAnchor {
		block_number: 42,
		timestamp_millis: event.purchase.timestamp * 1_000,
		block_hash: H256::repeat_byte(42),
		parent_hash: H256::zero(),
		state_root: H256::zero(),
		receipts_root: fixture["receiptsRoot"].as_str().unwrap().parse().unwrap(),
	};
	let proof = GatewayActivityReceiptProofBatch::<Runtime> {
		execution_block_proof: EthereumExecutionBlockProof {
			anchor_block_hash: anchor.block_hash,
			target_to_anchor_header_chain: Default::default(),
		},
		blocks: vec![EthereumReceiptLogProofBlock {
			target_block_number: anchor.block_number,
			receipt_proof: EthereumCombinedReceiptProof {
				nodes: fixture["nodes"]
					.as_array()
					.unwrap()
					.iter()
					.map(|node| {
						alloy_primitives::hex::decode(node.as_str().unwrap())
							.unwrap()
							.try_into()
							.unwrap()
					})
					.collect::<Vec<_>>()
					.try_into()
					.unwrap(),
				receipts: vec![EthereumReceiptProofReceipt {
					transaction_index,
					node_indexes: (0..fixture["nodes"].as_array().unwrap().len() as u16)
						.collect::<Vec<_>>()
						.try_into()
						.unwrap(),
				}]
				.try_into()
				.unwrap(),
			},
			receipt_logs: vec![EthereumReceiptLog {
				transaction_index,
				event_log: EthereumLog {
					address: gateway,
					topics: native_log
						.topics()
						.iter()
						.map(|topic| H256::from(topic.0))
						.collect::<Vec<_>>()
						.try_into()
						.unwrap(),
					data: native_log.data.data.to_vec().try_into().unwrap(),
				},
			}]
			.try_into()
			.unwrap(),
		}]
		.try_into()
		.unwrap(),
	};
	let mut ext =
		sp_io::TestExternalities::new(RuntimeGenesisConfig::default().build_storage().unwrap());
	ext.execute_with(|| {
		System::set_block_number(1);
		pallet_mining_slot::NextFrameId::<Runtime>::put(2);
		ExecutionHeaderAnchors::<Runtime>::insert(anchor.block_hash, anchor);
		assert_ok!(CrosschainTransfer::set_chain_config(
			RuntimeOrigin::root(),
			SourceChain::Ethereum,
			ChainConfig::Evm {
				chain_id,
				gateway,
				argon_token: H160::from_slice(event.token.as_slice()),
				argonot_token: H160::repeat_byte(9),
			}
		));
		let burn = CrosschainTransfer::burn_account(SourceChain::Ethereum);
		let backing = principal + event.gatewayState.argonCirculation;
		assert_ok!(<Balances as MutateT<AccountId>>::mint_into(&burn, backing));

		let tick = Ticks::ticker().tick_for_time(event.purchase.timestamp * 1_000);
		pallet_ticks::CurrentTick::<Runtime>::put(tick);
		ApprovedPool::<Runtime>::put(PurchasePool {
			chain_id,
			pool_manager: H160::from_slice(event.purchase.poolManager.as_slice()),
			pool_id: H256::from(event.purchase.poolId.0),
			usdc_token: H160::from_slice(event.purchase.usdcToken.as_slice()),
			argon_is_token0: event.token < event.purchase.usdcToken,
		});
		// A $2 target makes this authenticated native purchase eligible for the reward auction.
		StabilizerBuys::on_price_index(&PriceIndex {
			tick,
			btc_usd_price: FixedU128::from_u32(60_000),
			argonot_usd_price: FixedU128::one(),
			argon_usd_price: FixedU128::one(),
			argon_usd_target_price: FixedU128::from_u32(2),
			argon_time_weighted_average_liquidity: 1,
			usdc_usd_price: Some(FixedU128::one()),
			stabilizer_pool_sqrt_price_x96: None,
		});

		let mut forged = proof.clone();
		let mut forged_event = event.clone();
		forged_event.purchase.repayLiquidId = 0;
		forged.blocks[0].receipt_logs[0].event_log.data =
			forged_event.encode_data().try_into().unwrap();
		assert_noop!(
			CrosschainTransfer::prove_gateway_activity(
				RuntimeOrigin::signed(owner.clone()),
				SourceChain::Ethereum,
				0,
				forged
			),
			pallet_crosschain_transfer::Error::<Runtime>::InvalidProof
		);
		assert_noop!(
			with_storage_layer(|| -> DispatchResult {
				CrosschainTransfer::prove_gateway_activity(
					RuntimeOrigin::signed(owner.clone()),
					SourceChain::Ethereum,
					0,
					proof.clone(),
				)
				.map_err(|error| error.error)?;
				assert_eq!(
					Balances::balance_on_hold(
						&PurchaseHoldReason::PurchasePrincipal.into(),
						&owner
					),
					principal
				);
				Err(DispatchError::Other("outer failure"))
			}),
			DispatchError::Other("outer failure")
		);
		assert_eq!(Balances::free_balance(&burn), backing);
		assert_eq!(Balances::free_balance(&owner), 0);
		assert!(!PayoutStateByAccount::<Runtime>::contains_key(&owner));
		assert!(!LiquidRepaysByAccountAndGatewayNonce::<Runtime>::contains_key(
			&owner,
			nonce.to_ordered()
		));
		assert_eq!(
			GatewayStateBySourceChain::<Runtime>::get(SourceChain::Ethereum)
				.unwrap_or_default()
				.gateway_activity_nonce,
			0
		);

		assert_ok!(CrosschainTransfer::prove_gateway_activity(
			RuntimeOrigin::signed(owner.clone()),
			SourceChain::Ethereum,
			0,
			proof.clone()
		));
		assert_eq!(Balances::free_balance(&burn), event.gatewayState.argonCirculation);
		assert_eq!(Balances::free_balance(&owner), 0);
		assert_eq!(
			Balances::balance_on_hold(&PurchaseHoldReason::PurchasePrincipal.into(), &owner),
			principal
		);
		// The TypeScript-signed authorization reaches admission; its Liquid has no Fissions.
		System::assert_has_event(RuntimeEvent::StabilizerBuys(
			pallet_stabilizer_buys::Event::LiquidRepaymentInvalid {
				account: owner.clone(),
				gateway_activity_nonce: nonce,
				liquid_id: 1,
			},
		));
		assert!(!LiquidRepaysByAccountAndGatewayNonce::<Runtime>::contains_key(
			&owner,
			nonce.to_ordered()
		));

		assert_eq!(
			BuysByAccountAndFrame::<Runtime>::get(&owner, 1u64.to_ordered())
				.unwrap()
				.captured_discount,
			principal - event.purchase.usdcSpent.to::<u128>().div_ceil(2)
		);
		assert!(!StabilizerBuys::has_pending_repayment(&owner, 1));

		assert_noop!(
			CrosschainTransfer::prove_gateway_activity(
				RuntimeOrigin::signed(owner.clone()),
				SourceChain::Ethereum,
				0,
				proof
			),
			pallet_crosschain_transfer::Error::<Runtime>::UnexpectedPreviousGatewayActivityNonce
		);
		assert_eq!(
			Balances::balance_on_hold(&PurchaseHoldReason::PurchasePrincipal.into(), &owner),
			principal
		);
	});
}

#[test]
fn deferred_repayment_settles_bitcoin_principal_and_rewards_including_unreceivable_dust() {
	// Some(0) enables reward eligibility without a bid; None leaves the pool unapproved.
	for (
		dust_remainder,
		usdc_available,
		purchase_r,
		recovery_r,
		committed,
		burned,
		ratchet_first,
		reward_bid,
		treasury_funding_percent,
	) in [
		(false, true, 100, 100, 100_000, 100_000, false, None, 0),
		(true, true, 100, 100, 100_000, 100_000, false, None, 0),
		(false, true, 50, 50, 135_060, 135_060, false, None, 0),
		(false, true, 50, 80, 135_060, 105_480, false, None, 0),
		(false, false, 50, 100, 135_060, 100_000, false, None, 0),
		(false, true, 98, 100, 96_864, 96_864, false, None, 0),
		(false, true, 50, 100, 135_060, 100_000, true, None, 0),
		(false, true, 50, 100, 135_060, 100_000, false, Some(60), 100),
		(false, true, 50, 100, 135_060, 100_000, false, Some(0), 0),
		(false, true, 50, 100, 135_060, 100_000, false, Some(100), 50),
	] {
		let owner = AccountId::from([42; 32]);
		let sats = SATOSHIS_PER_BITCOIN;
		let redemption = 100_000 * MICROGONS_PER_ARGON;
		let minimum = <Balances as InspectT<AccountId>>::minimum_balance();
		let remainder = if dust_remainder { 1 } else { minimum };
		let expected_burn = burned * MICROGONS_PER_ARGON;
		let principal = committed * MICROGONS_PER_ARGON + remainder;
		let expected_refund = principal - expected_burn;
		let treasury_budget = Percent::from_percent(treasury_funding_percent)
			.mul_floor(Percent::from_percent(90).mul_floor(principal - principal / 2));
		let mut ext = sp_io::TestExternalities::new(
			frame_system::GenesisConfig::<Runtime>::default().build_storage().unwrap(),
		);
		ext.execute_with(|| {
			pallet_bitcoin_utxos::ConfirmedBitcoinBlockTip::<Runtime>::put(
				argon_primitives::bitcoin::BitcoinBlock {
					block_height: 100,
					block_hash: H256Le([1; 32]),
				},
			);
			System::set_block_number(1);
			pallet_mining_slot::NextFrameId::<Runtime>::put(2);
			assert_ok!(<Balances as MutateT<AccountId>>::mint_into(&owner, principal));
			pallet_price_index::Current::<Runtime>::put(argon_primitives::PriceIndex {
				tick: 0,
				btc_usd_price: FixedU128::from_u32(100_000),
				argonot_usd_price: FixedU128::from_u32(1),
				argon_usd_price: FixedU128::from_rational(purchase_r, 100),
				argon_usd_target_price: FixedU128::from_u32(1),
				argon_time_weighted_average_liquidity: 1,
				usdc_usd_price: if usdc_available { Some(FixedU128::one()) } else { None },
				stabilizer_pool_sqrt_price_x96: None,
			});
			pallet_treasury_positions::PositionsByAccount::<Runtime>::insert(
				&owner,
				pallet_treasury_positions::Position {
					bond_principal: 0,
					quantities: PositionQuantities {
						fission_liquidity: redemption,
						..Default::default()
					},
					upstream: None,
				},
			);
			pallet_treasury_positions::NetworkTotals::<Runtime>::put(NetworkPositionQuantities {
				account_quantities: PositionQuantities {
					fission_liquidity: redemption,
					..Default::default()
				},
				vault_securitized_satoshis: u128::from(sats),
			});
			pallet_mint::MintedBitcoinMicrogons::<Runtime>::put(redemption);
			pallet_vaults::VaultsById::<Runtime>::insert(
				1,
				Vault {
					operator_account_id: AccountId::from([43; 32]),
					delegate_account_id: None,
					securitization: if ratchet_first { redemption * 3 } else { redemption },
					securitization_target: if ratchet_first { redemption * 3 } else { redemption },
					securitization_locked: redemption,
					flexible_securitization_locked: 0,
					reserved_securitization_space: 0,
					securitization_pending_activation: 0,
					securitized_satoshis: sats,
					total_satoshis: sats,
					securitization_release_schedule: Default::default(),
					committed_microgons: if ratchet_first { redemption * 3 } else { redemption },
					is_closed: false,
					terms: VaultTerms {
						bitcoin_annual_percent_rate: FixedU128::from_u32(0),
						bitcoin_base_fee: 0,
					},
					pending_terms: None,
					opened_tick: 0,
				},
			);
			let basis = BitcoinSecuritizationBasis {
				satoshis: sats,
				microgons_at_target_per_btc: redemption,
				tick: 0,
			};
			pallet_bitcoin_locks::LocksById::<Runtime>::insert(
				1,
				pallet_bitcoin_locks::LockedBitcoin::<Runtime> {
					vault_id: 1,
					cosigner_guarantee: CosignerGuarantee {
						basis: BitcoinSecuritizationBasis { satoshis: 0, ..basis },
						guaranteed_microgons: 0,
						fund_hold_extensions: Default::default(),
					},
					securitization_hold_expiration_bitcoin_height: None,
					funded_satoshis: sats,
					funding_utxos: [(UtxoRef { txid: H256Le([1; 32]), output_index: 0 }, sats)]
						.into_iter()
						.collect::<BTreeMap<_, _>>()
						.try_into()
						.unwrap(),
					fissioned_satoshis: sats,
					owner_account: owner.clone(),
					security_fees: 0,
					coupon_paid_fees: 0,
					vault_pubkey: CompressedBitcoinPubkey([2; 33]),
					vault_claim_pubkey: CompressedBitcoinPubkey([2; 33]),
					vault_xpub_sources: Default::default(),
					owner_pubkey: CompressedBitcoinPubkey([2; 33]),
					vault_claim_height: 10_000,
					open_claim_height: 20_000,
					created_at_height: 0,
					utxo_script_pubkey: BitcoinCosignScriptPubkey::P2WSH {
						wscript_hash: H256::repeat_byte(1),
					},
					is_flexible: false,
					created_at_argon_block: 1,
				},
			);
			pallet_bitcoin_fissions::FissionByOwnerAndId::<Runtime>::insert(
				&owner,
				0,
				pallet_bitcoin_fissions::Fission::<Runtime> {
					liquid_id: 1,
					lock_id: 1,
					satoshis: sats,
					microgons_at_target_per_btc: redemption,
					last_ratchet_price_tick: 0,
					liquidity_promised: redemption,
					cosigner_guarantee: CosignerGuarantee {
						basis,
						guaranteed_microgons: redemption,
						fund_hold_extensions: Default::default(),
					},
					created_at_argon_block: 1,
					ratchet_number: 0,
					last_updated_argon_block: 1,
				},
			);
			pallet_bitcoin_fissions::FissionIdsByOwnerAndLiquid::<Runtime>::insert(
				(&owner, 1),
				0,
				(),
			);
			LiquidTicksByOwnerAndId::<Runtime>::insert(&owner, 1, LiquidTicks::default());
			pallet_bitcoin_fissions::FissionIdsByLockId::<Runtime>::mutate(1, |ids| {
				ids.try_insert(0).unwrap();
			});
			let index = pallet_price_index::Current::<Runtime>::get().unwrap();
			<(StabilizerBuys, BitcoinFissions) as PriceIndexUpdated>::on_price_index(&index);
			let mut purchase = UniswapPurchase {
				gateway_activity_nonce: 1,
				repay_liquid_id: Some(1),
				chain_id: 1,
				pool_manager: H160::repeat_byte(1),
				pool_id: H256::repeat_byte(1),
				usdc_token: H160::repeat_byte(2),
				usdc_spent: U256::one(),
				principal,
				sqrt_price_before_x96: U256::one() << 96,
				sqrt_price_after_x96: U256::one() << 96,
				tick: 0,
			};
			if reward_bid.is_some() {
				ApprovedPool::<Runtime>::put(PurchasePool {
					chain_id: purchase.chain_id,
					pool_manager: purchase.pool_manager,
					pool_id: purchase.pool_id,
					usdc_token: purchase.usdc_token,
					argon_is_token0: true,
				});
				purchase.usdc_spent = U256::from(principal / 2);
				purchase.sqrt_price_before_x96 = (U256::one() << 96) / U256::from(2_000_000);
				purchase.sqrt_price_after_x96 = (U256::one() << 96) / U256::from(1_000_000);
				RewardReserves::<Runtime>::mutate(|reserves| {
					reserves.liquidity_buys = treasury_budget;
				});
			}
			if ratchet_first {
				pallet_ticks::GenesisTicker::<Runtime>::put(argon_primitives::tick::Ticker::new(
					1_000, 2,
				));
				Timestamp::set_timestamp(1_000);
				let mut higher_btc = index;
				higher_btc.btc_usd_price = FixedU128::from_u32(200_000);
				pallet_price_index::Current::<Runtime>::put(higher_btc);
				pallet_ticks::CurrentTick::<Runtime>::put(1);
				BitcoinLocks::on_finalize(1);
				assert_ok!(BitcoinFissions::ratchet(
					RuntimeOrigin::signed(owner.clone()),
					0,
					redemption * 2,
					None
				));
			}
			assert_ok!(with_storage_layer(|| <StabilizerBuys as UniswapPurchaseProvider<
				AccountId,
			>>::record_transferred_purchase(&owner, &purchase)));
			if ratchet_first {
				assert!(!StabilizerBuys::has_pending_repayment(&owner, 1));
				assert_eq!(
					pallet_bitcoin_locks::LocksById::<Runtime>::get(1).unwrap().fissioned_satoshis,
					sats
				);
				return;
			}
			assert!(StabilizerBuys::has_pending_repayment(&owner, 1));
			let mut ordinary_reward = 0;
			let mut repayment_reward = 0;
			if let Some(rate) = reward_bid {
				pallet_mining_slot::NextFrameId::<Runtime>::put(3);
				StabilizerBuys::on_frame_start(2);
				if rate > 0 {
					assert_ok!(StabilizerBuys::bid(
						RuntimeOrigin::signed(owner.clone()),
						1,
						Percent::from_percent(rate)
					));
				} else {
					assert_eq!(
						pallet_stabilizer_buys::BidsByAuction::<Runtime>::iter_prefix(1).count(),
						1
					);
				}
				pallet_mining_slot::NextFrameId::<Runtime>::put(4);
				StabilizerBuys::on_frame_start(3);
				ordinary_reward = if rate > 0 {
					Percent::from_percent(rate).mul_floor(treasury_budget)
				} else {
					treasury_budget
				};
				let bonus_rate =
					if rate > 0 { Percent::from_percent(rate) } else { Percent::one() };
				repayment_reward =
					bonus_rate.mul_floor(Percent::from_percent(5).mul_floor(expected_burn));
			}
			assert_noop!(
				BitcoinFissions::ratchet(
					RuntimeOrigin::signed(owner.clone()),
					0,
					redemption * 2,
					None
				),
				pallet_bitcoin_fissions::Error::<Runtime>::LiquidRepaymentPending
			);
			let mut recovered = index;
			recovered.tick = 1;
			recovered.argon_usd_price = FixedU128::from_rational(recovery_r, 100);
			pallet_ticks::CurrentTick::<Runtime>::put(1);
			<(StabilizerBuys, BitcoinFissions) as PriceIndexUpdated>::on_price_index(&recovered);
			// Settlement must remain usable without a live Bitcoin or ARGON quote.
			pallet_price_index::Current::<Runtime>::kill();
			assert_ok!(<Balances as MutateT<AccountId>>::mint_into(&owner, redemption));
			assert_noop!(
				BitcoinFissions::close(RuntimeOrigin::signed(owner.clone()), 0),
				pallet_bitcoin_fissions::Error::<Runtime>::LiquidRepaymentPending
			);
			assert_ok!(Balances::force_set_balance(RuntimeOrigin::root(), owner.clone().into(), 0));
			let issuance = Balances::total_issuance();
			pallet_mining_slot::NextFrameId::<Runtime>::put(31);
			StabilizerBuys::on_initialize(2);
			assert_eq!(Balances::total_issuance(), issuance);
			assert_eq!(
				pallet_bitcoin_locks::LocksById::<Runtime>::get(1).unwrap().fissioned_satoshis,
				sats
			);
			pallet_mining_slot::NextFrameId::<Runtime>::put(32);
			StabilizerBuys::on_initialize(3);

			assert!(!pallet_bitcoin_fissions::FissionByOwnerAndId::<Runtime>::contains_key(
				&owner, 0
			));
			assert!(!StabilizerBuys::has_pending_repayment(&owner, 1));
			assert_eq!(
				pallet_bitcoin_locks::LocksById::<Runtime>::get(1).unwrap().fissioned_satoshis,
				0
			);
			assert_eq!(
				Balances::balance_on_hold(
					&pallet_stabilizer_buys::HoldReason::PurchasePrincipal.into(),
					&owner
				),
				0
			);
			assert_eq!(
				Balances::free_balance(&owner),
				expected_refund + ordinary_reward + repayment_reward -
					if dust_remainder { 1 } else { 0 }
			);
			assert_eq!(
				pallet_mint::MintedBitcoinMicrogons::<Runtime>::get(),
				redemption.saturating_sub(expected_burn)
			);
			assert_eq!(
				Balances::total_issuance(),
				issuance - expected_burn + ordinary_reward + repayment_reward -
					if dust_remainder { 1 } else { 0 }
			);
			if reward_bid.is_some() {
				assert_eq!(
					RewardReserves::<Runtime>::get().liquidity_buys,
					treasury_budget - ordinary_reward
				);
				assert_eq!(OutstandingPurchaseRewards::<Runtime>::get(), 0);
			}
			assert_eq!(
				pallet_treasury_positions::NetworkTotals::<Runtime>::get()
					.account_quantities
					.fission_liquidity,
				0
			);
			let paid_issuance = Balances::total_issuance();
			StabilizerBuys::on_initialize(5);
			assert_eq!(Balances::total_issuance(), paid_issuance);
		});
	}
}

#[test]
fn gateway_proof_weight_includes_each_authoritys_collateral_work() {
	use argon_ethereum_contracts::minting_gateway::{
		GatewayActivityState, MintingAuthorityCollateral, TransferOutOfArgonFinalized,
	};
	use argon_primitives::{
		EthereumAccountStorageProof, TreasuryPoolProvider, TreasuryPoolProviderWeightInfo,
	};
	use frame_support::dispatch::GetDispatchInfo;
	use pallet_crosschain_transfer::GatewayActivityStorageProof;

	let mut weights = Vec::new();
	for rows in 0..=2 {
		let event = TransferOutOfArgonFinalized {
			transferId: alloy_primitives::B256::ZERO,
			token: alloy_primitives::Address::repeat_byte(1),
			amount: 1,
			mintingCollateral: (0..rows)
				.map(|index| MintingAuthorityCollateral {
					signingKey: alloy_primitives::Address::repeat_byte(index + 1),
					microgonCollateral: 1,
					micronotCollateral: 1,
				})
				.collect(),
			gatewayState: GatewayActivityState {
				gatewayActivityNonce: 1,
				argonApprovalsNonce: 0,
				argonCirculation: 0,
				argonotCirculation: 0,
			},
		};
		let proof = GatewayActivityStorageProof {
			locator_index: 0,
			activity_root_seed: H256::zero(),
			storage_proof: EthereumAccountStorageProof {
				execution_block_proof: EthereumExecutionBlockProof {
					anchor_block_hash: H256::zero(),
					target_to_anchor_header_chain: Default::default(),
				},
				account_proof: Default::default(),
				storage_proof: Default::default(),
				slots: Default::default(),
			},
			activity_logs: vec![EthereumLog {
				address: H160::repeat_byte(2),
				topics: vec![H256::from(TransferOutOfArgonFinalized::SIGNATURE_HASH.0)]
					.try_into()
					.unwrap(),
				data: event.encode_data().try_into().unwrap(),
			}]
			.try_into()
			.unwrap(),
		};
		let call = pallet_crosschain_transfer::Call::<Runtime>::prove_gateway_activity_storage {
			source_chain: SourceChain::Ethereum,
			previous_gateway_activity_nonce: 0,
			proof,
		};
		weights.push(call.get_dispatch_info().call_weight);
	}

	type TreasuryWeights = <<Runtime as pallet_crosschain_transfer::Config>::TreasuryPoolProvider as TreasuryPoolProvider<AccountId>>::Weights;
	let first_authority = weights[1].saturating_sub(weights[0]);
	assert!(first_authority.all_gte(TreasuryWeights::burn_encumbered_bond_microgons()));
	assert!(first_authority.ref_time() > 0);
	assert_eq!(weights[2].saturating_sub(weights[1]), first_authority);
}
