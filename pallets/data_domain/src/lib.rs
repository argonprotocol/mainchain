#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
use scale_info::TypeInfo;
use sp_runtime::RuntimeDebug;
use sp_std::prelude::*;

pub use pallet::*;
use ulx_primitives::tick::Tick;
pub use weights::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

pub mod weights;

///
/// This pallet tracks data domains and their zone records. Data Domains are registered via
/// localchain tax write-offs that are synchronized through the notary network. Zone records are
/// updated by the domain owner and are used to track the latest version of a data domain and the
/// host addresses where it can be accessed.
///
/// If more than one data domain registration is received in a tick, they are canceled out.
#[frame_support::pallet(dev_mode)]
pub mod pallet {
	use frame_support::{pallet_prelude::*, traits::Len};
	use frame_system::pallet_prelude::*;

	use ulx_primitives::{
		notebook::NotebookHeader, DataDomain, DataDomainProvider, NotebookEventHandler,
		TickProvider, ZoneRecord, MAX_DOMAINS_PER_NOTEBOOK, MAX_NOTARIES,
	};

	use super::*;

	type DataDomainRegistrationOf<T> =
		DataDomainRegistration<<T as frame_system::Config>::AccountId>;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		type WeightInfo: WeightInfo;

		/// The number of blocks after which a domain will expire if not renewed.
		type DomainExpirationTicks: Get<Tick>;

		type TickProvider: TickProvider<Self::Block>;

