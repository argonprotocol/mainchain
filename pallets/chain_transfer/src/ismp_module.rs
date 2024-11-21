use super::*;
use alloc::{string::ToString, vec::Vec};
use alloy_sol_types::SolValue;
use anyhow::anyhow;
use argon_primitives::{ARGON_TOKEN_SYMBOL, OWNERSHIP_TOKEN_SYMBOL, TOKEN_DECIMALS};
use core::fmt::Debug;
use frame_support::{
	pallet_prelude::*,
	traits::{fungible::Mutate, tokens::Preservation},
	PalletId,
};
use ismp::{
	dispatcher::{DispatchPost, DispatchRequest, FeeMetadata, IsmpDispatcher},
	events::Meta,
	module::IsmpModule,
	router::{PostRequest, Request, Response, Timeout},
};
use pallet_hyperbridge::{SubstrateHostParams, VersionedHostParams, PALLET_HYPERBRIDGE};
use sp_core::{hashing::keccak_256, H256, U256};
use sp_runtime::traits::{AccountIdConversion, Zero};
use token_gateway_primitives::{
	token_gateway_id, token_governor_id, GatewayAssetRegistration, GatewayAssetUpdate,
	RemoteERC6160AssetRegistration,
};

pub const ERC6160_DECIMALS: u8 = 18;

// Hyperbridge configuration
pub const ISMP_POLKADOT_PARACHAIN_ID: u32 = 3367;
pub const ISMP_KUSAMA_PARACHAIN_ID: u32 = 3340;
pub const ISMP_PASEO_PARACHAIN_ID: u32 = 4009;

#[derive(Debug, Clone, Encode, Decode, TypeInfo, PartialEq, Eq)]
pub enum Asset {
	Argon,
	OwnershipToken,
}

impl Asset {
	pub fn asset_id(&self) -> H256 {
		keccak_256(self.symbol().as_ref()).into()
	}

	pub fn symbol(&self) -> BoundedVec<u8, ConstU32<20>> {
		let b = match self {
			Asset::Argon => ARGON_TOKEN_SYMBOL,
			Asset::OwnershipToken => OWNERSHIP_TOKEN_SYMBOL,
		};
		BoundedVec::truncate_from(b.as_bytes().to_vec())
	}

	pub fn name(&self) -> BoundedVec<u8, ConstU32<50>> {
		let b = match self {
			Asset::Argon => b"Argon".to_vec(),
			Asset::OwnershipToken => b"Argon Ownership".to_vec(),
		};
		BoundedVec::truncate_from(b)
	}
}
impl TryFrom<Vec<u8>> for Asset {
	type Error = anyhow::Error;

	fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
		let slice = value.as_slice();
		if slice == ARGON_TOKEN_SYMBOL.as_bytes() {
			Ok(Asset::Argon)
		} else if slice == OWNERSHIP_TOKEN_SYMBOL.as_bytes() {
			Ok(Asset::OwnershipToken)
		} else {
			Err(anyhow!("Unknown asset"))
		}
	}
}

/// Destinations: https://docs.hyperbridge.network/developers/evm/contract-addresses
#[derive(
	Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo, Default, PartialOrd, Ord,
)]
pub enum EvmChain {
	#[default]
	Ethereum,
	Base,
	Arbitrum,
	Optimism,
	Gnosis,
	BinanceSmartChain,
	// Polygon,
}

impl EvmChain {
	pub fn get_state_machine(&self, is_test: bool) -> StateMachine {
		let evm_id = if is_test {
			match self {
				EvmChain::Ethereum => 11155111,
				EvmChain::Optimism => 11155420,
				EvmChain::Arbitrum => 421614,
				EvmChain::Base => 84532,
				// EvmChain::Polygon => 80001,
				EvmChain::BinanceSmartChain => 97,
				EvmChain::Gnosis => 10200,
			}
		} else {
			match self {
				EvmChain::Ethereum => 1,
				EvmChain::Arbitrum => 42161,
				EvmChain::Optimism => 10,
				EvmChain::Base => 8453,
				EvmChain::BinanceSmartChain => 56,
				EvmChain::Gnosis => 100,
			}
		};
		StateMachine::Evm(evm_id)
	}

	pub fn try_from(id: u32, is_test: bool) -> Option<EvmChain> {
		if is_test {
			match id {
				11155111 => Some(EvmChain::Ethereum),
				11155420 => Some(EvmChain::Optimism),
				421614 => Some(EvmChain::Arbitrum),
				84532 => Some(EvmChain::Base),
				// 80001 => Some(EvmChain::Polygon),
				97 => Some(EvmChain::BinanceSmartChain),
				10200 => Some(EvmChain::Gnosis),
				_ => None,
			}
		} else {
			match id {
				1 => Some(EvmChain::Ethereum),
				42161 => Some(EvmChain::Arbitrum),
				10 => Some(EvmChain::Optimism),
				8453 => Some(EvmChain::Base),
				56 => Some(EvmChain::BinanceSmartChain),
				100 => Some(EvmChain::Gnosis),
				_ => None,
			}
		}
	}
}

