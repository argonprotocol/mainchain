use std::env::temp_dir;

use sp_core::{sr25519::Pair, Pair as PairT};

use argon_client::{
	api::{runtime_types::argon_node_runtime::RuntimeCall, tx},
	signer::Sr25519Signer,
};
use argon_testing::{
	clean_localchain_test_dir, run_localchain_cli, start_argon_test_node, ArgonTestNotary,
};

#[tokio::test]
async fn test_localchain_transfers_using_cli() {
	let test_node = start_argon_test_node().await;
	let alice_signer = Sr25519Signer::new(Pair::from_string("//Alice", None).unwrap());

	let test_notary = ArgonTestNotary::start(&test_node, None).await.unwrap();

	let owner = test_node.client.api_account(&test_notary.operator.public().into());
	// give ferdie//notary some balance
	let _ = test_node
		.client
		.live
		.tx()
		.sign_and_submit_then_watch_default(
			&tx().balances().transfer_keep_alive(owner.into(), 10_000),
			&alice_signer,
		)
		.await
		.unwrap()
		.wait_for_finalized_success()
		.await
		.unwrap();

	println!("Registering a notary operator");
	test_notary.register_operator(&test_node).await.unwrap();
	{
		println!("Sudo approving notary");
		let operator_account = test_node.client.api_account(&test_notary.operator.public().into());
		test_node
			.client
			.live
			.tx()
			.sign_and_submit_then_watch_default(
				&tx().sudo().sudo(RuntimeCall::Notaries(
					argon_client::api::runtime_types::pallet_notaries::pallet::Call::activate {
						operator_account,
					},
				)),
				&alice_signer,
			)
			.await
			.unwrap()
			.wait_for_finalized_success()
			.await
			.unwrap();
		println!("Sudo approved notary");
	}

	clean_localchain_test_dir();

	let primary_localchain = run_localchain_cli(
		&test_node,
		vec!["accounts", "create", "--suri", "//chicken dog pig box bread cat"],
	)
	.await
	.unwrap();
	println!("{}", primary_localchain);
	let alice = run_localchain_cli(
		&test_node,
		vec!["accounts", "create", "-n", "alice", "--suri", "//Alice"],
	)
	.await
	.unwrap();
	println!("{}", alice);

	let list = run_localchain_cli(&test_node, vec!["accounts", "list"]).await.unwrap();
	println!("{}", list);

	let mainchain_transfer =
		run_localchain_cli(&test_node, vec!["transactions", "from-mainchain", "-n", "alice", "10"])
			.await
			.unwrap();
	println!("{}", mainchain_transfer);

	loop {
		let refreshed = run_localchain_cli(
			&test_node,
			vec!["accounts", "info", "-n", "alice", "--sync-latest"],
		)
		.await
		.unwrap();
		println!("{}", refreshed);

		if refreshed.contains("processing") {
			tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
			continue;
		}
		assert!(refreshed.contains("₳10"));
		assert!(!refreshed.contains("pending") && !refreshed.contains("processing"));
		break;
	}

	let file_path = temp_dir().join("from-alice.argon");
	let file = run_localchain_cli(
		&test_node,
		vec![
			"transactions",
			"send",
			"5.2",
			"-n",
			"alice",
			"--save-to-path",
			file_path.to_string_lossy().as_ref(),
		],
	)
	.await
	.unwrap();
	println!("{}", file);

	// import by primary
	let imported = run_localchain_cli(
		&test_node,
		vec!["transactions", "receive", file_path.to_string_lossy().as_ref(), "-n", "primary"],
	)
	.await
	.unwrap();

	println!("{}", imported);

	loop {
		let refreshed = run_localchain_cli(
			&test_node,
			vec!["accounts", "info", "-n", "alice", "--sync-latest"],
		)
		.await
		.unwrap();
		println!("{}", refreshed);

		if refreshed.contains("pending") {
			tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
			continue;
		}
		// double taxing right now for jump accounts
		assert!(refreshed.contains("₳4.8"));
		assert!(!refreshed.contains("pending") && !refreshed.contains("processing"));
		break;
	}

	let primary =
		run_localchain_cli(&test_node, vec!["accounts", "info", "-n", "primary", "--sync-latest"])
			.await
			.unwrap();
	println!("{}", primary);
	assert!(primary.contains("₳4.8"));

	let to_mainchain = run_localchain_cli(
		&test_node,
		vec!["transactions", "to-mainchain", "4.8", "-n", "primary", "--wait-for-immortalized"],
	)
	.await
	.unwrap();
	println!("{}", to_mainchain);

	loop {
		let refreshed = run_localchain_cli(
			&test_node,
			vec!["accounts", "info", "-n", "primary", "--sync-latest"],
		)
		.await
		.unwrap();
		println!("{}", refreshed);

		if refreshed.contains("pending") {
			tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
			continue;
		}
		assert!(refreshed.contains("₳0"));
		assert!(!refreshed.contains("pending"));
		break;
	}

	let pair = Pair::from_string("//chicken dog pig box bread cat", None).unwrap();
	let mainchain_balance = test_node.client.get_argons(&pair.public().into()).await.unwrap();
	assert_eq!(mainchain_balance.free, 4800);
	println!("Mainchain account of primary localchain: {:#?}", mainchain_balance);

	drop(test_node);
	drop(test_notary);
}
