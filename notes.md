# Extracting Rust to Lean using Aeneas - Process Notes

## Project: SparsePostQuantumRatchet (spqr)
- Rust crate, edition 2021, rust-version 1.83.0
- Dependencies include: curve25519-dalek, hax-lib, libcrux-ml-kem, prost, sha2, etc.
- Already has hax proof infrastructure (`proofs/` dir, `hax-lib` dependency, `hax.py`)

## Goal
Extract the Rust code to Lean 4 using Aeneas for formal verification.

## Steps

### 1. Fork and clone (done)
- Forked the repo on GitHub
- Cloned locally and opened in VSCode

### 2. Initialize aeneas-config.yml
- Ran `npx aeneas-cli init` (interactive wizard)
- Set crate directory to `.` (current dir)
- Aeneas pinned to `main` branch of `https://github.com/AeneasVerif/aeneas.git`
- Charon preset: `aeneas`
- Aeneas options: `loops-to-rec`, `split-files`
- Output destination: `LeanOutput/`
- Fixed crate name from `SparsePostQuantumRatchet` to `spqr` (must match `Cargo.toml` package name)

### 3. Configure module-by-module extraction
- Listed all crate modules in `aeneas-config.yml` under `charon.start_from`
- Modules (in planned extraction order):
  1. `spqr::util` (active)
  2. `spqr::kdf`
  3. `spqr::chain`
  4. `spqr::encoding`
  5. `spqr::authenticator`
  6. `spqr::serialize`
  7. `spqr::incremental_mlkem768`
  8. `spqr::v1`
  9. `spqr::proto`
- All commented out except `spqr::util` to start with the simplest module

### 4. Install Aeneas and attempt extraction
- Ran `npx aeneas-cli install` to build Aeneas locally in `.aeneas/`
- Ran `npx aeneas-cli extract` for each module

### 5. Extraction results (module by module)

| Module | Charon | Aeneas | Notes |
|--------|--------|--------|-------|
| `spqr::util` | OK | OK | Clean extraction |
| `spqr::kdf` | Hangs | - | Charon runs indefinitely |
| `spqr::chain` | Hangs | - | Charon runs indefinitely |
| `spqr::encoding` | OK | Partial | Funs.lean generated but 7 functions have `sorry` |
| `spqr::authenticator` | OK | ? | Currently active alongside util |
| `spqr::serialize` | Not tried | - | |
| `spqr::incremental_mlkem768` | Not tried | - | |
| `spqr::v1` | Not tried | - | |
| `spqr::proto` | Not tried | - | |

### 6. Current output (util + authenticator + encoding)
- `LeanOutput/Types.lean` - 267 lines, type definitions extracted cleanly
- `LeanOutput/Funs.lean` - 3807 lines, function definitions
- `LeanOutput/TypesExternal_Template.lean` - external type stubs
- `LeanOutput/FunsExternal_Template.lean` - external function stubs

### 7. Functions with `sorry` (Aeneas couldn't fully extract)
1. `cpufeatures` init_inner (external crate inline asm)
2. `GF16.div_impl` (GF16 division, src/encoding/gf.rs:548)
3. `Poly.compute_at` (polynomial evaluation, src/encoding/polynomial.rs:255)
4. `Poly.from_complete_points` (src/encoding/polynomial.rs:291)
5. `PolyEncoder.from_pb` (protobuf deserialization)
6. `PolyDecoder.from_pb` (protobuf deserialization)
7. `PolyDecoder.decoded_message` (src/encoding/polynomial.rs:883)

### 8. Detailed error analysis for `spqr::encoding`

The encoding module has 3 submodules:
- `encoding::gf` - Galois field GF(2^16) arithmetic
- `encoding::polynomial` - Polynomial operations over GF(2^16)
- `encoding::round_robin` - Round-robin encoding

#### Aeneas errors (9 unique errors, from extraction log):

