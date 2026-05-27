/// Generic UltraHonk verification modules.
///
/// These modules are circuit-independent — the same code works for any
/// UltraHonk circuit regardless of size or public input count.
///
/// Provenance: `proof`, `relations`, `shplemini`, and `transcript` are
/// translated from Aztec/Barretenberg's HonkVerifier.sol, which is the
/// machine-generated Solidity verifier produced by `bb write_solidity_verifier`.
/// The Solidity source is the canonical reference for correctness.
///
/// `fr` is custom BN254 scalar field arithmetic (no audited no_std library
/// exists for PolkaVM). `g1` delegates EC operations to EVM precompiles.
pub mod fr;
pub mod fr_utils;
pub mod g1;
pub mod proof;
pub mod relations;
pub mod shplemini;
pub mod transcript;
