use pallet_prelude::*;

use super::*;
use crate as pallet_fee_control;
use frame_support::{derive_impl, parameter_types, traits::InstanceFilter};
use pallet_prelude::{
	frame_support::traits::{Imbalance, OnUnbalanced},
	pallet_balances::AccountData,
};
use polkadot_sdk::{
	frame_support::{traits::fungible, weights::IdentityFee},
	pallet_balances::Instance1,
	sp_core::{ConstU8, ConstU128},
};
use sp_runtime::{
	traits::{DispatchOriginOf, TransactionExtension},
	transaction_validity::ValidTransaction,
};

type Block = frame_system::mocking::MockBlock<Test>;

// Configure a mock Test to test the pallet.
frame_support::construct_runtime!(
	pub enum Test
	{
		System: frame_system,
		Balances: pallet_balances::<Instance1>::{Pallet, Call, Storage, Config<T>, Event<T>},
		TransactionPayment: pallet_transaction_payment,
		Proxy: pallet_proxy,
		FeeControl: pallet_fee_control,
		DummyPallet: pallet_dummy,
	}
);

#[derive_impl(frame_system::config_preludes::TestDefaultConfig as frame_system::DefaultConfig)]
impl frame_system::Config for Test {
	type Block = Block;
	type DbWeight = RocksDbWeight;
	type AccountData = AccountData<Balance>;
}

impl pallet_fee_control::Config for Test {
	type TransactionSponsorProviders = DummyPallet;
	type Balance = Balance;
	type FeelessCallTxPoolKeyProviders = DummyPallet;
}
parameter_types! {
	pub(crate) static TipUnbalancedAmount: Balance = 0;
	pub(crate) static FeeUnbalancedAmount: Balance = 0;
}

pub struct DealWithFees;
impl OnUnbalanced<fungible::Credit<<Test as frame_system::Config>::AccountId, Balances>>
	for DealWithFees
{
	fn on_unbalanceds(
		mut fees_then_tips: impl Iterator<
			Item = fungible::Credit<<Test as frame_system::Config>::AccountId, Balances>,
		>,
	) {
		if let Some(fees) = fees_then_tips.next() {
			FeeUnbalancedAmount::mutate(|a| *a += fees.peek());
			if let Some(tips) = fees_then_tips.next() {
				TipUnbalancedAmount::mutate(|a| *a += tips.peek());
			}
		}
	}
}
#[derive(
	Copy,
	Clone,
	Eq,
	PartialEq,
	Ord,
	PartialOrd,
	TypeInfo,
	Encode,
	Decode,
	DecodeWithMemTracking,
	RuntimeDebug,
	MaxEncodedLen,
)]
pub enum ProxyType {
	Any,
	DummyWrapper,
}
impl Default for ProxyType {
	fn default() -> Self {
		Self::Any
	}
}
impl InstanceFilter<RuntimeCall> for ProxyType {
	fn filter(&self, c: &RuntimeCall) -> bool {
		match self {
			ProxyType::Any => true,
			ProxyType::DummyWrapper =>
				matches!(c, RuntimeCall::DummyPallet(pallet_dummy::Call::sponsored { .. })),
		}
	}
	fn is_superset(&self, o: &Self) -> bool {
		match (self, o) {
			(x, y) if x == y => true,
			(ProxyType::Any, _) => true,
			(_, ProxyType::Any) => false,
			_ => false,
		}
	}
}
parameter_types! {
	pub const MaxProxies: u16 = 32;
	pub const MaxPending: u16 = 32;
	pub const ProxyDepositBase: Balance = deposit(1, 40);
	// Additional storage item size of 33 bytes.
	pub const ProxyDepositFactor: Balance = deposit(0, 33);
	// One storage item; key size 32, value size 16
	pub const AnnouncementDepositBase: Balance = deposit(1, 48);
	pub const AnnouncementDepositFactor: Balance = deposit(0, 66);
}
const fn deposit(items: u32, bytes: u32) -> Balance {
	items as Balance * 100 * 100 + (bytes as Balance) * 5
}
impl pallet_proxy::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type Currency = Balances;
	type ProxyType = ProxyType;
	type ProxyDepositBase = ProxyDepositBase;
	type ProxyDepositFactor = ProxyDepositFactor;
	type MaxProxies = MaxProxies;
	type WeightInfo = pallet_proxy::weights::SubstrateWeight<Test>;
	type MaxPending = MaxPending;
	type CallHasher = BlakeTwo256;
	type AnnouncementDepositBase = AnnouncementDepositBase;
	type AnnouncementDepositFactor = AnnouncementDepositFactor;
	type BlockNumberProvider = frame_system::Pallet<Test>;
}

impl pallet_transaction_payment::Config for Test {
	type OnChargeTransaction = pallet_transaction_payment::FungibleAdapter<Balances, DealWithFees>;
	type OperationalFeeMultiplier = ConstU8<5>;
	type WeightToFee = IdentityFee<Balance>;
	type LengthToFee = IdentityFee<Balance>;
	type FeeMultiplierUpdate = ();
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = ();
}

impl pallet_balances::Config<Instance1> for Test {
	type Balance = Balance;
	type RuntimeEvent = RuntimeEvent;
	type DustRemoval = ();
	type ExistentialDeposit = ConstU128<10_000>;
	type AccountStore = System;
	type MaxLocks = ();
	type WeightInfo = ();
	type DoneSlashHandler = ();
	type ReserveIdentifier = [u8; 8];
	type FreezeIdentifier = ();
	type MaxFreezes = ();
	type MaxReserves = ();
	type RuntimeFreezeReason = RuntimeFreezeReason;
	type RuntimeHoldReason = RuntimeHoldReason;
}

