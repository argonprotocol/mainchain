use super::{Balances, Get, Ismp, Runtime, RuntimeEvent, Timestamp};
use crate::{weights, ChainTransfer, Ownership, TokenGateway};
use alloc::{boxed::Box, vec, vec::Vec};
use argon_primitives::{AccountId, Balance};
use frame_support::{
	parameter_types,
	traits::{
		fungible,
		fungible::{Inspect as InspectT, Mutate, Unbalanced as UnbalancedT},
		fungibles,
		fungibles::{Dust, Inspect, Unbalanced},
		tokens::{
			DepositConsequence, Fortitude, Precision, Preservation, Provenance, WithdrawConsequence,
		},
		Currency, SortedMembers,
	},
};
use frame_system::{EnsureRoot, EnsureSignedBy};
use ismp::{host::StateMachine, module::IsmpModule, router::IsmpRouter, Error};
use sp_runtime::{DispatchError, DispatchResult};

parameter_types! {
	// The host state machine of this pallet
	pub const HostStateMachine: StateMachine = StateMachine::Substrate(*b"argn");
	// A constant that should represent the native asset id, this id must be unique to the native currency
	pub const NativeAssetId: u32 = 0;

	// The ownership token Asset Id
	pub const OwnershipTokenAssetId: u32 = 1;
	// Set the correct decimals for the native currency
	pub const Decimals: u8 = 6;
}

pub struct TokenAdmin;
impl Get<AccountId> for TokenAdmin {
	fn get() -> AccountId {
		ChainTransfer::hyperbridge_token_admin()
	}
}
pub struct TokenAdmins;
impl SortedMembers<AccountId> for TokenAdmins {
	fn sorted_members() -> Vec<AccountId> {
		vec![TokenAdmin::get()]
	}
	fn contains(t: &AccountId) -> bool {
		*t == TokenAdmin::get()
	}
	fn count() -> usize {
		1
	}
	#[cfg(feature = "runtime-benchmarks")]
	fn add(_t: &AccountId) {
		panic!("TokenAdmins is a singleton and cannot have members added to it.")
	}
}
#[cfg(not(feature = "canary"))]
parameter_types! {
	// The hyperbridge parachain on Polkadot
	pub const Coprocessor: Option<StateMachine> = Some(StateMachine::Polkadot(3367));
}

#[cfg(feature = "canary")]
parameter_types! {
	pub const Coprocessor: Option<StateMachine> = Some(StateMachine::Kusama(4009));
}

impl pallet_token_gateway::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	// Configured as Pallet Ismp
	type Dispatcher = Ismp;
	// Configured as Pallet balances
	type NativeCurrency = Balances;
	// AssetAdmin account to register new assets on the chain. We don't use this
	type AssetAdmin = TokenAdmin;
	// Configured as Pallet Assets
	type Assets = OwnershipTokenAsset;
	// The Native asset Id
	type NativeAssetId = NativeAssetId;
	// The precision of the native asset
	type Decimals = Decimals;
	type CreateOrigin = EnsureSignedBy<TokenAdmins, AccountId>;
	type WeightInfo = ();
	type EvmToSubstrate = ();
}

impl pallet_ismp::Config for Runtime {
	// configure the runtime event
	type RuntimeEvent = RuntimeEvent;
	// Permissioned origin who can create or update consensus clients
	type AdminOrigin = EnsureRoot<AccountId>;
	// The pallet_timestamp pallet
	type TimestampProvider = Timestamp;
	// The balance type for the currency implementation
	type Balance = Balance;
	// The currency implementation that is offered to relayers
	type Currency = Balances;
	// The state machine identifier for this state machine
	type HostStateMachine = HostStateMachine;
	// Optional coprocessor for incoming requests/responses
	type Coprocessor = Coprocessor;
	// Router implementation for routing requests/responses to their respective modules
	type Router = Router;
	// Supported consensus clients
	type ConsensusClients = (
		// Add the grandpa or beefy consensus client here
		ismp_grandpa::consensus::GrandpaConsensusClient<Runtime>,
	);
	// Weight provider for local modules
	type WeightProvider = ();
	// Optional merkle mountain range overlay tree, for cheaper outgoing request proofs.
	// You most likely don't need it, just use the `NoOpMmrTree`
	type OffchainDB = ();
}

impl ismp_grandpa::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type IsmpHost = Ismp;
	type WeightInfo = weights::ismp_grandpa::WeightInfo<Runtime>;
}

impl pallet_hyperbridge::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type IsmpHost = Ismp;
}

// Add the token gateway pallet to your ISMP router
#[derive(Default)]
pub struct Router;

impl IsmpRouter for Router {
	fn module_for_id(&self, id: Vec<u8>) -> Result<Box<dyn IsmpModule>, anyhow::Error> {
		match id.as_slice() {
			id if TokenGateway::is_token_gateway(id) => Ok(Box::new(TokenGateway::default())),
			_ => Err(Error::ModuleNotFound(id))?,
		}
	}
}

pub struct OwnershipTokenAsset;

impl Inspect<AccountId> for OwnershipTokenAsset {
	type AssetId = u32;
	type Balance = Balance;

