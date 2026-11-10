use crate::{self as pallet_account_security, *};
use argon_primitives::Signature;
use frame_metadata_hash_extension::CheckMetadataHash;
use frame_support::{derive_impl, traits::InstanceFilter};
use sp_core::{hashing::blake2_256, sr25519, Pair};
use sp_runtime::{
	generic,
	traits::{ConstU128, IdentityLookup},
	AccountId32,
};

pub type AccountId = AccountId32;
type SignedExtra =
	(frame_system::CheckNonce<Test>, CheckAccountSecurity<Test>, CheckMetadataHash<Test>);
pub type UncheckedExtrinsic =
	generic::UncheckedExtrinsic<AccountId, RuntimeCall, Signature, SignedExtra>;
type Block = generic::Block<generic::Header<u64, BlakeTwo256>, UncheckedExtrinsic>;
pub type Executive = frame_executive::Executive<
	Test,
	Block,
	frame_system::ChainContext<Test>,
	Test,
	AllPalletsWithSystem,
>;
pub const ARGON: u128 = 1_000_000;

frame_support::construct_runtime!(
	pub enum Test {
		System: frame_system,
		Balances: pallet_balances,
		Proxy: pallet_proxy,
		Multisig: pallet_multisig,
		Utility: pallet_utility,
		AccountSecurity: pallet_account_security,
	}
);

#[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
impl frame_system::Config for Test {
	type AccountId = AccountId;
	type Lookup = IdentityLookup<AccountId>;
	type Block = Block;
	type AccountData = pallet_balances::AccountData<u128>;
}

#[derive_impl(pallet_balances::config_preludes::TestDefaultConfig)]
impl pallet_balances::Config for Test {
	type Balance = u128;
	type AccountStore = System;
	type ExistentialDeposit = ConstU128<1>;
}

#[derive(
	Clone,
	Copy,
	Debug,
	Default,
	Encode,
	Decode,
	DecodeWithMemTracking,
	Eq,
	PartialEq,
	Ord,
	PartialOrd,
	TypeInfo,
	MaxEncodedLen,
)]
pub enum ProxyType {
	#[default]
	Any,
	Mining,
}

impl InstanceFilter<RuntimeCall> for ProxyType {
	fn filter(&self, _: &RuntimeCall) -> bool {
		*self == Self::Any
	}
	fn is_superset(&self, other: &Self) -> bool {
		self == other || *self == Self::Any
	}
}

impl pallet_proxy::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type Currency = Balances;
	type ProxyType = ProxyType;
	type ProxyDepositBase = ConstU128<1>;
	type ProxyDepositFactor = ConstU128<1>;
	type MaxProxies = ConstU32<8>;
	type WeightInfo = ();
	type MaxPending = ConstU32<8>;
	type CallHasher = BlakeTwo256;
	type AnnouncementDepositBase = ConstU128<1>;
	type AnnouncementDepositFactor = ConstU128<1>;
	type BlockNumberProvider = System;
}

impl pallet_multisig::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type Currency = Balances;
	type DepositBase = ConstU128<1>;
	type DepositFactor = ConstU128<1>;
	type MaxSignatories = ConstU32<8>;
	type WeightInfo = ();
	type BlockNumberProvider = System;
}

impl pallet_utility::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type PalletsOrigin = OriginCaller;
	type WeightInfo = ();
}

impl Config for Test {
	type CurrentTick = CurrentTick;
	type CallAuthorizationPolicy = TestPolicy;
	type Signature = Signature;
	type SigningPayload = SigningPayload;
	type MaxCallDepth = ConstU32<16>;
	type MaxCalls = ConstU32<128>;
	type MaxSigners = ConstU32<8>;
	type MaxApprovals = ConstU32<26>;
	type MaxApprovalLifetime = ConstU64<100>;
	type SecurityCurrency = Balances;
	type DepositBase = SecurityDepositBase;
	type DepositPerSigner = ConstU128<2>;
	type WeightInfo = ();
}

pub struct SigningPayload;

parameter_types! {
	pub static CurrentTick: Tick = 0;
	pub static SecurityDepositBase: u128 = 10;
}

impl ApprovalPayloadProvider<RuntimeCall> for SigningPayload {
	fn signing_payload(
		call: RuntimeCall,
		with_metadata_hash: bool,
	) -> Result<Vec<u8>, TransactionValidityError> {
		let metadata = if with_metadata_hash {
			CheckMetadataHash::<Test>::new_with_custom_hash([0x77; 32])
		} else {
			CheckMetadataHash::<Test>::new(false)
		};
		let extra = (
			frame_system::CheckNonce::<Test>::from(0),
			CheckAccountSecurity::<Test>::new(),
			metadata,
		);
		generic::SignedPayload::new(call, extra)
			.map(|payload| payload.using_encoded(|bytes| bytes.to_vec()))
	}
}

/// A minimal policy marks native balance movement and proxy changes as sensitive.
/// Argon's business rules and linked-account lookups are tested against the real runtimes.
pub struct TestPolicy;

impl AccountSecurityPolicy<Test> for TestPolicy {
	fn inspection_weight(_: &RuntimeCall) -> Weight {
		Weight::zero()
	}

	fn requirements(
		call: &RuntimeCall,
		context: &AuthorizationContext<Test>,
	) -> Result<Vec<CallRequirement<AccountId>>, Error<Test>> {
		use CallRequirement::{ChangesAuthorization, RequireApproval};
		let account = context.account.clone();
		Ok(match call {
			RuntimeCall::Balances(
				pallet_balances::Call::transfer_allow_death { .. } |
				pallet_balances::Call::transfer_keep_alive { .. } |
				pallet_balances::Call::transfer_all { .. } |
				pallet_balances::Call::burn { .. },
			) => vec![RequireApproval(account)],
			RuntimeCall::Proxy(
				pallet_proxy::Call::add_proxy { .. } |
				pallet_proxy::Call::remove_proxy { .. } |
				pallet_proxy::Call::remove_proxies { .. },
			) => vec![RequireApproval(account), ChangesAuthorization],
			_ => vec![],
		})
	}

