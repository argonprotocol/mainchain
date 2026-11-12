//! Shared account-exit rules, expanded against each runtime's concrete call enum.

use polkadot_sdk::frame_support::weights::Weight;

#[cfg(feature = "runtime-benchmarks")]
pub mod benchmarking;
#[cfg(feature = "runtime-benchmarks")]
pub use benchmarking::PolicyBenchmarks;

/// Measured runtime business-policy costs; native wrappers are charged by AccountSecurity.
pub trait WeightInfo {
	fn authorization_approval() -> Weight;
	fn authorization_transfer() -> Weight;
	fn authorization_link() -> Weight;
	fn authorization_bitcoin_vault() -> Weight;
	fn authorization_bitcoin_lock() -> Weight;
	fn authorization_bitcoin_fission() -> Weight;
}

impl WeightInfo for () {
	fn authorization_approval() -> Weight {
		Weight::zero()
	}
	fn authorization_transfer() -> Weight {
		Weight::zero()
	}
	fn authorization_link() -> Weight {
		Weight::zero()
	}
	fn authorization_bitcoin_vault() -> Weight {
		Weight::zero()
	}
	fn authorization_bitcoin_lock() -> Weight {
		Weight::zero()
	}
	fn authorization_bitcoin_fission() -> Weight {
		Weight::zero()
	}
}

