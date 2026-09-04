# 04 - Gas Optimization & Benchmark Report

Milestone 1 evidence for optimization work reducing verification and deployment gas costs, plus a benchmark report comparing PolkaVM, REVM, and EVM mainnet costs.

This doc covers both halves of the deliverable: the optimization work itself, and a comparative benchmark across PolkaVM, REVM, and EVM, measured fresh (2026-09-04) via real transactions on Paseo and Ethereum Sepolia.

## Optimization work, chronologically

| Commit | Change | Measured effect (Paseo, same-day before/after) |
| --- | --- | --- |
| [`b7c6e95`](https://github.com/zkemail/polkavm-noir-verifier/commit/b7c6e95) | `Fr::inverse` switched from binary extended-GCD to the `modexp` precompile (`0x05`, Fermat's little theorem) | noir-circuit verify: **-13.4%**. zkemail: unblocked from exceeding the chain's per-transaction gas cap |
| [`9fee765`](https://github.com/zkemail/polkavm-noir-verifier/commit/9fee765) | 17 Fr constants hoisted to precomputed Montgomery-form `pub const`s, replacing runtime construction | noir-circuit: -0.33%. zkemail: -0.72% |
| [`48595e3`](https://github.com/zkemail/polkavm-noir-verifier/commit/48595e3) | Allocator swapped from `picoalloc` (TLSF, alloc+free) to `simplealloc` (bump-only) - `verify()` never frees mid-call | Deploy: noir-circuit -12%, zkemail -11%. Binary: -20% / -17%. Verify unchanged (noise) |
| [`66c0631`](https://github.com/zkemail/polkavm-noir-verifier/commit/66c0631) | Fiat-Shamir transcript hashing moved from in-VM `tiny-keccak` to `pallet-revive-uapi`'s native `hash_keccak_256` host function | Verify: noir-circuit -4.66%, zkemail -4.16%. Deploy also improved. Reached REVM parity - PVM at or below REVM on both circuits, measured the same day |
| [`5b35c74`](https://github.com/zkemail/polkavm-noir-verifier/commit/5b35c74), [`ad3fea3`](https://github.com/zkemail/polkavm-noir-verifier/commit/ad3fea3) | 3 correctness bugs fixed (extended/truncated-proof handling, dead error codes) and custom-error selectors matched byte-for-byte to REVM's own | Verify gas unchanged. Correctness and REVM revert-data parity, not a gas change |
| [`74da870`](https://github.com/zkemail/polkavm-noir-verifier/commit/74da870) | Sumcheck rounds switched from generator-unrolled code to a runtime `for` loop | Binary: noir-circuit -0.7%, zkemail -4.7% |

## Where the remaining cost actually is

A no-op `verify(bytes,bytes32[])` was deployed on both backends and its gas diffed against the real verifier's, isolating transaction/calldata overhead from actual verifier-code cost. Across both circuits and both backends, roughly 93-95% of total verify gas is chain overhead - transaction base cost and calldata cost, not verifier code. This is why the optimization work above, once it closed the gap to REVM, had little further room left at the code level.

## Comparative benchmark: PVM vs REVM vs EVM

Four ways to get `HonkVerifier.sol`'s logic on-chain, measured fresh (2026-09-04, real transactions):

- **PVM (native)**: our hand-written Rust verifier, compiled to PVM. Deployed and documented in [`05_testnet_deployment_validation.md`](./05_testnet_deployment_validation.md).
- **PVM (resolc)**: the same `HonkVerifier.sol`, compiled straight to PVM by `resolc`, Solidity's own compiler for this target, no rewrite required.
- **REVM**: `HonkVerifier.sol` compiled with plain `solc` to standard EVM bytecode, deployed on Paseo and run through Polkadot Hub's REVM engine.
- **EVM**: the same REVM bytecode deployed to Ethereum Sepolia, as the real-mainnet cost baseline.

| | PVM (native) | PVM (resolc) | REVM | EVM |
| --- | ---: | ---: | ---: | ---: |
| Bytecode size - noir-circuit | 50,616 | 567,925 | 21,579 | 21,579 |
| Bytecode size - zkemail | 59,743 | 567,934 | 21,585 | 21,585 |
| Deploy gas - noir-circuit | 728,487 | 5,081,613 | 495,855 | 4,718,904 |
| Deploy gas - zkemail | 801,745 | 5,081,686 | 495,914 | 4,720,172 |
| Verify gas - noir-circuit | 74,371 | fails - OutOfGas | 76,779 | 1,826,487 |
| Verify gas - zkemail | 100,081 | fails - OutOfGas | 103,966 | 2,897,266 |

`resolc` compiling Solidity straight to PVM with no rewrite is the obvious first thing to try before hand-writing a native verifier. It deploys, but every `verify()` call on both fixtures fails with `OutOfGas` - the direct reason the native PVM verifier exists. This isn't a configuration issue on our side: the same outcome holds using the exact `resolc` version and optimizer settings this project's own tooling ([`polkavm-hardhat-template`](https://github.com/zkemail/polkavm-hardhat-template)) pins (`0.5.0`, optimizer `runs: 10000`) - that configuration produces bytecode nearly double the size and is rejected before deployment (`StaticMemoryTooLarge`), never reaching an on-chain gas figure at all.

REVM and EVM execute identical compiled bytecode, so the ~24-29x verify-gas gap between them is chain-level gas pricing, not code efficiency - Paseo's PVM-based fee metering versus real Ethereum gas costs.

**Deploy gas is steady-state ("warm") pricing.** Paseo (pallet-revive) stores contract code once per unique bytecode hash; deploying *identical* bytecode again reuses the stored code instead of paying to store it a second time, and is substantially cheaper as a result. This is reported consistently across all four legs - it's what any future re-run of this benchmark will observe from now on, since every leg's bytecode is already "known" on Paseo. Real Ethereum has no equivalent (`CODEDEPOSIT` charges the same per-byte cost every time), so the EVM column is unaffected regardless. Full methodology is in [`benchmarks/README.md`](./benchmarks/README.md).

### Addresses and transactions

| Fixture | Address | Deploy tx | Verify tx |
| --- | --- | --- | --- |
| PVM (resolc), noir-circuit | [`0x5F086Fd4140A296CF38C1AE30cC88a5CC5396e19`](https://blockscout-testnet.polkadot.io/address/0x5F086Fd4140A296CF38C1AE30cC88a5CC5396e19) | [`0x2c1d3af1...c6ce9`](https://blockscout-testnet.polkadot.io/tx/0x2c1d3af18c70cbb0555092aa9cb1f633a898a2575969d75719c70266487c6ce9) | every attempt reverts OutOfGas ([`0xd491133b...aae5ec`](https://blockscout-testnet.polkadot.io/tx/0xd491133b95b3b3aa3174d8060c79725489ff0996d4d00442df9a4b2ce7aae5ec)) |
| PVM (resolc), zkemail | [`0xe431ffAE13F58731d7A264d53238a006A5bAa5F7`](https://blockscout-testnet.polkadot.io/address/0xe431ffAE13F58731d7A264d53238a006A5bAa5F7) | [`0x22f97555...4fdb5`](https://blockscout-testnet.polkadot.io/tx/0x22f9755502669c13cd739de48ff1462e7aced39cc085085e6f7e2f21fa94fdb5) | every attempt reverts OutOfGas ([`0x900e72df...12ea0`](https://blockscout-testnet.polkadot.io/tx/0x900e72df7fc066989fa953c05da05eb35f2ff9171c5300e88bfd99cb06c12ea0)) |
| REVM, noir-circuit | [`0xf07Ad7f066f8fA09899F620C9Ace6FA87c2c3216`](https://blockscout-testnet.polkadot.io/address/0xf07Ad7f066f8fA09899F620C9Ace6FA87c2c3216) | [`0xbc647c00...0e706`](https://blockscout-testnet.polkadot.io/tx/0xbc647c00f9d317fac3119d70aeacf49b06730f48b1c76a65c12a2809b1c0e706) | [`0xb1e4cee8...bb1be`](https://blockscout-testnet.polkadot.io/tx/0xb1e4cee84dada687a5ba907b46aeb0716f8181746dd5e8afa52407367f0bb1be) |
| REVM, zkemail | [`0x7bF9091bfeF58bd0c217Ce1C75f961D9611e9FD0`](https://blockscout-testnet.polkadot.io/address/0x7bF9091bfeF58bd0c217Ce1C75f961D9611e9FD0) | [`0x343bd19f...9cc7a7`](https://blockscout-testnet.polkadot.io/tx/0x343bd19f3589933ec661452d86dcef8cb2604e873775b0bb3eaa738bb19cc7a7) | [`0x4d7bb186...88748f`](https://blockscout-testnet.polkadot.io/tx/0x4d7bb186e6929c1d0460b92b708c30ccebb9a766a024ba34eed5d486d888748f) |
| EVM (Sepolia), noir-circuit | [`0xb7ebf632fE49dA424b9bb362F03DCDF265F1a8EA`](https://sepolia.etherscan.io/address/0xb7ebf632fE49dA424b9bb362F03DCDF265F1a8EA) | [`0x61e73a6c...8c9f88`](https://sepolia.etherscan.io/tx/0x61e73a6cb4e09b39971385ba22f5f774e8bc8eb6a5ff48fca5211cea478c9f88) | [`0x0480a47d...b844c`](https://sepolia.etherscan.io/tx/0x0480a47d3f76c7b2dab7c4a8eb3190df19eeaf98071ab39e6d07383096fb844c) |
| EVM (Sepolia), zkemail | [`0x60276A3710AD578E8eecF221C8841b86dbfD757b`](https://sepolia.etherscan.io/address/0x60276A3710AD578E8eecF221C8841b86dbfD757b) | [`0x096a1e1c...7439a`](https://sepolia.etherscan.io/tx/0x096a1e1c135761295bfe15ee2faa7d68d88e2e7edeae5a0be85c0fd02b67439a) | [`0x055770ac...dabaf2`](https://sepolia.etherscan.io/tx/0x055770aca78b19836f4802051ca3acdcd9d49d89ab4f7bc07d5e076b38dabaf2) |

All four legs' on-chain bytecode was hash-checked against the local build before these numbers were recorded: `resolc` output matches byte-for-byte (keccak match, PVM magic-prefix `0x50564d0000` present); REVM/EVM on-chain runtime code matches the local build with the expected handful of bytes differing (Solidity `immutable` substitution during construction), confirmed identical between the Paseo and Sepolia deployments of the same source.

Raw results and the scripts that produced them are committed in [`benchmarks/`](./benchmarks/); its `aggregate.js` regenerates the table above directly from the committed JSON.
