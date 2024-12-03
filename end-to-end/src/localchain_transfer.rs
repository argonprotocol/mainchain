use std::env::temp_dir;

use crate::utils::create_active_notary;
use argon_testing::{start_argon_test_node, LocalchainCli};
use serial_test::serial;
use sp_core::{sr25519::Pair, Pair as PairT};

#[tokio::test(flavor = "multi_thread")]
#[serial]
async fn test_localchain_transfers_using_cli() {
	let test_node = start_argon_test_node().await;

	let test_notary = create_active_notary(&test_node).await.expect("Notary registered");

	let localchain_cli = LocalchainCli::new("localchain_transfers");

	let primary_localchain = localchain_cli
		.run(&test_node, vec!["accounts", "create", "--suri", "//chicken dog pig box bread cat"])
		.await
		.unwrap();
	println!("{}", primary_localchain);
	let alice = localchain_cli
		.run(&test_node, vec!["accounts", "create", "-n", "alice", "--suri", "//Alice"])
		.await
		.unwrap();
	println!("{}", alice);

	let list = localchain_cli.run(&test_node, vec!["accounts", "list"]).await.unwrap();
	println!("{}", list);

	let mainchain_transfer = localchain_cli
		.run(&test_node, vec!["transactions", "from-mainchain", "-n", "alice", "10"])
		.await
		.unwrap();
	println!("{}", mainchain_transfer);

	loop {
		let refreshed = localchain_cli
			.run(&test_node, vec!["accounts", "info", "-n", "alice", "--sync-latest"])
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
	let file = localchain_cli
		.run(
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
	let imported = localchain_cli
		.run(
			&test_node,
			vec!["transactions", "receive", file_path.to_string_lossy().as_ref(), "-n", "primary"],
		)
		.await
		.unwrap();

	println!("{}", imported);

	loop {
		let refreshed = localchain_cli
			.run(&test_node, vec!["accounts", "info", "-n", "alice", "--sync-latest"])
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

	let primary = localchain_cli
		.run(&test_node, vec!["accounts", "info", "-n", "primary", "--sync-latest"])
		.await
		.unwrap();
	println!("{}", primary);
	assert!(primary.contains("₳4.8"));

	let to_mainchain = localchain_cli
		.run(
			&test_node,
			vec!["transactions", "to-mainchain", "4.8", "-n", "primary", "--wait-for-immortalized"],
		)
		.await
		.unwrap();
	println!("{}", to_mainchain);

	loop {
		let refreshed = localchain_cli
			.run(&test_node, vec!["accounts", "info", "-n", "primary", "--sync-latest"])
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
	assert_eq!(mainchain_balance.free, 4_800_000);
	println!("Mainchain account of primary localchain: {:#?}", mainchain_balance);

	drop(test_node);
	drop(test_notary);
}