**Error 1: Arrow types not supported** (`gf.rs:371-372`)
- `static TOKEN: LazyLock<use_accelerated::InitToken> = LazyLock::new(use_accelerated::init);`
- The `cpufeatures::new!` macro generates function pointer types. Aeneas does not support arrow/function-pointer types.
- **Fix**: Mark `spqr::encoding::gf::check_accelerated` as `opaque` in config. This is CPU feature detection for hardware-accelerated GF multiplication (PCLMULQDQ/PMULL) - not needed for verification.

**Error 2: Invalid input for unop cast** (`gf.rs:372`)
- Same `cpufeatures` code path, cast from function to InitToken.
- **Fix**: Same as above - make opaque.

**Error 3: Iterator methods missing in Lean library** (warnings)
- `zip`, `map`, `collect` methods on `Iterator` trait not modeled in Aeneas Lean library.
- These are used in polynomial operations.
- **Fix**: May need to rewrite Rust to avoid iterator combinators, or accept `sorry` for those functions.

**Error 4: Internal errors** (4 occurrences during file generation)
- "Internal error: please file an issue" - likely cascading from the arrow type errors.

#### Iterative fixes applied

**Attempt 1**: Mark `check_accelerated` as `opaque`
- Reduced from 9 to 7 unique errors, but arrow type errors persisted (Aeneas still sees type signatures of opaque items)

**Attempt 2**: Use `exclude` instead of just `opaque` for `check_accelerated`
- Arrow type errors for `TOKEN` eliminated
- But `mul2_u16` and `MulAssign` still errored because they reference excluded `TOKEN`

**Attempt 3**: Also `exclude` `accelerated` module + mark `mul2_u16` and `MulAssign` as `opaque`
- Down to **5 unique errors** (from original 9)
- GF accelerated code fully handled
- Remaining errors are all "Unreachable" in `polynomial.rs` - iterator combinator patterns

**Attempt 4** (failed): `RUSTFLAGS="--cfg hax"` to use hax-friendly code paths
- The `cfg(hax)` flag is for hax toolchain, not Charon. It triggered compilation errors in `core-models` dependency.

#### Current state (5 remaining errors)
All remaining `sorry` functions use iterator combinators (`zip`, `map`, `collect`) unsupported by Aeneas Lean library:

| Function | File:Line | Root cause |
|----------|-----------|------------|
| `GF16.div_impl` | gf.rs:548 | Tuple destructuring in loop body |
| `Poly.compute_at` | polynomial.rs:255 | Iterator zip+fold pattern |
| `Poly.from_complete_points` | polynomial.rs:291 | Iterator map+collect pattern |
| `PolyEncoder.from_pb` | polynomial.rs:569 | Iterator map+collect pattern |
| `PolyDecoder.from_pb` | polynomial.rs:808 | Iterator map+collect pattern |
| `PolyDecoder.decoded_message` | polynomial.rs:883 | Iterator zip+map+collect pattern |

#### Attempt 5: Mark functions as `opaque` in Charon config
- Tried both simplified names (`PolyEncoder::from_pb`) and full Charon `{impl}` syntax
- Charon `opaque` flag does NOT prevent Aeneas from encountering "Unreachable" errors during its prepasses
- The errors happen in Aeneas's interpretation phase, before opaque/transparent distinction matters
- **Conclusion**: `opaque` in Charon config cannot suppress these Aeneas-internal errors

#### Attempt 6: Quantify impact of `opaque` list on iterator-combinator functions
- **With opaque** (6 entries): 5 errors (4 unique), 172 opaque functions, 136 transparent, **5 sorry** in Funs.lean
- **Without opaque** (entries commented out): 6 errors (5 unique), 174 opaque functions, 139 transparent, **6 sorry** in Funs.lean
- The only difference: `decoded_message` gains a sorry without the opaque entry (it was previously treated as opaque and skipped)
- The other 5 functions (`div_impl`, `compute_at`, `from_complete_points`, `PolyEncoder::from_pb`, `PolyDecoder::from_pb`) produce sorry regardless
- **Conclusion**: The opaque list has minimal effect - saves exactly 1 function from sorry. The real fix is refactoring the Rust code. Keeping the opaque entries is harmless but not a substitute for the refactoring.

