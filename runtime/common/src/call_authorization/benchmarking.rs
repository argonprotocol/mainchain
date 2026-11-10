//! Business-policy fixtures live with the runtime rules, not in the generic security pallet.

use alloc::{vec, vec::Vec};
use argon_primitives::{
	bitcoin::{CompressedBitcoinPubkey, H256Le, UtxoRef},
	vault::{
		BitcoinVaultProvider, CosignerGuarantee, SecuritizationScheduleEntry, Vault, VaultTerms,
		MAX_SECURITIZATION_RELEASE_SCHEDULE_ENTRIES,
	},
	Signature,
};
use codec::{Decode, Encode, MaxEncodedLen};
use core::marker::PhantomData;
use frame_benchmarking::{account, v2::*};
use pallet_account_security::{
	AuthorizationContext, Config, NextNonce, Pallet as AccountSecurity, Policy, Registration,
	Registrations, RuntimeCallOf,
};
use pallet_bitcoin_fissions::{Call as FissionsCall, Fission, FissionByOwnerAndId};
use pallet_bitcoin_locks::{Call as LocksCall, LockedBitcoin, LocksById};
use pallet_operational_accounts::{
	AccountOwnershipProof, Call as OperationalCall, OperationalAccount,
	OperationalAccountBySubAccount, OperationalAccounts, Registration as OperationalRegistration,
	RegistrationV1,
};
use pallet_vaults::{VaultIdByOperator, VaultsById};
use polkadot_sdk::{
	frame_support::{instances::Instance1, traits::Get},
	frame_system::pallet_prelude::BlockNumberFor,
	pallet_balances, pallet_proxy,
	sp_arithmetic::FixedU128,
	sp_core::{crypto::KeyTypeId, hashing::blake2_256},
	sp_io,
	sp_runtime::{
		traits::{Bounded, StaticLookup, TrailingZeroInput},
		AccountId32,
	},
};

pub struct PolicyBenchmarks<T>(PhantomData<T>);
type Pallet<T> = PolicyBenchmarks<T>;

#[benchmarks(
	where
		T: Config<AccountId = AccountId32>
			+ pallet_balances::Config<Instance1, Balance = u128>
			+ pallet_operational_accounts::Config<Balance = u128>
			+ pallet_bitcoin_locks::Config<Balance = u128>
			+ pallet_bitcoin_fissions::Config<Balance = u128>
			+ pallet_vaults::Config<Balance = u128>,
		RuntimeCallOf<T>: From<pallet_balances::Call<T, Instance1>>
			+ From<pallet_proxy::Call<T>> + From<OperationalCall<T>>
			+ From<LocksCall<T>> + From<FissionsCall<T>>,
)]
mod benchmarks {
	use super::*;

	#[benchmark]
	fn authorization_approval() {
		let (member, group) = protected_account::<T>(0);
		let call = pallet_proxy::Call::<T>::remove_proxies {}.into();

		#[block]
		{
			inspect::<T>(&member, call, group.clone());
		}
		assert_eq!(NextNonce::<T>::get(&group), 0);
	}

	#[benchmark]
	fn authorization_transfer() {
		let (destination, _) = protected_account::<T>(1);
		let unprotected = account("security-unprotected", 0, 0);
		let unprotected_group = account("security-unprotected-group", 0, 0);
		prepare_account::<T>(&unprotected, &unprotected_group);
		let call = pallet_balances::Call::<T, Instance1>::transfer_keep_alive {
			dest: T::Lookup::unlookup(destination),
			value: 1,
		}
		.into();

		#[block]
		{
			// An unprotected source does not short-circuit the destination's protection read.
			inspect::<T>(&unprotected, call, unprotected_group);
		}
	}

