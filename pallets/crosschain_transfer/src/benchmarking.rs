#![cfg(feature = "runtime-benchmarks")]

extern crate alloc;

use super::*;
#[allow(unused_imports)]
use crate::Pallet as CrosschainTransferPallet;
use alloc::vec::Vec;
use alloy_primitives::{Address as AlloyAddress, B256};
use alloy_sol_types::SolEvent;
use argon_ethereum_contracts::minting_gateway::{
	self as ethereum_contracts, TransferToArgonStarted,
};
use argon_primitives::{
	ethereum::{
		EthereumCombinedReceiptProof, EthereumExecutionBlockProof, EthereumReceiptProofReceipt,
	},
	vault::{Vault, VaultTerms},
	EthereumLog, EthereumReceiptLog, UniswapTransferProvider, VaultId,
};
use pallet_prelude::benchmarking::{
	reset_benchmark_bitcoin_locks_runtime_state, reset_benchmark_bitcoin_vault_provider_state,
	reset_benchmark_price_provider_state, set_benchmark_bitcoin_locks_runtime_state,
	set_benchmark_bitcoin_vault_provider_state, set_benchmark_price_provider_state,
	BenchmarkBitcoinLocksRuntimeState, BenchmarkBitcoinVaultProviderState,
	BenchmarkPriceProviderState,
};
use polkadot_sdk::{
	frame_benchmarking::v2::*,
	frame_support::{traits::fungible::Mutate, BoundedBTreeMap},
	frame_system::RawOrigin,
	sp_arithmetic::FixedU128,
	sp_core::ecdsa::KeccakSignature,
	sp_runtime::Permill,
};

const BENCHMARK_DESTINATION_CHAIN: SourceChain = SourceChain::Ethereum;

#[benchmarks(
	where
		T::AccountId: From<[u8; 32]> + Ord,
		[u8; 32]: From<T::AccountId>,
		T::NativeCurrency: Mutate<T::AccountId, Balance = T::Balance>
)]
mod benchmarks {
	use super::*;
	use argon_primitives::CollectBlockerProvider;

	#[benchmark]
	fn set_chain_config() -> Result<(), BenchmarkError> {
		reset_crosschain_benchmark_state::<T>();
		ChainConfigBySourceChain::<T>::insert(
			BENCHMARK_DESTINATION_CHAIN,
			benchmark_chain_config(0x20),
		);

		#[extrinsic_call]
		set_chain_config(
			RawOrigin::Root,
			BENCHMARK_DESTINATION_CHAIN,
			benchmark_chain_config(0x21),
		);

		assert_eq!(
			ChainConfigBySourceChain::<T>::get(BENCHMARK_DESTINATION_CHAIN),
			Some(benchmark_chain_config(0x21)),
		);
		Ok(())
	}

