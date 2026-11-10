use pallet_prelude::Weight;

/// Measured management, proof verification and authorization traversal costs.
pub trait WeightInfo {
	fn register() -> Weight;
	fn update_policy() -> Weight;
	fn schedule_policy_update() -> Weight;
	fn complete_policy_update() -> Weight;
	fn cancel_policy_update() -> Weight;
	fn execute(b: u32) -> Weight;
	fn verify_ed25519(a: u32) -> Weight;
	fn verify_sr25519(a: u32) -> Weight;
	fn verify_ecdsa(a: u32) -> Weight;
	fn verify_eth(a: u32) -> Weight;
	fn verify_p256(a: u32) -> Weight;
	fn native_p256() -> Weight;
	fn authorization_leaf() -> Weight;
	fn authorization_protection() -> Weight;
	fn authorization_multisig(s: u32, b: u32) -> Weight;
	fn authorization_multisig_derivation(s: u32) -> Weight;
	fn authorization_batch() -> Weight;
	fn authorization_derivative() -> Weight;
	fn on_operational_registration() -> Weight;
	fn on_operational_registration_existing() -> Weight;
}

impl WeightInfo for () {
	fn schedule_policy_update() -> Weight {
		Weight::zero()
	}
	fn complete_policy_update() -> Weight {
		Weight::zero()
	}
	fn cancel_policy_update() -> Weight {
		Weight::zero()
	}
	fn register() -> Weight {
		Weight::zero()
	}
	fn update_policy() -> Weight {
		Weight::zero()
	}
	fn execute(_b: u32) -> Weight {
		Weight::zero()
	}
	fn verify_ed25519(_a: u32) -> Weight {
		Weight::zero()
	}
	fn verify_sr25519(_a: u32) -> Weight {
		Weight::zero()
	}
	fn verify_ecdsa(_a: u32) -> Weight {
		Weight::zero()
	}
	fn verify_eth(_a: u32) -> Weight {
		Weight::zero()
	}
	fn verify_p256(_a: u32) -> Weight {
		Weight::zero()
	}
	fn native_p256() -> Weight {
		Weight::zero()
	}
	fn authorization_leaf() -> Weight {
		Weight::zero()
	}
	fn authorization_protection() -> Weight {
		Weight::zero()
	}
	fn authorization_multisig(_s: u32, _b: u32) -> Weight {
		Weight::zero()
	}
	fn authorization_multisig_derivation(_s: u32) -> Weight {
		Weight::zero()
	}
	fn authorization_batch() -> Weight {
		Weight::zero()
	}
	fn authorization_derivative() -> Weight {
		Weight::zero()
	}
	fn on_operational_registration() -> Weight {
		Weight::zero()
	}
	fn on_operational_registration_existing() -> Weight {
		Weight::zero()
	}
}