impl<T: Config> Pallet<T>
where
	<T as frame_system::Config>::AccountId: From<[u8; 32]>,
	[u8; 32]: From<<T as frame_system::Config>::AccountId>,
	u128: From<<T as pallet_ismp::Config>::Balance>,
{
	pub fn pallet_account() -> <T as frame_system::Config>::AccountId {
		let mut inner = [0u8; 8];
		inner.copy_from_slice(&token_gateway_id().0[0..8]);
		PalletId(inner).into_account_truncating()
	}

	pub fn hyperbridge_account_id() -> <T as frame_system::Config>::AccountId {
		// have to convert to our version of frame-support
		let pallet_id: PalletId = PalletId(PALLET_HYPERBRIDGE.0);
		pallet_id.into_sub_account_truncating(())
	}

	pub fn is_ismp_module_id(id: &[u8]) -> bool {
		id == token_gateway_id().0
	}

	pub fn charge_hyperbridge_registration_fee(
		who: &<T as frame_system::Config>::AccountId,
	) -> Result<(), DispatchError> {
		let VersionedHostParams::V1(SubstrateHostParams { asset_registration_fee, .. }) =
			pallet_hyperbridge::Pallet::<T>::host_params();

		let asset_registration_fee: u128 = asset_registration_fee.into();

		if asset_registration_fee != Zero::zero() {
			T::Argon::transfer(
				who,
				&Self::hyperbridge_account_id(),
				asset_registration_fee.into(),
				Preservation::Expendable,
			)?;
		}
		Ok(())
	}

	pub fn update_asset_registration(
		registration: RemoteERC6160AssetRegistration,
		who: &<T as frame_system::Config>::AccountId,
		asset: Asset,
	) -> DispatchResult {
		Self::charge_hyperbridge_registration_fee(who)?;

		let dispatcher = <T as Config>::Dispatcher::default();
		let coprocessor = T::Coprocessor::get().ok_or(Error::<T>::CoprocessorNotConfigured)?;

		let body = registration.encode();

		let (added_chains, removed_chains) = match registration {
			RemoteERC6160AssetRegistration::CreateAsset(GatewayAssetRegistration {
				chains,
				..
			}) => (chains, vec![]),
			RemoteERC6160AssetRegistration::UpdateAsset(GatewayAssetUpdate {
				add_chains,
				remove_chains,
				..
			}) => (add_chains.to_vec(), remove_chains.to_vec()),
		};

		let commitment = dispatcher
			.dispatch_request(
				DispatchRequest::Post(DispatchPost {
					dest: coprocessor,
					from: token_gateway_id().0.to_vec(),
					to: token_governor_id(),
					timeout: 0,
					body: body.clone(),
				}),
				FeeMetadata { payer: who.clone(), fee: Default::default() },
			)
			.map_err(|_| Error::<T>::Erc6160RegistrationFailed)?;
		Self::deposit_event(Event::<T>::ERC6160AssetRegistrationDispatched {
			commitment,
			asset: asset.clone(),
			added_chains: BoundedVec::truncate_from(added_chains.clone()),
			removed_chains: BoundedVec::truncate_from(removed_chains.clone()),
		});

		Ok(())
	}

	pub fn dispatch_request(
		body: Body,
		dest: StateMachine,
		sender: <T as frame_system::Config>::AccountId,
		timeout: u64,
		relayer_fee: <T as Config>::Balance,
	) -> Result<H256, Error<T>> {
		let gateway =
			TokenGatewayAddresses::<T>::get(dest).ok_or(Error::<T>::EvmChainNotConfigured)?;
		let dispatch_post = DispatchPost {
			dest,
			from: token_gateway_id().0.to_vec(),
			to: gateway,
			timeout,
			body: {
				// Prefix with the handleIncomingAsset enum variant
				let mut encoded = vec![0];
				encoded.extend_from_slice(&Body::abi_encode(&body));
				encoded
			},
		};

		let metadata = FeeMetadata { payer: sender, fee: relayer_fee };
		let dispatcher = <T as Config>::Dispatcher::default();
		dispatcher
			.dispatch_request(DispatchRequest::Post(dispatch_post), metadata)
			.map_err(|_| Error::<T>::FailedToTransferToEvm)
	}

	pub fn transfer(
		source: &<T as frame_system::Config>::AccountId,
		dest: &<T as frame_system::Config>::AccountId,
		amount: <T as Config>::Balance,
		asset: &Asset,
	) -> Result<<T as Config>::Balance, DispatchError> {
		if asset == &Asset::Argon {
			<T as Config>::Argon::transfer(source, dest, amount, Preservation::Expendable)
		} else {
			<T as Config>::OwnershipTokens::transfer(source, dest, amount, Preservation::Expendable)
		}
	}

	fn get_evm_chain(state_machine: StateMachine) -> Option<EvmChain> {
		match state_machine {
			StateMachine::Evm(id) => EvmChain::try_from(id, UseTestNetworks::<T>::get()),
			_ => None,
		}
	}

	fn convert_amount(
		amount: U256,
		meta: &Meta,
	) -> Result<<T as Config>::Balance, ismp::error::Error> {
		let balance = convert_to_balance(amount).map_err(|_| {
			dispatch_error("Token Gateway: Trying to withdraw Invalid amount", meta)
		})?;
		Ok(balance.into())
	}
}