	#[benchmark]
	fn force_set_global_issuance_council() -> Result<(), BenchmarkError> {
		reset_crosschain_benchmark_state::<T>();
		let caller: T::AccountId = account("council-member", 0, 0);
		let council_signer = benchmark_signer(1);
		let authority_signer = benchmark_signer(2);

		seed_benchmark_vault::<T>(&caller, 1, 50_000u128);
		seed_chain_config::<T>(0x21);
		CouncilSignerByDestinationChainAndAccountId::<T>::insert(
			BENCHMARK_DESTINATION_CHAIN,
			&caller,
			council_signer,
		);
		let active_council_hash =
			seed_active_council::<T>(&caller, council_signer, 50_000u128.into())?;
		let mut queued_rotation_council =
			GlobalIssuanceCouncilByHash::<T>::get(active_council_hash)
				.ok_or(BenchmarkError::Stop("active benchmark council missing"))?;
		queued_rotation_council.epoch_microgons_per_argonot =
			(2 * argon_primitives::MICROGONS_PER_ARGON).into();
		let queued_rotation_hash = Pallet::<T>::hash_global_issuance_council(
			&queued_rotation_council.members,
			queued_rotation_council.epoch_microgons_per_argonot,
		);
		GlobalIssuanceCouncilByHash::<T>::insert(queued_rotation_hash, queued_rotation_council);

		let rotation_entry = CouncilApprovalQueueEntry::<T> {
			approving_council_hash: active_council_hash,
			target: CouncilApprovalTargetId::GlobalIssuanceCouncilRotation(queued_rotation_hash),
			target_payload_hash: H256::repeat_byte(0x21),
			due_frame_id: 10,
			previous_approval_hash: H256::zero(),
			approval_hash: H256::repeat_byte(0x22),
			approved_total_weight: T::Balance::default(),
			signatures: BoundedBTreeMap::new(),
		};
		CouncilApprovalQueueByDestinationChainAndNonce::<T>::insert(
			BENCHMARK_DESTINATION_CHAIN,
			1,
			rotation_entry,
		);

		let mut activation_entry = benchmark_activation_queue_entry::<T>(
			queued_rotation_hash,
			2,
			H256::repeat_byte(0x22),
			authority_signer,
		)?;
		activation_entry.approval_hash = Pallet::<T>::hash_council_approval_queue_entry(
			BENCHMARK_DESTINATION_CHAIN,
			2,
			&activation_entry,
		)
		.map_err(|_| BenchmarkError::Stop("failed to hash benchmark activation queue entry"))?;
		CouncilApprovalQueueByDestinationChainAndNonce::<T>::insert(
			BENCHMARK_DESTINATION_CHAIN,
			2,
			activation_entry,
		);
		NextCouncilApprovalQueueNonceByDestinationChain::<T>::insert(
			BENCHMARK_DESTINATION_CHAIN,
			2,
		);
		Pallet::<T>::refresh_destination_chain_queue_tracking(BENCHMARK_DESTINATION_CHAIN)
			.map_err(|_| BenchmarkError::Stop("failed to refresh benchmark queue tracking"))?;
		MintingAuthoritiesBySigner::<T>::insert(
			authority_signer,
			MintingAuthority::<T> {
				account_id: caller.clone(),
				destination_chain: BENCHMARK_DESTINATION_CHAIN,
				destination_signing_key: authority_signer,
				state: MintingAuthorityState::PendingActivation,
				gateway_remaining_microgon_collateral: T::Balance::default(),
				gateway_remaining_micronot_collateral: T::Balance::default(),
				pending_reserved_microgon_collateral: T::Balance::default(),
				pending_reserved_micronot_collateral: T::Balance::default(),
				active_pending_transfer_ids: BoundedVec::default(),
				activation_approval_queue_nonce: 2,
				activation_base_repayment_quote: T::Balance::default(),
				activation_signature_repayment_quote: T::Balance::default(),
				deactivation_approval_queue_nonce: None,
			},
		);
		let members = vec![caller.clone()]
			.try_into()
			.map_err(|_| BenchmarkError::Stop("single council member exceeded benchmark bound"))?;

		#[extrinsic_call]
		force_set_global_issuance_council(RawOrigin::Root, BENCHMARK_DESTINATION_CHAIN, 0, members);

		let rebased_entry = CouncilApprovalQueueByDestinationChainAndNonce::<T>::get(
			BENCHMARK_DESTINATION_CHAIN,
			1,
		)
		.ok_or(BenchmarkError::Stop("rebased benchmark activation missing"))?;
		assert_eq!(
			rebased_entry.target,
			CouncilApprovalTargetId::MintingAuthorityActivation(authority_signer),
		);
		assert_eq!(
			rebased_entry.approving_council_hash,
			ActiveGlobalIssuanceCouncilByDestinationChain::<T>::get(BENCHMARK_DESTINATION_CHAIN,)
				.ok_or(BenchmarkError::Stop("active benchmark council missing after force set"))?,
		);
		assert!(CouncilApprovalQueueByDestinationChainAndNonce::<T>::get(
			BENCHMARK_DESTINATION_CHAIN,
			2,
		)
		.is_none());
		assert_eq!(
			NextCouncilApprovalQueueNonceByDestinationChain::<T>::get(BENCHMARK_DESTINATION_CHAIN,),
			1,
		);
		assert_eq!(
			MintingAuthoritiesBySigner::<T>::get(authority_signer)
				.ok_or(BenchmarkError::Stop("benchmark authority missing after force set"))?
				.activation_approval_queue_nonce,
			1,
		);
		assert!(GlobalIssuanceCouncilByHash::<T>::get(queued_rotation_hash).is_none());
		Ok(())
	}

	#[benchmark]
	fn pause_gateway() -> Result<(), BenchmarkError> {
		reset_crosschain_benchmark_state::<T>();
		GatewayStateBySourceChain::<T>::insert(
			BENCHMARK_DESTINATION_CHAIN,
			GatewayState::<T> {
				gateway_activity_nonce: 7,
				argon_approvals_nonce: 0,
				argon_circulation: T::Balance::default(),
				argonot_circulation: T::Balance::default(),
			},
		);

		#[extrinsic_call]
		pause_gateway(RawOrigin::Root, BENCHMARK_DESTINATION_CHAIN);

		assert!(GatewaySyncPauseBySourceChain::<T>::contains_key(BENCHMARK_DESTINATION_CHAIN),);
		Ok(())
	}