	#[benchmark]
	fn authorization_link() {
		let (member, group) = protected_account::<T>(0);
		let (vault, _) = protected_account::<T>(1);
		let (mining, _) = protected_account::<T>(2);
		let key_type = KeyTypeId(*b"asec");
		let public = sp_io::crypto::sr25519_generate(key_type, None);
		let proof = AccountOwnershipProof {
			signature: Signature::Sr25519(
				sp_io::crypto::sr25519_sign(key_type, &public, b"ownership").unwrap(),
			),
		};
		// These proofs are call data only: inspection stops before registration dispatch.
		let call = OperationalCall::<T>::register {
			registration: OperationalRegistration::V1(RegistrationV1 {
				operational_account: member.clone(),
				vault_account: vault,
				mining_account: mining,
				encryption_pubkey: Default::default(),
				operational_account_proof: proof.clone(),
				vault_account_proof: proof.clone(),
				mining_account_proof: proof,
				access_proof: None,
			}),
		}
		.into();

		#[block]
		{
			inspect::<T>(&member, call, group.clone());
		}
		assert_eq!(NextNonce::<T>::get(&group), 0);
	}

	// Full compact scalars, options and bounded collections make recorded proofs an upper
	// bound. Per-prefix MaxEncodedLen estimation understates these composite multi-record proofs.
	#[benchmark(pov_mode = Measured)]
	fn authorization_bitcoin_vault() {
		let (member, group, upstream_vault) = bitcoin_account::<T>();
		let call = LocksCall::<T>::create_receive_address {
			vault_id: u32::MAX,
			satoshis: 1,
			bitcoin_pubkey: CompressedBitcoinPubkey([0; 33]),
			options: None,
		}
		.into();

		#[block]
		{
			inspect::<T>(&member, call, group.clone());
			production_vault_reads::<T>(&upstream_vault);
		}
		assert_eq!(NextNonce::<T>::get(&group), 0);
	}

	#[benchmark(pov_mode = Measured)]
	fn authorization_bitcoin_lock() {
		let (member, group, upstream_vault) = bitcoin_account::<T>();
		LocksById::<T>::insert(u64::MAX, full_lock::<T>());
		let call =
			LocksCall::<T>::set_guarantee { lock_id: u64::MAX, satoshis: 1, options: None }.into();

		#[block]
		{
			inspect::<T>(&member, call, group.clone());
			production_vault_reads::<T>(&upstream_vault);
		}
		assert_eq!(NextNonce::<T>::get(&group), 0);
	}

	#[benchmark(pov_mode = Measured)]
	fn authorization_bitcoin_fission() {
		let (member, group, upstream_vault) = bitcoin_account::<T>();
		LocksById::<T>::insert(u64::MAX, full_lock::<T>());
		let mut fission = Fission::<T>::decode(&mut TrailingZeroInput::zeroes()).unwrap();
		fission.liquid_id = u64::MAX;
		fission.lock_id = u64::MAX;
		fission.satoshis = u64::MAX;
		fission.microgons_at_target_per_btc = u128::MAX;
		fission.liquidity_promised = u128::MAX;
		fission.cosigner_guarantee = full_guarantee();
		fission.created_at_argon_block = BlockNumberFor::<T>::max_value();
		fission.ratchet_number = u32::MAX;
		fission.last_ratchet_tick = u64::MAX;
		fission.last_updated_argon_block = BlockNumberFor::<T>::max_value();
		// The derived bound uses scalar widths: nine compact fields add one byte
		// each at their maximum, in addition to the guarantee's four-byte correction.
		assert_eq!(fission.encoded_size(), Fission::<T>::max_encoded_len() + 13);
		FissionByOwnerAndId::<T>::insert(&member, u64::MAX, fission);
		let call = FissionsCall::<T>::ratchet {
			fission_id: u64::MAX,
			microgons_at_target_per_btc: 1,
			fee_coupon: None,
		}
		.into();

		#[block]
		{
			inspect::<T>(&member, call, group.clone());
			production_vault_reads::<T>(&upstream_vault);
		}
		assert_eq!(NextNonce::<T>::get(&group), 0);
	}
}

