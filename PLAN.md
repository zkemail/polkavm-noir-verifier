# Plan: Noir Verifier on PolkaVM — Complete Implementation Plan

## Context

The user has Noir circuits. Barretenberg (`bb`) generates a Solidity UltraHonk verifier per circuit, but compiling that Solidity with `resolc` produces PVM bytecode that exceeds the size limit. The goal: implement a **native Rust PolkaVM contract** that can verify Noir (UltraHonk) proofs on Paseo Asset Hub testnet.

---

## Critical Correction from Research

**UltraHonk does NOT require the EIP-4844 blob precompile (0x0A).** That precompile is for Ethereum blob transactions only.

UltraHonk's KZG commitments use **standard BN254 pairing** — and ecPairing (0x08) IS available on Asset Hub. This means **UltraHonk CAN be verified on Asset Hub** if we implement the verification logic in Rust.

---

## What UltraHonk Verification Requires

| Component | Description | Available on Asset Hub |
|---|---|---|
| Fr arithmetic (BN254 scalar field) | Add, mul, pow, inverse | ✅ Already in zk.rs |
| G1 point addition | ecAdd (0x06) precompile | ✅ ~991 gas |
| G1 scalar multiplication | ecMul (0x07) precompile | ✅ ~991 gas |
| BN254 pairing | ecPairing (0x08) precompile | ✅ ~276 gas/pair |
| G2 point arithmetic | Used in verification key only (constants) | ✅ No ops needed on G2 in verifier |
| Sumcheck protocol | Polynomial evaluation in Fr | ✅ Pure arithmetic |
| ZM/Shplemini opening | KZG proof verification | ✅ Uses pairing above |
| Transcript (Fiat-Shamir) | Keccak256 hashing | ✅ Via `api::call()` or pure Rust |

---

## PolkaVM Constraints

- **Architecture**: RV32EM (32-bit embedded RISC-V, M extension only — no float, no SIMD)
- **Heap**: 10KB default (`SimpleAlloc`), configurable up to practical limits
- **Code blob limit**: 1MB
- **Call depth limit**: 25
- **Block limit**: 1000 instructions per basic block
- **Current contract size**: 20KB (huge headroom)

---

## Approach Options

### Approach A: Port miquelcabot/ultrahonk_verifier + ark-bn254 no_std (Recommended First Try)

**Reference**: https://github.com/miquelcabot/ultrahonk_verifier — Soroban (no_std) UltraHonk verifier using ark-bn254.

**Why this is the best starting point:**
- Already targets a no_std embedded environment (Soroban)
- Uses ark-bn254 with `default-features = false` (no_std compatible)
- Implements the full UltraHonk protocol (sumcheck, Shplemini/ZM)
- BN254 curves — same as ecPairing (0x08) on Asset Hub
- ~2,000-4,000 LOC to port vs writing from scratch

**Steps:**
1. **Test ark-bn254 compilation on PolkaVM target** (1-2 days)
   - Add to `Cargo.toml`:
     ```toml
     ark-bn254 = { version = "0.5.0", default-features = false }
     ark-ff = { version = "0.5.0", default-features = false }
     ark-ec = { version = "0.5.0", default-features = false }
     ark-serialize = { version = "0.5.0", default-features = false }
     ```
   - Write a minimal test file that imports `ark_bn254::Fr` and compiles
   - Run `./build.sh` — check for compile errors or linker issues
   - Check binary size (`ls -la pos.polkavm`)

2. **Verify ark compiles and binary is within limits** (1 day)
   - If compile fails: switch to Approach B (precompile calls)
   - If binary > 800KB: switch to Approach B
   - Expected size with ark: ~150-300KB (within 1MB limit)

3. **Port UltraHonk verifier logic** (1-2 weeks)
   - Clone miquelcabot/ultrahonk_verifier
   - Identify Soroban-specific parts (env calls, storage) — replace with PolkaVM equivalents
   - The core verification logic (transcript, sumcheck, polynomial evaluation) is pure math — copy directly
   - Replace `env.crypto().keccak256()` with pure-Rust keccak256 (add `tiny-keccak` crate, `default-features = false`)
   - Adapt the proof and verification key deserialization to ABI-encoded calldata format

4. **Add circuit-specific verification key** (1 day)
   - Export VK from Barretenberg: `bb write_vk_ultra_honk -b circuit.json -o vk.bin`
   - Parse VK binary format, embed as Rust constants in the contract
   - One contract = one circuit's VK (or accept VK as calldata)

5. **Add contract wrapper** (1 day)
   - Function selector for `verify(bytes proof, bytes32[] publicInputs) returns (bool)`
   - ABI decode calldata using `ethabi`
   - Call the verifier, return encoded bool
   - Handle heap: increase `SimpleAlloc` to at least 64KB (proof data is large)