	#[benchmark]
	fn unpause_gateway() -> Result<(), BenchmarkError> {
		reset_crosschain_benchmark_state::<T>();
		GatewaySyncPauseBySourceChain::<T>::insert(
			BENCHMARK_DESTINATION_CHAIN,
			GatewaySyncPause {
				last_good_gateway_activity_nonce: 1,
				failed_gateway_activity_nonce: 2,
				reason: GatewaySyncPauseReason::Manual,
			},
		);

		#[extrinsic_call]
		unpause_gateway(RawOrigin::Root, BENCHMARK_DESTINATION_CHAIN);

		assert!(!GatewaySyncPauseBySourceChain::<T>::contains_key(BENCHMARK_DESTINATION_CHAIN),);
		Ok(())
	}

	#[benchmark]
	fn register_council_signer() -> Result<(), BenchmarkError> {
		reset_crosschain_benchmark_state::<T>();
		let caller: T::AccountId = whitelisted_caller();
		let signer = benchmark_signer(2);

		seed_benchmark_vault::<T>(&caller, 2, 50_000u128);
		let signature = benchmark_council_signer_registration_signature::<T>(signer, &caller);

		#[extrinsic_call]
		register_council_signer(
			RawOrigin::Signed(caller.clone()),
			BENCHMARK_DESTINATION_CHAIN,
			signer,
			signature,
		);

		assert_eq!(
			CouncilSignerByDestinationChainAndAccountId::<T>::get(
				BENCHMARK_DESTINATION_CHAIN,
				&caller,
			),
			Some(signer),
		);
		Ok(())
	}

	#[benchmark]
	fn register_minting_authority() -> Result<(), BenchmarkError> {
		reset_crosschain_benchmark_state::<T>();
		let caller: T::AccountId = whitelisted_caller();
		let authority_signer = benchmark_signer(4);

		seed_chain_config::<T>(0x21);
		seed_activation_repayment_pricing::<T>();
		seed_benchmark_vault::<T>(&caller, 3, 50_000u128);
		let council_signer = benchmark_signer(3);
		let _ = seed_active_council::<T>(&caller, council_signer, 50_000u128.into())?;
		T::NativeCurrency::mint_into(&caller, 1_000_000_000u128.into())
			.map_err(|_| BenchmarkError::Stop("failed to fund benchmark authority operator"))?;
		let signature =
			benchmark_minting_authority_registration_signature::<T>(authority_signer, &caller);

		#[extrinsic_call]
		register_minting_authority(
			RawOrigin::Signed(caller.clone()),
			BENCHMARK_DESTINATION_CHAIN,
			authority_signer,
			signature,
			20_000u128.into(),
			T::Balance::default(),
		);

		assert!(MintingAuthoritiesBySigner::<T>::contains_key(authority_signer));
		Ok(())
	}

	#[benchmark]
	fn approve_queue_entries(a: Linear<1, 32>) -> Result<(), BenchmarkError> {
		reset_crosschain_benchmark_state::<T>();
		let caller: T::AccountId = whitelisted_caller();
		let council_signer = benchmark_signer(5);

		seed_chain_config::<T>(0x21);
		seed_benchmark_vault::<T>(&caller, 4, 50_000u128);
		let council_hash = seed_active_council::<T>(&caller, council_signer, 50_000u128.into())?;
		CouncilApprovalCursorByDestinationChainAndAccountId::<T>::insert(
			BENCHMARK_DESTINATION_CHAIN,
			&caller,
			0,
		);

		let mut previous_approval_hash = H256::zero();
		let mut signatures = Vec::with_capacity(a as usize);
		for queue_nonce in 1..=a {
			let queue_nonce = queue_nonce as CouncilApprovalQueueNonce;
			let target_signer = H160::repeat_byte(queue_nonce as u8);
			let mut entry = benchmark_activation_queue_entry::<T>(
				council_hash,
				queue_nonce,
				previous_approval_hash,
				target_signer,
			)?;
			entry.approval_hash = Pallet::<T>::hash_council_approval_queue_entry(
				BENCHMARK_DESTINATION_CHAIN,
				queue_nonce,
				&entry,
			)
			.map_err(|_| BenchmarkError::Stop("failed to hash benchmark approval entry"))?;
			previous_approval_hash = entry.approval_hash;
			signatures.push(benchmark_evm_message_signature(entry.approval_hash, council_signer));
			CouncilApprovalQueueByDestinationChainAndNonce::<T>::insert(
				BENCHMARK_DESTINATION_CHAIN,
				queue_nonce,
				entry,
			);
		}
		let signatures = signatures.try_into().map_err(|_| {
			BenchmarkError::Stop("benchmark approval signatures exceeded runtime bound")
		})?;

		#[extrinsic_call]
		approve_queue_entries(
			RawOrigin::Signed(caller.clone()),
			BENCHMARK_DESTINATION_CHAIN,
			signatures,
		);

		assert_eq!(
			CouncilApprovalCursorByDestinationChainAndAccountId::<T>::get(
				BENCHMARK_DESTINATION_CHAIN,
				&caller,
			),
			Some(a.into()),
		);
		Ok(())
	}

