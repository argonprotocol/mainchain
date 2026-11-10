#![cfg_attr(not(feature = "std"), no_std)]
// `construct_runtime!` does a lot of recursion and requires us to increase the limit to 256.
#![recursion_limit = "256"]
extern crate alloc;

pub mod apis;
#[cfg(feature = "runtime-benchmarks")]
pub mod benchmarking;
pub mod call_authorization;
mod call_filters;
pub mod config;
mod deal_with_fees;
mod fee_control;
pub mod migrations;
#[cfg(feature = "try-runtime")]
pub mod try_state;

pub mod prelude {
	pub use crate::config::*;
	pub use alloc::{borrow::Cow, boxed::Box, collections::BTreeMap, vec, vec::Vec};
	pub use argon_primitives::{
		account_security::ApprovalSignature,
		apis::*,
		bitcoin::*,
		block_seal::*,
		digests::*,
		notary::*,
		note::*,
		notebook::*,
		prelude::*,
		providers::{OnNewSlot, *},
		tick::Ticker,
		Balance, BlockHash, BlockVotingKey, HashOutput, Nonce, Signature, VotingKey,
	};

	pub use frame_support::{
		construct_runtime, derive_impl,
		dispatch::*,
		genesis_builder_helper::{build_state, get_preset},
		pallet_prelude::*,
		parameter_types,
		traits::{
			fungible,
			fungible::{
				hold::{Inspect, Mutate},
				Balanced, Dust, Inspect as InspectT, Mutate as MutateT, Unbalanced,
			},
			fungibles,
			fungibles::{metadata, roles},
			tokens::{
				DepositConsequence, Fortitude, Precision, Preservation, Provenance,
				WithdrawConsequence,
			},
			ConstBool, ConstU128, ConstU16, ConstU32, ConstU64, ConstU8, Contains, Currency,
			Everything, Imbalance, InsideBoth, InstanceFilter, KeyOwnerProofSystem, OnUnbalanced,
			Randomness, SortedMembers, StorageInfo, StorageMapShim, TransformOrigin,
		},
		weights::{
			constants::{
				BlockExecutionWeight, ExtrinsicBaseWeight, RocksDbWeight,
				WEIGHT_REF_TIME_PER_SECOND,
			},
			IdentityFee, Weight, WeightToFee, WeightToFeeCoefficient, WeightToFeeCoefficients,
			WeightToFeePolynomial,
		},
		PalletId, StorageValue,
	};
	pub use frame_system::{limits::BlockWeights as BlockWeightsT, pallet_prelude::*, EnsureRoot};
	pub use pallet_bitcoin_locks::BitcoinVerifier;
	pub use pallet_block_rewards::GrowthPath;
	pub use pallet_notebook::NotebookVerifyError;
	pub use pallet_tx_pause::RuntimeCallNameOf;

	pub use sp_api::{decl_runtime_apis, impl_runtime_apis};
	pub use sp_arithmetic::{FixedU128, Perbill, Percent};
	pub use sp_consensus_grandpa::{AuthorityId as GrandpaId, AuthorityList, AuthorityWeight};
	pub use sp_core::{Get, OpaqueMetadata, H256, U256};
	pub use sp_runtime::{
		generic, impl_tx_ext_default,
		traits::{
			BlakeTwo256, Block as BlockT, DispatchInfoOf, DispatchOriginOf, Dispatchable,
			Header as HeaderT, NumberFor, PostDispatchInfoOf, Saturating, TransactionExtension,
			UniqueSaturatedInto,
		},
		ApplyExtrinsicResult, Digest, DigestItem, KeyTypeId, MultiAddress,
	};
	pub use sp_version::RuntimeVersion;
}

#[macro_export]
#[cfg(feature = "runtime-benchmarks")]
macro_rules! use_unless_benchmark {
	($prod:ty, $bench:ty) => {
		$bench
	};
}

#[macro_export]
#[cfg(not(feature = "runtime-benchmarks"))]
macro_rules! use_unless_benchmark {
	($prod:ty, $bench:ty) => {
		$prod
	};
}

