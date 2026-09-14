# 01 - Internal Security Review

Milestone 2 deliverable: structured review of stack/heap sizing safety across circuit shapes, panic/overflow handling, and revert-data parity.

## What it is

A line-by-line review of `generator/honk-verifier/static/src/honk/*.rs` and the templated `main.rs.tmpl`/`sumcheck.rs.tmpl`/`vk.rs.tmpl`, plus `Cargo.toml`'s release profile (`overflow-checks = false`, `panic = "abort"`), against the four axes above. Every finding below is grounded in specific file/line references, cross-checked against the Solidity/REVM reference (`HonkVerifier.sol`) call-by-call rather than reasoned about in the abstract, and either fixed or explicitly ruled out with a concrete argument - nothing here is a generic checklist pass.

## Findings

### Finding 1 - Unprotected field inversions could panic into an expensive, opaque trap

7 call sites in `shplemini.rs` (lines 55, 78, 79, 83, 170, 171) and `main.rs.tmpl`'s `compute_public_input_delta` (line 243) used `.inverse().unwrap()` on Fiat-Shamir-transcript-derived values. `Fr::inverse()` returns `None` if the input is exactly zero; `sumcheck.rs.tmpl`'s `compute_next_target_sum` already handled the identical situation safely via `.unwrap_or(Fr::zero())`, but these 7 sites didn't follow that established pattern. A panic anywhere in this runtime hits the custom `#[panic_handler]`, which executes the RISC-V `unimp` instruction directly - an expensive, opaque trap rather than a clean, cheap revert.

Cryptographically negligible (~1/p, p ~ 2^254) under honest operation, since these values are keccak256-derived transcript challenges, not attacker-choosable directly - but real, and free to fix by matching an already-correct pattern already present in the same codebase.

**Fix**: all 7 sites now use `.unwrap_or(Fr::zero())`, matching `sumcheck.rs.tmpl`. Commit `3aebda1`.

**Verification**: not independently testable - triggering this branch requires a keccak256 preimage that hashes to exactly zero mod Fr, which is computationally infeasible, not just hard to construct a test for. Verified via regression only: the full 35/35 equivalence suite (7 fixtures x 5 test vectors, comparing byte-for-byte against the REVM reference) passes unchanged after the fix.

### Finding 2 - Precompile call-failure status silently discarded (modexp, ecPairing)

Traced every precompile call site in both the Solidity reference and the Rust runtime:

| Precompile | Solidity reference behavior on a failed call | Rust behavior (before fix) |
| --- | --- | --- |
| `modexp` (0x05) - `FrLib.invert()`, backs every field inversion | `if iszero(success) { revert(0, 0) }` | Discarded (`let _ = api::call(...)`); silently returned `Some(zero)` - a fabricated "inverse" - instead of `None` |
| `ecPairing` (0x08) - final pairing check | Checks `success`; would also revert via an `abi.decode` panic on the empty return data a failed call leaves behind | Discarded; failed call leaves output zeroed, so the pairing check correctly reports `false`, but via `ShpleminiFailed()` rather than matching Solidity's actual revert on that path |
| `ecAdd`/`ecMul` (0x06/0x07) - `batchMul` MSM loop | Accumulates a local `success` flag across the loop but **never checks or reverts on it** - dead code in the reference itself | Discarded, matching the reference's own gap |

None of these failure paths are reachable via crafted proof bytes (only a genuine host-call fault triggers them - gas exhaustion, runtime error), so there's no soundness/exploitability concern. This is about matching the reference's actual behavior, not closing an attack.

**Fix**: `modexp` (`fr.rs`) and `ecPairing` (`g1.rs`) now check the call's success and revert with empty data (`api::return_value(ReturnFlags::REVERT, &[])`) on failure, matching Solidity's own behavior at those two sites. `ecAdd`/`ecMul` deliberately left unchanged - the reference discards success there too, so leaving ours as-is is the truer match; adding a check would make us *stricter* than the reference, not more aligned with it. Commit `508691a` (comment cleanup in `5685b02`).