	#[benchmark]
	fn prove_gateway_activity(a: Linear<1, 10>) -> Result<(), BenchmarkError> {
		reset_crosschain_benchmark_state::<T>();
		let caller: T::AccountId = whitelisted_caller();
		let burn_account = CrosschainTransferPallet::<T>::burn_account(BENCHMARK_DESTINATION_CHAIN);
		let amount: T::Balance = 1_000_000_000u128.into();

		seed_chain_config::<T>(0x21);
		T::NativeCurrency::mint_into(&burn_account, 10_000_000_000u128.into())
			.map_err(|_| BenchmarkError::Stop("failed to fund benchmark burn account"))?;
		let receipt_logs = (0..a)
			.map(|index| {
				let recipient: T::AccountId = account("crosschain-transfer-recipient", index, 0);
				let argon_circulation = 10_000_000_000u128.saturating_sub(
					amount.into().saturating_mul(u128::from(index).saturating_add(1)),
				);
				EthereumReceiptLog {
					transaction_index: 0,
					event_log: transfer_to_argon_started_log(
						h160(0x21),
						h160(0x11),
						h160(0x31),
						amount.into(),
						<[u8; 32]>::from(recipient),
						u64::from(index).saturating_add(1),
						0,
						argon_circulation,
						0,
					),
				}
			})
			.collect::<Vec<_>>()
			.try_into()
			.map_err(|_| BenchmarkError::Stop("benchmark receipt logs exceeded pallet bound"))?;
		let proof_batch = GatewayActivityProofBatch::<T> {
			execution_block_proof: dummy_execution_block_proof(),
			blocks: vec![GatewayActivityProofBlock::<T> {
				target_block_number: 0,
				receipt_proof: dummy_receipt_proof(),
				receipt_logs,
			}]
			.try_into()
			.map_err(|_| BenchmarkError::Stop("benchmark proof blocks exceeded pallet bound"))?,
		};

		#[extrinsic_call]
		prove_gateway_activity(
			RawOrigin::Signed(caller),
			BENCHMARK_DESTINATION_CHAIN,
			0,
			proof_batch,
		);

		assert_eq!(
			GatewayStateBySourceChain::<T>::get(BENCHMARK_DESTINATION_CHAIN)
				.expect("gateway state should be written")
				.gateway_activity_nonce,
			u64::from(a)
		);
		Ok(())
	}

	#[benchmark]
	fn transfer_out() -> Result<(), BenchmarkError> {
		reset_crosschain_benchmark_state::<T>();
		let council_account: T::AccountId = account("transfer-council", 0, 0);
		let caller: T::AccountId = whitelisted_caller();

		seed_chain_config::<T>(0x21);
		seed_benchmark_vault::<T>(&council_account, 5, 50_000u128);
		let _ = seed_active_council::<T>(&council_account, benchmark_signer(6), 50_000u128.into())?;
		T::NativeCurrency::mint_into(&caller, 1_000_000u128.into())
			.map_err(|_| BenchmarkError::Stop("failed to fund benchmark transfer sender"))?;

		#[extrinsic_call]
		transfer_out(
			RawOrigin::Signed(caller.clone()),
			BENCHMARK_DESTINATION_CHAIN,
			AssetKind::Argon,
			h160(0x44),
			20_000u128.into(),
		);

		assert_eq!(
			NonTerminalTransferOutCountByDestinationChain::<T>::get(BENCHMARK_DESTINATION_CHAIN),
			1,
		);
		Ok(())
	}