6. **Verify protocol version compatibility** (critical risk)
   - Barretenberg's UltraHonk protocol version changes between releases
   - miquelcabot's implementation may target an older Barretenberg version
   - Must test: generate a proof with your exact `bb` version, verify with the ported contract
   - If protocol mismatch: diff against the Solidity verifier that `bb` generates — it's the canonical reference

**Estimated effort**: 2-3 weeks total
**Risk**: Medium — ark might not compile on riscv64emac, protocol version may differ
**Binary size**: ~150-300KB (within 1MB)

---

### Approach B: EVM Precompiles for Curve Ops + Hand-Written Protocol (No ark dependency)

If Approach A fails (ark won't compile), use the EVM precompiles for BN254 operations:

```rust
// Call ecMul precompile (0x07) from PolkaVM
fn ec_mul(point: G1Affine, scalar: Fr) -> G1Affine {
    let mut input = [0u8; 96]; // x(32) + y(32) + scalar(32)
    input[0..32].copy_from_slice(&point.x.to_bytes_be());
    input[32..64].copy_from_slice(&point.y.to_bytes_be());
    input[64..96].copy_from_slice(&scalar.to_bytes_be());

    let precompile_addr = [0u8; 19].chain([0x07]); // address(7)
    let mut output = [0u8; 64];
    api::call(CallFlags::empty(), &precompile_addr, gas/4, u64::MAX, &[u8::MAX;32], &[0;32], &input, Some(&mut &mut output[..]));
    // Parse output as G1 point
    G1Affine::from_bytes(&output)
}
```

**Steps:**
1. Keep existing `zk.rs` Fr arithmetic
2. Add G1 point type (affine coordinates, big-endian bytes) — no field ops, just byte formatting
3. Implement `ec_add`, `ec_mul`, `ec_pairing` wrappers around `api::call()` to precompiles
4. Port UltraHonk protocol from miquelcabot (same logic, different curve backend)
5. Test precompile reachability first with a simple ecAdd call before full port

**Estimated effort**: 3-4 weeks (extra week for precompile integration and G1 type)
**Risk**: Medium — precompile callability from PolkaVM needs testing; api::call() formatting overhead
**Binary size**: ~50-100KB (no ark dependency)

---

### Approach C: Full Hand-Roll (Last Resort)

Extend `zk.rs` with full BN254 implementation:
- Fp2 field (extension of Fp for G2)
- G1/G2 affine and projective points
- Scalar multiplication (double-and-add)
- Miller loop + final exponentiation (Ate pairing)
- UltraHonk protocol on top

**Estimated effort**: 8-12 weeks minimum
**Risk**: Very High — subtle bugs in pairing are hard to debug; not recommended unless both A and B fail

---

## Recommended Execution Order

```
Week 1: Test ark-bn254 compilation on PolkaVM target
  → If compiles: proceed with Approach A
  → If fails: proceed with Approach B (test precompile calls week 1)

Week 2-3: Port UltraHonk verifier logic (whichever approach)

Week 3-4: Protocol version alignment + integration testing

Week 4: Deploy to Paseo testnet, end-to-end test with real Noir proof
```

---

## Heap / Memory Considerations

UltraHonk proofs are large (~2KB+ for simple circuits). Increase `SimpleAlloc`:
```rust
// In erc20.rs — increase from 10KB to 256KB
pub static mut GLOBAL: SimpleAlloc<{ 256 * 1024 }> = SimpleAlloc::new();
```
This is fine since PolkaVM allocates heap lazily — unused pages don't cost gas.

---

## Key Files to Modify

| File | Change |
|---|---|
| `poseidon-contract/Cargo.toml` | Add ark-bn254, ark-ff, tiny-keccak (Approach A) |
| `poseidon-contract/src/erc20.rs` | Add `verify()` function, dispatch selector, increase heap |
| `poseidon-contract/src/zk.rs` | Keep as-is (Fr arithmetic reused); add G1 type if Approach B |
| `poseidon-contract/src/ultrahonk.rs` | New file — ported verifier logic |
| `poseidon-contract/src/vk.rs` | New file — embedded verification key constants |
| `poseidon-contract/ts/main.ts` | Add proof verification test after deploy |

---

## Verification / Testing Plan

1. **Compile test**: `./build.sh` — must succeed with `pos.polkavm` < 1MB
2. **Deploy**: `yarn erc20` — contract appears on Paseo testnet
3. **Generate test proof**:
   ```bash
   nargo prove  # generates proof + public inputs
   bb write_vk_ultra_honk -b circuit/target/circuit.json -o vk.bin
   ```
4. **Test valid proof** → contract returns `true`
5. **Test invalid proof** (flip one byte) → contract returns `false`
6. **Test wrong public inputs** → contract returns `false`
