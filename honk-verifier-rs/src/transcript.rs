use ark_bn254::Fr;
use ark_ff::{Field, PrimeField};

use crate::fr_utils::{fr_from_be_bytes, fr_to_be_bytes, keccak256, split_challenge};
use crate::proof::{Proof, BATCHED_RELATION_PARTIAL_LENGTH, CONST_PROOF_SIZE_LOG_N, NUMBER_OF_ENTITIES};

pub const NUMBER_OF_ALPHAS: usize = 25;

#[derive(Default)]
pub struct RelationParameters {
    pub eta: Fr,
    pub eta_two: Fr,
    pub eta_three: Fr,
    pub beta: Fr,
    pub gamma: Fr,
    pub public_inputs_delta: Fr,
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

fn hash_fields(fields: &[Fr]) -> Fr {
    let mut buf = Vec::with_capacity(fields.len() * 32);
    for f in fields {
        buf.extend_from_slice(&fr_to_be_bytes(*f));
    }
    let h = keccak256(&buf);
    fr_from_be_bytes(&h)
}

fn hash_u256s(values: &[[u8; 32]]) -> Fr {
    let mut buf = Vec::with_capacity(values.len() * 32);
    for v in values {
        buf.extend_from_slice(v);
    }
    let h = keccak256(&buf);
    fr_from_be_bytes(&h)
}

fn hash_single(challenge: Fr) -> Fr {
    let bytes = fr_to_be_bytes(challenge);
    let h = keccak256(&bytes);
    fr_from_be_bytes(&h)
}

/// Generate the full Fiat-Shamir transcript (matches TranscriptLib.generateTranscript).
pub fn generate_transcript(
    proof: &Proof,
    public_inputs: &[[u8; 32]],
    circuit_size: u64,
    public_inputs_size: u64,
    pub_inputs_offset: u64,
) -> Transcript {
    // --- Eta challenge ---
    // round0 = [circuitSize, publicInputsSize, pubInputsOffset, pub_inputs..., w1.x_0, w1.x_1, w1.y_0, w1.y_1, w2..., w3...]
    let mut round0 = Vec::new();
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
    let mut prev = prev2; // track "previousChallenge" after eta

    // --- Beta/Gamma challenge ---
    let round1: [[u8; 32]; 13] = [
        fr_to_be_bytes(prev),
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
        public_inputs_delta: Fr::from(0u64), // computed later
    };

    // --- Alpha challenges ---
    let alpha0_raw: [u64; 9] = [
        0, 0, 0, 0, 0, 0, 0, 0, 0, // placeholders, built below
    ];
    let _ = alpha0_raw;
    let alpha0: [[u8; 32]; 9] = [
        fr_to_be_bytes(prev),
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
    let mut alphas = [Fr::from(0u64); NUMBER_OF_ALPHAS];
    let (a0, a1) = split_challenge(prev);
    alphas[0] = a0;
    alphas[1] = a1;

    for i in 1..NUMBER_OF_ALPHAS / 2 {
        prev = hash_single(prev);
        let (a_even, a_odd) = split_challenge(prev);
        alphas[2 * i] = a_even;
        alphas[2 * i + 1] = a_odd;
    }
    // NUMBER_OF_ALPHAS = 25 (odd), so one more alpha needed
    if (NUMBER_OF_ALPHAS & 1) == 1 && NUMBER_OF_ALPHAS > 2 {
        prev = hash_single(prev);
        let (last, _) = split_challenge(prev);
        alphas[NUMBER_OF_ALPHAS - 1] = last;
    }

    // --- Gate challenges ---
    let mut gate_challenges = [Fr::from(0u64); CONST_PROOF_SIZE_LOG_N];
    for i in 0..CONST_PROOF_SIZE_LOG_N {
        prev = hash_single(prev);
        let (gc, _) = split_challenge(prev);
        gate_challenges[i] = gc;
    }

    // --- Sumcheck U challenges ---
    let mut sumcheck_u_challenges = [Fr::from(0u64); CONST_PROOF_SIZE_LOG_N];
    for i in 0..CONST_PROOF_SIZE_LOG_N {
        // univariateChal = [prevChallenge, sumcheckUnivariates[i][0..7]]
        let mut uc = Vec::with_capacity(9 * 32);
        uc.extend_from_slice(&fr_to_be_bytes(prev));
        for j in 0..BATCHED_RELATION_PARTIAL_LENGTH {
            uc.extend_from_slice(&fr_to_be_bytes(proof.sumcheck_univariates[i][j]));
        }
        let h = keccak256(&uc);
        prev = fr_from_be_bytes(&h);
        let (sc, _) = split_challenge(prev);
        sumcheck_u_challenges[i] = sc;
    }

    // --- Rho challenge ---
    let mut rho_elems: Vec<[u8; 32]> = Vec::with_capacity(NUMBER_OF_ENTITIES + 1);
    rho_elems.push(fr_to_be_bytes(prev));
    for i in 0..NUMBER_OF_ENTITIES {
        rho_elems.push(fr_to_be_bytes(proof.sumcheck_evaluations[i]));
    }
    prev = hash_u256s(&rho_elems);
    let (rho, _) = split_challenge(prev);

    // --- Gemini R challenge ---
    let mut gr: Vec<[u8; 32]> = Vec::with_capacity((CONST_PROOF_SIZE_LOG_N - 1) * 4 + 1);
    gr.push(fr_to_be_bytes(prev));
    for i in 0..CONST_PROOF_SIZE_LOG_N - 1 {
        gr.push(proof.gemini_fold_comms[i].x_0);
        gr.push(proof.gemini_fold_comms[i].x_1);
        gr.push(proof.gemini_fold_comms[i].y_0);
        gr.push(proof.gemini_fold_comms[i].y_1);
    }
    prev = hash_u256s(&gr);
    let (gemini_r, _) = split_challenge(prev);

    // --- Shplonk Nu challenge ---
    let mut nu_elems: Vec<[u8; 32]> = Vec::with_capacity(CONST_PROOF_SIZE_LOG_N + 1);
    nu_elems.push(fr_to_be_bytes(prev));
    for i in 0..CONST_PROOF_SIZE_LOG_N {
        nu_elems.push(fr_to_be_bytes(proof.gemini_a_evaluations[i]));
    }
    prev = hash_u256s(&nu_elems);
    let (shplonk_nu, _) = split_challenge(prev);

    // --- Shplonk Z challenge ---
    let z_elems: [[u8; 32]; 5] = [
        fr_to_be_bytes(prev),
        proof.shplonk_q.x_0,
        proof.shplonk_q.x_1,
        proof.shplonk_q.y_0,
        proof.shplonk_q.y_1,
    ];
    prev = hash_u256s(&z_elems);
    let (shplonk_z, _) = split_challenge(prev);

    Transcript {
        relation_parameters,
        alphas,
        gate_challenges,
        sumcheck_u_challenges,
        rho,
        gemini_r,
        shplonk_nu,
        shplonk_z,
    }
}

fn u64_to_be32(v: u64) -> [u8; 32] {
    let mut buf = [0u8; 32];
    buf[24..32].copy_from_slice(&v.to_be_bytes());
    buf
}
