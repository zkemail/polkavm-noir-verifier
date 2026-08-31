# 02 - Generator Tool

Milestone 1 evidence for the codegen tool: parses a circuit's `HonkVerifier.sol` and emits a buildable Rust crate plus linked PolkaVM bytecode.

## What it does

`generator/generate-verifier.ts` is the entry-point router; the UltraHonk-specific implementation lives in `generator/honk-verifier/`:

- [`parse_solidity.ts`](../../../generator/honk-verifier/parse_solidity.ts) - extracts `N`, `LOG_N`, `NUMBER_OF_PUBLIC_INPUTS`, and 27 named G1 verification-key points from a `HonkVerifier.sol` via regex. No circuit is hardcoded; any circuit's own generated Solidity is the input.
- [`generate.ts`](../../../generator/honk-verifier/generate.ts) - fills three circuit-specific templates (`vk.rs`, `main.rs`, `sumcheck.rs`) from [`templates/`](../../../generator/honk-verifier/templates/), copies the generic verifier modules from [`static/`](../../../generator/honk-verifier/static/) unchanged, and (with `--build`) runs `cargo build --release` + `polkatool link` to produce the deployable `.polkavm` binary.
- Heap size is computed per-circuit from `NUMBER_OF_PUBLIC_INPUTS` (`calculateHeapKB` in `generate.ts`), not hardcoded - see [`03_native_verifier_runtime.md`](./03_native_verifier_runtime.md) for how that heap budget is used.

Structure - circuit-specific router in `generate-verifier.ts` (2026-04-22, [`610ed43`](https://github.com/zkemail/polkavm-noir-verifier/commit/610ed43) "Refactor generator: router + template-specific modules + .rs.tmpl files") - is the same shape shipped today.

## Proof it works

Ran the full pipeline end-to-end against two circuit shapes at opposite ends of the size range, using the exact toolchain versions pinned in the README (`nargo` 1.0.0-beta.5, `bb` v0.84.0, Rust `nightly-2026-04-20`, `polkatool` 0.25.0):

| Circuit | `LOG_N` | Public inputs | Command | Result |
| --- | ---: | ---: | --- | --- |
| `fixtures/noir-circuit` (`assert x != y`) | 5 | 1 | `nargo execute && bb prove/write_vk/write_solidity_verifier && ./scripts/generate.sh` | Parsed correctly (`N=32, LOG_N=5, PUBLIC_INPUTS=1`, 27 G1 points found), built, linked. `honk_verifier.polkavm`: 50,970 bytes. |
| `fixtures/zkemail` (Twitter-linking circuit, 468,002 gates) | 19 | 155 | `./scripts/generate.sh <path>/fixtures/zkemail/target/HonkVerifier.sol <out>` | Built, linked. `honk_verifier.polkavm`: **62,680 bytes** - byte-for-byte identical to the figure independently measured and recorded during the original (pre-grant) research. |

The zkemail-circuit match is a real independent reproduction: the binary was rebuilt from source on a freshly cloned, history-cleaned checkout of this repo, using the committed `HonkVerifier.sol` as input, and produced the exact same byte count as the original measurement - confirming the generator's output is deterministic and that nothing was lost or altered in porting the code.

Both runs used `bb`'s own verifier to independently confirm the source proof was valid before generating the Rust verifier from it (`bb verify` → `Proof verified successfully`), so the generated verifier's correctness can be checked against a second, independent implementation, not just against itself.

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

For the zkemail circuit (uses the already-committed `HonkVerifier.sol`, no `nargo`/`bb` needed):

```bash
./scripts/generate.sh "$(pwd)/fixtures/zkemail/target/HonkVerifier.sol" "$(pwd)/contracts/honk-verifier-zkemail"
ls -la contracts/honk-verifier-zkemail/honk_verifier.polkavm
```
