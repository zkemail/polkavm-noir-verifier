# 02 - Bring-Your-Own-Circuit Developer Guide

Milestone 2 deliverable: public how-to documentation for external Noir developers to generate and deploy a verifier for their own circuit.

## What it is

A step-by-step guide for taking a Noir circuit you wrote yourself - not one of this repo's fixtures - through the full pipeline to a deployed, verified PolkaVM contract on Paseo Asset Hub testnet. This is distinct from two things already in the root [`README.md`](../../../README.md):

- **"Bring your own `HonkVerifier.sol`"** starts from an already-generated Solidity verifier - it skips the Noir/`bb` steps entirely.
- **"Try with the included example circuit"** walks the same pipeline, but hardcoded to this repo's own `fixtures/noir-circuit`.

This guide covers the part neither of those does: your own circuit, from `nargo new` through a real, working `verify()` call on-chain.

## Prerequisites

Same as the root [`README.md`](../../../README.md#requirements): Rust nightly pinned via `rust-toolchain.toml`, `polkatool` `0.25.0` exactly (newer versions emit bytecode Paseo's pallet-revive rejects), Node.js 18+, Foundry's `cast`, and PAS testnet tokens ([faucet](https://faucet.polkadot.io/?parachain=1000)). To compile your own circuit from source you additionally need `nargo` `1.0.0-beta.5` and `bb` `v0.84.0`.

## Step 1 - Write and compile your circuit

```bash
nargo new my_circuit
cd my_circuit
```

Edit `src/main.nr`. As a concrete example, here's a genuinely different circuit from anything in this repo's fixtures - proving knowledge of two private factors of a public product:

```rust
fn main(a: Field, b: Field, product: pub Field) {
    assert(a * b == product);
}
```

Fill in `Prover.toml` with real witness values (private inputs plus the public one):

```toml
a = "3"
b = "4"
product = "12"
```

Then compile and execute:

```bash
nargo execute
```

This produces `target/my_circuit.json` (the compiled circuit) and `target/my_circuit.gz` (the witness).

## Step 2 - Generate proof artifacts with `bb`

```bash
bb prove --bytecode_path ./target/my_circuit.json --witness_path ./target/my_circuit.gz --output_path ./target --oracle_hash keccak
bb write_vk --bytecode_path ./target/my_circuit.json --output_path ./target --oracle_hash keccak
bb write_solidity_verifier --vk_path ./target/vk --output_path ./target/HonkVerifier.sol
```

**`--oracle_hash keccak` is required on both `prove` and `write_vk`, not optional.** Barretenberg defaults to a Poseidon2 transcript if you omit it; this runtime's Fiat-Shamir transcript ([`transcript.rs`](../../../generator/honk-verifier/static/src/honk/transcript.rs)) is hardcoded to keccak256 to match `HonkVerifier.sol`'s own transcript. A proof/VK generated without this flag will not verify - not because anything is broken, but because it was built against a different transcript than the one this verifier implements.

## Step 3 - Generate the PolkaVM verifier

From the repo root:

```bash
./scripts/generate.sh /path/to/my_circuit/target/HonkVerifier.sol contracts/my-verifier
```

This reads the circuit's public-input count and gate structure out of the Solidity file, fills the Rust templates, and builds + links a `.polkavm` binary. Watch for compiler warnings in the output - a clean build ends with `Build complete!`.

## Step 4 - Deploy to Paseo testnet

```bash
cd contracts/my-verifier
cp .env.example .env   # set PRIVATE_KEY to a funded Paseo account
npm install
npx ts-node scripts/deploy.ts
```

This writes `deployment.json` (address, tx hash, deploy gas) into the contract directory.

## Step 5 - Confirm it works

```bash
cd ../..
./scripts/test.sh /path/to/my_circuit/target/proof /path/to/my_circuit/target/public_inputs contracts/my-verifier
```

This asserts the valid proof is accepted and that a wrong public input plus 3 corrupted-proof-byte variants are all correctly rejected - the same 5-test-vector pattern used throughout this repo's own equivalence testing.

## Interface reference

```solidity
interface IHonkVerifier {
    function verify(bytes calldata proof, bytes32[] calldata publicInputs) external view returns (bytes1);
}
```

Selector `0xea50d0e4`. On success, returns a single `0x01` byte (not the strict-Solidity 32-byte `bool true` ABI - see the pallet-revive receipt-status note below). On failure it reverts - it never returns `0x00` - with either a 4-byte custom-error selector matching the REVM-compiled Solidity reference byte-for-byte (`ProofLengthWrong()`, `PublicInputsLengthWrong()`, `SumcheckFailed()`, `ShpleminiFailed()`), or a plain ASCII revert string for calldata-shape failures that have no Solidity-reference equivalent (`INPUT_TOO_SHORT`, `UNKNOWN_FUNCTION`, `ABI_DECODE_FAILED`, `INPUT_TOO_LARGE` - see Milestone 2's [`01_internal_security_review.md`](../milestone-2/01_internal_security_review.md#finding-3---unbounded-heap-allocation-from-raw-calldata-length) for why the last one exists).

## Troubleshooting

Real problems this project has hit, not a generic checklist:

- **Don't compile `HonkVerifier.sol` straight to PVM with `resolc` instead of using this generator.** It deploys, but every `verify()` call fails with `OutOfGas` on real testnet measurements - see [`04_gas_optimization_benchmark_report.md`](../milestone-1/04_gas_optimization_benchmark_report.md) in Milestone 1 for the measured comparison. This generator's hand-written Rust runtime exists specifically because that path doesn't work.
- **Large binaries can break `cast send --create <hex>` on Linux.** Circuits with many public inputs or large gate counts produce bigger `.polkavm` binaries; once hex-encoded, some exceed Linux's ~128KB single-argument limit (`MAX_ARG_STRLEN`), and `cast send --create` fails with `Argument list too long` (this doesn't reproduce on macOS, which enforces no such limit - confirmed on a real CI run). `scripts/deploy.ts` already deploys via raw `eth_sendTransaction` JSON-RPC instead of `cast send --create`, which has no such ceiling - use the provided script rather than hand-rolling a `cast` deploy for large circuits.
- **A successful `verify()` transaction can show `status=0` in its receipt.** pallet-revive's EVM-RPC adapter currently flags multi-byte returns as `status=0` (reverted) in transaction receipts even when the contract executed successfully - genuine for this contract's single-byte `0x01` return specifically. Don't infer failure from receipt status alone; decode the actual return data, or make a read-only call first to confirm.
- **`polkatool` version matters.** This project pins `0.25.0` exactly - newer versions emit bytecode Paseo's currently-deployed pallet-revive runtime rejects. `./scripts/generate.sh` uses whatever `polkatool` is on your `PATH`, so check `polkatool --version` if a build succeeds but deployment fails.

## This guide was verified, not just written

Every step above was run end-to-end against a fresh circuit that doesn't exist anywhere else in this repo (`assert(a * b == product)`, 2 private inputs, 1 public input) - `nargo new` through `bb prove`/`write_vk`/`write_solidity_verifier` through `./scripts/generate.sh` through a real deployment. Deployed to a local devnet (the same one `test/equivalence/` uses, to avoid spending testnet funds on a throwaway example): the valid proof was accepted (`0x01`), and a wrong public input was correctly rejected (`revert 0x9fc3a218`, `SumcheckFailed()`) - confirming both the accept and reject paths work for a genuinely new circuit shape, not just the 7 shapes this repo already ships fixtures for.

If you want the same pre-testnet sanity check before spending real PAS, [`test/equivalence/README.md`](../../../test/equivalence/README.md) documents the local-devnet setup this guide's verification used.
