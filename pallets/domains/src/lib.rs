#![cfg_attr(not(feature = "std"), no_std)]
extern crate alloc;

use codec::{Decode, Encode};
use scale_info::TypeInfo;
use sp_runtime::RuntimeDebug;

use argon_primitives::tick::Tick;
pub use pallet::*;
pub use weights::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

pub mod weights;

///
/// This pallet tracks domains and their zone records. Domains are registered via
/// localchain tax write-offs that are synchronized through the notary network. Zone records are
/// updated by the domain owner and are used to track the latest version of a data domain and the
/// host addresses where it can be accessed.
///
/// If more than one data domain registration is received in a tick, they are canceled out.
#[frame_support::pallet(dev_mode)]
pub mod pallet {
	use argon_primitives::{
		notebook::NotebookHeader, DomainHash, NotebookEventHandler, TickProvider, ZoneRecord,
		MAX_DOMAINS_PER_NOTEBOOK, MAX_NOTARIES,
	};
	use frame_support::{pallet_prelude::*, traits::Len};
	use frame_system::pallet_prelude::*;
	use sp_core::crypto::AccountId32;

	use super::*;

	type DomainRegistrationOf<T> = DomainRegistration<<T as frame_system::Config>::AccountId>;

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
	pub(super) type RegisteredDomains<T: Config> =
		StorageMap<_, Blake2_128Concat, DomainHash, DomainRegistrationOf<T>, OptionQuery>;
	#[pallet::storage]
	pub(super) type ZoneRecordsByDomain<T: Config> =
		StorageMap<_, Blake2_128Concat, DomainHash, ZoneRecord<T::AccountId>, OptionQuery>;

	#[pallet::storage]
	pub(super) type ExpiringDomainsByBlock<T: Config> = StorageMap<
		_,
		Twox64Concat,
		Tick,
		BoundedVec<DomainHash, ConstU32<{ MAX_DOMAINS_PER_NOTEBOOK * MAX_NOTARIES }>>,
		ValueQuery,
	>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// A data domain zone record was updated
		ZoneRecordUpdated { domain_hash: DomainHash, zone_record: ZoneRecord<T::AccountId> },
		/// A data domain was registered
		DomainRegistered { domain_hash: DomainHash, registration: DomainRegistration<T::AccountId> },
		/// A data domain was registered
		DomainRenewed { domain_hash: DomainHash },
		/// A data domain was expired
		DomainExpired { domain_hash: DomainHash },
		/// A data domain registration was canceled due to a conflicting registration in the same
		/// tick
		DomainRegistrationCanceled {
			domain_hash: DomainHash,
			registration: DomainRegistration<T::AccountId>,
		},
		/// A data domain registration failed due to an error
		DomainRegistrationError {
			domain_hash: DomainHash,
			account_id: AccountId32,
			error: DispatchError,
		},
	}

	#[pallet::error]
	pub enum Error<T> {
		/// The domain is not registered.
		DomainNotRegistered,
		/// The sender is not the owner of the domain.
		NotDomainOwner,
		/// Failed to add to the address history.
		FailedToAddToAddressHistory,
		/// Failed to add to the expiring domain list
		FailedToAddExpiringDomain,
		/// Error decoding account from notary
		AccountDecodingError,
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(_n: BlockNumberFor<T>) -> Weight {
			let tick = T::TickProvider::current_tick();
			let expiring = ExpiringDomainsByBlock::<T>::take(tick);
			let entries = expiring.len() as u64;
			for domain_hash in expiring {
				RegisteredDomains::<T>::remove(domain_hash);
				ZoneRecordsByDomain::<T>::remove(domain_hash);
				Self::deposit_event(Event::DomainExpired { domain_hash });
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
			domain_hash: DomainHash,
			zone_record: ZoneRecord<T::AccountId>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let Some(registration) = RegisteredDomains::<T>::get(domain_hash) else {
				return Err(Error::<T>::DomainNotRegistered.into());
			};

			ensure!(registration.account_id == who, Error::<T>::NotDomainOwner);

			ZoneRecordsByDomain::<T>::insert(domain_hash, &zone_record);
			Self::deposit_event(Event::ZoneRecordUpdated { domain_hash, zone_record });

			Ok(())
		}
	}

	impl<T: Config> NotebookEventHandler for Pallet<T> {
		fn notebook_submitted(header: &NotebookHeader) {
			let expiration_ticks = T::DomainExpirationTicks::get();
			for (domain_hash, account) in header.domains.iter() {
				let mut is_renewal = false;
				let account_id = match T::AccountId::decode(&mut account.encode().as_slice()) {
					Ok(account_id) => account_id,
					Err(_) => {
						Self::deposit_event(Event::DomainRegistrationError {
							domain_hash: *domain_hash,
							account_id: account.clone(),
							error: Error::<T>::AccountDecodingError.into(),
						});
						continue;
					},
				};

				// if previous registration is at same tick, need to cancel it out
				if let Some(registration) = <RegisteredDomains<T>>::get(domain_hash) {
					let original_expiration = registration.registered_at_tick + expiration_ticks;
					let remove_expired = || {
						<ExpiringDomainsByBlock<T>>::mutate(original_expiration, |domains| {
							domains.retain(|d| d != domain_hash);
						});
					};

					if registration.registered_at_tick >= header.tick {
						<RegisteredDomains<T>>::remove(domain_hash);
						remove_expired();
						Self::deposit_event(Event::DomainRegistrationCanceled {
							domain_hash: *domain_hash,
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
					DomainRegistration { account_id, registered_at_tick: header.tick };
				<RegisteredDomains<T>>::insert(domain_hash, registration.clone());
				if <ExpiringDomainsByBlock<T>>::mutate(header.tick + expiration_ticks, |domains| {
					domains.try_push(*domain_hash)
				})
				.is_err()
				{
					Self::deposit_event(Event::DomainRegistrationError {
						domain_hash: *domain_hash,
						account_id: account.clone(),
						error: Error::<T>::FailedToAddExpiringDomain.into(),
					});
					continue;
				}

				if is_renewal {
					Self::deposit_event(Event::DomainRenewed { domain_hash: *domain_hash });
				} else {
					Self::deposit_event(Event::DomainRegistered {
						domain_hash: *domain_hash,
						registration,
					});
				}
			}
		}
	}
}

#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct DomainRegistration<AccountId> {
	pub account_id: AccountId,
	pub registered_at_tick: Tick,
}
