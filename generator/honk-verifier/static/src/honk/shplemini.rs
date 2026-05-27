/// Shplemini: KZG batch opening verification for UltraHonk.
///
/// Translated from HonkVerifier.sol `batchedShpleminiVerification()`.
/// Source: Aztec/Barretenberg `bb write_solidity_verifier` output.
///
/// Combines Gemini fold evaluations + Shplonk batching + KZG pairing check.
/// The MSM (multi-scalar multiplication) is done via sequential ecMul/ecAdd
/// precompile calls rather than a single MSM precompile.
/// Final pairing check uses ecPairing (EIP-197).
extern crate alloc;
use super::fr::{Fr, FR_TWO};
use super::fr_utils::{fq_from_split, fr_to_scalar};
use super::g1::{ec_add, ec_mul, ec_pairing_check, negate, G1Point};
use super::proof::{Proof, CONST_PROOF_SIZE_LOG_N, NUMBER_OF_ENTITIES};
use super::transcript::Transcript;
use crate::vk::{g2_generator, g2_kzg_srs, VerificationKey};

pub const NUMBER_UNSHIFTED: usize = 35;

fn convert_proof_point(pp: &super::proof::G1ProofPoint) -> G1Point {
    let x = fq_from_split(&pp.x_0, &pp.x_1);
    let y = fq_from_split(&pp.y_0, &pp.y_1);
    G1Point { x, y }
}

/// Compute powers of r: [r, r^2, r^4, ..., r^{2^{N-1}}] into a heap vec.
fn compute_squares(r: Fr) -> alloc::vec::Vec<Fr> {
    let mut squares = alloc::vec![Fr::zero(); CONST_PROOF_SIZE_LOG_N];
    squares[0] = r;
    for i in 1..CONST_PROOF_SIZE_LOG_N {
        squares[i] = squares[i - 1] * squares[i - 1];
    }
    squares
}

/// Reconstruct A_l(r^{2^l}) evaluations for l = 0..logN-1. Returns a heap vec.
fn compute_fold_pos_evaluations(
    sumcheck_u_challenges: &[Fr],
    batched_eval_accumulator: Fr,
    gemini_evaluations: &[Fr; CONST_PROOF_SIZE_LOG_N],
    gemini_eval_challenge_powers: &[Fr],
    log_size: usize,
) -> alloc::vec::Vec<Fr> {
    let mut fold_pos_evaluations = alloc::vec![Fr::zero(); CONST_PROOF_SIZE_LOG_N];
    let mut acc = batched_eval_accumulator;

    for i in (1..=CONST_PROOF_SIZE_LOG_N).rev() {
        let challenge_power = gemini_eval_challenge_powers[i - 1];
        let u = sumcheck_u_challenges[i - 1];

        let numerator = challenge_power * acc * FR_TWO
            - gemini_evaluations[i - 1] * (challenge_power * (Fr::one() - u) - u);
        let denominator = challenge_power * (Fr::one() - u) + u;

        let new_acc = numerator * denominator.inverse().unwrap();

        if i <= log_size {
            acc = new_acc;
            fold_pos_evaluations[i - 1] = new_acc;
        }
    }

    fold_pos_evaluations
}

