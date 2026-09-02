# zkemail twitter fixture

Real-world Noir circuit from `zkemail/ens-contracts:test/fixtures/linkHandleCommand/twitter`.
Verifies a zkEmail proof that links a Twitter/X handle to an email
address, with regex-based extraction of sender domain and handle.

- Gate count: 468,002 (run `bb gates -b target/zkemail/twitter@v1.json` to verify)
- Public inputs: 155
- `LOG_N`: 19

## Generate proof artifacts

Requires `nargo` (compatible with `compiler_version >=1.0.0`) and `bb`
(v0.84.0 or compatible). Dependencies (`zkemail.nr`, `zk-regex`,
`poseidon`) are fetched by nargo on first compile.

```bash
cd fixtures/zkemail
nargo execute
bb prove --bytecode_path ./target/zkemail/twitter@v1.json \
         --witness_path ./target/zkemail/twitter@v1.gz \
         --output_path ./target --oracle_hash keccak
bb write_vk --bytecode_path ./target/zkemail/twitter@v1.json \
            --output_path ./target --oracle_hash keccak
bb verify --vk_path ./target/vk --proof_path ./target/proof --oracle_hash keccak
bb write_solidity_verifier --vk_path ./target/vk --output_path ./target/HonkVerifier.sol
```

Output in `target/`: `proof`, `public_inputs`, `vk`, `HonkVerifier.sol`,
plus `target/zkemail/twitter@v1.{json,gz}`.

`Prover.toml` contains sample email inputs producing a deterministic
proof (same inputs and bb version always produce identical outputs).