parameter_types! {
	pub static PrepareCount: u32 = 0;
	pub static ValidateCount: u32 = 0;
	pub static TxId: Vec<u8> = vec![];
	pub static TipAmount: Balance = 500;
	pub static FeeAmount: Balance = 1000;
	pub static LastPayer: Option<u64> = None;
}

#[derive(Clone, Eq, PartialEq, Debug, Encode, Decode, DecodeWithMemTracking, TypeInfo)]
pub struct MockChargePaymentExtension;

impl TransactionExtension<RuntimeCall> for MockChargePaymentExtension {
	const IDENTIFIER: &'static str = "MockChargePaymentExtension";
	type Implicit = ();
	type Val = pallet_transaction_payment::Val<Test>;
	type Pre = pallet_transaction_payment::Pre<Test>;

	fn weight(&self, _: &RuntimeCall) -> Weight {
		Weight::zero()
	}

	fn validate(
		&self,
		origin: DispatchOriginOf<RuntimeCall>,
		_call: &RuntimeCall,
		_info: &DispatchInfoOf<RuntimeCall>,
		_len: usize,
		_self_implicit: Self::Implicit,
		_inherited_implication: &impl Encode,
		_source: TransactionSource,
	) -> ValidateResult<Self::Val, RuntimeCall> {
		ValidateCount::mutate(|c| *c += 1);
		let who = origin.clone().into_signer().unwrap();
		ensure!(
			Balances::free_balance(who) >= TipAmount::get() + FeeAmount::get(),
			InvalidTransaction::Payment
		);
		Ok((
			ValidTransaction::default(),
			pallet_transaction_payment::Val::Charge {
				tip: TipAmount::get(),
				who,
				fee_with_tip: FeeAmount::get().saturating_add(TipAmount::get()),
			},
			origin,
		))
	}

	fn prepare(
		self,
		val: Self::Val,
		_origin: &DispatchOriginOf<RuntimeCall>,
		_call: &RuntimeCall,
		_info: &DispatchInfoOf<RuntimeCall>,
		_len: usize,
	) -> Result<Self::Pre, TransactionValidityError> {
		PrepareCount::mutate(|c| *c += 1);

		match val {
			pallet_transaction_payment::Val::Charge { tip, who, fee_with_tip } => {
				let total_fee_with_tip: Balance = fee_with_tip;
				Balances::burn_from(
					&who,
					total_fee_with_tip,
					Preservation::Expendable,
					Precision::BestEffort,
					Fortitude::Force,
				)
				.unwrap();
				LastPayer::mutate(|c| *c = Some(who));
				let imbalance =
					fungible::Credit::<u64, Balances>::default().split(total_fee_with_tip).1;
				Ok(pallet_transaction_payment::Pre::Charge {
					tip,
					who,
					liquidity_info: Some(imbalance),
				})
			},
			pallet_transaction_payment::Val::NoCharge => {
				LastPayer::set(None);
				Ok(pallet_transaction_payment::Pre::NoCharge { refund: Default::default() })
			},
		}
	}
}

#[frame_support::pallet(dev_mode)]
pub mod pallet_dummy {
	use crate::mock::{RuntimeCall, pallet_dummy};
	use pallet_prelude::{
		argon_primitives::{FeelessCallTxPoolKeyProvider, TransactionSponsorProvider, TxSponsor},
		*,
	};

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::storage]
	pub type OneUseCodes<T> = StorageMap<_, Blake2_128Concat, u32, (u64, Balance), OptionQuery>;

	#[pallet::config]
	pub trait Config: polkadot_sdk::frame_system::Config {}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::feeless_if(|_origin: &OriginFor<T>, is_feeless: &bool, _key: &u32| -> bool {
			*is_feeless
		})]
		pub fn aux(_origin: OriginFor<T>, _is_feeless: bool, _key: u32) -> DispatchResult {
			unreachable!()
		}

		pub fn sponsored(_origin: OriginFor<T>, _key: u32) -> DispatchResult {
			let _who = ensure_signed(_origin)?;
			Ok(())
		}
	}
	impl<T: Config> FeelessCallTxPoolKeyProvider<RuntimeCall> for Pallet<T> {
		fn key_for(call: &RuntimeCall) -> Option<Vec<u8>> {
			if let RuntimeCall::DummyPallet(pallet_dummy::Call::aux { key, .. }) = call {
				Some(key.encode())
			} else {
				None
			}
		}
	}
	impl<T: Config> TransactionSponsorProvider<u64, RuntimeCall, Balance> for Pallet<T> {
		fn get_transaction_sponsor(
			_signer: &u64,
			call: &RuntimeCall,
		) -> Option<TxSponsor<u64, Balance>> {
			if let RuntimeCall::DummyPallet(pallet_dummy::Call::sponsored { key, .. }) = call {
				return OneUseCodes::<T>::get(key).map(|(sponsor, max_fee_with_tip)| TxSponsor {
					payer: sponsor,
					max_fee_with_tip: Some(max_fee_with_tip),
					unique_tx_key: Some(key.encode()),
				});
			}
			None
		}
	}
}

impl pallet_dummy::Config for Test {}

// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> TestState {
	new_test_with_genesis::<Test>(|_t| {})
}
