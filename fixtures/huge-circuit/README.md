# Huge Circuit

Circuit-shape matrix entry genuinely larger than the production circuit: 550,000 chained field multiplications, result exposed as a public return value so the ACIR optimizer can't eliminate it.

- 1 public input (the computed return value)
- Circuit size: N=2,097,152, LOG_N=21 (vs. LOG_N=19 for the zkemail production circuit - a different power-of-two bucket, not just a bigger number within the same one)

`nargo execute` takes ~24s and `bb prove` ~21s locally; the generated PolkaVM binary is 50,672 bytes - identical to every other 1-public-input shape regardless of `LOG_N`, confirming binary size no longer depends on circuit size after the sumcheck for-loop fix.

## Generate proof artifacts

```bash
nargo execute
bb prove --bytecode_path ./target/huge_circuit.json --witness_path ./target/huge_circuit.gz --output_path ./target --oracle_hash keccak
bb write_vk --bytecode_path ./target/huge_circuit.json --output_path ./target --oracle_hash keccak
bb verify --vk_path ./target/vk --proof_path ./target/proof --oracle_hash keccak
bb write_solidity_verifier --vk_path ./target/vk --output_path ./target/HonkVerifier.sol
```

Output in `target/`: `proof`, `public_inputs`, `vk`, `HonkVerifier.sol`
