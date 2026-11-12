use codec::Decode;
use definitions::network_specs::ShortSpecs;
use frame_metadata::{RuntimeMetadata, RuntimeMetadataPrefixed};
use std::{env, error::Error, fs};
use vault_parser::{parse_set, MetadataBundle};

// These exercise Polkadot Vault's decoder and review cards, not its mobile UI or QR scanner.
#[test]
fn reviews_the_underlying_transfer_and_batch() -> Result<(), Box<dyn Error>> {
	for kind in ["transfer", "batch"] {
		let review = review(kind)?;
		assert!(review.contains("AccountSecurity"));
		assert!(review.contains("execute"));
		assert!(review.contains("Balances"));
		assert!(review.contains("transfer_keep_alive"));
		assert!(review.contains("5CqTdCcXCC7kv1Nwq2sCeJUNVqxbBPXjMAUXrbhRJtrUiyPP"));
		assert!(review.contains("balance: 1.000000 ARGN"));
		assert!(review.contains("era: Immortal"));
		assert!(review.contains("\nnonce: 0\n"));
		assert!(review.contains("tip: 0 uARGN"));

		if kind == "batch" {
			assert!(review.contains("batch_all"));
			assert!(review.contains("5DDqWU3pDgJsbH5BDsKoY2d8g5SD7jaAZVaYN3YJjtHygLwZ"));
			assert!(review.contains("balance: 2.000000 ARGN"));
		}
	}
	Ok(())
}

#[test]
fn reviews_signers_quorums_and_recovery_for_both_policy_operations() -> Result<(), Box<dyn Error>> {
	for (kind, method) in [("register", "register"), ("updatePolicy", "update_policy")] {
		let review = review(kind)?;
		assert!(review.contains("AccountSecurity"));
		assert!(review.contains(method));
		assert!(review.contains("field_name: signers"));
		assert!(review.contains("field_name: recovery_account"));
		assert!(review.contains("field_name: recovery_delay"));
		assert!(review.contains("era: Immortal"));
		assert!(review.contains("\nnonce: 0\n"));
		assert!(review.contains("tip: 0 uARGN"));

		let fields = review.split_whitespace().collect::<Vec<_>>().join(" ");
		assert!(fields.contains("field_name: quorum default: 2 "));
		assert!(fields.contains("field_name: recovery_delay default: 1440 "));
		for signer in [
			"5DzbGzvQGeh6wpTe1YF1KUvf2YPSzRf2z9nZNwE5bsAyakax",
			"5ENyAGMhJ8tDd69sQNhcDD5RCms4vmhUCUtZtP4y2rcUXtt2",
			"5F9ivoEHM7GSydYLC3cozfNwZEpJoTnLd96auGkjtqVUSM86",
		] {
			assert!(review.contains(signer));
		}
	}
	Ok(())
}

#[test]
fn rejects_a_payload_from_another_network() {
	let error = review("wrongGenesis").expect_err("Vault must reject a different genesis hash");
	assert!(format!("{error:?}").contains("GenesisHashMismatch"));
}

fn review(kind: &str) -> Result<String, Box<dyn Error>> {
	let fixtures: serde_json::Value = serde_json::from_slice(&fs::read(
		env::var("ARGON_VAULT_FIXTURES").map_err(|_| "Run through testing/signers/run.sh vault")?,
	)?)?;
	let metadata = hex::decode(fixtures["metadata"].as_str().ok_or("missing metadata")?)?;
	let RuntimeMetadataPrefixed(_, RuntimeMetadata::V14(metadata)) =
		RuntimeMetadataPrefixed::decode(&mut metadata.as_slice())?
	else {
		return Err("Vault fixture must use the actual V14 client metadata".into());
	};

	let fixture = &fixtures["payloads"][kind];
	let payload = hex::decode(fixture["vault_payload"].as_str().ok_or("missing Vault payload")?)?;
	let genesis: [u8; 32] = hex::decode(fixture["genesis"].as_str().ok_or("missing genesis")?)?
		.try_into()
		.map_err(|_| "genesis must contain 32 bytes")?;
	let specs = ShortSpecs {
		base58prefix: 42,
		decimals: 6,
		genesis_hash: genesis.into(),
		name: "argon".into(),
		unit: "ARGN".into(),
	};
	let metadata = MetadataBundle::Sci {
		meta_v14: &metadata,
		network_version: fixture["spec_version"].as_u64().ok_or("missing spec version")? as u32,
	};

	let (method, extensions, _, _) = parse_set(&payload, &metadata, &specs, Some(false))?;
	Ok(method?
		.into_iter()
		.chain(extensions)
		.map(|card| card.card.show_no_docs(card.indent))
		.collect::<Vec<_>>()
		.join("\n"))
}
