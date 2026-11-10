//! Keep SDK transaction encoding and verification, but retain the outer signature's cost.

use codec::{Decode, DecodeWithMemTracking, Encode, Input};
use core::{marker::PhantomData, ops::Deref};
use polkadot_sdk::{
	frame_support::{
		dispatch::{DispatchInfo, GetDispatchInfo},
		traits::{InherentBuilder, SignedTransactionBuilder},
	},
	sp_runtime::{
		generic::{Preamble, UncheckedExtrinsic as SdkExtrinsic},
		traits::{Checkable, ExtrinsicCall, ExtrinsicLike, ExtrinsicMetadata, LazyExtrinsic},
		transaction_validity::TransactionValidityError,
		OpaqueExtrinsic,
	},
};
use scale_info::{Type, TypeInfo};

/// Runtime wiring puts the actual signature scheme's cost into a non-encoded extension field.
pub trait NativeSignatureWeight<Signature, Extension> {
	fn set_weight(signature: Option<&Signature>, extension: &mut Extension);
}

/// Transparent SDK extrinsic adapter. No signature verification or dispatch is duplicated here.
/// Construction and decoding normalize the cost; callers cannot mutate the inner preamble.
#[derive(Clone, Debug, PartialEq, Eq, Encode)]
pub struct UncheckedExtrinsic<Address, Call, Signature, Extension, Weights>(
	SdkExtrinsic<Address, Call, Signature, Extension>,
	#[codec(skip)] PhantomData<Weights>,
);

impl<A, C, S, E, W> From<UncheckedExtrinsic<A, C, S, E, W>> for OpaqueExtrinsic
where
	SdkExtrinsic<A, C, S, E>: Into<OpaqueExtrinsic>,
{
	fn from(extrinsic: UncheckedExtrinsic<A, C, S, E, W>) -> Self {
		extrinsic.0.into()
	}
}

impl<A, C, S, E, W: NativeSignatureWeight<S, E>> From<SdkExtrinsic<A, C, S, E>>
	for UncheckedExtrinsic<A, C, S, E, W>
{
	fn from(mut inner: SdkExtrinsic<A, C, S, E>) -> Self {
		match &mut inner.preamble {
			Preamble::Signed(_, signature, extension) => W::set_weight(Some(signature), extension),
			Preamble::General(_, extension) => W::set_weight(None, extension),
			Preamble::Bare(_) => {},
		}
		Self(inner, PhantomData)
	}
}

impl<A, C, S, E, W: NativeSignatureWeight<S, E>> UncheckedExtrinsic<A, C, S, E, W> {
	pub fn new_signed(call: C, address: A, signature: S, extension: E) -> Self {
		SdkExtrinsic::new_signed(call, address, signature, extension).into()
	}

	pub fn new_bare(call: C) -> Self {
		SdkExtrinsic::new_bare(call).into()
	}

	pub fn new_transaction(call: C, extension: E) -> Self {
		SdkExtrinsic::new_transaction(call, extension).into()
	}

	pub fn into_inner(self) -> SdkExtrinsic<A, C, S, E> {
		self.0
	}

	pub fn is_signed(&self) -> bool {
		self.0.is_signed()
	}
}

impl<A, C, S, E, W> Deref for UncheckedExtrinsic<A, C, S, E, W> {
	type Target = SdkExtrinsic<A, C, S, E>;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl<A, C, S, E, W: NativeSignatureWeight<S, E>> Decode for UncheckedExtrinsic<A, C, S, E, W>
where
	SdkExtrinsic<A, C, S, E>: Decode,
{
	fn decode<I: Input>(input: &mut I) -> Result<Self, codec::Error> {
		SdkExtrinsic::decode(input).map(Self::from)
	}
}

impl<A, C, S, E, W: NativeSignatureWeight<S, E>> DecodeWithMemTracking
	for UncheckedExtrinsic<A, C, S, E, W>
where
	SdkExtrinsic<A, C, S, E>: DecodeWithMemTracking,
{
}

impl<A, C, S, E, W: NativeSignatureWeight<S, E>> LazyExtrinsic for UncheckedExtrinsic<A, C, S, E, W>
where
	SdkExtrinsic<A, C, S, E>: LazyExtrinsic,
{
	fn decode_unprefixed(data: &[u8]) -> Result<Self, codec::Error> {
		SdkExtrinsic::decode_unprefixed(data).map(Self::from)
	}
}

impl<A, C, S, E, W: 'static> TypeInfo for UncheckedExtrinsic<A, C, S, E, W>
where
	SdkExtrinsic<A, C, S, E>: TypeInfo + 'static,
{
	type Identity = <SdkExtrinsic<A, C, S, E> as TypeInfo>::Identity;

	fn type_info() -> Type {
		SdkExtrinsic::<A, C, S, E>::type_info()
	}
}

impl<A, C, S, E, W> ExtrinsicLike for UncheckedExtrinsic<A, C, S, E, W> {
	#[allow(deprecated)]
	fn is_signed(&self) -> Option<bool> {
		ExtrinsicLike::is_signed(&self.0)
	}

