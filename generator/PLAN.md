# PolkaVM Verifier Generator Tool

## Goal

A TypeScript CLI tool that takes any `HonkVerifier.sol` (from `bb write_solidity_verifier`) and produces a complete, ready-to-build PolkaVM Rust verifier contract. This eliminates manual work when supporting new Noir circuits.

**Input:** `HonkVerifier.sol` (single file, contains everything circuit-specific)
**Output:** A complete buildable PolkaVM verifier project (Rust source + config + TS scripts)

**Usage:**
```
npx ts-node generator/generate_verifier.ts \
  --sol path/to/HonkVerifier.sol \
  --out path/to/output-project/
```

## What's Circuit-Specific vs Generic

**Circuit-specific (extracted from HonkVerifier.sol):**
- `N` / `LOG_N` -- circuit size (e.g., 32 / 5)
- `NUMBER_OF_PUBLIC_INPUTS` -- public input count (e.g., 1)
- 27 G1 commitment points in the VK (ql, qr, qo, q4, qm, qc, qArith, qDeltaRange, qElliptic, qAux, qLookup, qPoseidon2External, qPoseidon2Internal, s1-s4, id1-id4, t1-t4, lagrangeFirst, lagrangeLast)

**Generic (same for ALL UltraHonk circuits):**
- `fr.rs` -- field arithmetic
- `fr_utils.rs` -- keccak, split_challenge
- `g1.rs` -- EC precompile calls
- `proof.rs` -- proof parsing (CONST_PROOF_SIZE_LOG_N=28, NUMBER_OF_ENTITIES=40, etc.)
- `relations.rs` -- 26 sub-relation formulas
- `shplemini.rs` -- KZG batch opening
- `transcript.rs` -- Fiat-Shamir challenge generation
- G2 generator and G2 KZG SRS constants in vk.rs

## Tool Steps

### Step 1: Parse HonkVerifier.sol
Extract via regex:
- `N` and `LOG_N` from top-level constants
- `NUMBER_OF_PUBLIC_INPUTS` from top-level constant
- 27 G1 points from `HonkVerificationKey.loadVerificationKey()` -- each has `x: uint256(0x...)` and `y: uint256(0x...)`

### Step 2: Generate `vk.rs`
Template the VK file with the extracted points. Structure is fixed -- only hex values change. G2 constants are hardcoded (same for all circuits).

### Step 3: Generate `contract.rs`
Template with:
- `const LOG_N: usize = {extracted};`
- `if arr_len != {NUMBER_OF_PUBLIC_INPUTS}` in `parse_verify_args`
- Public input parsing loop for the correct count

### Step 4: Generate `sumcheck.rs`
The sumcheck has hardcoded unrolled rounds (to avoid PolkaVM loop codegen issues). Generate `LOG_N` rounds of:
```rust
// Round {i}
{
    let u = &proof.sumcheck_univariates[{i}];
    if !check_sum(u, round_target) { return false; }
    let ch = t.sumcheck_u_challenges[{i}];
    round_target = compute_next_target_sum(u, ch);
    pow_partial_evaluation = partially_evaluate_pow(t.gate_challenges[{i}], pow_partial_evaluation, ch);
}
```

### Step 5: Copy generic files
Copy unchanged: `fr.rs`, `fr_utils.rs`, `g1.rs`, `proof.rs`, `relations.rs`, `shplemini.rs`, `transcript.rs`
Also copy: `.cargo/config.toml`, `Cargo.toml`, `rust-toolchain.toml`, `riscv64emac-unknown-none-polkavm.json`, `.gitignore`
Also copy TS scripts: `deploy.ts`, `quick_test.ts`, `test_valid_and_invalid.ts`, `package.json`

### Step 6: Build (optional)
Run `cargo build --release` and `polkatool link --strip` in the output directory.

## File Structure

```
generator/
├── generate_verifier.ts      # The main generator script
├── template/                 # Generic source files (copied as-is to output)
│   ├── .cargo/config.toml
│   ├── Cargo.toml
│   ├── rust-toolchain.toml
│   ├── riscv64emac-unknown-none-polkavm.json
│   ├── src/
│   │   ├── fr.rs
│   │   ├── fr_utils.rs
│   │   ├── g1.rs
│   │   ├── proof.rs
│   │   ├── relations.rs
│   │   ├── shplemini.rs
│   │   └── transcript.rs
│   ├── deploy.ts
│   ├── quick_test.ts
│   ├── test_valid_and_invalid.ts
│   ├── package.json
│   └── .gitignore
├── package.json
└── tsconfig.json
```

Three files are GENERATED (not copied): `src/vk.rs`, `src/contract.rs`, `src/sumcheck.rs`.

## Implementation Status

All steps complete. Verified 2026-04-21.

- [x] Generator project structure (`package.json`, `tsconfig.json`)
- [x] Template files copied (7 Rust sources + configs + TS scripts)
- [x] `generate_verifier.ts` — parses HonkVerifier.sol, generates vk.rs/contract.rs/sumcheck.rs
- [x] Optional `--build` flag for cargo build + polkatool link
- [x] Verified: generated output matches existing `honk-verifier-polkavm/src/` (vk.rs and sumcheck.rs identical, contract.rs identical minus debug function)
- [x] Verified: compiled .polkavm binary is byte-for-byte identical (55,683 bytes)

### Cross-circuit verification
- [x] Simple circuit: LOG_N=5, 1 public input — 5/5 tests pass
- [x] 4-public-input circuit: LOG_N=5, 4 public inputs — 3/3 tests pass
- [x] ZK email circuit (zkemail/ens-contracts): LOG_N=19, 155 public inputs — 3/3 tests pass

## TODO

### Build tooling
- [ ] **PvmBuilder adoption** — `cargo-pvm-contract-builder` would give `cargo build --release` → `.polkavm` in one step. BLOCKED: upstream doesn't support `--min-stack-size` (needed for large circuits). Options: contribute PR to github.com/paritytech/cargo-pvm-contract, or wait for upstream.

### Testing
- [ ] **Make `cargo test` work on x86** — `_lib_test.rs` and `tests/verify_test.rs` are in place but `.cargo/config.toml` forces RISC-V target. Needs wrapper script or config override to run tests on host.
- [ ] **Pure Rust EC fallbacks for x86 testing** — implement ecAdd/ecMul/ecPairing in Rust, swap via `#[cfg(test)]`. Enables full end-to-end x86 testing including shplemini. Big effort (~30KB of BN254 code). Could adapt from PolkaZK's `bn254/` module.

### Optimizations
- [ ] **Streaming keccak in transcript** — replace `hash_u256s` Vec allocations with streaming `Keccak::update()` calls. Reduces peak heap. Less critical now with picoalloc (frees properly) but still cleaner.
- [ ] **Tighter heap formula** — picoalloc frees memory, so the conservative formula (designed for bump allocator) oversizes the heap. Could profile actual peak usage and reduce.

### Nice-to-have
- [ ] **alloy-core ABI encoding** — replace manual ABI decode with `sol!` macro (like PolkaZK). Cleaner code, less error-prone.
- [ ] **Remove `do_verify_diag` from reference contract** — debug function in `contracts/honk-verifier/src/main.rs`, not produced by generator. Dead code.
