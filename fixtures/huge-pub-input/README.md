# Huge Public Input Count

Circuit-shape matrix entry stress-testing public-input count far beyond the production circuit: sums 1000 field elements against a target, all public.

- 1,001 public inputs (1000-element array + the sum) - vs. 155 for the zkemail production circuit
- Circuit size: N=512, LOG_N=11 (the 1000 additions themselves cost almost nothing in ACIR - addition is linear - the constraint count comes from array handling and the final assert)

Deployed and verified on both the local devnet and real Paseo Asset Hub (see `06_automated_test_suite_ci.md` in the grant docs) - deploy gas 2,762,154 on Paseo, no size-related deployment issue despite the largest generated binary in the matrix (113,127 bytes, versus 50-60KB for every other shape - direct evidence of the per-public-input unrolling cost in `buildPubInputParsing`, see `01_planning_research.md`).

## Generate proof artifacts

```bash
nargo execute
bb prove --bytecode_path ./target/huge_pub_input.json --witness_path ./target/huge_pub_input.gz --output_path ./target --oracle_hash keccak
bb write_vk --bytecode_path ./target/huge_pub_input.json --output_path ./target --oracle_hash keccak
bb verify --vk_path ./target/vk --proof_path ./target/proof --oracle_hash keccak
bb write_solidity_verifier --vk_path ./target/vk --output_path ./target/HonkVerifier.sol
```

Output in `target/`: `proof`, `public_inputs`, `vk`, `HonkVerifier.sol`
