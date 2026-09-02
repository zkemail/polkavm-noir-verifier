# Huge Public Input Count

Circuit-shape matrix entry stress-testing public-input count far beyond the production circuit: sums 1000 field elements against a target, all public.

- Gate count: 350
- Public inputs: 1,001
- `LOG_N`: 11

Largest generated binary in the matrix (113,127 bytes, versus 50-60KB for every other shape). The size comes from `buildPubInputParsing` in `generator/honk-verifier/generate.ts`, which generates one explicit `copy_from_slice` line per public input rather than a runtime loop.

## Generate proof artifacts

```bash
nargo execute
bb prove --bytecode_path ./target/huge_pub_input.json --witness_path ./target/huge_pub_input.gz --output_path ./target --oracle_hash keccak
bb write_vk --bytecode_path ./target/huge_pub_input.json --output_path ./target --oracle_hash keccak
bb verify --vk_path ./target/vk --proof_path ./target/proof --oracle_hash keccak
bb write_solidity_verifier --vk_path ./target/vk --output_path ./target/HonkVerifier.sol
```

Output in `target/`: `proof`, `public_inputs`, `vk`, `HonkVerifier.sol`
