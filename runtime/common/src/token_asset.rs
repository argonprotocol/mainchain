#[macro_export]
macro_rules! token_asset {
	($tokenPallet:ty, $tokenAdminOwner:expr) => {
		pub struct OwnershipTokenAsset;

		impl fungibles::Inspect<AccountId> for OwnershipTokenAsset {
			type AssetId = u32;
			type Balance = Balance;

			fn total_issuance(asset: Self::AssetId) -> Self::Balance {
				if asset != OwnershipTokenAssetId::get() {
					return 0;
				}
				<$tokenPallet>::total_issuance()
			}

			fn minimum_balance(asset: Self::AssetId) -> Self::Balance {
				if asset != OwnershipTokenAssetId::get() {
					return 0;
				}
				<$tokenPallet as Currency<AccountId>>::minimum_balance()
			}

			fn total_balance(asset: Self::AssetId, who: &AccountId) -> Self::Balance {
				if asset != OwnershipTokenAssetId::get() {
					return 0;
				}
				<$tokenPallet as Currency<AccountId>>::total_balance(who)
			}

			fn balance(asset: Self::AssetId, who: &AccountId) -> Self::Balance {
				if asset != OwnershipTokenAssetId::get() {
					return 0;
				}
				<$tokenPallet>::balance(who)
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
				<$tokenPallet>::reducible_balance(who, preservation, force)
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
				<$tokenPallet>::can_deposit(who, amount, provenance)
			}

			fn can_withdraw(
				asset: Self::AssetId,
				who: &AccountId,
				amount: Self::Balance,
			) -> WithdrawConsequence<Self::Balance> {
				if asset != OwnershipTokenAssetId::get() {
					return WithdrawConsequence::UnknownAsset;
				}
				<$tokenPallet>::can_withdraw(who, amount)
			}

			fn asset_exists(asset: Self::AssetId) -> bool {
				asset == OwnershipTokenAssetId::get()
			}
		}

		impl fungibles::Unbalanced<AccountId> for OwnershipTokenAsset {
			fn handle_dust(dust: fungibles::Dust<AccountId, Self>) {
				if dust.0 != OwnershipTokenAssetId::get() {
					return;
				}
				<$tokenPallet>::handle_dust(fungible::Dust(dust.1))
			}

			fn write_balance(
				asset: Self::AssetId,
				who: &AccountId,
				amount: Self::Balance,
			) -> Result<Option<Self::Balance>, DispatchError> {
				if asset != OwnershipTokenAssetId::get() {
					return Err(DispatchError::Unavailable)?;
				}
				<$tokenPallet>::write_balance(who, amount)
			}

			fn set_total_issuance(asset: Self::AssetId, amount: Self::Balance) {
				if asset != OwnershipTokenAssetId::get() {
					return;
				}
				<$tokenPallet>::set_total_issuance(amount)
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
				<$tokenPallet>::burn_from(who, amount, preservation, precision, force)
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
				Some($tokenAdminOwner)
			}

			fn freezer(_asset: Self::AssetId) -> Option<AccountId> {
				None
			}
		}

		pub struct TokenAdmin;
		impl Get<AccountId> for TokenAdmin {
			fn get() -> AccountId {
				$tokenAdminOwner
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
	};
}
