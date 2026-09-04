# 06 - Automated Test Suite & CI

Milestone 1 evidence for equivalence testing against the Solidity reference, wired into CI.

## What it is

Before this deliverable, the only test tool (`scripts/test.sh`) called a contract already deployed on live Paseo testnet - not CI-safe (needs funds, a live chain, and a prior manual deploy step). [`test/equivalence/run.sh`](../../../test/equivalence/run.sh) replaces that with a self-contained local pipeline:

1. Deploys the circuit's Solidity reference verifier (`HonkVerifier.sol`) to a local EVM devnet (`anvil`).
2. Generates, builds, and deploys the native Rust/PolkaVM verifier to a local PVM devnet - `dev-node` + `eth-rpc`, the same binaries [`zkemail/polkavm-hardhat-template`](https://github.com/zkemail/polkavm-hardhat-template) downloads, run directly here rather than through Hardhat (this repo has no Hardhat dependency and didn't need to add one - the existing `scripts/deploy.ts` pattern already works against any JSON-RPC endpoint, and the dev-node's pre-funded accounts support unlocked `eth_sendTransaction` signing the same way Anvil's do).
3. Runs 5 identical test vectors against both deployments - a valid proof, a wrong public input, and 3 corrupted-proof-byte variants - and asserts they agree: either both accept, or both revert with the same 4-byte custom-error selector.

## Circuit-shape matrix

Seven fixtures, deliberately designed to cover public-input count and `LOG_N` as two *independent* axes (rather than trying to recover the untracked shapes referenced in earlier internal notes, and rather than just picking bigger/smaller circuits along a single dimension). Current gate count / public input count / `LOG_N` for every fixture is in the root [`README.md`](../../../README.md#tested-with)'s "Tested with" table - not restated here since it's re-verified by CI on every push and a second hardcoded copy here would only go stale.

| Fixture | Purpose |
| --- | --- |
| `fixtures/zero-pub-input` | Edge case: empty public-input array |
| `fixtures/noir-circuit` | Baseline |
| `fixtures/multi-pub-input` | Mid public-input count, small circuit |
| `fixtures/huge-pub-input` | Public-input count far beyond the production circuit's 155 |
| `fixtures/large-circuit` | Distinct circuit size from the tiny fixtures |
| `fixtures/huge-circuit` | Genuinely bigger than the production circuit, and a different power-of-two bucket (`LOG_N=21` vs. `LOG_N=19`), not just a bigger number in the same one |
| `fixtures/zkemail` | Real production circuit - both axes large simultaneously, the one combined case nothing else covers |

Real-Paseo deployment coverage across this matrix is Milestone 1's separate "Testnet Deployment & Validation" deliverable - see [`05_testnet_deployment_validation.md`](./05_testnet_deployment_validation.md).

## Proof it works

All 7 shapes pass all 5 test vectors (35/35) run locally against the two local devnets described above. The zero-public-input shape is a genuine, not just nominal, edge case: its "wrong public input" vector (a fabricated single entry where zero are expected) correctly triggers `PublicInputsLengthWrong()` (`0xfa066593`) on both backends, rather than the `SumcheckFailed()` selector the other shapes' corrupted vectors hit - confirming the length-validation path, not just the cryptographic verification path, is equivalent across backends.

## CI

[`.github/workflows/equivalence-tests.yml`](../../../.github/workflows/equivalence-tests.yml) runs this on every push to `main` and every pull request: installs `nargo`, `bb`, Foundry, and the pinned Rust nightly; downloads and caches the local PVM devnet binaries; regenerates proof artifacts for all 7 fixtures from source, including `fixtures/zkemail` (its external Noir dependencies are pinned to specific git tags, and a fresh build takes only a few seconds); and runs `test/equivalence/run.sh` across all 7 shapes, failing the job on any mismatch.

A real GitHub Actions run surfaced a devnet-startup race condition: a fixed `sleep` before checking readiness raced `dev-node`/`eth-rpc`'s actual startup time. Fixed with a polling loop. [Full green run](https://github.com/zkemail/polkavm-noir-verifier/actions/runs/33523468564), **25/25 equivalence checks** (5 test vectors x 5 circuit shapes) on real GitHub-hosted infrastructure.

The matrix grew to 7 shapes (`huge-pub-input`, `huge-circuit` added). Pushing the expanded matrix caught a second real CI-only bug: `huge-pub-input`'s 113KB binary hex-encodes to 226KB, and `cast send --create <hex>` passes that as a single argv entry - Linux caps a single argv entry at ~128KB, so the deploy failed with "Argument list too long" (never surfaced locally; macOS enforces no such limit). Fixed by deploying via raw `eth_sendTransaction` JSON-RPC instead. [Full green run](https://github.com/zkemail/polkavm-noir-verifier/actions/runs/33628567922), **35/35 equivalence checks** (5 test vectors x 7 circuit shapes) on real GitHub-hosted infrastructure.

## Reproduce

```bash
cd test/equivalence
./bin/setup-dev-node.sh   # one-time, downloads dev-node + eth-rpc

./bin/dev-node --dev --rpc-port 8001 &
./bin/eth-rpc --dev --node-rpc-url=ws://127.0.0.1:8001 --rpc-port 8546 &
anvil --port 8547 &

# for each fixture, generate proof artifacts first (see each fixture's README.md),
# then:
./run.sh ../../fixtures/noir-circuit
./run.sh ../../fixtures/zero-pub-input
./run.sh ../../fixtures/multi-pub-input
./run.sh ../../fixtures/large-circuit
./run.sh ../../fixtures/huge-pub-input
./run.sh ../../fixtures/huge-circuit
./run.sh ../../fixtures/zkemail
```

See [`test/equivalence/README.md`](../../../test/equivalence/README.md) for details.