**Verification**: same as Finding 1 - not independently testable (host-call failure isn't triggerable from a test), verified via regression only: full 35/35 equivalence suite unchanged.

### Finding 3 - Unbounded heap allocation from raw calldata length

`handle_verify()` (`main.rs.tmpl`) sized its first heap allocation directly off the raw incoming calldata length, with only a lower-bound check (`< 4 bytes`) before it:

```rust
let data_len = length.saturating_sub(4);
let mut data = alloc::vec![0u8; data_len];   // allocated BEFORE any format validation
```

`length` is `api::call_data_size()` - fully attacker-controlled. Any caller could pad a `verify()` call with an arbitrarily large garbage blob, no valid proof or even well-formed ABI encoding required, to force this allocation past the circuit's fixed heap capacity. Every *other* heap allocation in the runtime (7 of 8 total, traced individually) happens only after `parse_verify_args` has validated the input shape, so all of them are bounded by compile-time-fixed constants (`NUM_PUB`, `PROOF_SIZE`) - this was the only one sized by raw, unvalidated attacker input.

**What actually happens on allocation failure**, confirmed by reading the pinned toolchain's own source (`alloc/src/alloc.rs`, `nightly-2026-04-20`): `SimpleAlloc` (the bump allocator) fails safely, returning null rather than triggering UB. But this is a pure `#![no_std]` binary with no custom `#[alloc_error_handler]` defined, so Rust's default OOM path calls `panic_nounwind_fmt(...)`, which hits this codebase's own panic handler - the same `unimp` trap as Finding 1.

**Reproduced both sides live**, not just reasoned about: deployed a build without the fix to a local devnet and sent a genuine 100KB-oversized `verify()` call against it - result was an opaque `ContractTrapped` error with no diagnostic reason. Redeployed with the fix and sent the identical call - result was a clean `INPUT_TOO_LARGE` revert.

**Fix**: reject calldata above `2x` the circuit's exact well-formed size before allocating. The `2x` margin is deliberate, not arbitrary - a tight bound (exact expected size) broke a real equivalence-test vector on the first attempt: `zero-pub-input`'s "wrong public input" test sends a public-inputs array one element longer than expected, specifically to test that `parse_verify_args` correctly classifies it as `PublicInputsLengthWrong()` rather than something else - my tight check was intercepting it before it got that far. Widened to `2x`, confirmed via computed numbers across all 7 fixtures that this stays at roughly 40-60% of total heap capacity in every case (comfortable margin, nowhere near attack scale), reran the full 35/35 suite clean. Commit `e160ebe`.

**A note on Solidity parity for this one specifically**: unlike Findings 1 and 2, this fix does not restore parity with the reference - it diverges from it, deliberately. Solidity's `bytes calldata`/`bytes32[] calldata` parameters are zero-copy: solc's generated decoder never copies the full calldata into memory, it only reads the specific ranges the function body touches. A genuinely valid proof padded with 100KB of trailing garbage would **succeed normally in Solidity** (the padding is simply never read) and now **reverts with `INPUT_TOO_LARGE` in ours**. This divergence exists because our runtime's architecture (copy calldata into a heap buffer, then parse) is fundamentally different from Solidity's lazy calldata access, not because of a choice to diverge from the reference for its own sake. No legitimate ABI encoder produces trailing padding, so this only rejects a degenerate input shape no real caller would send - but it is a genuine, documented behavioral difference, not a restored match.

**Verification**: the only one of the three findings that was directly, empirically testable (calldata size is externally controllable, unlike the transcript-zero and host-call-failure cases above) - confirmed both the pre-fix trap and the post-fix clean revert live on a local devnet, plus full 35/35 regression.

## Additional axes investigated, no fix needed

### Stack sizing across circuit shapes

Confirmed zero recursion anywhere in the runtime (checked every function in every module). Rust has no VLAs, so every stack frame's size is fixed at compile time. Combined, stack usage per call is structurally independent of circuit shape - it cannot grow with `LOG_N`, `numPublicInputs`, or anything runtime-determined. If the fixed 64KB (`polkatool link --min-stack-size 65536`) is safe for the 7 tested shapes, it is safe for every shape, by construction - not just "safe so far."

### Heap formula generalization (`calculateHeapKB` in `generate.ts`)

Traced all 8 heap allocation sites in the runtime. `NUMBER_OF_ENTITIES` (40) and `CONST_PROOF_SIZE_LOG_N` (28) are fixed Barretenberg protocol maxima, not circuit-dependent - confirmed against `proof.rs`'s own array-size declarations. `numPublicInputs` is the only real per-circuit variable feeding the formula, and it scales linearly and correctly through it. 7 of 8 allocation sites are safely bounded by compile-time-fixed constants once past `parse_verify_args`'s validation; the 8th was Finding 3, now fixed.

### Overflow handling (`overflow-checks = false` in the release profile)

Field arithmetic (`Fr`'s `Add`/`Sub`/`Mul`/`Neg` in `fr.rs`, which the large majority of all arithmetic in the codebase - including all 26 sub-relations in `relations.rs` - routes through) uses `wrapping_add`, `wrapping_mul`, `overflowing_add`, and `overflowing_sub` exclusively, with explicit carry/borrow propagation (a standard CIOS Montgomery multiplication implementation). This is correct by construction and entirely unaffected by the `overflow-checks` profile flag, since it never relies on Rust's built-in overflow panic/wrap behavior in the first place.

Every array index and buffer size downstream of `parse_verify_args` (`relations.rs`, `sumcheck.rs.tmpl`, `transcript.rs`, `shplemini.rs`, `proof.rs`, `vk.rs.tmpl`) is either a literal constant, a `const` expression (Rust always overflow-checks `const` evaluation at compile time regardless of the release profile), or bounded by a fixed-size array sized by a protocol/VK constant - never by attacker-supplied data. The only attacker-facing arithmetic in the whole codebase is `parse_verify_args` (already using `checked_add`/`checked_mul` throughout) and `handle_verify`'s calldata-length handling (Finding 3, now fixed).

One minor, non-security note for completeness: `vk.rs.tmpl`'s `hex32()` decoder does unchecked length arithmetic (`64 - len`) on VK hex strings - but those strings are baked into the source at generation time from the (trusted) Solidity VK, not runtime calldata, so a malformed value there would be a generator/build-time bug, not a deployed-contract vulnerability. Worth a one-line mention in the write-up, not a fix.

## Commits

All on `main`:

- `3aebda1` - Fix unprotected field inversions to match sumcheck's safe pattern (Finding 1)
- `508691a` - Check precompile call success for modexp and ecPairing (Finding 2)
- `5685b02` - Make status=0 quirk comment self-contained, drop FINDINGS reference (unrelated cleanup, surfaced while fixing Finding 2)
- `e160ebe` - Reject oversized verify() calldata before the heap-copy allocation (Finding 3)

## Verification philosophy

Findings 1 and 2 are structurally untestable by construction, not just hard to test: Finding 1's failure branch requires a keccak256 preimage colliding with zero mod Fr (computationally infeasible), and Finding 2's requires a genuine host-call fault with no available mocking layer for `pallet_revive_uapi` in this codebase. Both were verified the only way actually available - code-level reasoning plus confirmed non-regression on the full 35/35 equivalence suite. Finding 3 was different: calldata size is externally controllable, so both the pre-fix and post-fix behavior were reproduced live against a real deployed contract on a local devnet, not just argued from source.
