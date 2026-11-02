#![no_main]

libfuzzer_sys::fuzz_target!(|data: &[u8]| {
	argon_runtime_fuzz::run(data);
});