#### Current practical state
- Lean files ARE generated despite exit code 1 (Aeneas treats errors as non-fatal for output)
- 5 functions have `sorry` bodies - these need manual Lean implementations or Rust rewrites
- All other functions (130+ transparent functions) extracted successfully

#### Why these 5 functions matter
These are not peripheral helpers - they are the mathematical core and serialization backbone:

- **`GF16.div_impl`** - Implements GF(2^16) division (via `a^(2^16-2)` exponentiation). Powers all `Div`/`DivAssign` operators. Used in Lagrange interpolation, the foundation of the erasure coding.
- **`Poly.compute_at`** - Polynomial evaluation at a point. Called by `decoded_message` and the encoder's `point_at`. Core to the encoding/decoding correctness.
- **`Poly.from_complete_points`** - Builds Lagrange interpolation polynomials from precomputed points. Used by `point_at` to initialize the encoding scheme.
- **`PolyEncoder.from_pb` / `PolyDecoder.from_pb`** - Reconstruct encoder/decoder state from protobuf. Used in every V1 protocol state deserialization path (`v1/chunked/send_ek/serialize.rs`, `v1/chunked/send_ct/serialize.rs`).
- **`PolyDecoder.decoded_message`** - Reassembles a message from erasure-coded chunks. Called by the V1 state machine in `send_ct.rs` and `send_ek.rs`. The whole point of the encoding layer.

#### Plan: Refactor these 5 functions for Aeneas extraction
We will return to refactor these functions to replace iterator combinators with explicit index-based loops:

| Function | Current pattern | Refactor to |
|----------|----------------|-------------|
| `GF16.div_impl` | `(a, b) = mul2_u16(...)` tuple destructuring | Separate assignments in loop |
| `Poly.compute_at` | `iter().zip().` fold | `for i in 0..len` with index access |
| `Poly.from_complete_points` | `iter().map().collect()` | `for` loop pushing to pre-allocated Vec |
| `PolyEncoder.from_pb` | `iter().enumerate()` + `chunks_exact` | `for i in 0..len` with manual chunking |
| `PolyDecoder.from_pb` | `chunks_exact` iteration | `for i in (0..len).step_by(4)` or index loop |
| `PolyDecoder.decoded_message` | `binary_search` + `Option::as_ref` | Keep structure but simplify iterator usage |

These refactors should be semantically equivalent and testable against the existing test suite. They could potentially be upstreamed since the project already avoids iterators in some places for hax compatibility (see comment in `encoding.rs:49`).

---

### 9. Refactoring encoding module for clean extraction

Applied minimal Rust changes to eliminate all `sorry` functions. All 53 tests pass after each change.

| Function | Problem | Minimal fix |
|----------|---------|-------------|
| `GF16.div_impl` | Opaque `mul2_u16` returns tuple; Aeneas can't handle tuple access from opaque fn | Replace `mul2_u16` call with direct `*` operator (already extracted) |
| `Poly.compute_at` | `.iter().zip()` pattern | Index-based `for i in 0..len` loop |
| `Poly.lagrange_sum` | `.iter().zip()` pattern | Index-based loop |
| `Poly.from_complete_points` | `.iter().map().collect()` in fallback branch | Explicit `for` loop with `push` |
| `Poly.deserialize` | `.chunks_exact(2)` | `while j+2 <= len` with manual indexing |
| `PolyEncoder.from_pb` | `?` inside `if-else` binding + `core::array::from_fn` closure | Flatten to sequential `if`/`return`; explicit array literal init |
| `PolyDecoder.from_pb` | `return Err` inside `for` loop (any early return in loop is "Unreachable") | Hoist validation to unrolled `if` before loop |

Key Aeneas limitations discovered:
1. **Iterator combinators** (`.zip()`, `.map()`, `.collect()`) - no Lean model, use index loops
2. **Early return inside loops** (`return Err(...)` in `for`) - triggers "Unreachable", hoist validation before loop
3. **`core::array::from_fn`** with closures - use explicit array literals
4. **Tuple access from opaque functions** (`.0`, `.1` on opaque return) - avoid calling opaque functions that return tuples
5. **`chunks_exact()`** - use `while` loop with manual index stepping

