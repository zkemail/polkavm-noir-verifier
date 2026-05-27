/// Fiat-Shamir transcript generation for UltraHonk.
///
/// Translated from HonkVerifier.sol `TranscriptLib.generateTranscript()`.
/// Source: Aztec/Barretenberg `bb write_solidity_verifier` output.
///
/// Produces deterministic challenges (eta, beta, gamma, alphas, gate challenges,
/// sumcheck challenges, rho, gemini_r, shplonk_nu, shplonk_z) from the proof
/// and public inputs via keccak256 hashing.
///
/// Uses the pallet-revive `hash_keccak_256` host function (native keccak
/// outside the VM) instead of the in-VM `tiny-keccak` implementation, so the
/// hashing work is not metered per-instruction. The streaming-style call sites
/// are replaced by a single contiguous buffer fed to the host fn in one shot.
extern crate alloc;
use alloc::alloc::{alloc_zeroed, Layout};
use alloc::boxed::Box;
use alloc::vec;
use alloc::vec::Vec;

use pallet_revive_uapi::{HostFn, HostFnImpl as api};

use super::fr::Fr;
use super::fr_utils::{fr_to_scalar, split_challenge};
use super::proof::{Proof, CONST_PROOF_SIZE_LOG_N};

pub const NUMBER_OF_ALPHAS: usize = 25;

pub struct RelationParameters {
    pub eta: Fr,
    pub eta_two: Fr,
    pub eta_three: Fr,
    pub beta: Fr,
    pub gamma: Fr,
    pub public_inputs_delta: Fr,
}

impl Default for RelationParameters {
    fn default() -> Self {
        RelationParameters {
            eta: Fr::zero(),
            eta_two: Fr::zero(),
            eta_three: Fr::zero(),
            beta: Fr::zero(),
            gamma: Fr::zero(),
            public_inputs_delta: Fr::zero(),
        }
    }
}

pub struct Transcript {
    pub relation_parameters: RelationParameters,
    pub alphas: [Fr; NUMBER_OF_ALPHAS],
    pub gate_challenges: [Fr; CONST_PROOF_SIZE_LOG_N],
    pub sumcheck_u_challenges: [Fr; CONST_PROOF_SIZE_LOG_N],
    pub rho: Fr,
    pub gemini_r: Fr,
    pub shplonk_nu: Fr,
    pub shplonk_z: Fr,
}

/// Hash a contiguous byte buffer via the host-fn keccak and convert to Fr.
#[inline(always)]
fn keccak_to_fr(input: &[u8]) -> Fr {
    let mut out = [0u8; 32];
    api::hash_keccak_256(input, &mut out);
    Fr::from_be_bytes(&out)
}

/// Hash multiple concatenated 32-byte values via keccak256.
fn hash_u256s(values: &[[u8; 32]]) -> Fr {
    // SAFETY: `[[u8; 32]]` has the same memory layout as a flat byte slice of
    // length `values.len() * 32`. Reinterpreting avoids an intermediate copy.
    let bytes: &[u8] =
        unsafe { core::slice::from_raw_parts(values.as_ptr() as *const u8, values.len() * 32) };
    keccak_to_fr(bytes)
}

/// Hash a single Fr challenge.
fn hash_single(challenge: Fr) -> Fr {
    let bytes = fr_to_scalar(challenge);
    keccak_to_fr(&bytes)
}