	fn total_issuance(asset: Self::AssetId) -> Self::Balance {
		if asset != OwnershipTokenAssetId::get() {
			return 0;
		}
		Ownership::total_issuance()
	}

	fn minimum_balance(asset: Self::AssetId) -> Self::Balance {
		if asset != OwnershipTokenAssetId::get() {
			return 0;
		}
		<Ownership as Currency<AccountId>>::minimum_balance()
	}

	fn total_balance(asset: Self::AssetId, who: &AccountId) -> Self::Balance {
		if asset != OwnershipTokenAssetId::get() {
			return 0;
		}
		<Ownership as Currency<AccountId>>::total_balance(who)
	}

	fn balance(asset: Self::AssetId, who: &AccountId) -> Self::Balance {
		if asset != OwnershipTokenAssetId::get() {
			return 0;
		}
		Ownership::balance(who)
	}

	fn reducible_balance(
		asset: Self::AssetId,
		who: &AccountId,
		preservation: Preservation,
		force: Fortitude,
	) -> Self::Balance {
		if asset != OwnershipTokenAssetId::get() {
			return 0;
		}
		Ownership::reducible_balance(who, preservation, force)
	}

	fn can_deposit(
		asset: Self::AssetId,
		who: &AccountId,
		amount: Self::Balance,
		provenance: Provenance,
	) -> DepositConsequence {
		if asset != OwnershipTokenAssetId::get() {
			return DepositConsequence::UnknownAsset;
		}
		Ownership::can_deposit(who, amount, provenance)
	}

	fn can_withdraw(
		asset: Self::AssetId,
		who: &AccountId,
		amount: Self::Balance,
	) -> WithdrawConsequence<Self::Balance> {
		if asset != OwnershipTokenAssetId::get() {
			return WithdrawConsequence::UnknownAsset;
		}
		Ownership::can_withdraw(who, amount)
	}

	fn asset_exists(asset: Self::AssetId) -> bool {
		asset == OwnershipTokenAssetId::get()
	}
}

impl Unbalanced<AccountId> for OwnershipTokenAsset {
	fn handle_dust(dust: Dust<AccountId, Self>) {
		if dust.0 != OwnershipTokenAssetId::get() {
			return;
		}
		Ownership::handle_dust(fungible::Dust(dust.1))
	}

	fn write_balance(
		asset: Self::AssetId,
		who: &AccountId,
		amount: Self::Balance,
	) -> Result<Option<Self::Balance>, DispatchError> {
		if asset != OwnershipTokenAssetId::get() {
			return Err(DispatchError::Unavailable)?;
		}
		Ownership::write_balance(who, amount)
	}

	fn set_total_issuance(asset: Self::AssetId, amount: Self::Balance) {
		if asset != OwnershipTokenAssetId::get() {
			return;
		}
		Ownership::set_total_issuance(amount)
	}
}

impl fungibles::Mutate<AccountId> for OwnershipTokenAsset {
	fn burn_from(
		asset: Self::AssetId,
		who: &AccountId,
		amount: Self::Balance,
		preservation: Preservation,
		precision: Precision,
		force: Fortitude,
	) -> Result<Self::Balance, DispatchError> {
		if asset != OwnershipTokenAssetId::get() {
			return Err(DispatchError::Unavailable)?;
		}
		Ownership::burn_from(who, amount, preservation, precision, force)
	}
}
impl fungibles::Create<AccountId> for OwnershipTokenAsset {
	fn create(
		_id: Self::AssetId,
		_admin: AccountId,
		_is_sufficient: bool,
		_min_balance: Self::Balance,
	) -> DispatchResult {
		Err(DispatchError::Unavailable)?
	}
}

impl fungibles::metadata::Inspect<AccountId> for OwnershipTokenAsset {
	fn name(asset: Self::AssetId) -> Vec<u8> {
		if asset != OwnershipTokenAssetId::get() {
			return Vec::new();
		}
		b"Argon Ownership Token".to_vec()
	}

	fn symbol(asset: Self::AssetId) -> Vec<u8> {
		if asset != OwnershipTokenAssetId::get() {
			return Vec::new();
		}
		b"ARGONOT".to_vec()
	}

	fn decimals(asset: Self::AssetId) -> u8 {
		if asset != OwnershipTokenAssetId::get() {
			return 0;
		}
		Decimals::get()
	}
}

impl fungibles::metadata::Mutate<AccountId> for OwnershipTokenAsset {
	fn set(
		_asset: Self::AssetId,
		_from: &AccountId,
		_name: Vec<u8>,
		_symbol: Vec<u8>,
		_decimals: u8,
	) -> frame_support::dispatch::DispatchResult {
		Err(DispatchError::Unavailable)?
	}
}

impl fungibles::roles::Inspect<AccountId> for OwnershipTokenAsset {
	fn owner(_asset: Self::AssetId) -> Option<AccountId> {
		None
	}

	fn issuer(_asset: Self::AssetId) -> Option<AccountId> {
		None
	}

	fn admin(_asset: Self::AssetId) -> Option<AccountId> {
		Some(ChainTransfer::hyperbridge_token_admin())
	}

	fn freezer(_asset: Self::AssetId) -> Option<AccountId> {
		None
	}
}
