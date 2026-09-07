# 03 - Kusama Mainnet Deployment

Milestone 2 deliverable: deploy a reference verifier to Kusama Asset Hub mainnet and verify correct functionality.

## Status: prep complete, execution on hold

Execution (the real mainnet transaction) is deliberately on hold pending Milestone 1's review by the Kusama curators, per an explicit decision made when M2 started - Milestone 1's testnet evidence should be reviewed before real KSM is committed on the strength of the same underlying pipeline. This doc tracks prep work completed now so the real deployment is fast and low-risk once that review is back, and will be filled in with real addresses, transactions, and gas numbers - matching Milestone 1's evidence standard - once it happens.

## Reference circuit: confirmed

[`fixtures/zkemail`](../../../fixtures/zkemail) - `zkemail/twitter@v1`, a real production circuit from `zkemail/ens-contracts:test/fixtures/linkHandleCommand/twitter` (verifies a zkEmail proof linking a Twitter/X handle to an email address), pinned by git tag to specific dependency versions (`zkemail.nr` v.1.0.1-beta.5, `zk-regex` 2.2.0, `poseidon` v0.1.0) so it's stable and reproducible, not at risk of silently drifting before deployment. 468,002 gates, 155 public inputs, `LOG_N` 19.

Chosen over this repo's other 6 fixtures because it's the only one that's an actual real-world circuit from this project's own namesake use case, rather than a synthetic edge-case/stress-test shape - deploying it as the mainnet reference verifier demonstrates the pipeline works for the thing this grant is actually about, not just in the abstract.

## Prep work completed

- **`generator/honk-verifier/static/scripts/deploy.ts`'s RPC URL is now configurable** (`main` commit `395a4fe`) - was hardcoded to Paseo testnet with no way to target another network, which would have made a mainnet deployment impossible via this script without a code change made under time pressure. Now defaults to Paseo (`RPC_URL` unset) with no change to existing workflows, and derives the balance-line token label from the connected chain's ID (`PAS`/`KSM`/generic fallback) instead of hardcoding `PAS`.