/// Verify Shplemini (KZG batch opening). Returns true if pairing check passes.
pub fn verify_shplemini(proof: &Proof, vk: &VerificationKey, t: &Transcript) -> bool {
    let log_n = vk.log_circuit_size as usize;

    let powers_of_r = compute_squares(t.gemini_r);

    const TOTAL: usize = NUMBER_OF_ENTITIES + CONST_PROOF_SIZE_LOG_N + 2;

    // Heap-allocate to avoid ~6.7KB stack overflow (2240 + 4480 bytes).
    let mut scalars: alloc::vec::Vec<Fr> = alloc::vec![Fr::zero(); TOTAL];
    let mut commitments: alloc::vec::Vec<G1Point> = alloc::vec![G1Point::infinity(); TOTAL];

    let pos_inv_denom = (t.shplonk_z - powers_of_r[0]).inverse().unwrap();
    let neg_inv_denom = (t.shplonk_z + powers_of_r[0]).inverse().unwrap();

    let unshifted_scalar = pos_inv_denom + t.shplonk_nu * neg_inv_denom;
    let shifted_scalar =
        t.gemini_r.inverse().unwrap() * (pos_inv_denom - t.shplonk_nu * neg_inv_denom);

    scalars[0] = Fr::one();
    commitments[0] = convert_proof_point(&proof.shplonk_q);

    let mut batching_challenge = Fr::one();
    let mut batched_evaluation = Fr::zero();

    for i in 1..=NUMBER_UNSHIFTED {
        scalars[i] = -(unshifted_scalar * batching_challenge);
        batched_evaluation =
            batched_evaluation + proof.sumcheck_evaluations[i - 1] * batching_challenge;
        batching_challenge = batching_challenge * t.rho;
    }

    for i in NUMBER_UNSHIFTED + 1..=NUMBER_OF_ENTITIES {
        scalars[i] = -(shifted_scalar * batching_challenge);
        batched_evaluation =
            batched_evaluation + proof.sumcheck_evaluations[i - 1] * batching_challenge;
        batching_challenge = batching_challenge * t.rho;
    }

    commitments[1] = vk.qm;
    commitments[2] = vk.qc;
    commitments[3] = vk.ql;
    commitments[4] = vk.qr;
    commitments[5] = vk.qo;
    commitments[6] = vk.q4;
    commitments[7] = vk.q_lookup;
    commitments[8] = vk.q_arith;
    commitments[9] = vk.q_delta_range;
    commitments[10] = vk.q_elliptic;
    commitments[11] = vk.q_aux;
    commitments[12] = vk.q_poseidon2_external;
    commitments[13] = vk.q_poseidon2_internal;
    commitments[14] = vk.s1;
    commitments[15] = vk.s2;
    commitments[16] = vk.s3;
    commitments[17] = vk.s4;
    commitments[18] = vk.id1;
    commitments[19] = vk.id2;
    commitments[20] = vk.id3;
    commitments[21] = vk.id4;
    commitments[22] = vk.t1;
    commitments[23] = vk.t2;
    commitments[24] = vk.t3;
    commitments[25] = vk.t4;
    commitments[26] = vk.lagrange_first;
    commitments[27] = vk.lagrange_last;

    commitments[28] = convert_proof_point(&proof.w1);
    commitments[29] = convert_proof_point(&proof.w2);
    commitments[30] = convert_proof_point(&proof.w3);
    commitments[31] = convert_proof_point(&proof.w4);
    commitments[32] = convert_proof_point(&proof.z_perm);
    commitments[33] = convert_proof_point(&proof.lookup_inverses);
    commitments[34] = convert_proof_point(&proof.lookup_read_counts);
    commitments[35] = convert_proof_point(&proof.lookup_read_tags);

    commitments[36] = convert_proof_point(&proof.w1);
    commitments[37] = convert_proof_point(&proof.w2);
    commitments[38] = convert_proof_point(&proof.w3);
    commitments[39] = convert_proof_point(&proof.w4);
    commitments[40] = convert_proof_point(&proof.z_perm);

    let fold_pos_evaluations = compute_fold_pos_evaluations(
        &t.sumcheck_u_challenges,
        batched_evaluation,
        &proof.gemini_a_evaluations,
        &powers_of_r,
        log_n,
    );

    let mut constant_term_accumulator = fold_pos_evaluations[0] * pos_inv_denom
        + proof.gemini_a_evaluations[0] * t.shplonk_nu * neg_inv_denom;

    let mut batching_challenge = t.shplonk_nu * t.shplonk_nu;

    for i in 0..CONST_PROOF_SIZE_LOG_N - 1 {
        let dummy_round = i >= log_n - 1;
        let idx = NUMBER_OF_ENTITIES + 1 + i;

        if !dummy_round {
            let pos_inv = (t.shplonk_z - powers_of_r[i + 1]).inverse().unwrap();
            let neg_inv = (t.shplonk_z + powers_of_r[i + 1]).inverse().unwrap();

            let scaling_factor_pos = batching_challenge * pos_inv;
            let scaling_factor_neg = batching_challenge * t.shplonk_nu * neg_inv;

            scalars[idx] = -(scaling_factor_neg) + (-(scaling_factor_pos));

            let accum_contrib = scaling_factor_neg * proof.gemini_a_evaluations[i + 1]
                + scaling_factor_pos * fold_pos_evaluations[i + 1];
            constant_term_accumulator = constant_term_accumulator + accum_contrib;

            batching_challenge = batching_challenge * t.shplonk_nu * t.shplonk_nu;
        }

        commitments[idx] = convert_proof_point(&proof.gemini_fold_comms[i]);
    }

    // G1(1, 2) — standard BN254 G1 generator
    let mut g1_one_two_x = [0u8; 32];
    let mut g1_one_two_y = [0u8; 32];
    g1_one_two_x[31] = 1;
    g1_one_two_y[31] = 2;
    let g1_one_two = G1Point {
        x: g1_one_two_x,
        y: g1_one_two_y,
    };

    let g1_idx = NUMBER_OF_ENTITIES + CONST_PROOF_SIZE_LOG_N;
    commitments[g1_idx] = g1_one_two;
    scalars[g1_idx] = constant_term_accumulator;

    let quotient_commitment = convert_proof_point(&proof.kzg_quotient);
    let kzg_idx = NUMBER_OF_ENTITIES + CONST_PROOF_SIZE_LOG_N + 1;
    commitments[kzg_idx] = quotient_commitment;
    scalars[kzg_idx] = t.shplonk_z;

    // P0 = batchMul(commitments, scalars)
    let p0 = batch_mul_precompile(&commitments, &scalars);
    // P1 = negate(kzgQuotient)
    let p1 = negate(quotient_commitment);

    let g2_gen = g2_generator();
    let g2_vk = g2_kzg_srs();

    // Pairing check: e(P0, G2_gen) * e(P1, G2_vk) = 1
    ec_pairing_check(p0, &g2_gen, p1, &g2_vk)
}

/// MSM via sequential ecMul + ecAdd precompile calls
fn batch_mul_precompile(points: &[G1Point], scalars: &[Fr]) -> G1Point {
    let mut acc = G1Point::infinity();
    for (p, s) in points.iter().zip(scalars.iter()) {
        if s.is_zero() {
            continue;
        }
        let scalar_bytes = fr_to_scalar(*s);
        let term = ec_mul(*p, &scalar_bytes);
        acc = ec_add(acc, term);
    }
    acc
}
