# 01 - Planning & Research

## Context

The starting point was a working Noir circuit (`assert(x != y)`, 1 public input, `bb` v0.84.0, `nargo` 1.0.0-beta.5) and the goal of a native Rust PolkaVM contract that verifies its UltraHonk proof on Paseo Asset Hub, without going through `resolc` (which produces undeployable-in-practice bytecode for `HonkVerifier.sol` - see the Context section of the grant proposal and [`04_gas_optimization_benchmark_report.md`](./04_gas_optimization_benchmark_report.md) for the measured comparison).

## Translation strategy: a tested, rejected alternative

The first plan was not to translate `HonkVerifier.sol` by hand. It was to reuse an existing Rust UltraHonk verifier crate. That approach was tested directly and found not to work, which is why the generator translates the Solidity reference instead of wrapping a library.

Every known Rust UltraHonk verifier was tried:

| Library | Issue found |
| --- | --- |
| `zkVerify/ultrahonk_verifier` | VK/proof binary format changes with every `bb` version. The version the Cargo.lock resolved to (`986b79b`) expected 32-byte EVM-word VK fields from an older `bb` format; a later commit expects 8-byte `u64` fields and a different `ProofType` API instead. |
| `miquelcabot/ultrahonk_verifier` | A fork of zkVerify. Confirmed to not have the Soroban dependency or `-t evm` flag requirement initially assumed - but the underlying VK binary format still changes with every `bb` version, making version-matching fragile regardless. |
| `willemolding/ultrahonk_verifier_soroban` | Depends on Soroban host functions; not portable to PolkaVM. |
| `zkpassport/noir_rs`, `zkmopro/noir-rs` | Wrap the C++ Barretenberg library via FFI; not portable to a `no_std` PolkaVM target. |

The library was tested against real proof artifacts across multiple `bb` versions. None produced a working end-to-end verification; failures included VK-parsing `KeyError`s, `InvalidProofError: Failed parsing ZK proof`, and `VerificationError: Sumcheck Failed`, tracking version mismatches between the locked library and whichever `bb` version generated the test artifacts.

The conclusion, reached after this testing rather than assumed upfront: `HonkVerifier.sol`, produced directly by the same `bb` version used to generate the proof, is the one artifact guaranteed to be format-compatible. Translating it mechanically to Rust removes the version-matching problem entirely, since the translation happens per-circuit, from that circuit's own generated reference, not against a separately-versioned library.

**Sources:**
- [`27cba28`](https://github.com/zkemail/polkavm-noir-verifier/commit/27cba28) (2026-04-16) - initial implementation plan.
- [`b418fa2`](https://github.com/zkemail/polkavm-noir-verifier/commit/b418fa2) (2026-04-17) - decision to try `ultrahonk_no_std` first, before the direct-translation approach.
- [`5e09912`](https://github.com/zkemail/polkavm-noir-verifier/commit/5e09912) (2026-04-17) - `ultrahonk_no_std` confirmed incompatible via a direct spike, pivot to `HonkVerifier.sol` translation.
- [`ba7c479`](https://github.com/zkemail/polkavm-noir-verifier/commit/ba7c479) (2026-04-17) - root cause documented, `bb` version reverted to 0.84.0 to match the circuit artifacts in use.
- [`c902572`](https://github.com/zkemail/polkavm-noir-verifier/commit/c902572) (2026-04-19) - full library comparison table above, written up in detail.

## BN254 operations to PolkaVM precompile mapping

`HonkVerifier.sol`'s elliptic-curve and field operations map onto the EVM precompiles already available on Polkadot Asset Hub (EIP-196/197), used natively rather than reimplemented in Rust:

| Precompile | Address | Usage |
| --- | --- | --- |
| `modexp` | `0x05` | Field (Fr) inversion, `x^(p-2) mod p` |
| `ecAdd` | `0x06` | G1 point addition (used inside batched MSM) |
| `ecMul` | `0x07` | G1 scalar multiplication (used inside batched MSM) |
| `ecPairing` | `0x08` | Final pairing check (Shplemini/KZG batch opening) |

`keccak256`, used throughout for the Fiat-Shamir transcript, is not a precompile call in the shipped runtime - the initial plan called for `tiny-keccak` run in-VM, later replaced with `pallet-revive-uapi`'s native `hash_keccak_256` host function once benchmarking showed the in-VM version was a measurable share of verify gas (see Milestone 1's Gas Optimization & Benchmark Report deliverable).

**Source:** [`c902572`](https://github.com/zkemail/polkavm-noir-verifier/commit/c902572) (2026-04-19), "What HonkVerifier.sol Actually Uses" section.

## Generator design: circuit-specific vs. generic

Once the translation approach was settled, the generator's own design followed directly from separating what changes per circuit from what doesn't:

**Circuit-specific** (extracted from a given `HonkVerifier.sol`):
- `N` / `LOG_N` (circuit size)
- `NUMBER_OF_PUBLIC_INPUTS`
- 27 named G1 commitment points in the verification key

**Generic** (identical for every UltraHonk circuit, copied unchanged):
- Field arithmetic (`fr.rs`, `fr_utils.rs`)
- EC operations via precompiles (`g1.rs`)
- Proof parsing (`proof.rs`)
- The 26 UltraHonk sub-relations (`relations.rs`)
- Shplemini/KZG batch opening (`shplemini.rs`)
- Fiat-Shamir transcript (`transcript.rs`)

The generator parses the circuit-specific constants and G1 points out of `HonkVerifier.sol` via regex, templates three circuit-specific files (`vk.rs`, `main.rs`, `sumcheck.rs`), and copies the generic modules verbatim. This is the same structure the shipped `generator/` still uses.

**Sources:**
- [`cba73a4`](https://github.com/zkemail/polkavm-noir-verifier/commit/cba73a4) (2026-04-21) - generator design plan.
- [`fa33c47`](https://github.com/zkemail/polkavm-noir-verifier/commit/fa33c47) (2026-04-22) - generator plan updated with remaining TODOs, superseded once the generator itself was working (both plan documents were removed once implementation caught up - [`24114cf`](https://github.com/zkemail/polkavm-noir-verifier/commit/24114cf) and [`13e6737`](https://github.com/zkemail/polkavm-noir-verifier/commit/13e6737)).
