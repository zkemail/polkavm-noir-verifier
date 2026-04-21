extern crate alloc;
use alloc::vec::Vec;
use alloc::boxed::Box;
use alloc::alloc::{alloc_zeroed, Layout};

use super::fr::Fr;
use super::fr_utils::{fr_to_scalar, keccak256, split_challenge};
use super::proof::{
    Proof, CONST_PROOF_SIZE_LOG_N, NUMBER_OF_ENTITIES,
};

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

fn hash_u256s(values: &[[u8; 32]]) -> Fr {
    let mut buf = Vec::with_capacity(values.len() * 32);
    for v in values {
        buf.extend_from_slice(v);
    }
    let h = keccak256(&buf);
    Fr::from_be_bytes(&h)
}

fn hash_single(challenge: Fr) -> Fr {
    let bytes = fr_to_scalar(challenge);
    let h = keccak256(&bytes);
    Fr::from_be_bytes(&h)
}

fn u64_to_be32(v: u64) -> [u8; 32] {
    let mut buf = [0u8; 32];
    buf[24..32].copy_from_slice(&v.to_be_bytes());
    buf
}

/// Generate the full Fiat-Shamir transcript (matches TranscriptLib.generateTranscript).
/// Returns a heap-boxed Transcript to avoid ~2.9KB stack allocation in the caller.
pub fn generate_transcript(
    proof: &Proof,
    public_inputs: &[[u8; 32]],
    circuit_size: u64,
    public_inputs_size: u64,
    pub_inputs_offset: u64,
) -> Box<Transcript> {
    // --- Eta challenge ---
    let mut round0: Vec<[u8; 32]> = Vec::new();
    round0.push(u64_to_be32(circuit_size));
    round0.push(u64_to_be32(public_inputs_size));
    round0.push(u64_to_be32(pub_inputs_offset));
    for pi in public_inputs {
        round0.push(*pi);
    }
    round0.push(proof.w1.x_0);
    round0.push(proof.w1.x_1);
    round0.push(proof.w1.y_0);
    round0.push(proof.w1.y_1);
    round0.push(proof.w2.x_0);
    round0.push(proof.w2.x_1);
    round0.push(proof.w2.y_0);
    round0.push(proof.w2.y_1);
    round0.push(proof.w3.x_0);
    round0.push(proof.w3.x_1);
    round0.push(proof.w3.y_0);
    round0.push(proof.w3.y_1);

    let prev = hash_u256s(&round0);
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
    // Use iter_mut() to avoid integer index variable (compiler bug workaround).
    let mut gate_challenges = [Fr::zero(); CONST_PROOF_SIZE_LOG_N];
    for gc_dst in gate_challenges.iter_mut() {
        prev = hash_single(prev);
        let (gc, _) = split_challenge(prev);
        *gc_dst = gc;
    }

    // --- Sumcheck U challenges ---
    let mut sumcheck_u_challenges = [Fr::zero(); CONST_PROOF_SIZE_LOG_N];
    for (sc_dst, univariates) in sumcheck_u_challenges.iter_mut().zip(proof.sumcheck_univariates.iter()) {
        let mut uc: Vec<u8> = Vec::with_capacity(9 * 32);
        uc.extend_from_slice(&fr_to_scalar(prev));
        for elem in univariates.iter() {
            uc.extend_from_slice(&fr_to_scalar(*elem));
        }
        let h = keccak256(&uc);
        prev = Fr::from_be_bytes(&h);
        let (sc, _) = split_challenge(prev);
        *sc_dst = sc;
    }

    // --- Rho challenge ---
    let mut rho_elems: Vec<[u8; 32]> = Vec::with_capacity(NUMBER_OF_ENTITIES + 1);
    rho_elems.push(fr_to_scalar(prev));
    for eval in proof.sumcheck_evaluations.iter() {
        rho_elems.push(fr_to_scalar(*eval));
    }
    prev = hash_u256s(&rho_elems);
    let (rho, _) = split_challenge(prev);

    // --- Gemini R challenge ---
    let mut gr: Vec<[u8; 32]> = Vec::with_capacity((CONST_PROOF_SIZE_LOG_N - 1) * 4 + 1);
    gr.push(fr_to_scalar(prev));
    for comm in proof.gemini_fold_comms.iter() {
        gr.push(comm.x_0);
        gr.push(comm.x_1);
        gr.push(comm.y_0);
        gr.push(comm.y_1);
    }
    prev = hash_u256s(&gr);
    let (gemini_r, _) = split_challenge(prev);

    // --- Shplonk Nu challenge ---
    let mut nu_elems: Vec<[u8; 32]> = Vec::with_capacity(CONST_PROOF_SIZE_LOG_N + 1);
    nu_elems.push(fr_to_scalar(prev));
    for eval in proof.gemini_a_evaluations.iter() {
        nu_elems.push(fr_to_scalar(*eval));
    }
    prev = hash_u256s(&nu_elems);
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
