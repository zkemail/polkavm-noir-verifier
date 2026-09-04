# 07 - Documentation

Milestone 1 evidence for generator usage docs and architecture/provenance notes (mapping from the Solidity reference to the emitted Rust).

## Generator usage docs

- [`README.md`](../../../README.md)
- [`generator/honk-verifier/static/README.md`](../../../generator/honk-verifier/static/README.md) (copied verbatim into every project the generator produces)

## Architecture / provenance mapping

Every module that's a direct translation of part of `HonkVerifier.sol` documents that in its own doc comment:

| Rust file | Translated from (HonkVerifier.sol) |
| --- | --- |
| [`main.rs.tmpl`](../../../generator/honk-verifier/templates/main.rs.tmpl) (templated) | `BaseHonkVerifier.verify()` |
| [`vk.rs.tmpl`](../../../generator/honk-verifier/templates/vk.rs.tmpl) (templated) | `HonkVerificationKey.loadVerificationKey()` - 27 named G1 commitment points |
| [`sumcheck.rs.tmpl`](../../../generator/honk-verifier/templates/sumcheck.rs.tmpl) (templated) | `BaseHonkVerifier.verifySumcheck()` - a runtime `for` loop over `LOG_N` ([`buildSumcheckRounds`](../../../generator/honk-verifier/generate.ts)), not generator-unrolled per circuit |
| [`transcript.rs`](../../../generator/honk-verifier/static/src/honk/transcript.rs) | `TranscriptLib.generateTranscript()` |
| [`proof.rs`](../../../generator/honk-verifier/static/src/honk/proof.rs) | `TranscriptLib.loadProof()` |
| [`relations.rs`](../../../generator/honk-verifier/static/src/honk/relations.rs) | `RelationsLib.accumulateRelationEvaluations()` - 26 sub-relations (Arithmetic 2, Permutation 2, Lookup 2, DeltaRange 4, Elliptic 2, Auxiliary 6, Poseidon2External 4, Poseidon2Internal 4) |
| [`shplemini.rs`](../../../generator/honk-verifier/static/src/honk/shplemini.rs) | `CommitmentSchemeLib.verifyShplemini()` |
| [`fr_utils.rs`](../../../generator/honk-verifier/static/src/honk/fr_utils.rs) | `convertProofPoint()` |
| [`fr.rs`](../../../generator/honk-verifier/static/src/honk/fr.rs) | N/A - custom, not a translation (no audited `no_std` BN254 Fr library exists for PolkaVM); inversion via the EVM `modexp` precompile (`0x05`, Fermat's little theorem) |
| [`g1.rs`](../../../generator/honk-verifier/static/src/honk/g1.rs) | N/A - delegates to EVM precompiles (EIP-196/197): `ecAdd` `0x06`, `ecMul` `0x07`, `ecPairing` `0x08` |
