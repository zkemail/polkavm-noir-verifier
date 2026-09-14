# Milestone 2 - Security Review, Developer Guide & Kusama Mainnet Deployment

Primary Goal: review the tool for security, document it for external developers, and deploy a reference verifier to Kusama Asset Hub mainnet.

## Status

2 of 4 deliverables complete. Mainnet deployment is prepped but deliberately not executed yet - held pending the Kusama curators' review of Milestone 1, since that review covers the same underlying pipeline this deployment would spend real KSM on. The video walkthrough is on hold behind the mainnet deployment, since it should demonstrate the finished, deployed product rather than a pre-mainnet snapshot.

## Deliverables

| # | Name | Description | Hours | What was done / Proof |
| --- | --- | --- | ---: | --- |
| 1 | Internal Security Review | Structured review of stack/heap sizing safety across circuit shapes, panic/overflow handling, and revert-data parity. | 100 | All three named axes covered, plus a full `overflow-checks=false` sweep. 3 real findings, all fixed: unprotected field inversions that could panic into an opaque trap, discarded precompile call-success status (diverging from the Solidity reference's own behavior), and an unbounded heap allocation reachable from raw calldata length - the last one reproduced live on a local devnet both before (opaque `ContractTrapped`) and after (clean revert) the fix. Stack sizing and the overflow sweep concluded clean with no fix needed. **Proof:** [`01_internal_security_review.md`](./01_internal_security_review.md). |
| 2 | Bring-Your-Own-Circuit Developer Guide | Public how-to documentation for external Noir developers to generate and deploy a verifier for their own circuit. | 60 | The full external-developer pipeline (`nargo new` through a real deployment), distinct from the two flows already in the root README. Verified end-to-end against a fresh circuit that exists nowhere else in this repo, not written from memory of the existing pipeline - both the valid-proof-accepts and invalid-proof-rejects paths confirmed live. **Proof:** [`02_byo_circuit_guide.md`](./02_byo_circuit_guide.md). |
| 3 | Kusama Mainnet Deployment | Deploy a reference verifier (e.g. the zkemail circuit) to Kusama Asset Hub mainnet and verify correct functionality. | 100 | Prep complete, execution on hold. Reference circuit confirmed (`zkemail/twitter@v1`, the only fixture that's a real production circuit rather than a synthetic test shape). `deploy.ts`'s hardcoded Paseo RPC URL made configurable. The real deploy script rehearsed end-to-end on Paseo against the post-security-review bytecode, including a real mined `verify()` call. Mainnet cost estimated from a live gas-price read against the real Kusama Asset Hub RPC, corrected to account for pallet-revive's cold-vs-warm bytecode-storage pricing (a real, applicable consequence of a fact already documented in Milestone 1, not new). **Proof:** [`03_kusama_mainnet_deployment.md`](./03_kusama_mainnet_deployment.md). |
| 4 | Video Walkthrough & Final Delivery | Record a short video walkthrough and prepare final delivery materials. | 60 | Not started - on hold until the mainnet deployment above actually happens, so the walkthrough demonstrates the finished, deployed product. |

## Evidence Index

- [`01_internal_security_review.md`](./01_internal_security_review.md) - 3 findings, all fixed and verified (2 by regression + live devnet reproduction, 1 by direct live testing), plus stack-sizing and overflow-handling axes ruled out with concrete arguments.
- [`02_byo_circuit_guide.md`](./02_byo_circuit_guide.md) - step-by-step guide for a developer's own Noir circuit, verified against a circuit built specifically for that verification, not one of this repo's existing fixtures.
- [`03_kusama_mainnet_deployment.md`](./03_kusama_mainnet_deployment.md) - reference circuit confirmed, `deploy.ts` fixed and rehearsed on Paseo, mainnet cost estimated from a live chain read. Real mainnet execution and its evidence (address, deploy/verify tx, bytecode provenance) still pending curator go-ahead.
