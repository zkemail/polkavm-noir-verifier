# Large Circuit

Circuit-shape matrix entry with a distinct `LOG_N` from every other fixture: 400 chained field multiplications, result exposed as a public return value so the ACIR optimizer can't eliminate it.

- 1 public input (the computed return value)
- Circuit size: N=1024, LOG_N=10 (vs. LOG_N=5 for the other small fixtures and LOG_N=19 for zkemail)

## Generate proof artifacts

```bash
nargo execute
bb prove --bytecode_path ./target/large_circuit.json --witness_path ./target/large_circuit.gz --output_path ./target --oracle_hash keccak
bb write_vk --bytecode_path ./target/large_circuit.json --output_path ./target --oracle_hash keccak
bb verify --vk_path ./target/vk --proof_path ./target/proof --oracle_hash keccak
bb write_solidity_verifier --vk_path ./target/vk --output_path ./target/HonkVerifier.sol
```

Output in `target/`: `proof`, `public_inputs`, `vk`, `HonkVerifier.sol`
