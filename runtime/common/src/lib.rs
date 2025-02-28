#![cfg_attr(not(feature = "std"), no_std)]
// `construct_runtime!` does a lot of recursion and requires us to increase the limit to 256.
#![recursion_limit = "256"]
extern crate alloc;

pub mod apis;
mod call_filters;
pub mod config;
mod deal_with_fees;
pub mod token_asset;

pub mod prelude {
	pub use crate::config::*;
	pub use alloc::{boxed::Box, collections::BTreeMap, vec, vec::Vec};
	pub use argon_primitives::{
		apis::*, bitcoin::*, block_seal::*, digests::*, notary::*, note::*, notebook::*,
		prelude::*, providers::*, tick::Ticker, Balance, BlockHash, BlockVotingKey, HashOutput,
		Nonce, Signature, VotingKey,
	};
	pub use frame_support::{
		construct_runtime, derive_impl,
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
			IdentityFee, Weight, WeightToFeeCoefficient, WeightToFeeCoefficients,
			WeightToFeePolynomial,
		},
		PalletId, StorageValue,
	};
	pub use frame_system::{limits::BlockWeights as BlockWeightsT, pallet_prelude::*, EnsureRoot};
	pub use ismp::host::StateMachine;
	pub use pallet_bitcoin_locks::BitcoinVerifier;
	pub use pallet_block_rewards::GrowthPath;
	pub use pallet_mining_slot::OnNewSlot;
	pub use pallet_notebook::NotebookVerifyError;
	pub use pallet_tx_pause::RuntimeCallNameOf;
	pub use sp_api::{decl_runtime_apis, impl_runtime_apis};
	pub use sp_arithmetic::{FixedU128, Perbill, Percent};
	pub use sp_consensus_grandpa::{AuthorityId as GrandpaId, AuthorityList, AuthorityWeight};
	pub use sp_core::{Get, OpaqueMetadata, H256, U256};
	pub use sp_runtime::{
		generic,
		traits::{BlakeTwo256, Block as BlockT, Header as HeaderT, NumberFor},
		ApplyExtrinsicResult, Digest, DigestItem, KeyTypeId,
	};
	pub use sp_version::{create_runtime_str, RuntimeVersion};
}

#[macro_export]
macro_rules! inject_runtime_vars {
	() => {
		// To learn more about runtime versioning, see:
		// https://docs.substrate.io/main-docs/build/upgrade#runtime-versioning
		#[sp_version::runtime_version]
		pub const VERSION: RuntimeVersion = RuntimeVersion {
			spec_name: create_runtime_str!("argon"),
			impl_name: create_runtime_str!("argon"),
			authoring_version: 1,
			// The version of the runtime specification. A full node will not attempt to use its
			// native   runtime in substitute for the on-chain Wasm runtime unless all of
			// `spec_name`,   `spec_version`, and `authoring_version` are the same between Wasm and
			// native. This value is set to 100 to notify Polkadot-JS App (https://polkadot.js.org/apps) to use
			//   the compatible custom types.
			spec_version: 116,
			impl_version: 6,
			apis: RUNTIME_API_VERSIONS,
			transaction_version: 2,
			state_version: 1,
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
		/// The SignedExtension to the basic transaction logic.
		pub type SignedExtra = (
			frame_system::CheckNonZeroSender<Runtime>,
			frame_system::CheckSpecVersion<Runtime>,
			frame_system::CheckTxVersion<Runtime>,
			frame_system::CheckGenesis<Runtime>,
			frame_system::CheckMortality<Runtime>,
			frame_system::CheckNonce<Runtime>,
			frame_system::CheckWeight<Runtime>,
			pallet_transaction_payment::ChargeTransactionPayment<Runtime>,
			frame_metadata_hash_extension::CheckMetadataHash<Runtime>,
		);
		/// All migrations of the runtime, aside from the ones declared in the pallets.
		///
		/// This can be a tuple of types, each implementing `OnRuntimeUpgrade`.
		type Migrations = (
            pallet_mining_slot::migrations::BiddingMigration<Runtime>,
			pallet_vaults::migrations::BondedBitcoinBidPoolMigration<Runtime>,
        );

		/// Unchecked extrinsic type as expected by this runtime.
		pub type UncheckedExtrinsic =
			generic::UncheckedExtrinsic<Address, RuntimeCall, Signature, SignedExtra>;
		/// The payload being signed in transactions.
		pub type SignedPayload = generic::SignedPayload<RuntimeCall, SignedExtra>;
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
