use anyhow::{Context, Result, ensure};
use std::{
	env, fs,
	path::{Path, PathBuf},
};
use wasm_bindgen_cli_support::Bindgen;
use wasm_opt::OptimizationOptions;

fn main() -> Result<()> {
	let package_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
	let repo_root = package_dir
		.parent()
		.and_then(|path| path.parent())
		.context("unable to locate repo root")?;
	let output_dir = package_dir.join("ts/wasm");
	let target_dir = env::var_os("CARGO_TARGET_DIR")
		.map(PathBuf::from)
		.unwrap_or_else(|| repo_root.join("target"));
	let input_wasm = target_dir.join("wasm32-unknown-unknown/release/argon_bitcoin_wasm.wasm");

	ensure!(input_wasm.exists(), "expected built wasm at {}", input_wasm.display());

	fs::create_dir_all(&output_dir)
		.with_context(|| format!("creating {}", output_dir.display()))?;

	let mut bindgen = Bindgen::new();
	bindgen
		.input_path(&input_wasm)
		.out_name("bitcoin_bindings")
		.bundler(true)?
		.typescript(true)
		.generate(&output_dir)
		.with_context(|| format!("generating bindings into {}", output_dir.display()))?;

	optimize_output_wasm(&output_dir)?;

	Ok(())
}

fn optimize_output_wasm(output_dir: &Path) -> Result<()> {
	let output_wasm = output_dir.join("bitcoin_bindings_bg.wasm");
	let optimized_wasm = output_dir.join("bitcoin_bindings_bg.optimized.wasm");

	ensure!(output_wasm.exists(), "expected generated wasm at {}", output_wasm.display());

	OptimizationOptions::new_optimize_for_size()
		.run(&output_wasm, &optimized_wasm)
		.with_context(|| format!("optimizing {}", output_wasm.display()))?;

	#[cfg(windows)]
	if output_wasm.exists() {
		fs::remove_file(&output_wasm)
			.with_context(|| format!("removing {}", output_wasm.display()))?;
	}

	fs::rename(&optimized_wasm, &output_wasm).with_context(|| {
		format!(
			"replacing optimized wasm {} -> {}",
			optimized_wasm.display(),
			output_wasm.display()
		)
	})?;

	Ok(())
}
