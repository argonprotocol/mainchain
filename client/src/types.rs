use argon_primitives::ADDRESS_PREFIX;
use codec::{Decode, Encode};
use core::{
	fmt,
	fmt::{Debug, Display, LowerHex, UpperHex},
	hash,
	hash::Hash,
};
use polkadot_sdk::*;
use scale_info::TypeInfo;
use sp_core::crypto::{AccountId32 as CoreAccountId32, PublicError, Ss58Codec};
use subxt::ext::{scale_decode::DecodeAsType, scale_encode::EncodeAsType};

#[derive(Clone, Copy, PartialEq, Eq, Decode, Encode, DecodeAsType, EncodeAsType, TypeInfo)]
#[decode_as_type(crate_path = ":: subxt :: ext :: subxt_core :: ext :: scale_decode")]
#[encode_as_type(crate_path = ":: subxt :: ext :: subxt_core :: ext :: scale_encode")]
pub struct H256(pub [u8; 32]);

impl Debug for H256 {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "{self:#x}")
	}
}

impl Display for H256 {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "0x")?;
		for i in &self.0[0..2] {
			write!(f, "{i:02x}")?;
		}
		write!(f, "…")?;
		for i in &self.0[30..32] {
			write!(f, "{i:02x}")?;
		}
		Ok(())
	}
}

impl LowerHex for H256 {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		if f.alternate() {
			write!(f, "0x")?;
		}
		for i in &self.0 {
			write!(f, "{i:02x}")?;
		}
		Ok(())
	}
}

impl UpperHex for H256 {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		if f.alternate() {
			write!(f, "0X")?;
		}
		for i in &self.0 {
			write!(f, "{i:02X}")?;
		}
		Ok(())
	}
}

impl From<Vec<u8>> for H256 {
	fn from(value: Vec<u8>) -> Self {
		let mut result = [0u8; 32];
		result[..value.len()].copy_from_slice(&value);
		Self(result)
	}
}

impl Hash for H256 {
	fn hash<H>(&self, state: &mut H)
	where
		H: hash::Hasher,
	{
		state.write(&self.0);
	}
}
impl AsRef<[u8]> for H256 {
	fn as_ref(&self) -> &[u8] {
		&self.0
	}
}

impl From<sp_core::H256> for H256 {
	fn from(value: sp_core::H256) -> Self {
		Self(value.0)
	}
}
impl From<H256> for sp_core::H256 {
	fn from(value: H256) -> Self {
		Self(value.0)
	}
}

#[derive(Debug, Clone, PartialEq, Eq, Decode, Encode, EncodeAsType, DecodeAsType, TypeInfo)]
#[decode_as_type(crate_path = ":: subxt :: ext :: subxt_core :: ext :: scale_decode")]
#[encode_as_type(crate_path = ":: subxt :: ext :: subxt_core :: ext :: scale_encode")]
pub struct AccountId32(pub [u8; 32]);

impl AsRef<[u8; 32]> for AccountId32 {
	fn as_ref(&self) -> &[u8; 32] {
		&self.0
	}
}

impl AccountId32 {
	pub fn to_address(&self) -> String {
		CoreAccountId32::from(self.0).to_ss58check_with_version(ADDRESS_PREFIX.into())
	}

	pub fn from_address(address: &str) -> Result<Self, PublicError> {
		CoreAccountId32::from_ss58check(address).map(|x| AccountId32(x.into()))
	}
}

impl core::fmt::Display for AccountId32 {
	fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
		write!(f, "{}", self.to_address())
	}
}

impl core::str::FromStr for AccountId32 {
	type Err = PublicError;
	fn from_str(s: &str) -> Result<Self, Self::Err> {
		Self::from_address(s)
	}
}

impl From<sp_core::sr25519::Public> for AccountId32 {
	fn from(value: sp_core::sr25519::Public) -> Self {
		let acc: CoreAccountId32 = value.into();
		acc.into()
	}
}
impl From<sp_core::ed25519::Public> for AccountId32 {
	fn from(value: sp_core::ed25519::Public) -> Self {
		let acc: CoreAccountId32 = value.into();
		acc.into()
	}
}
impl From<[u8; 32]> for AccountId32 {
	fn from(x: [u8; 32]) -> Self {
		AccountId32(x)
	}
}

impl From<CoreAccountId32> for AccountId32 {
	fn from(value: CoreAccountId32) -> Self {
		Self(value.into())
	}
}
impl From<&CoreAccountId32> for AccountId32 {
	fn from(value: &CoreAccountId32) -> Self {
		Self(value.clone().into())
	}
}
impl From<AccountId32> for CoreAccountId32 {
	fn from(value: AccountId32) -> Self {
		CoreAccountId32::from(value.0)
	}
}