	fn is_bare(&self) -> bool {
		self.0.is_bare()
	}
}

impl<A, C, S, E, W> ExtrinsicCall for UncheckedExtrinsic<A, C, S, E, W> {
	type Call = C;

	fn call(&self) -> &C {
		self.0.call()
	}

	fn into_call(self) -> C {
		self.0.into_call()
	}
}

impl<A, C, S, E, W: NativeSignatureWeight<S, E>> InherentBuilder
	for UncheckedExtrinsic<A, C, S, E, W>
where
	SdkExtrinsic<A, C, S, E>: InherentBuilder<Call = C>,
{
	fn new_inherent(call: Self::Call) -> Self {
		SdkExtrinsic::new_inherent(call).into()
	}
}

impl<A, C, S, E, W: NativeSignatureWeight<S, E>> SignedTransactionBuilder
	for UncheckedExtrinsic<A, C, S, E, W>
{
	type Address = A;
	type Signature = S;
	type Extension = E;

	fn new_signed_transaction(call: C, address: A, signature: S, extension: E) -> Self {
		Self::new_signed(call, address, signature, extension)
	}
}

impl<A, C, S, E, W> ExtrinsicMetadata for UncheckedExtrinsic<A, C, S, E, W>
where
	SdkExtrinsic<A, C, S, E>: ExtrinsicMetadata,
{
	const VERSIONS: &'static [u8] = <SdkExtrinsic<A, C, S, E> as ExtrinsicMetadata>::VERSIONS;
	type TransactionExtensions = E;
}

impl<A, C, S, E, W, Lookup> Checkable<Lookup> for UncheckedExtrinsic<A, C, S, E, W>
where
	SdkExtrinsic<A, C, S, E>: Checkable<Lookup>,
{
	type Checked = <SdkExtrinsic<A, C, S, E> as Checkable<Lookup>>::Checked;

	fn check(self, lookup: &Lookup) -> Result<Self::Checked, TransactionValidityError> {
		self.0.check(lookup)
	}

	#[cfg(feature = "try-runtime")]
	fn unchecked_into_checked_i_know_what_i_am_doing(
		self,
		lookup: &Lookup,
	) -> Result<Self::Checked, TransactionValidityError> {
		self.0.unchecked_into_checked_i_know_what_i_am_doing(lookup)
	}
}

impl<A, C, S, E, W> GetDispatchInfo for UncheckedExtrinsic<A, C, S, E, W>
where
	SdkExtrinsic<A, C, S, E>: GetDispatchInfo,
{
	fn get_dispatch_info(&self) -> DispatchInfo {
		self.0.get_dispatch_info()
	}
}

impl<A, C, S, E, W> serde::Serialize for UncheckedExtrinsic<A, C, S, E, W>
where
	SdkExtrinsic<A, C, S, E>: serde::Serialize,
{
	fn serialize<Serializer: serde::Serializer>(
		&self,
		serializer: Serializer,
	) -> Result<Serializer::Ok, Serializer::Error> {
		self.0.serialize(serializer)
	}
}

impl<'de, A, C, S, E, W: NativeSignatureWeight<S, E>> serde::Deserialize<'de>
	for UncheckedExtrinsic<A, C, S, E, W>
where
	SdkExtrinsic<A, C, S, E>: serde::Deserialize<'de>,
{
	fn deserialize<Deserializer: serde::Deserializer<'de>>(
		deserializer: Deserializer,
	) -> Result<Self, Deserializer::Error> {
		SdkExtrinsic::deserialize(deserializer).map(Self::from)
	}
}