#[macro_export]
macro_rules! account_security_policy {
	() => {
		mod call_authorization {
			use alloc::{vec, vec::Vec};
			use argon_primitives::{
				bitcoin::{BitcoinLockId, FissionId}, OperationalAccountProvider, VaultId,
			};
			use frame_support::weights::Weight;
			use frame_system::Config as SystemConfig;
			use pallet_account_security::{
				AccountSecurityPolicy,
				AuthorizationContext, CallRequirement, Error,
				CallRequirement::{
					ChangesAuthorization, CheckProtectionForBatch,
					RequireApproval, RequireAccountLinkApproval,
				},
			};
			use pallet_balances::Call as BalancesCall;
			use pallet_bitcoin_fissions::{Call as BitcoinFissionsCall, FissionByOwnerAndId};
			use pallet_bitcoin_locks::{Call as BitcoinLocksCall, LocksById};
			use pallet_crosschain_transfer::Call as CrosschainCall;
			use pallet_localchain_transfer::Call as LocalchainCall;
			use pallet_operational_accounts::{
				Call as OperationalCall, OperationalAccountBySubAccount, OperationalAccounts,
				Pallet as OperationalAccountsPallet, Registration, RegistrationV1,
			};
			use pallet_proxy::Call as ProxyCall;
			use pallet_vaults::{Call as VaultsCall, VaultIdByOperator};
			use polkadot_sdk::*;
			use sp_runtime::traits::StaticLookup;
			use $crate::call_authorization::WeightInfo;

			use super::{
				weights::argon_runtime_common_call_authorization::WeightInfo as PolicyWeightInfo,
				AccountId, Runtime, RuntimeCall,
			};

			type AccountLookup = <Runtime as SystemConfig>::Lookup;
			type PolicyWeights = PolicyWeightInfo<Runtime>;

			pub struct CallAuthorizationPolicy;

			/// Borrowed sensitive-call rules shared by requirements and storage-free cost selection.
			enum CallRule<'a> {
				Approval,
				ChangesAuthorization,
				BitcoinVault(VaultId),
				BitcoinLock(BitcoinLockId),
				BitcoinFission(FissionId),
				Transfer(&'a <AccountLookup as StaticLookup>::Source),
				Link(&'a RegistrationV1<Runtime>),
			}

			impl AccountSecurityPolicy<Runtime> for CallAuthorizationPolicy {
				fn inspection_weight(call: &RuntimeCall) -> Weight {
					match CallRule::for_call(call) {
						Some(CallRule::Approval | CallRule::ChangesAuthorization) =>
							PolicyWeights::authorization_approval(),
						Some(CallRule::Transfer(_)) => PolicyWeights::authorization_transfer(),
						Some(CallRule::Link(_)) => PolicyWeights::authorization_link(),
						Some(CallRule::BitcoinVault(_)) =>
							PolicyWeights::authorization_bitcoin_vault(),
						Some(CallRule::BitcoinLock(_)) =>
							PolicyWeights::authorization_bitcoin_lock(),
						Some(CallRule::BitcoinFission(_)) =>
							PolicyWeights::authorization_bitcoin_fission(),
						None => Weight::zero(),
					}
				}

				fn requirements(
					call: &RuntimeCall,
					context: &AuthorizationContext<Runtime>,
				) -> Result<Vec<CallRequirement<AccountId>>, Error<Runtime>> {
					let account = &context.account;
					let requirements = match CallRule::for_call(call) {
						Some(CallRule::Approval) => vec![RequireApproval(account.clone())],
						Some(CallRule::ChangesAuthorization) =>
							vec![RequireApproval(account.clone()), ChangesAuthorization],
						Some(CallRule::BitcoinVault(vault_id)) =>
							Self::bitcoin_vault_requirements(account, vault_id),
						Some(CallRule::BitcoinLock(lock_id)) =>
							Self::bitcoin_lock_requirements(account, lock_id),
						Some(CallRule::BitcoinFission(fission_id)) =>
							match FissionByOwnerAndId::<Runtime>::get(account, fission_id) {
								Some(fission) =>
									Self::bitcoin_lock_requirements(account, fission.lock_id),
								None => vec![],
							},

						Some(CallRule::Transfer(dest)) => {
							let destination = AccountLookup::lookup(dest.clone())
								.map_err(|_| Error::<Runtime>::InvalidProxy)?;
							let is_internal = if account == &destination {
								true
							} else {
								Self::default_account(account) == Self::default_account(&destination)
							};

							// Internal transfers still touch protected state for the batch guard.
							let mut requirements = vec![
								CheckProtectionForBatch(account.clone()),
								CheckProtectionForBatch(destination),
							];
							if !is_internal {
								requirements.push(RequireApproval(account.clone()));
							}
							requirements
						},

						// Operational-account linking.
						Some(CallRule::Link(registration)) => {
							// register -> transfer can change the default used by later decisions.
							// Require existing participant approvals and compatible protection;
							// the mixed-batch marker is an additional conservative restriction.
							vec![
								RequireAccountLinkApproval([
									registration.operational_account.clone(),
									registration.vault_account.clone(),
									registration.mining_account.clone(),
								]),
								ChangesAuthorization,
							]
						},

						None => vec![],
					};

					Ok(requirements)
				}

				fn default_account(account: &AccountId) -> AccountId {
					if let Some(record) = OperationalAccounts::<Runtime>::get(account) {
						return record.vault_account
					}

					let Some(owner) = OperationalAccountBySubAccount::<Runtime>::get(account)
					else {
						return account.clone()
					};
					let Some(record) = OperationalAccounts::<Runtime>::get(owner) else {
						return account.clone()
					};
					record.vault_account
				}

				#[cfg(feature = "runtime-benchmarks")]
				fn prepare_benchmark_account(member: &AccountId, default_account: &AccountId) {
					$crate::call_authorization::benchmarking::prepare_account::<Runtime>(
						member, default_account,
					);
				}
			}

			impl<'a> CallRule<'a> {
				fn for_call(call: &'a RuntimeCall) -> Option<Self> {
					use RuntimeCall::{
						Balances, BitcoinFissions, BitcoinLocks, CrosschainTransfer,
						LocalchainTransfer, OperationalAccounts, Ownership, Proxy, Vaults,
					};

					match call {
						Balances(BalancesCall::burn { .. }) |
						Ownership(BalancesCall::burn { .. }) |
						BitcoinLocks(BitcoinLocksCall::request_release { .. } |
							BitcoinLocksCall::request_orphaned_utxo_release { .. }) |
						// Same-address L2 deposits can subsequently be spent without an exit signer.
						LocalchainTransfer(LocalchainCall::send_to_localchain { .. }) |
						CrosschainTransfer(CrosschainCall::transfer_out { .. } |
							CrosschainCall::collateralize_transfer { .. }) |
						Vaults(VaultsCall::replace_bitcoin_xpub { .. }) |
						CrosschainTransfer(CrosschainCall::register_council_signer { .. } |
							CrosschainCall::register_minting_authority { .. }) =>
							Some(Self::Approval),
						// These changes may alter a later call's authority or own-vault lookup.
						Proxy(ProxyCall::add_proxy { .. } | ProxyCall::remove_proxy { .. } |
							ProxyCall::remove_proxies { .. } | ProxyCall::create_pure { .. } |
							ProxyCall::kill_pure { .. }) |
						Vaults(VaultsCall::create { .. } | VaultsCall::set_delegate_account { .. }) =>
							Some(Self::ChangesAuthorization),
						BitcoinLocks(BitcoinLocksCall::create_receive_address { vault_id, .. }) =>
							Some(Self::BitcoinVault(*vault_id)),
						BitcoinLocks(BitcoinLocksCall::set_guarantee { lock_id, .. }) |
						BitcoinFissions(BitcoinFissionsCall::create { lock_id, .. }) =>
							Some(Self::BitcoinLock(*lock_id)),
						BitcoinFissions(BitcoinFissionsCall::ratchet { fission_id, .. }) =>
							Some(Self::BitcoinFission(*fission_id)),
						Balances(BalancesCall::transfer_allow_death { dest, .. } |
							BalancesCall::transfer_keep_alive { dest, .. } |
							BalancesCall::transfer_all { dest, .. }) |
						Ownership(BalancesCall::transfer_allow_death { dest, .. } |
							BalancesCall::transfer_keep_alive { dest, .. } |
							BalancesCall::transfer_all { dest, .. }) => Some(Self::Transfer(dest)),
						OperationalAccounts(OperationalCall::register {
							registration: Registration::V1(registration),
						}) => Some(Self::Link(registration)),
						_ => None,
					}
				}
			}

			impl CallAuthorizationPolicy {
				fn bitcoin_vault_requirements(
					account: &AccountId,
					vault_id: VaultId,
				) -> Vec<CallRequirement<AccountId>> {
					let own_account = Self::default_account(account);
					let is_trusted =
						if VaultIdByOperator::<Runtime>::get(&own_account) == Some(vault_id) {
							true
						} else {
							OperationalAccountsPallet::<Runtime>::upstream_vault(account)
								.is_some_and(|(_, upstream_vault)| upstream_vault == vault_id)
						};

					let protection = CheckProtectionForBatch(account.clone());
					if is_trusted {
						vec![protection]
					} else {
						vec![protection, RequireApproval(account.clone())]
					}
				}

				fn bitcoin_lock_requirements(
					account: &AccountId,
					lock_id: BitcoinLockId,
				) -> Vec<CallRequirement<AccountId>> {
					match LocksById::<Runtime>::get(lock_id) {
						Some(lock) => Self::bitcoin_vault_requirements(account, lock.vault_id),
						None => vec![CheckProtectionForBatch(account.clone())],
					}
				}
			}
		}
	};
}
