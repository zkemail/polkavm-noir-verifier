# Multi Public Input

Circuit-shape matrix entry between the 1-public-input and 155-public-input extremes: `secret + a + b + c + d == sum`, 5 public inputs.

- 5 public inputs (`a`, `b`, `c`, `d`, `sum`)
- Circuit size: N=32, LOG_N=5

## Generate proof artifacts

```bash
nargo execute
bb prove --bytecode_path ./target/multi_pub_input.json --witness_path ./target/multi_pub_input.gz --output_path ./target --oracle_hash keccak
bb write_vk --bytecode_path ./target/multi_pub_input.json --output_path ./target --oracle_hash keccak
bb verify --vk_path ./target/vk --proof_path ./target/proof --oracle_hash keccak
bb write_solidity_verifier --vk_path ./target/vk --output_path ./target/HonkVerifier.sol
```

Output in `target/`: `proof`, `public_inputs`, `vk`, `HonkVerifier.sol`
