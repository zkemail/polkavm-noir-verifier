# Test Circuit

Simple Noir circuit for testing the verifier generator: `assert(x != y)`.

- Gate count: 18
- Public inputs: 1
- `LOG_N`: 5

## Generate proof artifacts

```bash
nargo execute
bb prove --bytecode_path ./target/circuit.json --witness_path ./target/circuit.gz --output_path ./target --oracle_hash keccak
bb write_vk --bytecode_path ./target/circuit.json --output_path ./target --oracle_hash keccak
bb verify --vk_path ./target/vk --proof_path ./target/proof --oracle_hash keccak
bb write_solidity_verifier --vk_path ./target/vk --output_path ./target/HonkVerifier.sol
```

Output in `target/`: `proof`, `public_inputs`, `vk`, `HonkVerifier.sol`
