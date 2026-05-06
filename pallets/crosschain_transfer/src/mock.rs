use crate as pallet_crosschain_transfer;
use argon_primitives::{
	EthereumLog, EthereumProof, EthereumVerifyError, EthereumVerifyProvider,
	OperationalAccountsHook,
};
use frame_support::traits::StorageMapShim;
use pallet_prelude::*;
use sp_runtime::{traits::AccountIdConversion, AccountId32};

const LEGACY_TOKEN_GATEWAY_PALLET_ID: [u8; 8] = [0xa0, 0x9b, 0x1c, 0x60, 0xe8, 0x65, 0x02, 0x45];

type Block = frame_system::mocking::MockBlock<Test>;
pub type TestAccountId = AccountId32;
type ArgonToken = pallet_balances::Instance1;
type OwnershipToken = pallet_balances::Instance2;

frame_support::construct_runtime!(
	pub enum Test {
		System: frame_system,
		Balances: pallet_balances::<Instance1>::{Pallet, Call, Storage, Config<T>, Event<T>},
		Ownership: pallet_balances::<Instance2>::{Pallet, Call, Storage, Config<T>, Event<T>},
		CrosschainTransfer: pallet_crosschain_transfer,
	}
);

#[derive_impl(frame_system::config_preludes::TestDefaultConfig as frame_system::DefaultConfig)]
impl frame_system::Config for Test {
	type AccountId = TestAccountId;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Block = Block;
	type AccountData = pallet_balances::AccountData<Balance>;
	type DbWeight = RocksDbWeight;
}

parameter_types! {
	pub const ExistentialDeposit: Balance = 1;
	pub const OwnershipExistentialDeposit: Balance = 1;
	pub const CrosschainTransferPalletId: PalletId = PalletId(*b"xchaintr");
	pub CrosschainTransferEthereumBurnAccount: TestAccountId = CrosschainTransferPalletId::get()
		.into_sub_account_truncating((crate::SourceChain::Ethereum, *b"burn"));
	pub const RecentTransferRetentionTicks: Tick = 5;
	pub static CurrentTick: Tick = 0;
	pub static ProofVerificationAllowed: bool = true;
	pub static ConfirmedTransfers: Vec<(TestAccountId, Balance)> = Vec::new();
}

impl pallet_balances::Config<ArgonToken> for Test {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeHoldReason = RuntimeHoldReason;
	type RuntimeFreezeReason = RuntimeFreezeReason;
	type WeightInfo = ();
	type Balance = Balance;
	type DustRemoval = ();
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = System;
	type ReserveIdentifier = [u8; 8];
	type FreezeIdentifier = ();
	type MaxLocks = ();
	type MaxReserves = ();
	type MaxFreezes = ();
	type DoneSlashHandler = ();
}

impl pallet_balances::Config<OwnershipToken> for Test {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeHoldReason = RuntimeHoldReason;
	type RuntimeFreezeReason = RuntimeFreezeReason;
	type WeightInfo = ();
	type Balance = Balance;
	type DustRemoval = ();
	type ExistentialDeposit = OwnershipExistentialDeposit;
	type AccountStore = StorageMapShim<
		pallet_balances::Account<Test, OwnershipToken>,
		TestAccountId,
		pallet_balances::AccountData<Balance>,
	>;
	type ReserveIdentifier = [u8; 8];
	type FreezeIdentifier = ();
	type MaxLocks = ();
	type MaxReserves = ();
	type MaxFreezes = ();
	type DoneSlashHandler = ();
}

pub struct MockEthereumVerifier;
impl EthereumVerifyProvider for MockEthereumVerifier {
	type Weights = ();

	fn verify_event_log(
		_event_log: &EthereumLog,
		_proof: &EthereumProof,
	) -> Result<(), EthereumVerifyError> {
		if ProofVerificationAllowed::get() {
			Ok(())
		} else {
			Err(EthereumVerifyError::InvalidProof)
		}
	}
}

pub struct MockOperationalAccountsHook;
impl OperationalAccountsHook<TestAccountId, Balance> for MockOperationalAccountsHook {
	fn vault_created_weight() -> Weight {
		Weight::zero()
	}

	fn bitcoin_lock_funded_weight() -> Weight {
		Weight::zero()
	}

	fn mining_seat_won_weight() -> Weight {
		Weight::zero()
	}

	fn treasury_pool_participated_weight() -> Weight {
		Weight::zero()
	}

	fn uniswap_transfer_confirmed_weight() -> Weight {
		Weight::zero()
	}

	fn uniswap_transfer_confirmed(account_id: &TestAccountId, amount: Balance) {
		ConfirmedTransfers::mutate(|confirmed| confirmed.push((account_id.clone(), amount)));
	}
}

impl pallet_crosschain_transfer::Config for Test {
	type Balance = Balance;
	type EthereumBurnAccount = CrosschainTransferEthereumBurnAccount;
	type NativeCurrency = Balances;
	type OwnershipCurrency = Ownership;
	type EthereumVerifier = MockEthereumVerifier;
	type OperationalAccountsHook = MockOperationalAccountsHook;
	type CurrentTick = CurrentTick;
	type RecentTransferRetentionTicks = RecentTransferRetentionTicks;
	type WeightInfo = ();
}

pub fn new_test_ext() -> TestState {
	new_test_with_genesis::<Test>(|t: &mut Storage| {
		pallet_balances::GenesisConfig::<Test, ArgonToken> {
			balances: vec![(account(1), 1_000_000), (legacy_token_gateway_account(), 200_000_000)],
			dev_accounts: None,
		}
		.assimilate_storage(t)
		.unwrap();

		pallet_balances::GenesisConfig::<Test, OwnershipToken> {
			balances: vec![(legacy_token_gateway_account(), 500_000)],
			dev_accounts: None,
		}
		.assimilate_storage(t)
		.unwrap();
	})
}

pub fn account(byte: u8) -> TestAccountId {
	AccountId32::new([byte; 32])
}

pub fn h160(byte: u8) -> H160 {
	H160::repeat_byte(byte)
}

pub fn legacy_token_gateway_account() -> TestAccountId {
	PalletId(LEGACY_TOKEN_GATEWAY_PALLET_ID).into_account_truncating()
}
