# Plan: Noir UltraHonk Verifier on PolkaVM — Final Implementation Plan

## Context

The user has Noir circuits (bb v0.84.0, nargo 1.0.0-beta.5). `bb` generates `HonkVerifier.sol` per circuit. Compiling with `resolc` → PVM bytecode too large. Goal: native Rust PolkaVM contract that verifies UltraHonk proofs on Paseo Asset Hub testnet.

The circuit (`circuit/src/main.nr`) is `assert(x != y)` — simple, 1 public input, finalized circuit size 18, LOG_N=5.

Proofs are generated with:
```bash
bb prove --bytecode_path ./target/circuit.json --witness_path ./target/circuit.gz \
         --output_path ./target --oracle_hash keccak --output_format bytes_and_fields
```

---

## Library Research Findings

`zkVerify/ultrahonk_verifier` (crate: `ultrahonk_no_std`) was investigated but **does not work** with bb's output format:
- Library expects VK of **1888 bytes** (28 G1 points + 3×32 header)
- bb v0.84.0 and v0.87.0 both produce **1760 bytes** (26 G1 points + 3×32 header)
- No `bb` CLI flag produces the format the library needs — it was built for an internal Aztec format
- `zkpassport/noir_rs` and `zkmopro/noir-rs` wrap C++ barretenberg FFI — not suitable

**Conclusion: no ready-made Rust UltraHonk verifier is compatible with standard bb CLI output. We translate HonkVerifier.sol directly.**

---

## What HonkVerifier.sol Actually Uses (for reference / PolkaVM port)

| Precompile | Address | Usage | Available on Asset Hub |
|---|---|---|---|
| modexp | 0x05 | Field inversion | ✅ (replace with fr_pow(x, p-2)) |
| ecAdd | 0x06 | Point addition in batchMul | ✅ ~991 gas |
| ecMul | 0x07 | Scalar multiplication in batchMul | ✅ ~991 gas |
| ecPairing | 0x08 | Final pairing check | ✅ ~276 gas/pair |

**PROOF_SIZE = 440 field elements = 14,080 bytes** per proof.
**VK**: 22 named G1 points + circuit metadata.

---

## Project Structure

```
kusama-demo/
├── circuit/                   # Noir circuit + artifacts (bb v0.84.0)
├── poseidon-contract/         # Reference: working PolkaVM contract example
├── honk-verifier-rs/          # Step 1: standard Rust (std) — use ultrahonk_no_std crate
│                              #   — confirm proof verifies before touching PolkaVM
├── polkavm-ark-test/          # Step 2: test ultrahonk_no_std (no_std) on PolkaVM target
│                              #   — if it compiles, use it directly in final contract
└── honk-verifier-polkavm/    # Step 3: final PolkaVM contract
```

---

## Implementation Plan

### Phase 0a: honk-verifier-rs — Verify with ultrahonk_no_std (2-3 days)

Use the `ultrahonk_no_std` crate in a standard Rust binary to confirm end-to-end verification works with our actual proof artifacts:
- `cargo new honk-verifier-rs --bin`
- Add `ultrahonk_no_std` as dependency
- Read `circuit/target/proof` and `circuit/target/public_inputs`
- Read `circuit/target/vk`
- Call `verify()` → confirm `true`
- Test with bad proof → confirm `false`

If the crate doesn't work with our bb v0.84.0 output → fall back to translating HonkVerifier.sol directly.

### Phase 0b: polkavm-ark-test — ultrahonk_no_std on PolkaVM target (2-3 days, parallel)

Since `ultrahonk_no_std` is already `no_std` + arkworks-based, test if it compiles on the PolkaVM RISC-V target:
- Create dir with full PolkaVM project config (`.cargo/config.toml`, `rust-toolchain.toml`, target JSON)
- Add `ultrahonk_no_std` to `Cargo.toml` with `default-features = false`
- Write minimal `src/main.rs`: `#![no_std]`, call the verifier with hardcoded test data
- Run `./build.sh` — does it compile? binary size?

**If it compiles**: the PolkaVM contract is essentially just wrapping `ultrahonk_no_std` in a contract shell — very little custom code needed.
**If it fails**: fall back to translating HonkVerifier.sol using precompile calls for curve ops.

### Phase 1: honk-verifier-polkavm — Final PolkaVM Contract