	fn default_account(account: &AccountId) -> AccountId {
		account.clone()
	}

	#[cfg(feature = "runtime-benchmarks")]
	fn prepare_benchmark_account(_: &AccountId, _: &AccountId) {}
}

pub fn externalities() -> TestState {
	let mut ext = new_test_with_genesis::<Test>(|_| {});
	ext.execute_with(|| {
		CurrentTick::set(0);
		SecurityDepositBase::set(10);
		System::set_block_number(1);
		for seed in 1..=8 {
			assert_ok!(Balances::force_set_balance(
				RuntimeOrigin::root(),
				account(seed),
				100 * ARGON
			));
		}
	});
	ext
}

pub fn signed(seed: u8, call: RuntimeCall) -> UncheckedExtrinsic {
	let pair = sr25519::Pair::from_seed(&[seed; 32]);
	let signer = account(seed);
	let extra = (
		frame_system::CheckNonce::<Test>::from(System::account_nonce(&signer)),
		CheckAccountSecurity::<Test>::new(),
		CheckMetadataHash::<Test>::new(false),
	);
	let payload = generic::SignedPayload::new(call.clone(), extra.clone()).unwrap();
	let signature = payload.using_encoded(|bytes| pair.sign(bytes));
	UncheckedExtrinsic::new_signed(call, signer, signature.into(), extra)
}

pub fn account(seed: u8) -> AccountId {
	sr25519::Pair::from_seed(&[seed; 32]).public().0.into()
}

pub fn policy(seeds: &[u8], quorum: u16, recovery: u8) -> Policy<Test> {
	let mut signers: Vec<_> = seeds.iter().map(|seed| account(*seed)).collect();
	signers.sort();
	Policy {
		signers: signers.try_into().unwrap(),
		quorum,
		recovery_account: account(recovery),
		recovery_delay: 0,
	}
}

pub fn authorization(account: &AccountId) -> Authorization<Test> {
	let group = TestPolicy::default_account(account);
	let policy = Registrations::<Test>::get(&group).map(|registration| registration.policy);
	Authorization {
		nonce: NextNonce::<Test>::get(&group),
		group,
		expires_at: 100,
		policy_hash: policy.using_encoded(blake2_256),
	}
}

pub fn enrollment(exit: u8, recovery: u8) -> Policy<Test> {
	policy(&[exit], 1, recovery)
}

pub fn registration_call(main: u8, policy: Policy<Test>) -> RuntimeCall {
	RuntimeCall::AccountSecurity(Call::register {
		account: account(main),
		signers: policy.signers,
		quorum: policy.quorum,
		recovery_account: policy.recovery_account,
		recovery_delay: policy.recovery_delay,
		authorization: authorization(&account(main)),
		approvals: Default::default(),
	})
}

pub fn update_call(main: u8, policy: Policy<Test>) -> RuntimeCall {
	RuntimeCall::AccountSecurity(Call::update_policy {
		account: account(main),
		signers: policy.signers,
		quorum: policy.quorum,
		recovery_account: policy.recovery_account,
		recovery_delay: policy.recovery_delay,
		authorization: authorization(&account(main)),
		approvals: Default::default(),
	})
}

pub fn execution_call(main: u8, call: RuntimeCall) -> RuntimeCall {
	RuntimeCall::AccountSecurity(Call::execute {
		account: account(main),
		call: Box::new(call),
		authorization: authorization(&account(main)),
		approvals: Default::default(),
	})
}

pub fn cancel_call(main: u8, update_nonce: u64) -> RuntimeCall {
	RuntimeCall::AccountSecurity(Call::cancel_policy_update {
		account: account(main),
		update_nonce,
		authorization: authorization(&account(main)),
		approvals: Default::default(),
	})
}

pub fn sign_call(mut call: RuntimeCall, seeds: &[u8]) -> RuntimeCall {
	let payload = AccountSecurity::approval_payload(&call, false).unwrap();
	let signatures = seeds
		.iter()
		.map(|seed| {
			let pair = sr25519::Pair::from_seed(&[*seed; 32]);
			Approval {
				signer: account(*seed),
				signature: pair.sign(&payload).into(),
				with_metadata_hash: false,
			}
		})
		.collect::<Vec<_>>()
		.try_into()
		.unwrap();
	let RuntimeCall::AccountSecurity(security) = &mut call else {
		panic!("expected security call")
	};
	match security {
		Call::register { approvals, .. } |
		Call::update_policy { approvals, .. } |
		Call::cancel_policy_update { approvals, .. } |
		Call::execute { approvals, .. } => *approvals = signatures,
		_ => panic!("expected security entrypoint"),
	}
	call
}

pub fn enroll(main: u8, exit: u8, recovery: u8) {
	let call = sign_call(registration_call(main, enrollment(exit, recovery)), &[exit, recovery]);
	assert_ok!(Executive::apply_extrinsic(signed(main, call)).unwrap());
}

pub fn register_proxy(real: AccountId, delegate: AccountId, proxy_type: ProxyType, delay: u64) {
	assert_ok!(Proxy::add_proxy(RuntimeOrigin::signed(real), delegate, proxy_type, delay));
}

pub fn transfer() -> RuntimeCall {
	RuntimeCall::Balances(pallet_balances::Call::transfer_keep_alive {
		dest: account(4),
		value: ARGON,
	})
}