/// Seeds the longest default-account branch without depending on an identity policy stub.
pub fn prepare_account<T>(member: &AccountId32, default_account: &AccountId32)
where
	T: pallet_operational_accounts::Config<AccountId = AccountId32, Balance = u128>,
{
	let owner = AccountId32::new((b"security-owner", member).using_encoded(blake2_256));
	let mut record = OperationalAccount::<T>::decode(&mut TrailingZeroInput::zeroes()).unwrap();
	record.vault_account = default_account.clone();
	record.mining_account = member.clone();
	record.upstream_account = Some(account("security-upstream", 0, 0));
	record.name = Some(vec![0xff; 18].try_into().unwrap());
	record.last_name_change_tick = Some(u64::MAX);
	record.uniswap_argon_transfers_in_amount = u128::MAX;
	record.account_bitcoin_amount = u128::MAX;
	record.account_vault_bond_amount = u128::MAX;
	record.vault_bitcoin_accrual = u128::MAX;
	record.vault_bitcoin_applied_total = u128::MAX;
	record.mining_seat_accrual = u32::MAX;
	record.mining_seat_applied_total = u32::MAX;
	record.operational_certifications_count = u32::MAX;
	record.available_access_codes = u32::MAX;
	record.rewards_earned_count = u32::MAX;
	record.rewards_earned_amount = u128::MAX;
	record.rewards_collected_amount = u128::MAX;
	// The derived bound uses u32's width for five Compact<u32> fields. SCALE's
	// maximum compact representation is one byte wider for each of them.
	assert_eq!(record.encoded_size(), OperationalAccount::<T>::max_encoded_len() + 5);
	OperationalAccounts::<T>::insert(&owner, record);
	OperationalAccountBySubAccount::<T>::insert(member, owner);
}

fn protected_account<T>(index: u32) -> (AccountId32, AccountId32)
where
	T: Config<AccountId = AccountId32> + pallet_operational_accounts::Config<Balance = u128>,
{
	let member = account("security-member", index, 0);
	let group = account("security-group", index, 0);
	prepare_account::<T>(&member, &group);
	let count = T::MaxSigners::get();
	let mut signers =
		(0..count).map(|index| account("security-signer", index, 0)).collect::<Vec<_>>();
	signers.sort();
	let policy = Policy::<T> {
		signers: signers.try_into().unwrap(),
		quorum: count as u16,
		recovery_account: account("security-recovery", 0, 0),
		recovery_delay: 0,
	};
	Registrations::<T>::insert(&group, Registration { policy, deposit: 0 });
	(member, group)
}

fn inspect<T: Config>(member: &T::AccountId, call: RuntimeCallOf<T>, group: T::AccountId) {
	let context = AuthorizationContext {
		account: member.clone(),
		authorized_group: Some(group),
		executes: true,
	};
	AccountSecurity::<T>::ensure_call_authorized(&call, &context).unwrap();
}

fn bitcoin_account<T>() -> (AccountId32, AccountId32, AccountId32)
where
	T: Config<AccountId = AccountId32>
		+ pallet_operational_accounts::Config<Balance = u128>
		+ pallet_vaults::Config<Balance = u128>,
{
	let (member, group) = protected_account::<T>(0);
	let upstream = account("security-upstream", 0, 0);
	let upstream_vault = account("security-upstream-vault", 0, 0);
	prepare_account::<T>(&upstream, &upstream_vault);
	let owner = OperationalAccountBySubAccount::<T>::take(&upstream).unwrap();
	let upstream_record = OperationalAccounts::<T>::take(owner).unwrap();
	OperationalAccounts::<T>::insert(&upstream, upstream_record);
	VaultIdByOperator::<T>::insert(&group, u32::MAX - 1);
	VaultIdByOperator::<T>::insert(&upstream_vault, 1);
	let mut vault = Vault::<AccountId32, u128>::decode(&mut TrailingZeroInput::zeroes()).unwrap();
	vault.operator_account_id = upstream_vault.clone();
	vault.delegate_account_id = Some(member.clone());
	vault.securitization = u128::MAX;
	vault.securitization_target = u128::MAX;
	vault.securitization_locked = u128::MAX;
	vault.flexible_securitization_locked = u128::MAX;
	vault.reserved_securitization_space = u128::MAX;
	vault.securitization_pending_activation = u128::MAX;
	vault.securitized_satoshis = u64::MAX;
	vault.total_satoshis = u64::MAX;
	vault.committed_microgons = u128::MAX;
	vault.opened_tick = u64::MAX;
	vault.terms = VaultTerms {
		bitcoin_annual_percent_rate: FixedU128::from_inner(u128::MAX),
		bitcoin_base_fee: u128::MAX,
	};
	vault.pending_terms = Some((u64::MAX, vault.terms.clone()));
	for index in 0..MAX_SECURITIZATION_RELEASE_SCHEDULE_ENTRIES {
		vault
			.securitization_release_schedule
			.try_insert(
				u64::MAX - u64::from(index),
				SecuritizationScheduleEntry {
					locked_commitments: u128::MAX,
					relockable_commitments: u128::MAX,
					argon_withdrawals: u128::MAX,
					argonot_withdrawals: u128::MAX,
				},
			)
			.unwrap();
	}
	// Ten compact scalar fields, two fields in each of the current/pending terms,
	// and four fields per schedule entry each need one byte beyond their scalar width.
	let compact_growth = 10 + 4 + 4 * MAX_SECURITIZATION_RELEASE_SCHEDULE_ENTRIES as usize;
	assert_eq!(
		vault.encoded_size(),
		Vault::<AccountId32, u128>::max_encoded_len() + compact_growth
	);
	VaultsById::<T>::insert(1, vault);
	(member, group, upstream_vault)
}

