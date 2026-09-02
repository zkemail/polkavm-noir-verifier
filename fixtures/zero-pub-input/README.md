# Zero Public Inputs

Circuit-shape edge case for the equivalence test matrix: no public inputs at all (`assert(secret != 0)`, `secret` private).

- Gate count: 17
- Public inputs: 0
- `LOG_N`: 5

## Generate proof artifacts

```bash
nargo execute
bb prove --bytecode_path ./target/zero_pub_input.json --witness_path ./target/zero_pub_input.gz --output_path ./target --oracle_hash keccak
bb write_vk --bytecode_path ./target/zero_pub_input.json --output_path ./target --oracle_hash keccak
bb verify --vk_path ./target/vk --proof_path ./target/proof --oracle_hash keccak
bb write_solidity_verifier --vk_path ./target/vk --output_path ./target/HonkVerifier.sol
```

Output in `target/`: `proof`, `public_inputs` (empty file), `vk`, `HonkVerifier.sol`
