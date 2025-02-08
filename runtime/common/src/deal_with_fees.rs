#[macro_export]
macro_rules! deal_with_fees {
	() => {
		pub struct DealWithFees<R>(PhantomData<R>);

		impl<R> OnUnbalanced<fungible::Credit<R::AccountId, pallet_balances::Pallet<R, ArgonToken>>>
			for DealWithFees<R>
		where
			R: pallet_authorship::Config + pallet_balances::Config<ArgonToken>,
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
					if let Some(author) = pallet_authorship::Pallet::<R>::author() {
						let _ = <pallet_balances::Pallet<R, ArgonToken>>::resolve(&author, fees)
							.map_err(drop);
					}
				}
			}
		}
	};
}