- **Rehearsed the real `deploy.ts` script end to end on Paseo** (not the raw-JSON-RPC workaround used for local-devnet testing elsewhere in this project) - built `zkemail`'s verifier from current `main` (i.e. including all three Milestone 2 security-review fixes) and ran the actual planned process below against it for real, including a real mined `verify()` call, not just a read-only one:

  | | Address | Tx | Gas |
  | --- | --- | --- | ---: |
  | Deploy | [`0x91cD2E1eB22055f1Adf5D1402F210114abBe7170`](https://blockscout-testnet.polkadot.io/address/0x91cD2E1eB22055f1Adf5D1402F210114abBe7170) | [`0xec238b60...9be8d2`](https://blockscout-testnet.polkadot.io/tx/0xec238b60c85a779bc01e6e9ecbfd897eaa2332a8c331a387e9a917f4b49be8d2) | 1,802,781 |
  | Verify | same contract | [`0x2a9d129c...7b879`](https://blockscout-testnet.polkadot.io/tx/0x2a9d129c334b070c57baa45a52156f0a616cc363b98cd3fcd6cfa22cdb27b879) | 99,992 |

- **Applied an already-documented Milestone 1 fact to catch a stale cost assumption, not a new discovery**: [`04_gas_optimization_benchmark_report.md`](../milestone-1/04_gas_optimization_benchmark_report.md) already established that Paseo (pallet-revive) stores contract code once per unique bytecode hash, and redeploying *identical* bytecode reuses the stored code at a substantially cheaper "warm" price. The rehearsal's deploy gas (1,802,781) came back more than 2x Milestone 1's originally recorded number for this same circuit (801,745), despite the post-fix binary being only 164 bytes larger (59,907 vs 59,743) - exactly what that already-known mechanism predicts if M1's original number was itself a warm one. Confirmed directly with two follow-up deployments rather than assumed:

  | | Bytecode | Address | Tx | Gas |
  | --- | --- | --- | --- | ---: |
  | Redeploy 1 | old (pre-fix, identical to M1's) | [`0xf8375aFD3611305E02cae5AF7E8Ac244B8A6d2af`](https://blockscout-testnet.polkadot.io/address/0xf8375aFD3611305E02cae5AF7E8Ac244B8A6d2af) | [`0xca477545...c3b4f0`](https://blockscout-testnet.polkadot.io/tx/0xca477545b455d3858f19885ff68ddebddbfa8a4540c0424699fa723e00c3b4f0) | 801,745 |
  | Redeploy 2 | new (post-fix, same as the cold deploy above) | [`0xd20053b109bb468CC3FaAAe59f3bDeA558E863B2`](https://blockscout-testnet.polkadot.io/address/0xd20053b109bb468CC3FaAAe59f3bDeA558E863B2) | [`0xda5b89ab...19e054d`](https://blockscout-testnet.polkadot.io/tx/0xda5b89abe06aefbcf0a6e56c4d1fd2565810fd2a1149cc3c431d15ff919e054d) | 803,061 |

  Both redeploys of *already-stored* bytecode came back at the cheap, ~800K warm price - the old bytecode redeploy matched M1's original number exactly, confirming M1's 801,745 was itself a warm number (that exact bytecode was already stored from earlier test runs before M1's evidence-of-record deployment). Kusama mainnet has never stored any of this project's bytecode, so **the real mainnet deployment will pay the cold price**, not the warm one - a real, applicable consequence of M1's own documented mechanism that a naive reuse of its headline number would have missed.

- **Cost estimate corrected accordingly, using the real cold number instead of Milestone 1's warm one**: gas price queried live from `eth-rpc-kusama.polkadot.io` (`eth_chainId` confirmed `420420418`, genuinely Kusama Asset Hub mainnet), 10 Gwei via `eth_gasPrice`.

  | | Gas | KSM |
  | --- | ---: | ---: |
  | Deploy (cold - first time this bytecode hash is stored, which mainnet will be) | 1,802,781 | 0.01802781 |
  | Verify (one call) | 99,992 | 0.00099992 |
  | **Total** | 1,902,773 | **~0.019 KSM** |

  Still a point-in-time gas-price read, not a guarantee - but the gas *usage* numbers themselves are now real cold-path measurements, not a warm number applied to a cold scenario.

## Still needed before execution

- **Curator go-ahead**, post Milestone 1 review.
- **A funded Kusama mainnet wallet** (real KSM) - has to come from the grant recipient; there's no testnet-faucet equivalent for mainnet currency.

## Planned process (once unblocked)

```bash
cd fixtures/zkemail
nargo execute
bb prove --bytecode_path ./target/zkemail/twitter@v1.json --witness_path ./target/zkemail/twitter@v1.gz --output_path ./target --oracle_hash keccak
bb write_vk --bytecode_path ./target/zkemail/twitter@v1.json --output_path ./target --oracle_hash keccak
bb write_solidity_verifier --vk_path ./target/vk --output_path ./target/HonkVerifier.sol

cd ../..
./scripts/generate.sh "$(pwd)/fixtures/zkemail/target/HonkVerifier.sol" "$(pwd)/contracts/zkemail-mainnet"
cd contracts/zkemail-mainnet
cp .env.example .env
# set PRIVATE_KEY to a funded Kusama mainnet account
# set RPC_URL=https://eth-rpc-kusama.polkadot.io/
npm install
npx ts-node scripts/deploy.ts

cd ../..
./scripts/test.sh fixtures/zkemail/target/proof fixtures/zkemail/target/public_inputs contracts/zkemail-mainnet
```

## Evidence to be added after real deployment

Matching Milestone 1's [`05_testnet_deployment_validation.md`](../milestone-1/05_testnet_deployment_validation.md) standard: contract address, deploy tx hash and gas, a real mined `verify()` transaction (not just a read-only call) with its tx hash and gas, and independently-confirmed bytecode provenance (`keccak256(deployedBytecode)` matching the local build, plus the PVM magic-byte check).
