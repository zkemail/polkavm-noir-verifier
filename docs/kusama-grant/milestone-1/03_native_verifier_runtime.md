# 03 - Native Verifier Runtime

Milestone 1 evidence for the verifier implementation itself: `pallet-revive-uapi` precompile calls, a streaming keccak transcript, and a sized heap allocator.

## What it is

The generic UltraHonk verification modules, copied unchanged into every generated project from [`generator/honk-verifier/static/src/honk/`](../../../generator/honk-verifier/static/src/honk/), plus three circuit-specific files templated per build ([`generator/honk-verifier/templates/`](../../../generator/honk-verifier/templates/)):

| Module | Role |
| --- | --- |
| [`fr.rs`](../../../generator/honk-verifier/static/src/honk/fr.rs), [`fr_utils.rs`](../../../generator/honk-verifier/static/src/honk/fr_utils.rs) | BN254 scalar-field (Fr) arithmetic, Montgomery form |
| [`g1.rs`](../../../generator/honk-verifier/static/src/honk/g1.rs) | EC group operations, dispatched to the EVM precompiles already available on Polkadot Asset Hub - `ecAdd` (`0x06`), `ecMul` (`0x07`), `ecPairing` (`0x08`) via `api::call` |
| [`transcript.rs`](../../../generator/honk-verifier/static/src/honk/transcript.rs) | Fiat-Shamir transcript. Hashing goes through `pallet-revive-uapi`'s `api::hash_keccak_256` host function - native keccak execution outside the PolkaVM meter, not an in-VM implementation |
| [`proof.rs`](../../../generator/honk-verifier/static/src/honk/proof.rs) | Proof deserialization |
| [`relations.rs`](../../../generator/honk-verifier/static/src/honk/relations.rs) | The 26 UltraHonk sub-relations |
| [`shplemini.rs`](../../../generator/honk-verifier/static/src/honk/shplemini.rs) | Shplemini/KZG batch opening |
| `vk.rs`, `main.rs`, `sumcheck.rs` (templated) | Circuit-specific verification key, contract entry point, and unrolled sumcheck rounds (see [`01_planning_research.md`](./01_planning_research.md) for why sumcheck is unrolled rather than a runtime loop) |

**Sized heap allocator:** `main.rs.tmpl` declares `#[global_allocator] static ALLOC: simplealloc::SimpleAlloc<{ {{HEAP_KB}} * 1024 }>`, with `HEAP_KB` computed per-circuit from `NUMBER_OF_PUBLIC_INPUTS` by the generator (`calculateHeapKB` in `generate.ts`), not a fixed constant. `simplealloc` is a bump allocator with no `free` support, adopted deliberately since `verify()` never frees during execution (`7991375`, "Swap picoalloc -> simplealloc (deploy gas -12%, binary -20%)").

**`pallet-revive-uapi` dependency**, pinned in [`static/Cargo.toml`](../../../generator/honk-verifier/static/Cargo.toml):

```toml
polkavm-derive = "0.30"
simplealloc = { version = "0.0.1", git = "https://github.com/paritytech/polkavm.git", rev = "bd2d14ed105467fd158b4744fd92f7045d288bb8" }
pallet-revive-uapi = { version = "0.10", default-features = false }
```

The streaming-keccak swap was a deliberate, measured change, not the original implementation: the runtime originally used in-VM `tiny-keccak`, replaced with the host-function call after benchmarking showed it was a measurable share of verify gas (`c62eb46` "Streaming keccak in transcript: eliminate all Vec allocations", `e679bee` "Swap tiny-keccak -> pallet-revive api::hash_keccak_256 (4.5% verify, REVM parity)") - see [`04_gas_optimization_benchmark_report.md`](./04_gas_optimization_benchmark_report.md) (pending) for the measured numbers.

## Proof it works

Same build run documented in [`02_generator_tool.md`](./02_generator_tool.md) exercises this runtime directly - the generator's templating step and this runtime's compilation are one pipeline, not separately testable. Both circuit shapes (`LOG_N=5`/1 public input and `LOG_N=19`/155 public inputs) compiled under the pinned `nightly-2026-04-20` toolchain with zero warnings and linked to a valid PolkaVM binary via `polkatool link --strip --min-stack-size 65536`.

The zkemail-circuit binary (62,680 bytes) matching the pre-grant measurement byte-for-byte is direct evidence that this runtime - the actual verification logic, not just the generator's templating - reproduces deterministically from source.

## Reproduce

Same as [`02_generator_tool.md`](./02_generator_tool.md#reproduce) - the runtime is compiled as part of the same `./scripts/generate.sh --build` step, not a separate build target.
