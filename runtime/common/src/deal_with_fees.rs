#[macro_export]
macro_rules! deal_with_fees {
	() => {
		pub struct DealWithFees<R>(PhantomData<R>);

		impl<R> OnUnbalanced<fungible::Credit<R::AccountId, pallet_balances::Pallet<R, ArgonToken>>>
			for DealWithFees<R>
		where
			R: pallet_block_rewards::Config<Balance = Balance>
				+ pallet_balances::Config<ArgonToken, Balance = Balance>,
			AccountIdOf<R>: From<AccountId> + Into<AccountId>,
			<R as frame_system::Config>::RuntimeEvent: From<pallet_balances::Event<R, ArgonToken>>,
		{
			fn on_unbalanceds(
				mut fees_then_tips: impl Iterator<
					Item = fungible::Credit<R::AccountId, pallet_balances::Pallet<R, ArgonToken>>,
				>,
			) {
				if let Some(mut fees) = fees_then_tips.next() {
					if let Some(tips) = fees_then_tips.next() {
						tips.merge_into(&mut fees);
					}
					let amount: Balance = fees.peek();
					if amount.is_zero() {
						drop(fees);
						return;
					}
					let author = pallet_block_rewards::Pallet::<R>::fees_account();
					match <pallet_balances::Pallet<R, ArgonToken>>::resolve(&author, fees) {
						Ok(()) => pallet_block_rewards::Pallet::<R>::track_fee(amount),
						Err(x) => drop(x),
					}
				}
			}
		}
	};
}
