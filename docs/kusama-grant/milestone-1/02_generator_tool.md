# 02 - Generator Tool

Milestone 1 evidence for the codegen tool: parses a circuit's `HonkVerifier.sol` and emits a buildable Rust crate plus linked PolkaVM bytecode.

## What it does

`generator/generate-verifier.ts` is the entry-point router; the UltraHonk-specific implementation lives in `generator/honk-verifier/`:

- [`parse_solidity.ts`](../../../generator/honk-verifier/parse_solidity.ts) - extracts `N`, `LOG_N`, `NUMBER_OF_PUBLIC_INPUTS`, and 27 named G1 verification-key points from a `HonkVerifier.sol` via regex. No circuit is hardcoded; any circuit's own generated Solidity is the input.
- [`generate.ts`](../../../generator/honk-verifier/generate.ts) - fills three circuit-specific templates (`vk.rs`, `main.rs`, `sumcheck.rs`) from [`templates/`](../../../generator/honk-verifier/templates/), copies the generic verifier modules from [`static/`](../../../generator/honk-verifier/static/) unchanged, and (with `--build`) runs `cargo build --release` + `polkatool link` to produce the deployable `.polkavm` binary.
- Heap size is computed per-circuit from `NUMBER_OF_PUBLIC_INPUTS` (`calculateHeapKB` in `generate.ts`), not hardcoded - see [`03_native_verifier_runtime.md`](./03_native_verifier_runtime.md) for how that heap budget is used.

Structure - circuit-specific router in `generate-verifier.ts` (2026-04-22, [`610ed43`](https://github.com/zkemail/polkavm-noir-verifier/commit/610ed43) "Refactor generator: router + template-specific modules + .rs.tmpl files") - is the same shape shipped today.

## Proof it works

Ran the full pipeline end-to-end against two circuit shapes at opposite ends of the size range - `fixtures/noir-circuit` and `fixtures/zkemail` (real production circuit) - using the exact toolchain versions pinned in the root [`README.md`](../../../README.md#requirements). Current gate count / public input count / `LOG_N` for every fixture, including these two, is in that README's "Tested with" table; not restated here since it's re-verified by CI on every push and a second hardcoded copy here would only go stale.

Both builds parsed correctly (27 G1 points found, matching the UltraHonk verification key structure), built, and linked into a valid PolkaVM binary. `bb`'s own verifier independently confirmed each source proof was valid before generating the Rust verifier from it (`bb verify` → `Proof verified successfully`), so the generated verifier's correctness can be checked against a second, independent implementation, not just against itself.

Rebuilding either circuit from source on a freshly cloned checkout of this repo reproduces the exact same binary byte-for-byte every time - confirming the generator's output is deterministic. This is checked automatically on every push by [`test/equivalence/run.sh`](../../../test/equivalence/run.sh) (see [`06_automated_test_suite_ci.md`](./06_automated_test_suite_ci.md)), not just asserted here.

## Reproduce

From the repo root, for the simple fixture:

```bash
cd fixtures/noir-circuit
nargo execute
bb prove --bytecode_path ./target/circuit.json --witness_path ./target/circuit.gz --output_path ./target --oracle_hash keccak
bb write_vk --bytecode_path ./target/circuit.json --output_path ./target --oracle_hash keccak
bb write_solidity_verifier --vk_path ./target/vk --output_path ./target/HonkVerifier.sol
cd ../..
./scripts/generate.sh
ls -la contracts/honk-verifier/honk_verifier.polkavm
```

For the zkemail circuit, same shape (its `target/` isn't committed - see `fixtures/zkemail/README.md`):

```bash
cd fixtures/zkemail
nargo execute
bb prove --bytecode_path ./target/zkemail/twitter@v1.json --witness_path ./target/zkemail/twitter@v1.gz --output_path ./target --oracle_hash keccak
bb write_vk --bytecode_path ./target/zkemail/twitter@v1.json --output_path ./target --oracle_hash keccak
bb write_solidity_verifier --vk_path ./target/vk --output_path ./target/HonkVerifier.sol
cd ../..
./scripts/generate.sh "$(pwd)/fixtures/zkemail/target/HonkVerifier.sol" "$(pwd)/contracts/honk-verifier-zkemail"
ls -la contracts/honk-verifier-zkemail/honk_verifier.polkavm
```