impl<T: Config> IsmpModule for Pallet<T>
where
	<T as frame_system::Config>::AccountId: From<[u8; 32]>,
	[u8; 32]: From<<T as frame_system::Config>::AccountId>,
	u128: From<<T as pallet_ismp::Config>::Balance>,
{
	fn on_accept(
		&self,
		PostRequest { body, from, source, dest, nonce, .. }: PostRequest,
	) -> Result<(), anyhow::Error> {
		let meta = Meta { source, dest, nonce };
		ensure!(
			from == TokenGatewayAddresses::<T>::get(source).unwrap_or_default().to_vec(),
			dispatch_error("Token Gateway: Unsupported source chain", &meta)
		);

		let evm_chain = Self::get_evm_chain(source)
			.ok_or_else(|| dispatch_error("Token Gateway: Unsupported destination chain", &meta))?;

		let body = Body::abi_decode(&body[1..], true)
			.map_err(|_| dispatch_error("Token Gateway: Failed to decode request body", &meta))?;

		let amount = Self::convert_amount(body.amount_u256(), &meta)?;
		let to: <T as frame_system::Config>::AccountId = body.to.0.into();

		let asset = body.get_asset(&meta)?;

		if PauseBridge::<T>::get() {
			Self::deposit_event(Event::<T>::TransferFromEvmWhilePaused {
				from: H160::from_slice(&body.from.0[12..]),
				to: to.clone(),
				amount,
				evm_chain: evm_chain.clone(),
				asset: asset.clone(),
			});
			return Ok(());
		}

		Self::transfer(&Self::pallet_account(), &to, amount, &asset).map_err(|_| {
			dispatch_error("Token Gateway: Failed to complete asset transfer", &meta)
		})?;

		Self::deposit_event(Event::<T>::TransferFromEvm {
			from: H160::from_slice(&body.from.0[12..]),
			to,
			amount,
			evm_chain,
			asset,
		});
		Ok(())
	}

	fn on_response(&self, _response: Response) -> Result<(), anyhow::Error> {
		Err(anyhow!("Module does not accept responses"))
	}

	fn on_timeout(&self, request: Timeout) -> Result<(), anyhow::Error> {
		match request {
			// NOTE: duplicate checking and timeout checking is handled in ismp pallet and
			// ismp-core before they get here, so we don't do it again
			Timeout::Request(Request::Post(PostRequest { body, source, dest, nonce, .. })) => {
				let meta = Meta { source, dest, nonce };
				let evm_chain = Self::get_evm_chain(source).ok_or_else(|| {
					dispatch_error(
						"Token Gateway: Unsupported source chain - possible forgery",
						&meta,
					)
				})?;

				let body = Body::abi_decode(&body[1..], true).map_err(|_| {
					dispatch_error("Token Gateway: Failed to decode request body", &meta)
				})?;
				let from = body.from.0.into();

				let amount = Self::convert_amount(body.amount_u256(), &meta)?;
				let asset = body.get_asset(&meta)?;
				// Refund the amount
				Self::transfer(&Self::pallet_account(), &from, amount, &asset).map_err(|_| {
					dispatch_error(
						"Token Gateway: Failed to refund timed-out asset transfer",
						&meta,
					)
				})?;

				Pallet::<T>::deposit_event(Event::<T>::TransferToEvmExpired {
					from,
					to: H160::from_slice(&body.to.0[12..]),
					amount,
					evm_chain,
					asset,
				});
			},
			Timeout::Request(Request::Get(get)) => Err(ismp::error::Error::ModuleDispatchError {
				msg: "Tried to timeout unsupported request type".to_string(),
				meta: Meta { source: get.source, dest: get.dest, nonce: get.nonce },
			})?,

			Timeout::Response(response) => Err(ismp::error::Error::ModuleDispatchError {
				msg: "Tried to timeout unsupported request type".to_string(),
				meta: Meta {
					source: response.source_chain(),
					dest: response.dest_chain(),
					nonce: response.nonce(),
				},
			})?,
		}
		Ok(())
	}
}

