# UltraHonk Verifier (PolkaVM)

This is a generated PolkaVM smart contract that verifies UltraHonk ZK proofs on Polkadot Asset Hub.

## Build

```bash
cargo build --release
polkatool link --strip --min-stack-size 65536 --output honk_verifier.polkavm \
  target/riscv64emac-unknown-none-polkavm/release/honk_verifier.elf
```

## Deploy

```bash
cp .env.example .env   # add your PRIVATE_KEY
npm install
npx ts-node scripts/deploy.ts
```

## Solidity interface

```solidity
interface IHonkVerifier {
    function verify(bytes calldata proof, bytes32[] calldata publicInputs) external view returns (bytes1);
}
```

Call with `0xea50d0e4` selector. Returns `0x01` (valid) or `0x00` (invalid).

## Structure

```
├── src/
│   ├── main.rs              # Contract entry point (generated)
│   ├── sumcheck.rs           # Sumcheck rounds (generated, circuit-specific)
│   ├── vk.rs                 # Verification key (generated, circuit-specific)
│   └── honk/                 # Generic UltraHonk modules
│       ├── fr.rs             # BN254 scalar field arithmetic
│       ├── g1.rs             # EC precompile wrappers (EIP-196/197)
│       ├── proof.rs          # Proof deserialization
│       ├── relations.rs      # 26 sub-relation evaluations
│       ├── shplemini.rs      # KZG batch opening
│       └── transcript.rs     # Fiat-Shamir transcript (streaming keccak)
├── scripts/
│   └── deploy.ts             # Deployment script
├── interfaces/
│   └── IHonkVerifier.sol     # Solidity interface
└── Cargo.toml
```