	#[benchmark]
	fn collateralize_transfer() -> Result<(), BenchmarkError> {
		reset_crosschain_benchmark_state::<T>();
		let council_account: T::AccountId = account("collateral-council", 0, 0);
		let authority_account: T::AccountId = whitelisted_caller();
		let user: T::AccountId = account("collateral-user", 0, 0);
		let authority_signer = benchmark_signer(8);

		seed_chain_config::<T>(0x21);
		seed_benchmark_vault::<T>(&council_account, 6, 50_000u128);
		let _ = seed_active_council::<T>(&council_account, benchmark_signer(7), 50_000u128.into())?;
		seed_active_minting_authority::<T>(
			authority_account.clone(),
			authority_signer,
			1,
			30_000u128.into(),
		)?;
		T::NativeCurrency::mint_into(&user, 1_000_000u128.into())
			.map_err(|_| BenchmarkError::Stop("failed to fund benchmark transfer sender"))?;
		Pallet::<T>::do_transfer_out(
			user,
			BENCHMARK_DESTINATION_CHAIN,
			AssetKind::Argon,
			h160(0x45),
			20_000u128.into(),
		)
		.map_err(|_| BenchmarkError::Stop("failed to seed benchmark transfer"))?;
		let transfer_id = TransferOutById::<T>::iter_keys()
			.next()
			.ok_or(BenchmarkError::Stop("benchmark transfer id missing"))?;
		let signature = benchmark_transfer_collateral_signature::<T>(
			authority_signer,
			transfer_id,
			20_000u128.into(),
			T::Balance::default(),
		)?;

		#[extrinsic_call]
		collateralize_transfer(
			RawOrigin::Signed(authority_account),
			transfer_id,
			signature,
			20_000u128.into(),
			T::Balance::default(),
		);

		assert_eq!(
			TransferOutById::<T>::get(transfer_id)
				.ok_or(BenchmarkError::Stop("benchmark transfer missing after collateralize"))?
				.state,
			crate::transfer_out::TransferOutState::Ready,
		);
		Ok(())
	}

	#[benchmark]
	fn deactivate_minting_authority() -> Result<(), BenchmarkError> {
		reset_crosschain_benchmark_state::<T>();
		let caller: T::AccountId = whitelisted_caller();
		let authority_signer = benchmark_signer(10);

		seed_chain_config::<T>(0x21);
		seed_benchmark_vault::<T>(&caller, 7, 50_000u128);
		let council_hash =
			seed_active_council::<T>(&caller, benchmark_signer(9), 50_000u128.into())?;
		seed_active_minting_authority::<T>(caller.clone(), authority_signer, 1, 20_000u128.into())?;
		let signature = benchmark_minting_authority_deactivation_signature::<T>(
			authority_signer,
			authority_signer,
			council_hash,
		)?;

		#[extrinsic_call]
		deactivate_minting_authority(
			RawOrigin::Signed(caller.clone()),
			authority_signer,
			signature,
		);

		assert_eq!(
			MintingAuthoritiesBySigner::<T>::get(authority_signer)
				.ok_or(BenchmarkError::Stop("authority missing after deactivation"))?
				.state,
			MintingAuthorityState::Deactivating,
		);
		Ok(())
	}

	#[benchmark]
	fn on_initialize_cleanup(e: Linear<1, 1_000>) -> Result<(), BenchmarkError> {
		reset_crosschain_benchmark_state::<T>();
		let current_tick = T::CurrentTick::get();

		for index in 0..e {
			let account_id: T::AccountId = account("expiring-crosschain-account", index, 0);
			RecentArgonTransfersByAccount::<T>::insert(&account_id, 1);
			InboundTransfersExpiringAt::<T>::append(current_tick, account_id);
		}

		#[block]
		{
			CrosschainTransferPallet::<T>::on_initialize(frame_system::Pallet::<T>::block_number());
		}

		assert!(InboundTransfersExpiringAt::<T>::get(current_tick).is_empty());
		assert_eq!(LastTransferExpiryCleanupTick::<T>::get(), current_tick);
		Ok(())
	}

	#[benchmark]
	fn provider_is_crosschain_activated() -> Result<(), BenchmarkError> {
		reset_crosschain_benchmark_state::<T>();
		seed_chain_config::<T>(0x21);

		#[block]
		{
			assert!(
				<CrosschainTransferPallet<T> as UniswapTransferProvider<T::AccountId>>::is_crosschain_activated()
			);
		}

		Ok(())
	}

	#[benchmark]
	fn provider_has_recent_argon_transfer() -> Result<(), BenchmarkError> {
		reset_crosschain_benchmark_state::<T>();
		let account_id: T::AccountId = account("recent-crosschain-transfer-account", 0, 0);
		RecentArgonTransfersByAccount::<T>::insert(&account_id, 1);

		#[block]
		{
			assert!(
				<CrosschainTransferPallet<T> as UniswapTransferProvider<T::AccountId>>::has_recent_argon_transfer(&account_id)
			);
		}

		Ok(())
	}

