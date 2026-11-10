use super::mock::{account, externalities, signed};
use crate::{
	Address, Balances, Executive, RuntimeCall, RuntimeOrigin, Signature, SignedPayload, System,
	TxExtension, UncheckedExtrinsic, ARGON,
};
use argon_primitives::{
	crypto::{P256Public, P256Signature},
	MultiSigner,
};
use codec::{Decode, Encode};
use p256::ecdsa::{signature::Signer, Signature as DeviceSignature, SigningKey};
use polkadot_sdk::{
	frame_support::assert_ok,
	pallet_balances,
	sp_runtime::{
		generic::{Preamble, UncheckedExtrinsic as SdkExtrinsic},
		traits::IdentifyAccount,
	},
};

#[test]
fn native_p256_transactions_keep_sdk_encoding_and_dispatch_normally() {
	externalities().execute_with(|| {
		let key = SigningKey::from_bytes((&[0x71; 32]).into()).unwrap();
		let public =
			P256Public::from_sec1_bytes(key.verifying_key().to_encoded_point(true).as_bytes())
				.unwrap();
		let device = MultiSigner::P256(public).into_account();
		assert_ok!(Balances::force_set_balance(
			RuntimeOrigin::root(),
			device.clone().into(),
			100 * ARGON,
		));

		let call = RuntimeCall::Balances(pallet_balances::Call::transfer_keep_alive {
			dest: account(4).into(),
			value: ARGON,
		});
		let Preamble::Signed(_, _, extra) = signed(1, call.clone()).into_inner().preamble else {
			unreachable!()
		};
		let payload = SignedPayload::new(call.clone(), extra.clone()).unwrap();
		let signature = payload.using_encoded(|bytes| {
			let signature: DeviceSignature = key.sign(bytes);
			Signature::P256(
				P256Signature::from_device_signature(bytes, &signature.to_bytes().into(), &public)
					.unwrap(),
			)
		});
		let sdk = SdkExtrinsic::<Address, RuntimeCall, Signature, TxExtension>::new_signed(
			call,
			device.clone().into(),
			signature,
			extra,
		);
		let tx: UncheckedExtrinsic = sdk.clone().into();
		assert_eq!(tx.encode(), sdk.encode());
		let decoded = UncheckedExtrinsic::decode(&mut tx.encode().as_slice()).unwrap();

		let recipient_before = Balances::free_balance(account(4));
		assert_ok!(Executive::apply_extrinsic(decoded).unwrap());
		assert_eq!(System::account_nonce(&device), 1);
		assert_eq!(Balances::free_balance(account(4)), recipient_before + ARGON);
	});
}
