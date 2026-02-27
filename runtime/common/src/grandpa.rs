#[macro_export]
macro_rules! inject_grandpa_support {
	() => {
		use codec::{Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
		use core::marker::PhantomData;
		use frame_system::offchain::{CreateBare, SubmitTransaction};
		use pallet_grandpa::{Call as GrandpaCall, Error as GrandpaError};
		use pallet_mining_slot::grandpa::{
			derive_authorities_from_recent_mining, DeriveAuthoritiesError,
		};
		use polkadot_sdk::sp_staking::{offence::OffenceReportSystem, SessionIndex};
		use scale_info::TypeInfo;
		use sp_consensus_grandpa::{
			AuthorityId as GrandpaAuthorityId, AuthorityList, EquivocationProof,
			OpaqueKeyOwnershipProof, SetId,
		};
		use sp_runtime::{
			transaction_validity::{InvalidTransaction, TransactionValidityError},
			DispatchError,
		};
		use sp_session::{GetSessionNumber, GetValidatorCount};

		type GrandpaEquivocationProof = EquivocationProof<HashOutput, BlockNumberFor<Runtime>>;

		pub struct GrandpaSlotRotation;

		impl OnNewSlot<AccountId> for GrandpaSlotRotation {
			type Key = GrandpaId;

			fn rotate_grandpas(
				_current_frame_id: FrameId,
				_removed_authorities: Vec<(&AccountId, Self::Key)>,
				_added_authorities: Vec<(&AccountId, Self::Key)>,
			) {
				let current_authorities: AuthorityList = Grandpa::grandpa_authorities();
				let next_authorities =
					match derive_authorities_from_recent_mining::<Runtime, GrandpaId>() {
						Ok(authorities) => authorities,
						Err(DeriveAuthoritiesError::RegisteredMiningInactive) => {
							current_authorities.clone()
						},
						Err(DeriveAuthoritiesError::NoEligibleWeights) => {
							log::warn!(
								"Skipping grandpa rotation because no miners were active in the recent window"
							);
							return;
						},
					};

				log::info!("Scheduling grandpa change (authorities={})", next_authorities.len());
				if let Err(err) = Grandpa::schedule_change(next_authorities, 0, None) {
					log::error!("Failed to schedule grandpa change: {err:?}");
					return;
				}
				MiningSlot::record_previous_grandpa_authorities(current_authorities);
				pallet_grandpa::CurrentSetId::<Runtime>::mutate(|x| *x += 1);
			}
		}

		#[derive(
			Clone,
			Debug,
			Encode,
			Decode,
			DecodeWithMemTracking,
			Eq,
			PartialEq,
			TypeInfo,
			MaxEncodedLen,
		)]
		pub struct GrandpaKeyOwnerProof {
			pub set_id: SetId,
		}

		impl GetSessionNumber for GrandpaKeyOwnerProof {
			fn session(&self) -> SessionIndex {
				0
			}
		}

		impl GetValidatorCount for GrandpaKeyOwnerProof {
			fn validator_count(&self) -> u32 {
				validator_count_for_set(self.set_id)
			}
		}

		pub struct GrandpaEquivocationReportSystem<L>(PhantomData<L>);

		impl<L>
			OffenceReportSystem<
				Option<AccountId>,
				(GrandpaEquivocationProof, GrandpaKeyOwnerProof),
			> for GrandpaEquivocationReportSystem<L>
		where
			L: Get<u64>,
		{
			type Longevity = L;

			fn publish_evidence(
				evidence: (GrandpaEquivocationProof, GrandpaKeyOwnerProof),
			) -> Result<(), ()> {
				let (equivocation_proof, key_owner_proof) = evidence;
				let call = GrandpaCall::report_equivocation_unsigned {
					equivocation_proof: Box::new(equivocation_proof),
					key_owner_proof,
				};
				let xt = <Runtime as CreateBare<GrandpaCall<Runtime>>>::create_bare(call.into());
				let res = SubmitTransaction::<Runtime, GrandpaCall<Runtime>>::submit_transaction(xt);
				match res {
					Ok(_) => {
						log::info!(target: sp_consensus_grandpa::RUNTIME_LOG_TARGET, "Submitted equivocation report");
					},
					Err(err) => log::error!(
						target: sp_consensus_grandpa::RUNTIME_LOG_TARGET,
						"Error submitting equivocation report: {err:?}",
					),
				}
				res
			}

			fn check_evidence(
				evidence: (GrandpaEquivocationProof, GrandpaKeyOwnerProof),
			) -> Result<(), TransactionValidityError> {
				let (equivocation_proof, key_owner_proof) = evidence;

				if !sp_consensus_grandpa::check_equivocation_proof(equivocation_proof.clone()) {
					return Err(InvalidTransaction::BadProof.into());
				}

				validate_key_owner_proof(&equivocation_proof, &key_owner_proof)
					.map_err(|_| InvalidTransaction::BadProof.into())
			}

			fn process_evidence(
				_reporter: Option<AccountId>,
				evidence: (GrandpaEquivocationProof, GrandpaKeyOwnerProof),
			) -> Result<(), DispatchError> {
				let (equivocation_proof, key_owner_proof) = evidence;

				if !sp_consensus_grandpa::check_equivocation_proof(equivocation_proof.clone()) {
					return Err(GrandpaError::<Runtime>::InvalidEquivocationProof.into());
				}

				validate_key_owner_proof(&equivocation_proof, &key_owner_proof)
					.map_err(|_| GrandpaError::<Runtime>::InvalidKeyOwnershipProof)?;

				MiningSlot::record_grandpa_equivocation_observed(
					equivocation_proof.offender().clone(),
					equivocation_proof.set_id(),
					equivocation_proof.round(),
				);
				Ok(())
			}
		}

		pub fn submit_unsigned_equivocation_report(
			equivocation_proof: GrandpaEquivocationProof,
			key_owner_proof: OpaqueKeyOwnershipProof,
		) -> Option<()> {
			let key_owner_proof = key_owner_proof.decode::<GrandpaKeyOwnerProof>()?;
			Grandpa::submit_unsigned_equivocation_report(
				equivocation_proof,
				key_owner_proof,
			)
		}

		pub fn generate_key_ownership_proof(
			set_id: SetId,
			authority_id: GrandpaAuthorityId,
		) -> Option<OpaqueKeyOwnershipProof> {
			let current_set_id = Grandpa::current_set_id();
			let current_authorities = Grandpa::grandpa_authorities();

			if set_id == current_set_id && contains_authority(&current_authorities, &authority_id) {
				return Some(OpaqueKeyOwnershipProof::new(
					GrandpaKeyOwnerProof { set_id }.encode(),
				));
			}

			if current_set_id == 0 {
				return None;
			}

			let previous_set_id = current_set_id - 1;
			if set_id != previous_set_id {
				return None;
			}

			let previous_authorities = MiningSlot::previous_grandpa_authorities()?;
			if !contains_authority(&previous_authorities, &authority_id) {
				return None;
			}

			Some(OpaqueKeyOwnershipProof::new(
				GrandpaKeyOwnerProof { set_id }.encode(),
			))
		}

		fn validate_key_owner_proof(
			equivocation_proof: &GrandpaEquivocationProof,
			key_owner_proof: &GrandpaKeyOwnerProof,
		) -> Result<(), ()> {
			let offender = equivocation_proof.offender();
			let current_set_id = Grandpa::current_set_id();
			let current_authorities = Grandpa::grandpa_authorities();
			let reported_set_id = equivocation_proof.set_id();

			if key_owner_proof.set_id != reported_set_id {
				return Err(());
			}

			if reported_set_id == current_set_id {
				if !contains_authority(&current_authorities, offender) {
					return Err(());
				}
				return Ok(());
			}

			if current_set_id == 0 {
				return Err(());
			}

			let previous_set_id = current_set_id - 1;
			if reported_set_id != previous_set_id {
				return Err(());
			}

			let Some(previous_authorities) = MiningSlot::previous_grandpa_authorities() else {
				return Err(());
			};

			if !contains_authority(&previous_authorities, offender) {
				return Err(());
			}

			Ok(())
		}

		fn contains_authority(
			authorities: &AuthorityList,
			authority_id: &GrandpaAuthorityId,
		) -> bool {
			authorities.iter().any(|(authority, _)| authority == authority_id)
		}

		fn validator_count_for_set(set_id: SetId) -> u32 {
			let current_count = Grandpa::grandpa_authorities().len() as u32;
			let current_set_id = Grandpa::current_set_id();
			if set_id == current_set_id {
				return current_count;
			}

			let previous_count = MiningSlot::previous_grandpa_authorities()
				.map(|authorities| authorities.len() as u32)
				.unwrap_or(current_count);
			if current_set_id != 0 && set_id == current_set_id - 1 {
				return previous_count;
			}

			current_count.max(previous_count)
		}
	};
}