	#[benchmark]
	fn provider_has_overdue_collect_blocker() -> Result<(), BenchmarkError> {
		reset_crosschain_benchmark_state::<T>();
		let caller: T::AccountId = account("overdue-council-member", 0, 0);
		seed_chain_config::<T>(0x21);
		let council_hash =
			seed_active_council::<T>(&caller, benchmark_signer(11), 50_000u128.into())?;
		CouncilApprovalCursorByDestinationChainAndAccountId::<T>::insert(
			BENCHMARK_DESTINATION_CHAIN,
			&caller,
			0,
		);
		let mut entry = benchmark_activation_queue_entry::<T>(
			council_hash,
			1,
			H256::zero(),
			H160::repeat_byte(12),
		)?;
		entry.due_frame_id = 1;
		CouncilApprovalQueueByDestinationChainAndNonce::<T>::insert(
			BENCHMARK_DESTINATION_CHAIN,
			1,
			entry,
		);
		set_benchmark_bitcoin_locks_runtime_state(BenchmarkBitcoinLocksRuntimeState {
			current_frame_id: 1,
			current_tick: 1,
			did_start_new_frame: true,
		});

		#[block]
		{
			assert!(
				<CrosschainTransferPallet<T> as CollectBlockerProvider<T::AccountId>>::has_overdue_collect_blocker(&caller)
			);
		}

		Ok(())
	}

	impl_benchmark_test_suite!(
		CrosschainTransferPallet,
		crate::mock::new_test_ext(),
		crate::mock::Test
	);
}

fn reset_crosschain_benchmark_state<T: Config>() {
	reset_benchmark_bitcoin_locks_runtime_state();
	reset_benchmark_bitcoin_vault_provider_state();
	reset_benchmark_price_provider_state();
	set_benchmark_price_provider_state(BenchmarkPriceProviderState::default());
	set_benchmark_bitcoin_locks_runtime_state(BenchmarkBitcoinLocksRuntimeState::default());
}

fn benchmark_chain_config(gateway_byte: u8) -> ChainConfig {
	ChainConfig::Evm {
		chain_id: 1,
		gateway: h160(gateway_byte),
		argon_token: h160(0x31),
		argonot_token: h160(0x32),
	}
}

fn benchmark_vault_terms<T: Config>() -> VaultTerms<T::Balance> {
	VaultTerms {
		bitcoin_annual_percent_rate: FixedU128::one(),
		bitcoin_base_fee: 1u128.into(),
		treasury_profit_sharing: Permill::zero(),
	}
}

fn seed_benchmark_vault<T: Config>(operator: &T::AccountId, vault_id: VaultId, securitization: u128)
where
	T::AccountId: Ord,
{
	let mut state = BenchmarkBitcoinVaultProviderState::<T::AccountId, T::Balance>::default();
	state.vaults.insert(
		vault_id,
		Vault::<T::AccountId, T::Balance> {
			operator_account_id: operator.clone(),
			bitcoin_lock_delegate_account: None,
			name: None,
			last_name_change_tick: None,
			securitization: securitization.into(),
			securitization_target: securitization.into(),
			securitization_locked: securitization.into(),
			securitization_pending_activation: T::Balance::default(),
			locked_satoshis: 0,
			securitized_satoshis: 0,
			securitization_release_schedule: BoundedBTreeMap::new(),
			securitization_ratio: FixedU128::one(),
			is_closed: false,
			terms: benchmark_vault_terms::<T>(),
			pending_terms: None,
			opened_tick: 0,
			operational_minimum_release_tick: None,
		},
	);
	set_benchmark_bitcoin_vault_provider_state(state);
}

fn seed_chain_config<T: Config>(gateway_byte: u8) {
	ChainConfigBySourceChain::<T>::insert(
		BENCHMARK_DESTINATION_CHAIN,
		benchmark_chain_config(gateway_byte),
	);
}

fn seed_activation_repayment_pricing<T: Config>() {
	MintingAuthorityActivationRepaymentPricingByDestinationChain::<T>::insert(
		BENCHMARK_DESTINATION_CHAIN,
		MintingAuthorityActivationRepaymentPricing::<T> {
			activation_gas_cost: 100_000,
			signature_gas_cost: 50_000,
			estimated_wei_per_gas: 1_000_000_000,
			estimated_microgons_per_eth: 1_000_000u128.into(),
		},
	);
}

fn benchmark_signer(seed_byte: u8) -> H160 {
	H160::repeat_byte(seed_byte)
}

fn benchmark_council_signer_registration_signature<T: Config>(
	signer: H160,
	_account_id: &T::AccountId,
) -> KeccakSignature {
	benchmark_personal_signature(signer)
}

fn benchmark_minting_authority_registration_signature<T: Config>(
	signer: H160,
	_account_id: &T::AccountId,
) -> KeccakSignature {
	benchmark_personal_signature(signer)
}