fn production_vault_reads<T: pallet_vaults::Config>(upstream_vault: &T::AccountId) {
	// Other benchmarks substitute this provider; one inspection makes one production lookup.
	assert!(<pallet_vaults::Pallet<T> as BitcoinVaultProvider>::get_registration_vault_data(
		upstream_vault
	)
	.is_some());
}

fn full_lock<T: pallet_bitcoin_locks::Config<Balance = u128>>() -> LockedBitcoin<T> {
	let mut lock = LockedBitcoin::<T>::decode(&mut TrailingZeroInput::zeroes()).unwrap();
	lock.vault_id = u32::MAX;
	lock.cosigner_guarantee = full_guarantee();
	lock.securitization_hold_expiration_bitcoin_height = Some(u64::MAX);
	lock.funded_satoshis = u64::MAX;
	lock.fissioned_satoshis = u64::MAX;
	lock.security_fees = u128::MAX;
	lock.coupon_paid_fees = u128::MAX;
	lock.vault_claim_height = u64::MAX;
	lock.open_claim_height = u64::MAX;
	lock.created_at_height = u64::MAX;
	lock.created_at_argon_block = BlockNumberFor::<T>::max_value();
	for index in 0..T::MaxUtxosPerLock::get() {
		lock.funding_utxos
			.try_insert(
				UtxoRef {
					txid: H256Le(index.using_encoded(blake2_256)),
					output_index: u32::MAX - index,
				},
				u64::MAX,
			)
			.unwrap();
	}
	// Nine compact scalars, the guarantee's four-byte correction, and one compact
	// output index per UTXO each encode beyond the widths used by the derived bound.
	let compact_growth = 13 + T::MaxUtxosPerLock::get() as usize;
	assert_eq!(lock.encoded_size(), LockedBitcoin::<T>::max_encoded_len() + compact_growth);
	lock
}

fn full_guarantee() -> CosignerGuarantee<u128> {
	let mut guarantee =
		CosignerGuarantee::<u128>::decode(&mut TrailingZeroInput::zeroes()).unwrap();
	guarantee.basis.satoshis = u64::MAX;
	guarantee.basis.microgons_at_target_per_btc = u128::MAX;
	guarantee.basis.tick = u64::MAX;
	guarantee.guaranteed_microgons = u128::MAX;
	for index in 0..MAX_SECURITIZATION_RELEASE_SCHEDULE_ENTRIES {
		guarantee
			.fund_hold_extensions
			.try_insert(u64::MAX - u64::from(index), u128::MAX)
			.unwrap();
	}
	// All four compact scalars encode one byte beyond their derived scalar widths.
	assert_eq!(guarantee.encoded_size(), CosignerGuarantee::<u128>::max_encoded_len() + 4);
	guarantee
}
