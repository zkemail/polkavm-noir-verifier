# Equivalence Tests

Deploys a circuit's Solidity reference verifier (`HonkVerifier.sol`) to a local EVM
devnet and its generated Rust/PolkaVM verifier to a local PVM devnet, then runs
identical proof/corruption test vectors against both and asserts they agree.

## One-time setup

```bash
./bin/setup-dev-node.sh   # downloads dev-node + eth-rpc from paritytech/hardhat-polkadot releases
```

## Start the local devnets (once, reused across circuits)

```bash
./bin/dev-node --dev --rpc-port 8001 &
./bin/eth-rpc --dev --node-rpc-url=ws://127.0.0.1:8001 --rpc-port 8546 &
anvil --port 8547 &
```

## Run against a circuit

```bash
./run.sh /path/to/circuit   # must contain target/HonkVerifier.sol, target/proof, target/public_inputs
```

Example, using this repo's fixtures:

```bash
./run.sh ../../fixtures/noir-circuit
./run.sh ../../fixtures/zkemail
```

## What it checks

Five test vectors per circuit: a valid proof, a wrong public input, and three
corrupted-byte variants (matching `scripts/test.sh`'s existing negative cases).
For each, both deployed contracts must agree - either both accept, or both
revert with the same 4-byte custom-error selector (EVM and PVM differ in
revert-message formatting, but the selector is expected to match: the native
runtime deliberately emits the same selectors as the REVM-compiled Solidity
reference - `ProofLengthWrong`, `PublicInputsLengthWrong`, `SumcheckFailed`,
`ShpleminiFailed` - this was engineered and verified, not incidental).
