# Benchmarking Critical Patterns & Anti-Patterns

> **ALWAYS READ THIS FIRST** - Contains patterns specific to Substrate benchmarking in this project

## How to Benchmark

### 1. Determine the actual flow of calls that fill in data that will be accessed

A call or hook will often access Storage Items, and make Provider calls or Event Handler calls to
other pallets. You must understand the entire flow of calls that will be made to access all data
that is needed in the longest flow of logic.

### 2. Understand how Providers and Event Handlers are wired together

There are several flow documents that involve deep analysis of the cross pallet flow for things like
Notebooks, Bitcoin Locks, and Mining Slots. These documents are critical to understanding how to set
up the data so that we can simulate all data access.

Event Handlers can often source their own weights and provide them to a pallet (like notebook
events), but providers are often accessed inline and so need to have data set-up (like Bitcoin Locks
and Bitcoin UTXO tracking).

### 3. Set up the state owned by the pallet being measured

Seed the pallet's own storage and use its configured providers for external state. When a provider
has separately measured weights, use the benchmark-runtime mock for its responses and compose the
owner's weight in the real call. Check both runtime configurations and the mock used by the fixture
test. The [pallet dependency rule](../AGENTS.md#pallet-test-dependencies) also applies to new uses
of concrete pallets in benchmarks. Keep fixture setup out of production provider interfaces.

### 4. Use existing benchmark setup functions when possible

Many pallets have benchmark setup functions that can be used to create the necessary data for things
like Vaults, Bitcoin UTXOs, and Price Indexes. These functions are often found in the benchmarking
module of the pallet.

### 5. Write benchmarks starting simple, adding complexity iteratively

Start with a basic benchmark that exercises the extrinsic using the `_(RawOrigin::Signed(caller))`
or `_(RawOrigin::None)` pattern.

**For operations that scale with input:**

- Use `Linear<1, MAX>` parameters where MAX is based on runtime limits (e.g., 100 vaults)
- Test edge cases: minimum (1) and realistic maximum values
- Verify linear scaling - watch for O(n²) surprises in nested loops
- Use `#[block]` to isolate just the operation being measured

**Development tip:** Run single benchmark during development:

```bash
frame-omni-bencher v1 benchmark pallet --extrinsic="your_benchmark" ...
```

### 6. Add the benchmark to WeightInfo trait (ALWAYS use zero weight)

**In the pallet's src/weight.rs:**

1. Add to the `WeightInfo` trait: `fn your_extrinsic() -> Weight;`
2. Add to the default impl with **ZERO weight only**:

```rust
fn your_extrinsic() -> Weight {
  Weight::zero()  // ALWAYS zero - never add actual weights here
}
```

### 7. Handle missing benchmarks in BOTH runtime weight files

When a benchmark hasn't been implemented yet, the generated weight files will have compilation
errors. You must fix BOTH runtimes:

**In runtime/argon/src/weights/pallet_name.rs:**

```rust
fn your_extrinsic() -> Weight {
  // TODO: Benchmark not yet implemented
  Weight::zero()  // Placeholder to allow compilation
}
```

**In runtime/canary/src/weights/pallet_name.rs:**

```rust
fn your_extrinsic() -> Weight {
  // TODO: Benchmark not yet implemented
  Weight::zero()  // Placeholder to allow compilation
}
```

**Key points:**

- BOTH argon and canary runtimes need the placeholder
- This is ONLY to allow the runtimes to compile
- ALWAYS use `Weight::zero()` - never estimate weights
- The comment makes it clear this is not a real measurement
- Once the benchmark is implemented and weights generated, these get replaced automatically

### 8. Verify benchmark coverage before finalizing

Before considering a benchmark complete:

- [ ] Benchmark triggers the worst-case code path?
- [ ] All storage reads/writes are captured?
- [ ] No unbounded loops that could be DOS vectors?
- [ ] Not double-counting operations already weighted elsewhere?
- [ ] Cross-pallet calls have realistic state setup?

### 9. Common gotchas checklist

Before running benchmarks:

- [ ] Built runtime with `runtime-benchmarks` feature first?
- [ ] Using `--template="scripts/weight_template.hbs"`?
- [ ] Set up tick/frame state to avoid historical data explosion?
- [ ] Does the benchmark execute the cryptographic operations included in its weight?
- [ ] Checked for existing helper functions in other pallets?
- [ ] Weight function name matches extrinsic name exactly?

## 🚫 CRITICAL ANTI-PATTERNS (STOP DOING THESE)

### Runtime benchmark fixtures

Use the benchmark providers wired in the Wasm runtime. A test runtime may supply the same fixture
values from its own mock. Host-only mutable statics cannot supply state to a Wasm benchmark.

### 3. Weight file modifications

**NEVER manually edit generated weight files except for zero-weight placeholders** - they are
auto-generated from benchmarking code.

**NEVER add non-zero weights** - always use `Weight::zero()` with a TODO comment for unimplemented
benchmarks.

### 4. Benchmark helper functions and cryptography

**NEVER duplicate benchmark helper functions** - check for existing helpers in other pallets first
and reuse/extend them.

Use real signatures and keys when measuring verification. A caller that composes a separately
benchmarked verifier's weight can use a provider mock for its result.

### Pallet feature flags

Keep benchmark-only dependencies behind `runtime-benchmarks` and forward their required features
in both runtimes. Reuse workspace dependency declarations.

## 🔧 KEY SOLUTIONS TO COMMON ISSUES

### Real cryptography in benchmarks

**Problem**: Signature verification or key derivation fails in benchmarking **Solution**: Use real
cryptographic libraries:

- For Bitcoin: Use real BIP32 libraries to create valid derivable xpubs
- For Ed25519: Use `ed25519-dalek` with deterministic seeds
- Never bypass signature verification in pallet logic

Example:

```rust
use ed25519_dalek::{Keypair, Signer};
let keypair = Keypair::from_bytes( & keypair_bytes).expect("valid keypair");
let signature = keypair.sign(header_hash);
ed25519::Signature::from_raw(signature.to_bytes())
```

### Build and template issues

**Problem**: Generated weight files missing imports or using outdated functions **Solution**:

1. **ALWAYS** build runtime first:
   `cargo build --release --features=runtime-benchmarks --bin argon-node`
2. Use project template: `--template="scripts/weight_template.hbs"`

### Price scenarios for economic functions

**Problem**: Economic functions like ratchet need specific price conditions **Solution**: Set up
realistic price scenarios (e.g., double Bitcoin price for arbitrage)

## 🔑 KEY PATTERNS FOR SUCCESS

### Proper Weight Function Usage

```rust
// ❌ WRONG - Using wrong weight function
#[pallet::weight(T::WeightInfo::request_release())]
pub fn ratchet(...) { ... }

// ✅ CORRECT - Function name matches weight function
#[pallet::weight(T::WeightInfo::ratchet())]
pub fn ratchet(...) { ... }
```

### Dynamic Weights for Variable Data

```rust
// ❌ WRONG - Static weight for dynamic data
#[pallet::weight(T::WeightInfo::sync())]
pub fn sync(data: Vec<Item>) { ... }

// ✅ CORRECT - Dynamic weight based on data size
#[pallet::weight(T::WeightInfo::sync(data.len() as u32))]
pub fn sync(data: Vec<Item>) { ... }
```

## 🎯 QUICK REFERENCE CHECKLIST

1. **Build runtime first**: `cargo build --release --features=runtime-benchmarks --bin argon-node`
2. **Use project template**: `--template="scripts/weight_template.hbs"`
3. **For external state**: Check the configured provider, its fixture and its declared weight
4. **For measured verification**: Generate real keypairs and execute signature checks
5. **For economic functions**: Set up realistic price scenarios
6. **For dynamic data**: Pass counts as weight function parameters
7. **Never manually edit**: Generated weight files except for zero-weight placeholders
8. **Always verify**: Function names match their weight functions
9. **Set up tick/frame state properly**: Avoid historical data explosion
10. **For inherent pallets**: Manually simulate `on_finalize` state cleanup
11. **For voting/consensus**: Use same data for merkle root generation and proof validation