fn seed_active_council<T: Config>(
	account_id: &T::AccountId,
	signer: H160,
	weight: T::Balance,
) -> Result<H256, BenchmarkError> {
	let (council_hash, council) = benchmark_council(
		account_id,
		signer,
		weight,
		argon_primitives::MICROGONS_PER_ARGON.into(),
	)?;
	GlobalIssuanceCouncilByHash::<T>::insert(council_hash, council);
	ActiveGlobalIssuanceCouncilByDestinationChain::<T>::insert(
		BENCHMARK_DESTINATION_CHAIN,
		council_hash,
	);
	let current_transfer_out_rate: T::Balance = argon_primitives::MICROGONS_PER_ARGON.into();
	CurrentTransferOutMicrogonsPerArgonotByDestinationChain::<T>::insert(
		BENCHMARK_DESTINATION_CHAIN,
		current_transfer_out_rate,
	);
	CouncilSignerByDestinationChainAndAccountId::<T>::insert(
		BENCHMARK_DESTINATION_CHAIN,
		account_id,
		signer,
	);
	Ok(council_hash)
}

fn benchmark_council<T: Config>(
	account_id: &T::AccountId,
	signer: H160,
	weight: T::Balance,
	epoch_microgons_per_argonot: T::Balance,
) -> Result<(H256, GlobalIssuanceCouncil<T>), BenchmarkError> {
	let mut members = BoundedBTreeMap::new();
	let _ = members
		.try_insert(
			signer,
			GlobalIssuanceCouncilMember::<T> { account_id: account_id.clone(), signer, weight },
		)
		.map_err(|_| BenchmarkError::Stop("benchmark council members exceeded runtime bound"))?;
	let council =
		GlobalIssuanceCouncil::<T> { epoch_microgons_per_argonot, total_weight: weight, members };
	let council_hash = Pallet::<T>::hash_global_issuance_council(
		&council.members,
		council.epoch_microgons_per_argonot,
	);

	Ok((council_hash, council))
}

fn seed_active_minting_authority<T: Config>(
	account_id: T::AccountId,
	destination_signing_key: H160,
	activation_approval_queue_nonce: CouncilApprovalQueueNonce,
	microgon_collateral: T::Balance,
) -> Result<(), BenchmarkError> {
	MintingAuthoritiesBySigner::<T>::insert(
		destination_signing_key,
		MintingAuthority::<T> {
			account_id,
			destination_chain: BENCHMARK_DESTINATION_CHAIN,
			destination_signing_key,
			state: MintingAuthorityState::Active,
			gateway_remaining_microgon_collateral: microgon_collateral,
			gateway_remaining_micronot_collateral: T::Balance::default(),
			pending_reserved_microgon_collateral: T::Balance::default(),
			pending_reserved_micronot_collateral: T::Balance::default(),
			active_pending_transfer_ids: BoundedVec::default(),
			activation_approval_queue_nonce,
			activation_base_repayment_quote: T::Balance::default(),
			activation_signature_repayment_quote: T::Balance::default(),
			deactivation_approval_queue_nonce: None,
		},
	);
	Ok(())
}

fn benchmark_activation_queue_entry<T: Config>(
	approving_council_hash: H256,
	queue_nonce: CouncilApprovalQueueNonce,
	previous_approval_hash: H256,
	destination_signing_key: H160,
) -> Result<CouncilApprovalQueueEntry<T>, BenchmarkError> {
	let target_payload_hash = Pallet::<T>::hash_activate_minting_authority(
		BENCHMARK_DESTINATION_CHAIN,
		10_000u128.into(),
		T::Balance::default(),
		destination_signing_key,
	)
	.map_err(|_| BenchmarkError::Stop("failed to hash benchmark activation target"))?;
	Ok(CouncilApprovalQueueEntry::<T> {
		approving_council_hash,
		target: CouncilApprovalTargetId::MintingAuthorityActivation(destination_signing_key),
		target_payload_hash,
		due_frame_id: queue_nonce.saturating_add(10),
		previous_approval_hash,
		approval_hash: H256::zero(),
		approved_total_weight: T::Balance::default(),
		signatures: BoundedBTreeMap::new(),
	})
}

fn benchmark_transfer_collateral_signature<T: Config>(
	signer: H160,
	transfer_id: H256,
	microgon_collateral: T::Balance,
	micronot_collateral: T::Balance,
) -> Result<KeccakSignature, BenchmarkError> {
	let transfer = TransferOutById::<T>::get(transfer_id)
		.ok_or(BenchmarkError::Stop("benchmark transfer missing for signature"))?;
	let (chain_id, gateway) = Pallet::<T>::evm_gateway_signature_domain(transfer.destination_chain)
		.map_err(|_| BenchmarkError::Stop("missing benchmark gateway signature domain"))?;
	let approval_hash = H256::from_slice(
		ethereum_contracts::hash_minting_authorization(
			chain_id,
			AlloyAddress::from_slice(gateway.as_bytes()),
			B256::from(
				Pallet::<T>::transfer_out_request_id(&transfer)
					.map_err(|_| BenchmarkError::Stop("failed to hash benchmark transfer id"))?
					.0,
			),
			microgon_collateral.into(),
			micronot_collateral.into(),
		)
		.as_slice(),
	);

	Ok(benchmark_evm_message_signature(approval_hash, signer))
}