#[macro_export]
macro_rules! inject_runtime_vars {
	() => {
		use frame_metadata_hash_extension::CheckMetadataHash;
		use frame_system::{
			AuthorizeCall, CheckGenesis, CheckMortality, CheckNonZeroSender, CheckNonce,
			CheckSpecVersion, CheckTxVersion, CheckWeight, WeightReclaim,
		};
		use pallet_account_security::{ApprovalPayloadProvider, CheckAccountSecurity};
		use pallet_fee_control::CheckFeeWrapper;
		use pallet_transaction_payment::ChargeTransactionPayment;
		use sp_runtime::transaction_validity::TransactionValidityError;
		use $crate::migrations::CosignerGuaranteesMigration;

		// To learn more about runtime versioning, see:
		// https://docs.substrate.io/main-docs/build/upgrade#runtime-versioning
		#[sp_version::runtime_version]
		pub const VERSION: RuntimeVersion = RuntimeVersion {
			spec_name: Cow::Borrowed("argon"),
			impl_name: Cow::Borrowed("argon"),
			authoring_version: 1,
			// The version of the runtime specification. A full node will not attempt to use its
			// native   runtime in substitute for the on-chain Wasm runtime unless all of
			// `spec_name`,   `spec_version`, and `authoring_version` are the same between Wasm and
			// native. This value is set to 100 to notify Polkadot-JS App (https://polkadot.js.org/apps) to use
			//   the compatible custom types.
			spec_version: 161,
			impl_version: 10,
			apis: RUNTIME_API_VERSIONS,
			transaction_version: 6,
			system_version: 1,
		};
		parameter_types! {
			pub const Version: RuntimeVersion = VERSION;
		}

		/// The version information used to identify this runtime when compiled natively.
		#[cfg(feature = "std")]
		pub fn native_version() -> NativeVersion {
			NativeVersion { runtime_version: VERSION, can_author_with: Default::default() }
		}

		/// The address format for describing accounts.
		pub type Address = sp_runtime::MultiAddress<AccountId, ()>;
		/// Block header type as expected by this runtime.
		pub type Header = generic::Header<BlockNumber, BlockHash>;
		/// Block type as expected by this runtime.
		pub type Block = generic::Block<Header, UncheckedExtrinsic>;
		/// The `TransactionExtension` to the basic transaction logic.
		pub type TxExtension = (
			AuthorizeCall<Runtime>,
			CheckNonZeroSender<Runtime>,
			CheckSpecVersion<Runtime>,
			CheckTxVersion<Runtime>,
			CheckGenesis<Runtime>,
			CheckMortality<Runtime>,
			CheckNonce<Runtime>,
			CheckWeight<Runtime>,
			CheckAccountSecurity<Runtime>,
			CheckFeeWrapper<Runtime, ChargeTransactionPayment<Runtime>>,
			CheckMetadataHash<Runtime>,
			WeightReclaim<Runtime>,
		);
		/// All migrations of the runtime.
		///
		/// This can be a tuple of types, each implementing `OnRuntimeUpgrade`.
		type Migrations = (
			CosignerGuaranteesMigration<Runtime>,
			$crate::migrations::AddVaultSecuritizedSatoshis<Runtime>,
			pallet_crosschain_transfer::migrations::AddGatewaySyncTimeMigration<Runtime>,
			pallet_crosschain_transfer::migrations::InitializePendingQuoteCountsMigration<Runtime>,
			pallet_ethereum_verifier::migrations::AddGloasForkMigration<Runtime>,
		);

		/// Unchecked extrinsic type as expected by this runtime.
		pub type UncheckedExtrinsic =
			generic::UncheckedExtrinsic<Address, RuntimeCall, Signature, TxExtension>;
		/// The payload being signed in transactions.
		pub type SignedPayload = generic::SignedPayload<RuntimeCall, TxExtension>;

		/// Detached security proofs use the same native signing format as wallet transactions.
		/// Their replay bounds live in the call, not the submitter's nonce, era or fee fields.
		pub struct AccountSecuritySigningPayload;

		impl ApprovalPayloadProvider<RuntimeCall> for AccountSecuritySigningPayload {
			fn signing_payload(
				call: RuntimeCall,
				with_metadata_hash: bool,
			) -> Result<Vec<u8>, TransactionValidityError> {
				let extra: TxExtension = (
					AuthorizeCall::<Runtime>::new(),
					CheckNonZeroSender::<Runtime>::new(),
					CheckSpecVersion::<Runtime>::new(),
					CheckTxVersion::<Runtime>::new(),
					CheckGenesis::<Runtime>::new(),
					CheckMortality::<Runtime>::from(generic::Era::Immortal),
					CheckNonce::<Runtime>::from(0),
					CheckWeight::<Runtime>::new(),
					CheckAccountSecurity::<Runtime>::new(),
					CheckFeeWrapper::from(ChargeTransactionPayment::<Runtime>::from(0)),
					CheckMetadataHash::<Runtime>::new(with_metadata_hash),
					WeightReclaim::<Runtime>::new(),
				);
				SignedPayload::new(call, extra)
					.map(|payload| payload.using_encoded(|bytes| bytes.to_vec()))
			}
		}
		/// Executive: handles dispatch to the various modules.
		pub type Executive = frame_executive::Executive<
			Runtime,
			Block,
			frame_system::ChainContext<Runtime>,
			Runtime,
			AllPalletsWithSystem,
			Migrations,
		>;
	};
}
