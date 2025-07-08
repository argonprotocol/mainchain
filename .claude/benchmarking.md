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

### 3. Setup data that spans all pallets being accessed.

The runtime (runtimes/argon/src/lib.rs) is often the source for which pallets must be accessed and
prepared for a benchmark. You must determine if you can link the needed pallets in the
runtime-benchmarks feature flag, or you will need to use the Provider Trait Pattern.

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
- [ ] Using real crypto operations, not mocks?
- [ ] Checked for existing helper functions in other pallets?
- [ ] Weight function name matches extrinsic name exactly?

## 🚫 CRITICAL ANTI-PATTERNS (STOP DOING THESE)

### 1. Using provider #[cfg(feature=runtime-bencharks)] feature flags as a first resort

**Preferred: Direct Pallet Storage Access (when possible)**

```rust
// ✅ PREFERRED - Use direct pallet storage imports
pallet_price_index::Current::<T>::put(price_data);
```

**Last Resort: Provider Trait Pattern (for circular dependencies)**

```rust
// ✅ ACCEPTABLE WHEN NECESSARY - Provider trait with benchmark method
T::BitcoinUtxoTracker::set_for_benchmark(utxo_id, utxo_ref, satoshis, watch_until);
```

### 2. Mock files and feature flags in benchmarking

**NEVER modify mock files for benchmarking** - they are only for unit tests. Benchmarking occurs in
the actual runtime environment.

**NEVER use `#[cfg(feature = "runtime-benchmarks")]` inside benchmarking files** - these files only
compile with runtime-benchmarks feature.

### 3. Weight file modifications

**NEVER manually edit generated weight files except for zero-weight placeholders** - they are
auto-generated from benchmarking code.

**NEVER add non-zero weights** - always use `Weight::zero()` with a TODO comment for unimplemented
benchmarks.

### 4. Benchmark helper functions and cryptography

**NEVER duplicate benchmark helper functions** - check for existing helpers in other pallets first
and reuse/extend them.

**NEVER simulate signatures or keys** - always use real cryptographic primitives since benchmarks
run in the actual runtime.

### CRITICAL: Pallet Feature Flags for Benchmarking

When using other pallets in benchmarking, you MUST:

1. Add them to Cargo.toml with proper features:

```toml
[dependencies]
pallet-notaries = { path = "../notaries", default-features = false }

[features]
std = [
  "pallet-notaries/std", # MUST include /std
  # ... other std features
]
runtime-benchmarks = [
  "pallet-notaries/runtime-benchmarks", # MUST include if pallet has benchmarks
  # ... other benchmark features
]
```

2. In benchmarking, add type constraints to T:

```rust
#[benchmark]
fn my_benchmark() -> Result<(), BenchmarkError>
where
  T: pallet_notaries::Config + pallet_price_index::Config,
{
  // Now you can access the pallet's storage
  pallet_notaries::ActiveNotaries::<T>::put(notaries);

  Ok(())
}
```

### 2. NEVER assume mock patterns work in runtime

```rust
// ❌ WRONG - Mock patterns from tests don't work in WASM
StaticProvider::set(value); // parameter_types can't be modified in WASM

// ✅ CORRECT - Use direct storage manipulation or provider traits
pallet_price_index::Current::<T>::put(price_data);
```

## ✅ CORRECT PATTERNS (USE THESE)

### 1. Simple benchmark setup

```rust
#[benchmark]
fn extrinsic_name() -> Result<(), BenchmarkError> {
  // Direct import approach (preferred)
  pallet_price_index::Current::<T>::put(price_data);

  // OR provider trait approach (if needed for circular deps)
  T::BitcoinUtxoTracker::set_for_benchmark(utxo_id, utxo_ref, satoshis, watch_until);

  let caller = create_funded_account::<T>("caller", 1);

  #[extrinsic_call]
  _(RawOrigin::Signed(caller), arg1, arg2);

  Ok(())
}
```

### 3. Environment differences

- **Tests**: Use mock implementations
- **Runtime benchmarks**: Use real pallet implementations, WASM execution
- **Weight generation**: Runs against compiled WASM runtime

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

### Provider Pattern for Cross-Pallet Benchmark State

**When to use `set_for_benchmark` methods on provider traits:**

This pattern should be used as a **last resort** when ALL of the following conditions are met:

1. **Circular dependency** - Direct pallet imports would create circular dependencies
2. **Private storage** - The storage you need to manipulate is not public
3. **Benchmark-only** - The state setup is ONLY needed for benchmarking
4. **Cross-pallet requirement** - You need to set up state in another pallet for realistic
   benchmarks

**Simple approach:**

1. Can you import the other pallet directly? Use direct storage access (preferred)
2. Circular dependency? Use provider trait pattern as last resort

**Example - BitcoinUtxoTracker (necessary due to circular dependency):**

```rust
// In primitives/src/providers.rs
pub trait BitcoinUtxoTracker {
  fn watch_for_utxo(...) -> Result<(), DispatchError>;
  fn get(utxo_id: UtxoId) -> Option<UtxoRef>;

  #[cfg(feature = "runtime-benchmarks")]
  fn set_for_benchmark(utxo_id: UtxoId, utxo_ref: UtxoRef, satoshis: Satoshis, watch_until: BitcoinHeight);
}

// In bitcoin_utxos/src/lib.rs
impl<T: Config> BitcoinUtxoTracker for Pallet<T> {
  #[cfg(feature = "runtime-benchmarks")]
  fn set_for_benchmark(utxo_id: UtxoId, utxo_ref: UtxoRef, satoshis: Satoshis, watch_until: BitcoinHeight) {
    // Direct storage manipulation for benchmarking only
    <UtxoIdToRef<T>>::insert(utxo_id, &utxo_ref);
    <LockedUtxos<T>>::insert(&utxo_ref, &utxo_value);
    // ... other storage setup
  }
}

// In bitcoin_locks benchmarking
T::BitcoinUtxoTracker::set_for_benchmark(utxo_id, utxo_ref, satoshis, watch_until);
```

**Why this was unavoidable:**

- bitcoin_locks → bitcoin_utxos → bitcoin_locks (circular dependency)
- UtxoIdToRef storage is not public
- Needed only for benchmark state setup

### Direct Pallet Storage Access (Preferred Pattern)

```rust
// Access storage directly (make sure it's `pub` not `pub(super)`)
pallet_notaries::ActiveNotaries::<T>::put(notaries);
pallet_notaries::NotaryKeyHistory::<T>::insert(notary_id, key_history);
```

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
3. **For cross-pallet storage**: Use direct imports (check trait bounds)
4. **For signatures**: Generate real keypairs, never bypass verification
5. **For economic functions**: Set up realistic price scenarios
6. **For dynamic data**: Pass counts as weight function parameters
7. **Never manually edit**: Generated weight files except for zero-weight placeholders
8. **Always verify**: Function names match their weight functions
9. **Set up tick/frame state properly**: Avoid historical data explosion
10. **For inherent pallets**: Manually simulate `on_finalize` state cleanup
11. **For voting/consensus**: Use same data for merkle root generation and proof validation
