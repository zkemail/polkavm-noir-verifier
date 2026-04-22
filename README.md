# UltraHonk Verifier for PolkaVM

Generate and deploy UltraHonk ZK proof verifiers as PolkaVM smart contracts on Polkadot Asset Hub.

Given any Noir circuit, this tool produces a complete Rust verifier contract that runs on-chain.

## How it works

```
Noir circuit → bb (Barretenberg) → HonkVerifier.sol → Generator → PolkaVM contract
```

The generator reads the Solidity verifier output from Barretenberg's `bb write_solidity_verifier` and produces a buildable Rust PolkaVM project with:
- Circuit-specific verification key, sumcheck rounds, and entry point
- Generic UltraHonk verification modules (transcript, relations, shplemini/KZG)
- EC operations via EVM precompiles (ecAdd, ecMul, ecPairing)
- Deploy and test scripts

## Quick start

```bash
# 1. Compile your Noir circuit and generate artifacts
cd fixtures/noir-circuit
nargo execute
bb prove --bytecode_path ./target/circuit.json --witness_path ./target/circuit.gz --output_path ./target --oracle_hash keccak
bb write_vk --bytecode_path ./target/circuit.json --output_path ./target --oracle_hash keccak
bb write_solidity_verifier --vk_path ./target/vk --output_path ./target/HonkVerifier.sol

# 2. Generate the PolkaVM verifier
cd ../..
./scripts/generate.sh
# Or: cd generator && npx ts-node generate-verifier.ts honk --sol ../fixtures/noir-circuit/target/HonkVerifier.sol --out ../contracts/honk-verifier --build

# 3. Deploy and test
cd contracts/honk-verifier
cp .env.example .env  # add your PRIVATE_KEY
npm install
npx ts-node scripts/deploy.ts
npx ts-node scripts/quick_test.ts
npx ts-node scripts/test_valid_and_invalid.ts
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
│           ├── scripts/            # Deploy and test scripts
│           └── interfaces/         # IHonkVerifier.sol
├── fixtures/
│   └── noir-circuit/               # Test circuit (assert x != y)
├── scripts/
│   └── generate.sh                 # Convenience wrapper
└── contracts/                      # Generated output (gitignored)
```

## Tested with

- **Simple circuit**: LOG_N=5, 1 public input — 5/5 tests pass
- **ZK email circuit** ([zkemail/ens-contracts](https://github.com/zkemail/ens-contracts)): LOG_N=19, 155 public inputs — 3/3 tests pass

## Requirements

- [Rust](https://rustup.rs/) (nightly 2025+)
- [nargo](https://noir-lang.org/) 1.0.0-beta.5+
- [bb](https://github.com/AztecProtocol/barretenberg) (Barretenberg) 0.84.0+
- [polkatool](https://github.com/nicpottier/polkatool) for PolkaVM linking
- Node.js 18+ (for generator and deploy scripts)
- PAS tokens on Paseo testnet ([faucet](https://faucet.polkadot.io/?parachain=1111))

## Architecture

The verifier translates Aztec/Barretenberg's `HonkVerifier.sol` to Rust for PolkaVM:

- **Sumcheck** (LOG_N rounds) — verifies polynomial identity
- **Relations** (26 sub-relations) — UltraHonk constraint system
- **Shplemini** — KZG batch opening via EC precompiles
- **Transcript** — Fiat-Shamir challenge generation (streaming keccak)

EC operations use EVM precompiles (EIP-196/197) running natively in the Polkadot runtime — much cheaper than pure-Rust implementations inside the VM.
