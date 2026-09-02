# Huge Circuit

Circuit-shape matrix entry genuinely larger than the production circuit: 550,000 chained field multiplications, result exposed as a public return value so the ACIR optimizer can't eliminate it.

- Gate count: 1,100,015
- Public inputs: 1
- `LOG_N`: 21

## Generate proof artifacts

```bash
nargo execute
bb prove --bytecode_path ./target/huge_circuit.json --witness_path ./target/huge_circuit.gz --output_path ./target --oracle_hash keccak
bb write_vk --bytecode_path ./target/huge_circuit.json --output_path ./target --oracle_hash keccak
bb verify --vk_path ./target/vk --proof_path ./target/proof --oracle_hash keccak
bb write_solidity_verifier --vk_path ./target/vk --output_path ./target/HonkVerifier.sol
```

Output in `target/`: `proof`, `public_inputs`, `vk`, `HonkVerifier.sol`
