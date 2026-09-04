# 04 - Gas Optimization & Benchmark Report

Milestone 1 evidence for optimization work reducing verification and deployment gas costs, plus a benchmark report comparing PolkaVM, REVM, and EVM mainnet costs.

This doc currently covers the first half of the deliverable - the optimization work itself. The comparative benchmark (PolkaVM vs. REVM vs. EVM mainnet, measured fresh under identical current conditions) is a separate, larger piece of work still pending.

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