**Result: 0 errors, 0 sorry, 126 transparent functions extracted cleanly.**

---

## Status: Active

Current state:
- Aeneas installed and configured (`aeneas-config.yml`)
- `spqr::util`, `spqr::serialize`, `spqr::encoding` all extract cleanly (0 sorry)
- `spqr::kdf`, `chain`, `authenticator`, `v1`, `proto` - Charon hangs (external crypto dependency graphs)
- `spqr::incremental_mlkem768` - Charon OK, Aeneas 2 cosmetic errors, **0 sorry**, 139 transparent functions

### 10. Investigating `spqr::incremental_mlkem768`

**Charon**: Completes successfully (~10s). No hanging — `libcrux_ml_kem` dependency graph is tractable.

**Aeneas errors (2, both from same root cause)**:
1. `log::__private_api::log` — "Internal error" from `log` crate macros.rs. The `log::info!` and `log::warn!` macros expand to function pointer types (arrow types) that Aeneas doesn't support.
2. Cascading: body of `potentially_fix_state_incorrectly_encoded_by_libcrux_issue_1275` ignored due to error 1.

**Root cause**: The function uses `#[cfg(not(hax))] log::info!(...)` and `log::warn!(...)` (lines 126, 132). Charon doesn't set the `hax` cfg, so these calls are compiled in. The `log` crate's internal `__private_api::log` function uses arrow types.

**Fix**: Added `log` to the `exclude` list in `aeneas-config.yml`. This reduced errors from 3 to 2. The function is already `#[hax_lib::opaque]` in the source, so Aeneas correctly generates an axiom (opaque declaration) instead of a sorry body.

**Result**: 0 sorry, 139 transparent functions, 203 opaque functions. The 2 remaining errors are cosmetic — they don't affect the output quality.

**Warnings** (non-blocking):
- Unknown types with region parameters: `core::slice::iter::Chunks`, `log::Metadata`, `log::Record`, `core::panic::location::Location`
- Missing Iterator methods: `map`, `collect` (not used by incremental_mlkem768's transparent functions)

### Next steps
1. ~~Refactor the sorry functions in encoding~~ **DONE** - 0 sorry
2. ~~Clean up opaque list~~ **DONE**
3. ~~Investigate Charon hanging modules~~ **BLOCKED** - Charon hangs at the MIR analysis level (inside charon-driver/rustc), before `--opaque`/`--exclude`/`--start-from` flags take effect. Tested with `kdf` (simplest hanging module, only 2 functions). Adding `hkdf::*`, `sha2::*`, etc. to opaque/exclude lists has no effect. Even `--start-from "spqr::kdf::hkdf_to_vec"` with `--exclude "hkdf"` hangs. Root cause: Charon must process the full rustc MIR including all type resolution for generic trait hierarchies (`hkdf::Hkdf<sha2::Sha256>` -> `Digest` -> `CoreWrapper` -> `BlockSizeUser` -> `generic-array` -> `typenum` -> ...). This is a Charon upstream limitation - needs either Charon improvements or `cfg(hax)`-gated alternative code paths that avoid referencing external crypto crates.
4. ~~Try `spqr::incremental_mlkem768`~~ **DONE** - 0 sorry, excluded `log` crate
5. Set up a Lean project to actually build/typecheck the generated output

### Observations
- Charon hanging on `kdf` and `chain` may be due to heavy use of external crypto crates (hkdf, libcrux-hmac) pulling in large dependency graphs
- `sorry` functions tend to involve: loops with complex control flow, inline asm (cpufeatures), or iterator patterns
- The `encoding` module extracted the most code successfully since it's mostly pure math (GF16 arithmetic, polynomial ops)
- Arrow/function-pointer types are a hard Aeneas limitation - must use `exclude` (not just `opaque`) to fully remove them
- Missing Iterator method models (`zip`, `map`, `collect`) in Aeneas Lean library is the primary remaining blocker
- `RUSTFLAGS="--cfg hax"` doesn't work because downstream deps (core-models) fail to compile with hax cfg
