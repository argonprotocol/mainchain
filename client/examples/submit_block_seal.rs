use codec::Encode;
use serde::Deserialize;
use sp_core::U256;
use std::net::Ipv4Addr;
use subxt::{
	backend::rpc::{RpcClient, RpcParams},
	ext::{futures, sp_core::blake2_256, sp_runtime::traits::UniqueSaturatedInto},
	rpc_params,
	utils::AccountId32,
};
use subxt_signer::ecdsa::dev;

use ulixee_client::{
	api,
	api::runtime_types::{
		bounded_collections::bounded_vec::BoundedVec,
		ulx_primitives::block_seal::{BlockProof, Host, SealStamper},
	},
	local_client,
	signature_messages::to_seal_nonce_hash_message,
};

#[derive(Deserialize, Debug)]
struct ApprovalResponse {
	pub signature: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	let client = local_client().await?;

	let author = dev::ferdie();
	let author_id = AccountId32::from(author.public_key());
	let parent_hash = client.blocks().at_latest().await?.hash();
	let query = api::runtime_apis::ulx_consensus_api::UlxConsensusApi.next_work();
	let work_info = client.runtime_api().at_latest().await?.call(query).await?;

	let sealers_query = api::runtime_apis::mining_authority_apis::MiningAuthorityApis
		.block_peers(author_id.clone());
	let sealers = client.runtime_api().at_latest().await?.call(sealers_query).await?;

	let first_sealer = sealers[0].clone();

	let mut block_proof = BlockProof {
		author_id,
		seal_stampers: sealers
			.iter()
			.map(|a| SealStamper { authority_idx: a.authority_index, signature: None })
			.collect::<Vec<_>>(),
		tax_proof_id: 1,
		tax_amount: 5_000,
	};

	// concurrently call the validator rpc to sign the message until all have completed
	let signatures = futures::future::join_all(sealers.iter().map(|id| {
		let seal_params = rpc_params![parent_hash, block_proof.clone()];
		let idx = id.authority_index;
		let host_url = format_host(&id.rpc_hosts.0[0]);
		get_seal_approval(host_url, idx, seal_params.clone())
	}))
	.await;

	let signature_count: u8 = signatures.len().unique_saturated_into();

	for res in signatures {
		match res {
			Ok((idx, signature)) => {
				let pos = block_proof
					.seal_stampers
					.iter()
					.position(|s| s.authority_idx == idx)
					.expect("Should have found a seal index");
				block_proof.seal_stampers[pos].signature = Some(BoundedVec(signature));
			},
			Err(e) => {
				println!("Error getting validator signature: {}", e);
			},
		}
	}

	let nonce =
		to_seal_nonce_hash_message(block_proof.clone(), parent_hash).using_encoded(blake2_256);
	let nonce_u256 = U256::from(nonce);
	let easing_query = api::runtime_apis::ulx_consensus_api::UlxConsensusApi
		.calculate_easing(block_proof.tax_amount, signature_count);
	let easing = client.runtime_api().at_latest().await?.call(easing_query).await?;

	let threshold = (U256::MAX / U256::from(work_info.difficulty)) - easing;
	let seal_hash = U256::from_big_endian(&blake2_256(&[&parent_hash[..], &nonce[..]].concat()));
	println!(
		"Final proposed seal_hash: {:?}, threshold {:?}. nonce={}, params={}",
		seal_hash,
		threshold,
		nonce_u256,
		rpc_params![parent_hash, nonce_u256, block_proof.clone()].build().unwrap()
	);
	if seal_hash >= threshold {
		let host_url = format_host(&first_sealer.rpc_hosts.0[0]);
		let peer_client = RpcClient::from_url(host_url).await?;
		// must submit to first client
		let response = peer_client
			.request("blockSeal_submit", rpc_params![parent_hash, nonce_u256, block_proof])
			.await?;
		println!("Response: {:?}", response);
	}

	Ok(())
}

async fn get_seal_approval(
	host_url: String,
	idx: u16,
	seal_params: RpcParams,
) -> Result<(u16, Vec<u8>), subxt::Error> {
	let peer_client = RpcClient::from_url(host_url).await?;
	if let Some(signature) = peer_client
		.request::<Option<ApprovalResponse>>("blockSeal_seekApproval", seal_params)
		.await
		.map_err(|e| format!("Error getting seal approval: {}", e))?
	{
		let signature = signature.signature.trim_start_matches("0x");
		println!("Response from seal approval ({idx}) {:?}", signature);
		let signature =
			hex::decode(signature).map_err(|e| format!("Error decoding seal signature: {}", e))?;
		Ok((idx, signature))
	} else {
		Err(format!("Validator {} did not approve seal", idx).into())
	}
}

fn format_host(host: &Host) -> String {
	let ip = Ipv4Addr::from(host.ip);
	let protocol = if host.is_secure { "wss" } else { "ws" };
	format!("{protocol}://{ip}:{}", host.port)
}
