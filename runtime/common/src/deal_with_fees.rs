#[macro_export]
macro_rules! deal_with_fees {
	() => {
		use sp_runtime::AccountId32;
		pub struct DealWithFees<R>(PhantomData<R>);

		impl<R> OnUnbalanced<fungible::Credit<AccountId, pallet_balances::Pallet<R, ArgonToken>>>
			for DealWithFees<R>
		where
			R: frame_system::Config<AccountId = AccountId32>
				+ pallet_block_rewards::Config<Balance = Balance>
				+ pallet_balances::Config<ArgonToken, Balance = Balance>,
			AccountIdOf<R>: From<AccountId> + Into<AccountId>,
			<R as frame_system::Config>::RuntimeEvent: From<pallet_balances::Event<R, ArgonToken>>,
		{
			fn on_nonzero_unbalanced(
				imbalance: fungible::Credit<AccountId, pallet_balances::Pallet<R, ArgonToken>>,
			) {
				let author = pallet_block_rewards::Pallet::<R>::fees_account();
				let amount = imbalance.peek();
				match <pallet_balances::Pallet<R, ArgonToken>>::resolve(&author, imbalance) {
					Ok(()) => pallet_block_rewards::Pallet::<R>::track_fee(amount),
					Err(x) => drop(x),
				}
			}
		}

		pub struct MiningBidProxyFeeSponsor<R>(PhantomData<R>);
		impl<R>
			TransactionSponsorProvider<
				R::AccountId,
				<R as frame_system::Config>::RuntimeCall,
				Balance,
			> for MiningBidProxyFeeSponsor<R>
		where
			R: frame_system::Config<RuntimeCall = RuntimeCall, AccountId = AccountId32>
				+ pallet_balances::Config<ArgonToken, Balance = Balance>
				+ pallet_utility::Config<RuntimeCall = RuntimeCall>
				+ pallet_proxy::Config<RuntimeCall = RuntimeCall, ProxyType = ProxyType>
				+ pallet_mining_slot::Config<RuntimeCall = RuntimeCall>,
			AccountIdOf<R>: From<AccountId> + Into<AccountId>,
		{
			fn get_transaction_sponsor(
				signer: &R::AccountId,
				call: &<R as frame_system::Config>::RuntimeCall,
			) -> Option<TxSponsor<R::AccountId, Balance>> {
				// Only sponsor mining bid proxy calls where the proxy type indicates "real pays
				// fee", and the proxied call is either a mining bid call or a utility batch
				// consisting solely of mining bid calls.
				if let RuntimeCall::Proxy(pallet_proxy::Call::proxy {
					real,
					force_proxy_type,
					call: inner_call,
					..
				}) = call
				{
					let MultiAddress::Id(real_account) = real else {
						return None;
					};

					// Helper: check whether a call is a mining bid call.
					fn is_mining_bid_call(call: &RuntimeCall) -> bool {
						matches!(
							call,
							RuntimeCall::MiningSlot(pallet_mining_slot::Call::bid { .. })
						)
					}

					// Helper: allow a batch if (and only if) every inner call is a mining bid call.
					fn is_allowed_inner_call(call: &RuntimeCall) -> bool {
						if is_mining_bid_call(call) {
							return true;
						}
						if let RuntimeCall::Utility(utility_call) = call {
							match utility_call {
								pallet_utility::Call::batch { calls } |
								pallet_utility::Call::batch_all { calls } |
								pallet_utility::Call::force_batch { calls } => {
									return calls.iter().all(is_mining_bid_call);
								},
								_ => {},
							}
						}
						false
					}

					let Ok(def) = pallet_proxy::Pallet::<R>::find_proxy(
						real_account,
						signer,
						*force_proxy_type,
					) else {
						return None;
					};

					if def.proxy_type != ProxyType::MiningBidRealPaysFee {
						return None;
					}

					// Only sponsor if the proxied call is allowed.
					if !is_allowed_inner_call(inner_call.as_ref()) {
						return None;
					}

					const MAX_MINING_BID_FEE_WITH_TIP: Balance = 1 * ARGON;

					return Some(TxSponsor {
						payer: real_account.clone(),
						max_fee_with_tip: Some(MAX_MINING_BID_FEE_WITH_TIP),
						unique_tx_key: Some(
							(b"mining_slot", real_account)
								.using_encoded(sp_crypto_hashing::blake2_256)
								.to_vec(),
						),
					});
				}
				None
			}
		}
	};
}