fn benchmark_minting_authority_deactivation_signature<T: Config>(
	signer: H160,
	destination_signing_key: H160,
	approving_council_hash: H256,
) -> Result<KeccakSignature, BenchmarkError> {
	let queue_nonce = 1;
	let mut queue_entry = CouncilApprovalQueueEntry::<T> {
		approving_council_hash,
		target: CouncilApprovalTargetId::MintingAuthorityDeactivation(destination_signing_key),
		target_payload_hash: Pallet::<T>::hash_deactivate_minting_authority_target(
			destination_signing_key,
		),
		due_frame_id: 10,
		previous_approval_hash: H256::zero(),
		approval_hash: H256::zero(),
		approved_total_weight: T::Balance::default(),
		signatures: BoundedBTreeMap::new(),
	};
	queue_entry.approval_hash = Pallet::<T>::hash_council_approval_queue_entry(
		BENCHMARK_DESTINATION_CHAIN,
		queue_nonce,
		&queue_entry,
	)
	.map_err(|_| BenchmarkError::Stop("failed to hash benchmark deactivation queue entry"))?;
	Ok(benchmark_evm_message_signature(queue_entry.approval_hash, signer))
}

fn benchmark_personal_signature(signer: H160) -> KeccakSignature {
	benchmark_keccak_message_signature(signer)
}

fn benchmark_evm_message_signature(_message_hash: H256, signer: H160) -> KeccakSignature {
	benchmark_keccak_message_signature(signer)
}

fn benchmark_keccak_message_signature(signer: H160) -> KeccakSignature {
	let mut bytes = [0u8; 65];
	bytes[..20].copy_from_slice(signer.as_bytes());
	KeccakSignature::from_raw(bytes)
}

fn transfer_to_argon_started_log(
	gateway: H160,
	from: H160,
	token: H160,
	amount: u128,
	destination: [u8; 32],
	gateway_activity_nonce: GatewayActivityNonce,
	argon_approvals_nonce: ArgonApprovalsNonce,
	argon_circulation: u128,
	argonot_circulation: u128,
) -> EthereumLog {
	let mut data = Vec::with_capacity(192);
	data.extend_from_slice(&u64_word(amount as u64));
	data.extend_from_slice(&destination);
	data.extend_from_slice(&u64_word(gateway_activity_nonce));
	data.extend_from_slice(&u64_word(argon_approvals_nonce));
	data.extend_from_slice(&u64_word(argon_circulation as u64));
	data.extend_from_slice(&u64_word(argonot_circulation as u64));

	EthereumLog {
		address: gateway,
		topics: vec![
			H256::from_slice(TransferToArgonStarted::SIGNATURE_HASH.as_slice()),
			indexed_address_word(from),
			indexed_address_word(token),
		]
		.try_into()
		.expect("topics stay within Ethereum log topic bounds"),
		data: data.try_into().expect("burn event data stays within bounded log payload"),
	}
}

fn indexed_address_word(address: H160) -> H256 {
	let mut bytes = [0u8; 32];
	bytes[12..].copy_from_slice(address.as_bytes());
	H256::from(bytes)
}

fn u64_word(value: u64) -> [u8; 32] {
	let mut bytes = [0u8; 32];
	bytes[24..].copy_from_slice(&value.to_be_bytes());
	bytes
}

fn dummy_execution_block_proof() -> EthereumExecutionBlockProof {
	EthereumExecutionBlockProof {
		anchor_block_hash: H256::repeat_byte(1),
		target_to_anchor_header_chain: Vec::new()
			.try_into()
			.expect("empty header chain stays within bounds"),
	}
}

fn dummy_receipt_proof() -> EthereumCombinedReceiptProof {
	EthereumCombinedReceiptProof {
		nodes: vec![vec![1u8].try_into().expect("tiny receipt proof node stays within bounds")]
			.try_into()
			.expect("single-node receipt proof stays within bounds"),
		receipts: vec![EthereumReceiptProofReceipt {
			transaction_index: 0,
			node_indexes: vec![0u16]
				.try_into()
				.expect("single node index stays within bounded receipt proof refs"),
		}]
		.try_into()
		.expect("single receipt proof stays within bounded receipt count"),
	}
}

fn h160(byte: u8) -> H160 {
	H160::repeat_byte(byte)
}