alloy_sol_macro::sol! {
	#![sol(all_derives)]
	struct Body {
		// Amount of the asset to be sent
		uint256 amount;
		// The asset identifier
		bytes32 asset_id;
		// Flag to redeem the erc20 asset on the destination
		bool redeem;
		// Sender address
		bytes32 from;
		// Recipient address
		bytes32 to;
	}
}

impl Body {
	pub fn send_to_evm<AccountId: Into<[u8; 32]>, B: Into<u128>>(
		amount: B,
		asset_id: H256,
		from: AccountId,
		send_to: H160,
	) -> Self {
		// create h256 equivalent of the h160 which is left padded with 0s
		let mut to = [0u8; 32];
		to[12..].copy_from_slice(&send_to.0);

		let from: [u8; 32] = from.into();

		Self {
			amount: {
				let mut bytes = [0u8; 32];
				convert_to_erc20(amount.into()).to_big_endian(&mut bytes);
				alloy_primitives::U256::from_be_bytes(bytes)
			},
			asset_id: asset_id.0.into(),
			redeem: false,
			from: from.into(),
			to: to.into(),
		}
	}

	pub fn get_asset(&self, meta: &Meta) -> Result<Asset, ismp::error::Error> {
		let local_asset_id = H256::from(self.asset_id.0);
		if local_asset_id == Asset::Argon.asset_id() {
			Ok(Asset::Argon)
		} else if local_asset_id == Asset::OwnershipToken.asset_id() {
			Ok(Asset::OwnershipToken)
		} else {
			Err(dispatch_error("Token Gateway: Unknown asset", meta))?
		}
	}

	fn amount_u256(&self) -> U256 {
		U256::from_big_endian(&self.amount.to_be_bytes::<32>())
	}
}

fn dispatch_error(msg: &str, meta: &Meta) -> ismp::error::Error {
	ismp::error::Error::ModuleDispatchError { msg: msg.to_string(), meta: meta.clone() }
}

const ERC6160_TO_ARGON_DECIMALS: u128 = 10u128.pow((ERC6160_DECIMALS - TOKEN_DECIMALS) as u32);
/// Converts an ERC20 U256 to u128
fn convert_to_balance(value: U256) -> Result<u128, anyhow::Error> {
	let dec_str = (value / U256::from(ERC6160_TO_ARGON_DECIMALS)).to_string();
	// uses strings to avoid floating point errors
	dec_str.parse().map_err(|e| anyhow::anyhow!("{e:?}"))
}

/// Converts a u128 to an Erc20 denomination
pub fn convert_to_erc20(value: u128) -> U256 {
	U256::from(value) * U256::from(ERC6160_TO_ARGON_DECIMALS)
}

#[cfg(test)]
mod tests {
	use super::{convert_to_balance, convert_to_erc20, Asset};
	use sp_core::{H160, U256};
	use sp_keyring::AccountKeyring::Bob;

	#[test]
	fn balance_conversions() {
		let supposedly_small_u256 = U256::from_dec_str("1000000000000000000").unwrap();
		// convert erc20 value to dot value
		let converted_balance = convert_to_balance(supposedly_small_u256).unwrap();
		println!("{}", converted_balance);

		let argon = 1_000_000u128;

		assert_eq!(converted_balance, argon);

		// Convert 1 argon to erc20
		let argon = 1_908_000u128;
		let erc_20_val = convert_to_erc20(argon);
		assert_eq!(erc_20_val, U256::from_dec_str("1908000000000000000").unwrap());
	}

	#[test]
	fn max_value_check() {
		let max = U256::MAX;

		let converted_balance = convert_to_balance(max);
		assert!(converted_balance.is_err())
	}

	#[test]
	fn min_value_check() {
		let min = U256::from(1u128);

		let converted_balance = convert_to_balance(min).unwrap();
		assert_eq!(converted_balance, 0);
	}

	#[test]
	fn can_create_evm_body() {
		let amount = 1_009_000u128;
		let from = Bob.to_account_id();
		let to = H160([8u8; 20]);
		let body = super::Body::send_to_evm(amount, Asset::Argon.asset_id(), from, to);
		assert_eq!(body.amount_u256(), U256::from(1_009_000_000_000_000_000u128));
		// should pad the to address to 32 bytes
		assert_eq!(body.to.0.as_slice(), [&[0u8; 12][..], &[8u8; 20][..]].concat());
	}
}
