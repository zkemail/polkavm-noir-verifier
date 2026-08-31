# Milestone 1 - PolkaVM Verifier Generator, Native Runtime & Benchmark Report

Primary Goal: deliver the PolkaVM-native UltraHonk verifier generator: the tool itself, its native verification runtime, gas optimization work and benchmark report, testnet validation, and automated test coverage.

## Deliverables

| # | Name | Description | Hours | What was done / Proof |
| --- | --- | --- | ---: | --- |
| 1 | Planning & Research | Review Barretenberg's `HonkVerifier.sol`, define the Solidity-to-Rust translation strategy, and map BN254 operations to PolkaVM precompiles. | 40 | Retrospective covering the real translation-strategy decision (including a tested, rejected alternative) and the BN254-to-precompile mapping, drawn directly from the project's own planning documents. **Proof:** [`01_planning_research.md`](./01_planning_research.md). |
| 2 | Generator Tool | Codegen tool that parses a circuit's Solidity HonkVerifier and verification key and emits a buildable Rust crate plus linked PolkaVM bytecode. | 100 | Circuit-agnostic TypeScript generator (`generator/`), rebuilt from a clean checkout against two circuit shapes (`LOG_N=5`/1 public input and `LOG_N=19`/155 public inputs); the larger circuit's linked binary (62,680 bytes) matches the pre-grant measurement byte-for-byte. **Proof:** [`02_generator_tool.md`](./02_generator_tool.md). |
| 3 | Native Verifier Runtime | Verifier implementation using `pallet-revive-uapi` precompile calls, a streaming keccak transcript, and a sized heap allocator. | 120 | Generic UltraHonk verification modules (`fr.rs`, `g1.rs`, `transcript.rs`, `relations.rs`, `shplemini.rs`) plus per-circuit templated entry point, verified via the same build run as the Generator Tool above. **Proof:** [`03_native_verifier_runtime.md`](./03_native_verifier_runtime.md). |
| 4 | Gas Optimization & Benchmark Report | Optimization work reducing verification and deployment gas costs, plus a published, reproducible benchmark report comparing PolkaVM, REVM, and EVM mainnet costs. | 160 | Pending - deliberately last; existing measurements will be re-run against current chain state before publishing. |
| 5 | Testnet Deployment & Validation | Deployment and verification across multiple circuit shapes on Paseo Asset Hub, from a minimal fixture up to a production-scale circuit. | 40 | Pending. |
| 6 | Automated Test Suite & CI | Equivalence tests against the Solidity reference, wired into CI so future changes are regression-tested. | 100 | Local-devnet equivalence harness (anvil for the Solidity reference, dev-node+eth-rpc for the native runtime), 5 circuit shapes x 5 test vectors = 25/25 passing, wired into GitHub Actions on every push/PR. **Proof:** [`06_automated_test_suite_ci.md`](./06_automated_test_suite_ci.md). |
| 7 | Documentation | Generator usage docs and architecture/provenance notes (mapping from the Solidity reference to the emitted Rust). | 80 | Pending. |

## Evidence Index

- [`01_planning_research.md`](./01_planning_research.md) - translation-strategy decision and BN254 precompile mapping, drawn from the project's own planning documents.
- [`02_generator_tool.md`](./02_generator_tool.md) - generator code walkthrough and build verification across two circuit shapes.
- [`03_native_verifier_runtime.md`](./03_native_verifier_runtime.md) - runtime module walkthrough (precompiles, streaming keccak, sized allocator).
- [`06_automated_test_suite_ci.md`](./06_automated_test_suite_ci.md) - local-devnet equivalence harness, circuit-shape matrix, and CI wiring.
