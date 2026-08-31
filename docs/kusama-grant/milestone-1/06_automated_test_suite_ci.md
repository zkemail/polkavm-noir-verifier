# 06 - Automated Test Suite & CI

Milestone 1 evidence for equivalence testing against the Solidity reference, wired into CI.

## What it is

Before this deliverable, the only test tool (`scripts/test.sh`) called a contract already deployed on live Paseo testnet - not CI-safe (needs funds, a live chain, and a prior manual deploy step). [`test/equivalence/run.sh`](../../../test/equivalence/run.sh) replaces that with a self-contained local pipeline:

1. Deploys the circuit's Solidity reference verifier (`HonkVerifier.sol`) to a local EVM devnet (`anvil`).
2. Generates, builds, and deploys the native Rust/PolkaVM verifier to a local PVM devnet - `dev-node` + `eth-rpc`, the same binaries [`zkemail/polkavm-hardhat-template`](https://github.com/zkemail/polkavm-hardhat-template) downloads, run directly here rather than through Hardhat (this repo has no Hardhat dependency and didn't need to add one - the existing `scripts/deploy.ts` pattern already works against any JSON-RPC endpoint, and the dev-node's pre-funded accounts support unlocked `eth_sendTransaction` signing the same way Anvil's do).
3. Runs 5 identical test vectors against both deployments - a valid proof, a wrong public input, and 3 corrupted-proof-byte variants - and asserts they agree: either both accept, or both revert with the same 4-byte custom-error selector.

## Circuit-shape matrix

Two existing fixtures plus three new ones, deliberately designed to cover shapes the original two didn't (rather than trying to recover the untracked shapes referenced in earlier internal notes):

| Fixture | Public inputs | `LOG_N` | Purpose |
| --- | ---: | ---: | --- |
| `fixtures/noir-circuit` | 1 | 5 | Baseline (existing) |
| `fixtures/zkemail` | 155 | 19 | Production-scale baseline (existing) |
| `fixtures/zero-pub-input` | **0** | 5 | Edge case: empty public-input array |
| `fixtures/multi-pub-input` | 5 | 5 | Mid-size, between the two extremes |
| `fixtures/large-circuit` | 1 | **10** | Distinct circuit size from every other fixture |

## Proof it works

All 5 shapes pass all 5 test vectors (25/25) run locally against the two local devnets described above. The zero-public-input shape is a genuine, not just nominal, edge case: its "wrong public input" vector (a fabricated single entry where zero are expected) correctly triggers `PublicInputsLengthWrong()` (`0xfa066593`) on both backends, rather than the `SumcheckFailed()` selector the other shapes' corrupted vectors hit - confirming the length-validation path, not just the cryptographic verification path, is equivalent across backends.

## CI

[`.github/workflows/equivalence-tests.yml`](../../../.github/workflows/equivalence-tests.yml) runs this on every push to `main` and every pull request: installs `nargo`, `bb`, Foundry, and the pinned Rust nightly; downloads and caches the local PVM devnet binaries; regenerates proof artifacts for the 4 small fixtures from source (`fixtures/zkemail`'s are pre-committed, since it's a real production circuit, not trivially regeneratable); and runs `test/equivalence/run.sh` across all 5 shapes, failing the job on any mismatch.

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
./run.sh ../../fixtures/zkemail
```

See [`test/equivalence/README.md`](../../../test/equivalence/README.md) for details.
