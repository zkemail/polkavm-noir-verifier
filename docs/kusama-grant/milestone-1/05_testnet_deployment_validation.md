# 05 - Testnet Deployment & Validation

Milestone 1 evidence for deployment and verification across multiple circuit shapes on Paseo Asset Hub, from a minimal fixture up to a production-scale circuit.

## What it is

Each fixture's generated PolkaVM verifier ([`generator/honk-verifier/static/scripts/deploy.ts`](../../../generator/honk-verifier/static/scripts/deploy.ts)) is deployed as a raw contract-creation transaction to Paseo Asset Hub (`https://eth-rpc-testnet.polkadot.io/`), then exercised with the same 5 test vectors [`scripts/test.sh`](../../../scripts/test.sh) uses locally - a valid proof, a wrong public input, and 3 corrupted-proof-byte variants - asserting `verify()` accepts the valid proof and reverts on every corrupted one.

This is the same 7-fixture matrix documented in [`06_automated_test_suite_ci.md`](./06_automated_test_suite_ci.md) (see the root [`README.md`](../../../README.md#tested-with) for current gate count / public input count / `LOG_N` per fixture), deployed for real rather than to a local devnet - the point being to confirm nothing about the local-devnet equivalence results is an artifact of the local environment (gas metering, deployment size limits, RPC behavior) rather than genuine on real infrastructure.

## Proof it works

All 7 fixtures deployed to Paseo Asset Hub on 2026-09-04 and all 35/35 test vectors (5 per fixture, via [`scripts/test.sh`](../../../scripts/test.sh)) passed, matching the local-devnet result in `06_automated_test_suite_ci.md` exactly. Each fixture's `deployment.json` (written by `scripts/deploy.ts`) is committed in [`deployments/`](./deployments/) alongside this doc, so the table below is backed by a committed artifact, not just prose.

| Fixture | Address | Tx hash | Deploy gas | Tests |
| --- | --- | --- | ---: | --- |
| `fixtures/zero-pub-input` | [`0x178D9F2CdF294Fe786fA93CbaF02D12E61168093`](https://blockscout-testnet.polkadot.io/address/0x178D9F2CdF294Fe786fA93CbaF02D12E61168093) | [`0x26d2cae0...7648d`](https://blockscout-testnet.polkadot.io/tx/0x26d2cae0fc5a72a4e45bf93ca7848a28be80c697ccbd9da4c4a969e7aff7648d) | 727,251 | 5/5 |
| `fixtures/noir-circuit` | [`0x803bf952247f691b13Dd0D3A475D5C8f7F5B990e`](https://blockscout-testnet.polkadot.io/address/0x803bf952247f691b13Dd0D3A475D5C8f7F5B990e) | [`0x7e5cb750...9d11c`](https://blockscout-testnet.polkadot.io/tx/0x7e5cb7505fe8eb00dc5d6a77f86a75031eb3514584590db55523ca0ca8d9d11c) | 728,487 | 5/5 |
| `fixtures/multi-pub-input` | [`0xBF65a1058ed1A28842a1d3D7Af32aDf6b3aFf02D`](https://blockscout-testnet.polkadot.io/address/0xBF65a1058ed1A28842a1d3D7Af32aDf6b3aFf02D) | [`0x50b59ab7...4c081`](https://blockscout-testnet.polkadot.io/tx/0x50b59ab7751678fc9b4084915c734807215bb5cd3448aa2f3fe7c97c20b4c081) | 730,590 | 5/5 |
| `fixtures/huge-pub-input` | [`0x3448AD1c56E532b8bFf210B879DF231DbcA84125`](https://blockscout-testnet.polkadot.io/address/0x3448AD1c56E532b8bFf210B879DF231DbcA84125) | [`0x03e7a59c...5d2fad`](https://blockscout-testnet.polkadot.io/tx/0x03e7a59c42071779385eb6b71c4126332d60efa13c8eb4a578b1b630f75d2fad) | 1,230,239 | 5/5 |
| `fixtures/large-circuit` | [`0x66cb4A0963a770057e0e0D58430c899618dd9F72`](https://blockscout-testnet.polkadot.io/address/0x66cb4A0963a770057e0e0D58430c899618dd9F72) | [`0xed2b9976...85754`](https://blockscout-testnet.polkadot.io/tx/0xed2b99765ae498d676548b8c551a2b878509c6094a02fd951712b2c101885754) | 728,929 | 5/5 |
| `fixtures/huge-circuit` | [`0xD071BcDab20bCb2aD13684174C421Ce86D5AAeb3`](https://blockscout-testnet.polkadot.io/address/0xD071BcDab20bCb2aD13684174C421Ce86D5AAeb3) | [`0xc995c776...ec2cd5`](https://blockscout-testnet.polkadot.io/tx/0xc995c7767202fe4e35ba47403effaacd45c5fcdfb3191c22d3f7c89eb5ec2cd5) | 728,937 | 5/5 |
| `fixtures/zkemail` | [`0x6D0328072e8D39252066c8bc6489772e505df9a2`](https://blockscout-testnet.polkadot.io/address/0x6D0328072e8D39252066c8bc6489772e505df9a2) | [`0x24798654...e34d84`](https://blockscout-testnet.polkadot.io/tx/0x247986548d317476adcc93cee24ee9214211ccd10fa7eb2e4695b7b318e34d84) | 801,745 | 5/5 |

## Bytecode Provenance

PolkaVM has no source-verification service equivalent to Etherscan yet: provenance here means the locally-computed bytecode hash matching the deployed runtime's own bytecode, checked directly against the chain via RPC, not asserted from a block explorer. Every generated `.polkavm` binary also starts with a fixed 5-byte magic prefix (`0x50564d0000`, ASCII `PVM`) from `polkatool link` - checking for it directly against the deployed code is a cheap sanity check that what's on-chain is genuinely PolkaVM bytecode, not e.g. an empty or malformed deployment. All 7 below were confirmed via the `cast` commands in Reproduce, run against the real addresses above.

| Fixture | `keccak256(deployedBytecode)` | Match | PVM magic |
| --- | --- | --- | --- |
| `fixtures/zero-pub-input` | `0xa2d4e009041471b2147b227f7533903fcb964a8355fd8e03cabe9a522213b9c9` | on-chain == local | `0x50564d0000` |
| `fixtures/noir-circuit` | `0xbea07898e53d03101cde8979cd00c8279cb9b9c087635990996152d9daf6a62c` | on-chain == local | `0x50564d0000` |
| `fixtures/multi-pub-input` | `0x2a466752824a828e8654f22539d4d7700f6a273a88689b6f36376a17faeffe6e` | on-chain == local | `0x50564d0000` |
| `fixtures/huge-pub-input` | `0x739a90c8e1fefbbc777deb5f1f87566d0ef32d07fce58e16ac0b29a465db8695` | on-chain == local | `0x50564d0000` |
| `fixtures/large-circuit` | `0x6acb27a7a7dab55850e874f624be1259e34a01452d79169467b0c72e1db0638a` | on-chain == local | `0x50564d0000` |
| `fixtures/huge-circuit` | `0x5fd0783987acd36a9fa22c76e6286d7f0fc7dcc2f121d9da7a074950c78ab704` | on-chain == local | `0x50564d0000` |
| `fixtures/zkemail` | `0x0f1ee18933512a784ddd488e91ded72a2e8c89830e9899cf141a75a0c092a45e` | on-chain == local | `0x50564d0000` |

## Reproduce

Deploy and test a fixture:

```bash
cd fixtures/<fixture-name>
nargo execute
bb prove --bytecode_path ./target/<name>.json --witness_path ./target/<name>.gz --output_path ./target --oracle_hash keccak
bb write_vk --bytecode_path ./target/<name>.json --output_path ./target --oracle_hash keccak
bb write_solidity_verifier --vk_path ./target/vk --output_path ./target/HonkVerifier.sol
cd ../..
./scripts/generate.sh "$(pwd)/fixtures/<fixture-name>/target/HonkVerifier.sol" "$(pwd)/contracts/<fixture-name>"
cd contracts/<fixture-name>
cp .env.example .env   # add a funded PRIVATE_KEY (PAS faucet: https://faucet.polkadot.io/?parachain=1000)
npm install
npx ts-node scripts/deploy.ts
cd ../..
./scripts/test.sh fixtures/<fixture-name>/target/proof fixtures/<fixture-name>/target/public_inputs contracts/<fixture-name>
```

Independently confirm the bytecode provenance table above for any deployed address:

```bash
# PVM bytecode magic (expect 50564d0000)
cast code <address> --rpc-url https://eth-rpc-testnet.polkadot.io/ | cut -c1-12

# on-chain vs local runtime-bytecode hash (expect a match)
cast code <address> --rpc-url https://eth-rpc-testnet.polkadot.io/ | cast keccak
cat contracts/<fixture-name>/honk_verifier.polkavm | xxd -p -c0 | (echo -n "0x"; cat) | cast keccak
```
