use argon_client::{ArgonOnlineClient, api};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	// Create a client to use:

	let client = ArgonOnlineClient::new().await?;

	// Subscribe to all finalized blocks:
	let mut blocks_sub = client.blocks().subscribe_finalized().await?;

	// For each block, print a bunch of information about it:
	while let Some(block) = blocks_sub.next().await {
		let block = block?;

		let block_number = block.header().number;
		let block_hash = block.hash();
		let events = block.events().await?;

		println!("Block #{block_number}:");
		println!("  Hash: {block_hash}");
		println!("  {events:?}");

		// Log each of the extrinsic with it's associated events:
		let extrinsics = block.extrinsics().await?;
		for ext in extrinsics.iter() {
			let idx = ext.index();
			let events = ext.events().await?;
			let bytes_hex = format!("0x{}", hex::encode(ext.bytes()));

			// See the API docs for more ways to decode extrinsics:
			let decoded_ext = ext.as_root_extrinsic::<api::Call>();

			println!("    Extrinsic #{idx}:");
			println!("      Bytes: {bytes_hex}");
			println!("      Decoded: {decoded_ext:?}");
			println!("      Events:");

			for evt in events.iter() {
				let evt = evt?;

				let pallet_name = evt.pallet_name();
				let event_name = evt.variant_name();
				let event_values = evt.field_values()?;

				println!("        {pallet_name}_{event_name}");
				println!("          {event_values}");
			}
		}
	}

	Ok(())
}
