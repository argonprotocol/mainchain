use crate::mock::{System, *};

#[test]
fn it_can_track_mint_last_updated() {
	new_test_ext().execute_with(|| {
		let who = 1;
		System::set_block_number(500);
		set_argons(who, 10_000);
	});
}