		type HistoricalPaymentAddressTicksToKeep: Get<Tick>;
	}

	#[pallet::storage]
	pub(super) type RegisteredDataDomains<T: Config> =
		StorageMap<_, Blake2_128Concat, DataDomain, DataDomainRegistrationOf<T>, OptionQuery>;
	#[pallet::storage]
	pub(super) type ZoneRecordsByDomain<T: Config> =
		StorageMap<_, Blake2_128Concat, DataDomain, ZoneRecord<T::AccountId>, OptionQuery>;

	#[pallet::storage]
	pub(super) type DomainPaymentAddressHistory<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		DataDomain,
		BoundedVec<(T::AccountId, Tick), T::HistoricalPaymentAddressTicksToKeep>,
		ValueQuery,
	>;

	#[pallet::storage]
	pub(super) type ExpiringDomainsByBlock<T: Config> = StorageMap<
		_,
		Twox64Concat,
		Tick,
		BoundedVec<DataDomain, ConstU32<{ MAX_DOMAINS_PER_NOTEBOOK * MAX_NOTARIES }>>,
		ValueQuery,
	>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// A data domain zone record was updated
		ZoneRecordUpdated { domain: DataDomain, zone_record: ZoneRecord<T::AccountId> },
		/// A data domain was registered
		DataDomainRegistered {
			domain: DataDomain,
			registration: DataDomainRegistration<T::AccountId>,
		},
		/// A data domain was registered
		DataDomainRenewed { domain: DataDomain },
		/// A data domain was expired
		DataDomainExpired { domain: DataDomain },
		/// A data domain registration was canceled due to a conflicting registration in the same
		/// tick
		DataDomainRegistrationCanceled {
			domain: DataDomain,
			registration: DataDomainRegistration<T::AccountId>,
		},
	}

	#[pallet::error]
	pub enum Error<T> {
		/// The domain is not registered.
		DomainNotRegistered,
		/// The sender is not the owner of the domain.
		NotDomainOwner,
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(_n: BlockNumberFor<T>) -> Weight {
			let tick = T::TickProvider::current_tick();
			let expiring = ExpiringDomainsByBlock::<T>::take(&tick);
			let entries = expiring.len() as u64;
			for domain in expiring {
				RegisteredDataDomains::<T>::remove(&domain);
				ZoneRecordsByDomain::<T>::remove(&domain);
				Self::clean_old_payment_addresses(&domain, tick);
				Self::deposit_event(Event::DataDomainExpired { domain });
			}

			T::DbWeight::get().reads_writes(2 + (entries * 2), 2 + (entries * 2))
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::call_index(0)]
		#[pallet::weight(0)]
		pub fn set_zone_record(
			origin: OriginFor<T>,
			domain: DataDomain,
			zone_record: ZoneRecord<T::AccountId>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let Some(registration) = RegisteredDataDomains::<T>::get(&domain) else {
				return Err(Error::<T>::DomainNotRegistered.into());
			};

			ensure!(registration.account_id == who, Error::<T>::NotDomainOwner);

			ZoneRecordsByDomain::<T>::insert(&domain, &zone_record);
			let tick = T::TickProvider::current_tick();
			Self::clean_old_payment_addresses(&domain, tick);
			DomainPaymentAddressHistory::<T>::try_mutate(&domain, |entry| {
				entry.try_push((zone_record.payment_account.clone(), tick))
			})
			.map_err(|_| DispatchError::Other("Failed to add payment address to history"))?;
			Self::deposit_event(Event::ZoneRecordUpdated { domain, zone_record });

			Ok(())
		}
	}

	impl<T: Config> NotebookEventHandler for Pallet<T> {
		fn notebook_submitted(header: &NotebookHeader) -> sp_runtime::DispatchResult {
			let expiration_ticks = T::DomainExpirationTicks::get();
			for (domain, account) in header.data_domains.iter() {
				let mut is_renewal = false;
				let account_id = T::AccountId::decode(&mut account.encode().as_slice())
					.map_err(|_| DispatchError::Other("Failed to decode account id"))?;
				// if previous registration is at same tick, need to cancel it out
				if let Some(registration) = <RegisteredDataDomains<T>>::get(domain) {
					let original_expiration = registration.registered_at_tick + expiration_ticks;
					let remove_expired = || {
						<ExpiringDomainsByBlock<T>>::mutate(original_expiration, |domains| {
							domains.retain(|d| d != domain);
						});
					};

					if registration.registered_at_tick >= header.tick {
						<RegisteredDataDomains<T>>::remove(&domain);
						remove_expired();
						Self::deposit_event(Event::DataDomainRegistrationCanceled {
							domain: domain.clone(),
							registration,
						});
						continue;
					}

					if registration.account_id == account_id {
						remove_expired();
						is_renewal = true;
					} else {
						// don't process if account is different
						continue;
					}
				}

				let registration =
					DataDomainRegistration { account_id, registered_at_tick: header.tick };
				<RegisteredDataDomains<T>>::insert(domain, registration.clone());
				<ExpiringDomainsByBlock<T>>::mutate(header.tick + expiration_ticks, |domains| {
					domains.try_push(domain.clone())
				})
				.map_err(|_| DispatchError::Other("Failed to add domain to expiration list"))?;

				if is_renewal {
					Self::deposit_event(Event::DataDomainRenewed { domain: domain.clone() });
				} else {
					Self::deposit_event(Event::DataDomainRegistered {
						domain: domain.clone(),
						registration,
					});
				}
			}

			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		fn clean_old_payment_addresses(domain: &DataDomain, current_tick: Tick) {
			let oldest_history_to_preserve =
				current_tick.saturating_sub(T::HistoricalPaymentAddressTicksToKeep::get());
			DomainPaymentAddressHistory::<T>::mutate_exists(domain, |entry| {
				if let Some(records) = entry {
					records.retain(|(_, tick)| tick >= &oldest_history_to_preserve);
					if records.is_empty() {
						*entry = None;
					}
				}
			});
		}
	}

	impl<T: Config> DataDomainProvider<T::AccountId> for Pallet<T> {
		fn is_registered_payment_account(
			data_domain: &DataDomain,
			account_id: &T::AccountId,
			tick_range: (Tick, Tick),
		) -> bool {
			if let Some(zone) = ZoneRecordsByDomain::<T>::get(data_domain) {
				if zone.payment_account == *account_id {
					return true
				}
			}

			for (addr, tick) in <DomainPaymentAddressHistory<T>>::get(data_domain) {
				if addr == *account_id && tick >= tick_range.0 && tick <= tick_range.1 {
					return true
				}
			}

			false
		}
	}
}

#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct DataDomainRegistration<AccountId> {
	pub account_id: AccountId,
	pub registered_at_tick: Tick,
}
