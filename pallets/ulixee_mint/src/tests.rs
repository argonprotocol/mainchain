use crate::{
	mock::{System, *},
	pallet::{MintedArgons, UlixeeAccountLastTransferBlock},
};
use frame_support::assert_ok;
use ulx_primitives::{block_seal::BlockPayout, BlockRewardsEventHandler};

#[test]
fn it_can_track_mint_last_updated() {
	new_test_ext().execute_with(|| {
		let who = 1;
		System::set_block_number(500);
		set_argons(who, 10_000);

		assert_ok!(Balances::transfer_allow_death(RuntimeOrigin::signed(who), 2, 1_000));
		assert_eq!(UlixeeAccountLastTransferBlock::<Test>::get(who), Some(500));
		assert_eq!(UlixeeAccountLastTransferBlock::<Test>::get(2), Some(500));
	});
}

#[test]
fn it_records_burnt_argons_by_prorata() {
	new_test_ext().execute_with(|| {
		MintedArgons::<Test>::set(100);
		BitcoinMintCirculation::set(100);
		UlixeeMint::on_argon_burn(50);
		assert_eq!(MintedArgons::<Test>::get(), 100 - 25);

		MintedArgons::<Test>::set(200);
		BitcoinMintCirculation::set(0);
		UlixeeMint::on_argon_burn(50);
		assert_eq!(MintedArgons::<Test>::get(), 200 - 50);

		MintedArgons::<Test>::set(0);
		BitcoinMintCirculation::set(100);
		UlixeeMint::on_argon_burn(50);
		assert_eq!(MintedArgons::<Test>::get(), 0);

		MintedArgons::<Test>::set(33);
		BitcoinMintCirculation::set(66);
		UlixeeMint::on_argon_burn(10);
		assert_eq!(MintedArgons::<Test>::get(), 33 - 3);
	});
}

#[test]
fn it_tracks_block_rewards() {
	new_test_ext().execute_with(|| {
		<UlixeeMint as BlockRewardsEventHandler<_, _>>::rewards_created(&vec![
			BlockPayout { account_id: 1, argons: 100, ulixees: 100 },
			BlockPayout { account_id: 1, argons: 1, ulixees: 1 },
			BlockPayout { account_id: 2, argons: 5, ulixees: 5 },
		]);

		assert_eq!(MintedArgons::<Test>::get(), 106);
	});
}