Built directly from Phase 0b results:

**If ultrahonk_no_std compiled (happy path)**:
- New PolkaVM contract dir
- Wrap `ultrahonk_no_std::verify()` in a contract with ABI-encoded calldata
- Embed VK as constants
- Set `SimpleAlloc` to 256KB
- Deploy and test

**If ultrahonk_no_std failed to compile (fallback)**:
- Translate HonkVerifier.sol to Rust manually (Phases 1-6 from original plan)
- Use precompile calls via `api::call()` for ecAdd/ecMul/ecPairing
- Use `tiny-keccak` for transcript

---

## The Automated Tool (Future)

```
Input:  vk binary (from: bb write_vk)  +  HonkVerifier.sol (for VK point extraction)
        ↓
  Convert VK to format expected by ultrahonk_no_std
        ↓
  Embed in Rust contract constants → src/vk.rs
        ↓
  cargo build + polkatool link
        ↓
Output: verifier.polkavm  (deployable on Paseo Asset Hub)
```

Per-circuit workflow:
1. `nargo compile && bb write_vk && bb prove --oracle_hash keccak`
2. Run tool on VK → generates `src/vk.rs`
3. `./build.sh` → deploy `verifier.polkavm`

---

## Key Files

| File | Role |
|---|---|
| `circuit/target/proof` | Proof bytes (keccak flavor, bytes_and_fields format) |
| `circuit/target/public_inputs` | Public inputs for the proof |
| `circuit/target/vk` | Verification key binary |
| `circuit/target/HonkVerifier.sol` | Fallback reference if crate doesn't work |
| `poseidon-contract/` | PolkaVM project structure reference |

---

## TODO

### honk-verifier-rs (translate HonkVerifier.sol to Rust)
- [x] Created project, confirmed `ultrahonk_no_std` incompatible (VK format mismatch)
- [ ] Add dependencies: `ark-bn254`, `ark-ff`, `ark-ec`, `ark-serialize`, `tiny-keccak`
- [ ] Implement `Fr` ops (add, mul, inv, pow) using ark-bn254
- [ ] Implement `G1Point` + `ec_add`, `ec_mul`, `ec_pairing` using ark-bn254
- [ ] Implement `Transcript` (Fiat-Shamir keccak256) from Solidity `TranscriptLib`
- [ ] Implement `load_proof()` — deserialize from `circuit/target/proof`
- [ ] Implement `load_vk()` — parse from `circuit/target/vk` binary
- [ ] Implement `load_public_inputs()` — read from `circuit/target/public_inputs`
- [ ] Translate `verifySumcheck()` from HonkVerifier.sol
- [ ] Translate `RelationsLib` (8 sub-relations)
- [ ] Translate `verifyShplemini()` from HonkVerifier.sol
- [ ] `cargo run` → prints `true`
- [ ] Test with 1 flipped byte in proof → prints `false`

### polkavm-ark-test (test ark-bn254 no_std compile on PolkaVM)
- [ ] Create dir, copy `.cargo/config.toml`, `rust-toolchain.toml`, `riscv64emac-unknown-none-polkavm.json` from `poseidon-contract/`
- [ ] Write `Cargo.toml` with `ark-bn254 = { version = "0.5.0", default-features = false }`
- [ ] Write minimal `src/main.rs`: `#![no_std]`, use `ark_bn254::Fr`, do one field mul
- [ ] Run `./build.sh` — record: compiles? binary size?
- [ ] If compile fails: note exact error → use precompile calls instead

### honk-verifier-polkavm (final PolkaVM contract — after both above)
- [ ] Create dir with full PolkaVM project structure (copy from `poseidon-contract/`)
- [ ] **If polkavm-ark-test passed**: add `ultrahonk_no_std` + wrap in contract shell
- [ ] **If polkavm-ark-test failed**: translate HonkVerifier.sol to Rust with precompile calls
- [ ] Embed VK as constants in `src/vk.rs`
- [ ] Add `verify(bytes,bytes32[])` function selector + ABI decode
- [ ] Set `SimpleAlloc` to 256KB
- [ ] `./build.sh` → `verifier.polkavm` < 1MB
- [ ] Deploy to Paseo testnet
- [ ] Call with real proof → `true`
- [ ] Call with bad proof → `false`
