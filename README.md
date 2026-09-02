# UltraHonk Verifier for PolkaVM

[![Equivalence Tests](https://github.com/zkemail/polkavm-noir-verifier/actions/workflows/equivalence-tests.yml/badge.svg)](https://github.com/zkemail/polkavm-noir-verifier/actions/workflows/equivalence-tests.yml)

Generate and deploy UltraHonk ZK proof verifiers as PolkaVM smart contracts on Polkadot Asset Hub.

Given any Noir circuit, this tool produces a complete Rust verifier contract that runs on-chain.

## How it works

```
HonkVerifier.sol → Generator → PolkaVM contract (.polkavm)
```

The generator reads a `HonkVerifier.sol` (produced by Barretenberg's `bb write_solidity_verifier`) and outputs a buildable Rust PolkaVM project with:
- Circuit-specific verification key, sumcheck rounds, and entry point
- Generic UltraHonk verification modules (transcript, relations, shplemini/KZG)
- EC operations via EVM precompiles (ecAdd, ecMul, ecPairing)
- Deploy script

## Usage

### Bring your own HonkVerifier.sol

If you already have a `HonkVerifier.sol` from your circuit:

```bash
# Install generator dependencies
cd generator && npm install && cd ..

# Generate the verifier contract
cd generator && npx ts-node generate-verifier.ts honk \
  --sol /path/to/your/HonkVerifier.sol \
  --out ../contracts/my-verifier \
  --build

# Deploy
cd ../contracts/my-verifier
cp .env.example .env   # add your PRIVATE_KEY
npm install
npx ts-node scripts/deploy.ts

# Test (requires cast from foundry + proof/public_inputs next to HonkVerifier.sol)
cd ../..
./scripts/test.sh /path/to/your/proof /path/to/your/public_inputs
```

### Try with the included example circuit

A simple test circuit (`assert(x != y)`) is included for development and testing:

```bash
# 1. Compile the circuit and generate proof artifacts
cd fixtures/noir-circuit
nargo execute
bb prove --bytecode_path ./target/circuit.json --witness_path ./target/circuit.gz --output_path ./target --oracle_hash keccak
bb write_vk --bytecode_path ./target/circuit.json --output_path ./target --oracle_hash keccak
bb write_solidity_verifier --vk_path ./target/vk --output_path ./target/HonkVerifier.sol

# 2. Generate, build, and link the verifier
cd ../..
./scripts/generate.sh

# 3. Deploy and test
cd contracts/honk-verifier
cp .env.example .env   # add your PRIVATE_KEY
npm install
npx ts-node scripts/deploy.ts
cd ../..
./scripts/test.sh
```

## Project structure

```
├── generator/                      # Verifier generator tool
│   ├── generate-verifier.ts        # Entry point (router)
│   ├── utils.ts                    # Shared utilities
│   └── honk-verifier/              # UltraHonk template
│       ├── generate.ts             # Honk-specific generator
│       ├── parse_solidity.ts       # HonkVerifier.sol parser
│       ├── templates/              # Rust files with {{placeholders}}
│       └── static/                 # Files copied verbatim
│           ├── src/honk/           # Generic verification modules
│           ├── scripts/            # Deploy script
│           └── interfaces/         # IHonkVerifier.sol
├── fixtures/                       # Circuit-shape matrix (7 shapes - see "Tested with" below)
│   ├── noir-circuit/                 # Baseline: 1 public input, LOG_N=5
│   ├── zero-pub-input/                # Edge case: 0 public inputs
│   ├── multi-pub-input/               # 5 public inputs
│   ├── huge-pub-input/                # 1,001 public inputs
│   ├── large-circuit/                 # LOG_N=10
│   ├── huge-circuit/                  # LOG_N=21
│   └── zkemail/                       # Real production circuit: LOG_N=19, 155 public inputs
├── scripts/
│   ├── generate.sh                 # Generate + build verifier
│   └── test.sh                     # Run verification tests against a deployed contract (requires cast)
├── test/equivalence/                # Local-devnet equivalence tests (see test/equivalence/README.md)
├── .github/workflows/               # CI: runs the equivalence-test matrix on every push/PR
└── contracts/                      # Generated output (gitignored)
```

## Tested with

7 circuit shapes, covering public-input count and circuit size (`LOG_N`) as independent axes:

| Fixture | Gate count | Public inputs | `LOG_N` |
| --- | ---: | ---: | ---: |
| `fixtures/zero-pub-input` | 17 | 0 | 5 |
| `fixtures/noir-circuit` | 18 | 1 | 5 |
| `fixtures/multi-pub-input` | 18 | 5 | 5 |
| `fixtures/huge-pub-input` | 350 | 1,001 | 11 |
| `fixtures/large-circuit` | 815 | 1 | 10 |
| `fixtures/huge-circuit` | 1,100,015 | 1 | 21 |
| `fixtures/zkemail` ([zkemail/ens-contracts](https://github.com/zkemail/ens-contracts), real production circuit) | 468,002 | 155 | 19 |

## Continuous integration

[`.github/workflows/equivalence-tests.yml`](./.github/workflows/equivalence-tests.yml) runs the full equivalence-test matrix above on every push and pull request against a local devnet (no live testnet funds required).

## Requirements

- [Rust](https://rustup.rs/) nightly, pinned to `nightly-2026-04-20` via `generator/honk-verifier/static/rust-toolchain.toml`
- [polkatool](https://github.com/paritytech/polkavm) **0.25.0** for PolkaVM linking. Install via `cargo install polkatool --version 0.25.0 --locked`. Newer versions emit bytecode Paseo's pallet-revive rejects, so the version is pinned to match the chain's pallet-revive runtime.
- Node.js 18+ (for generator and deploy scripts)
- [Foundry](https://getfoundry.sh/) (`cast`) for running tests
- PAS tokens on Paseo testnet ([faucet](https://faucet.polkadot.io/?parachain=1111))

Only needed if compiling circuits from source:
- [nargo](https://noir-lang.org/) `1.0.0-beta.5` (install via `noirup --version 1.0.0-beta.5`)
- [bb](https://github.com/AztecProtocol/barretenberg) (Barretenberg) `v0.84.0` (installed automatically by `bbup` to match the pinned nargo version)

## Architecture

The verifier translates Aztec/Barretenberg's `HonkVerifier.sol` to Rust for PolkaVM:

- **Sumcheck** (LOG_N rounds): verifies polynomial identity
- **Relations** (26 sub-relations): UltraHonk constraint system
- **Shplemini**: KZG batch opening via EC precompiles
- **Transcript**: Fiat-Shamir challenge generation (streaming keccak)

EC operations use EVM precompiles (EIP-196/197) running natively in the Polkadot runtime, much cheaper than pure-Rust implementations inside the VM.