/// Generate the full Fiat-Shamir transcript (matches TranscriptLib.generateTranscript).
/// Returns a heap-boxed Transcript to avoid ~2.9KB stack allocation in the caller.
///
/// All hashing goes through the pallet-revive `hash_keccak_256` host function.
pub fn generate_transcript(
    proof: &Proof,
    public_inputs: &[[u8; 32]],
    circuit_size: u64,
    public_inputs_size: u64,
    pub_inputs_offset: u64,
) -> Box<Transcript> {
    // --- Eta challenge ---
    // Build a contiguous buffer: [circuit_size, pub_inputs_size, offset,
    //                            ...pub_inputs, w1.x0, w1.x1, ..., w3.y1]
    // Size: 3 + public_inputs.len() + 12 chunks of 32 bytes.
    let prev = {
        let n = 3 + public_inputs.len() + 12;
        let mut buf: Vec<u8> = vec![0u8; n * 32];
        // 3 u64 fields padded to 32 bytes BE
        buf[24..32].copy_from_slice(&circuit_size.to_be_bytes());
        buf[32 + 24..32 + 32].copy_from_slice(&public_inputs_size.to_be_bytes());
        buf[64 + 24..64 + 32].copy_from_slice(&pub_inputs_offset.to_be_bytes());
        let mut off = 96;
        for pi in public_inputs {
            buf[off..off + 32].copy_from_slice(pi);
            off += 32;
        }
        let wires: [&[u8; 32]; 12] = [
            &proof.w1.x_0,
            &proof.w1.x_1,
            &proof.w1.y_0,
            &proof.w1.y_1,
            &proof.w2.x_0,
            &proof.w2.x_1,
            &proof.w2.y_0,
            &proof.w2.y_1,
            &proof.w3.x_0,
            &proof.w3.x_1,
            &proof.w3.y_0,
            &proof.w3.y_1,
        ];
        for w in wires.iter() {
            buf[off..off + 32].copy_from_slice(*w);
            off += 32;
        }
        debug_assert_eq!(off, buf.len());
        keccak_to_fr(&buf)
    };
    let (eta, eta_two) = split_challenge(prev);
    let prev2 = hash_single(prev);
    let (eta_three, _) = split_challenge(prev2);
    let mut prev = prev2;

    // --- Beta/Gamma challenge ---
    let round1: [[u8; 32]; 13] = [
        fr_to_scalar(prev),
        proof.lookup_read_counts.x_0,
        proof.lookup_read_counts.x_1,
        proof.lookup_read_counts.y_0,
        proof.lookup_read_counts.y_1,
        proof.lookup_read_tags.x_0,
        proof.lookup_read_tags.x_1,
        proof.lookup_read_tags.y_0,
        proof.lookup_read_tags.y_1,
        proof.w4.x_0,
        proof.w4.x_1,
        proof.w4.y_0,
        proof.w4.y_1,
    ];
    prev = hash_u256s(&round1);
    let (beta, gamma) = split_challenge(prev);

    let relation_parameters = RelationParameters {
        eta,
        eta_two,
        eta_three,
        beta,
        gamma,
        public_inputs_delta: Fr::zero(), // computed later in main
    };

    // --- Alpha challenges ---
    let alpha0: [[u8; 32]; 9] = [
        fr_to_scalar(prev),
        proof.lookup_inverses.x_0,
        proof.lookup_inverses.x_1,
        proof.lookup_inverses.y_0,
        proof.lookup_inverses.y_1,
        proof.z_perm.x_0,
        proof.z_perm.x_1,
        proof.z_perm.y_0,
        proof.z_perm.y_1,
    ];
    prev = hash_u256s(&alpha0);
    let mut alphas = [Fr::zero(); NUMBER_OF_ALPHAS];
    let (a0, a1) = split_challenge(prev);
    alphas[0] = a0;
    alphas[1] = a1;

    for chunk in alphas[2..NUMBER_OF_ALPHAS - 1].chunks_mut(2) {
        prev = hash_single(prev);
        let (a_even, a_odd) = split_challenge(prev);
        chunk[0] = a_even;
        chunk[1] = a_odd;
    }
    // NUMBER_OF_ALPHAS = 25 (odd), one more alpha needed
    if (NUMBER_OF_ALPHAS & 1) == 1 && NUMBER_OF_ALPHAS > 2 {
        prev = hash_single(prev);
        let (last, _) = split_challenge(prev);
        alphas[NUMBER_OF_ALPHAS - 1] = last;
    }

    // --- Gate challenges ---
    let mut gate_challenges = [Fr::zero(); CONST_PROOF_SIZE_LOG_N];
    for gc_dst in gate_challenges.iter_mut() {
        prev = hash_single(prev);
        let (gc, _) = split_challenge(prev);
        *gc_dst = gc;
    }

    // --- Sumcheck U challenges ---
    // Stack buffer: prev + 8 univariate elements = 9 * 32 bytes per round.
    let mut sumcheck_u_challenges = [Fr::zero(); CONST_PROOF_SIZE_LOG_N];
    let mut uc_buf = [0u8; 9 * 32];
    for (sc_dst, univariates) in sumcheck_u_challenges
        .iter_mut()
        .zip(proof.sumcheck_univariates.iter())
    {
        uc_buf[0..32].copy_from_slice(&fr_to_scalar(prev));
        for (chunk, elem) in uc_buf[32..].chunks_mut(32).zip(univariates.iter()) {
            chunk.copy_from_slice(&fr_to_scalar(*elem));
        }
        prev = keccak_to_fr(&uc_buf);
        let (sc, _) = split_challenge(prev);
        *sc_dst = sc;
    }

    // --- Rho challenge ---
    // Stack buffer: [prev, eval0, eval1, ..., eval39] = 41 * 32 = 1312 bytes.
    {
        let mut buf = [0u8; 41 * 32];
        buf[0..32].copy_from_slice(&fr_to_scalar(prev));
        for (chunk, eval) in buf[32..]
            .chunks_mut(32)
            .zip(proof.sumcheck_evaluations.iter())
        {
            chunk.copy_from_slice(&fr_to_scalar(*eval));
        }
        prev = keccak_to_fr(&buf);
    }
    let (rho, _) = split_challenge(prev);

    // --- Gemini R challenge ---
    // Stack buffer: prev + (CONST_PROOF_SIZE_LOG_N - 1) commitments * 4 fields * 32 bytes
    // = (1 + 27*4) * 32 = 109 * 32 = 3488 bytes.
    {
        const N_COMMS: usize = CONST_PROOF_SIZE_LOG_N - 1;
        let mut buf = [0u8; (1 + N_COMMS * 4) * 32];
        buf[0..32].copy_from_slice(&fr_to_scalar(prev));
        let mut off = 32;
        for comm in proof.gemini_fold_comms.iter() {
            buf[off..off + 32].copy_from_slice(&comm.x_0);
            off += 32;
            buf[off..off + 32].copy_from_slice(&comm.x_1);
            off += 32;
            buf[off..off + 32].copy_from_slice(&comm.y_0);
            off += 32;
            buf[off..off + 32].copy_from_slice(&comm.y_1);
            off += 32;
        }
        debug_assert_eq!(off, buf.len());
        prev = keccak_to_fr(&buf);
    }
    let (gemini_r, _) = split_challenge(prev);

    // --- Shplonk Nu challenge ---
    // Stack buffer: prev + CONST_PROOF_SIZE_LOG_N a_evaluations
    // = (1 + 28) * 32 = 928 bytes.
    {
        let mut buf = [0u8; (1 + CONST_PROOF_SIZE_LOG_N) * 32];
        buf[0..32].copy_from_slice(&fr_to_scalar(prev));
        for (chunk, eval) in buf[32..]
            .chunks_mut(32)
            .zip(proof.gemini_a_evaluations.iter())
        {
            chunk.copy_from_slice(&fr_to_scalar(*eval));
        }
        prev = keccak_to_fr(&buf);
    }
    let (shplonk_nu, _) = split_challenge(prev);

    // --- Shplonk Z challenge ---
    let z_elems: [[u8; 32]; 5] = [
        fr_to_scalar(prev),
        proof.shplonk_q.x_0,
        proof.shplonk_q.x_1,
        proof.shplonk_q.y_0,
        proof.shplonk_q.y_1,
    ];
    prev = hash_u256s(&z_elems);
    let (shplonk_z, _) = split_challenge(prev);

    // Allocate the Transcript directly on the heap to avoid ~2.9KB stack allocation.
    let layout = Layout::new::<Transcript>();
    let ptr = unsafe { alloc_zeroed(layout) as *mut Transcript };
    let t = unsafe { &mut *ptr };
    t.relation_parameters = relation_parameters;
    t.alphas = alphas;
    t.gate_challenges = gate_challenges;
    t.sumcheck_u_challenges = sumcheck_u_challenges;
    t.rho = rho;
    t.gemini_r = gemini_r;
    t.shplonk_nu = shplonk_nu;
    t.shplonk_z = shplonk_z;
    unsafe { Box::from_raw(ptr) }
}
