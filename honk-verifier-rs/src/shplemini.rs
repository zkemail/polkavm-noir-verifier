use ark_bn254::{Fq, Fr, G1Affine, G1Projective};
use ark_bn254::Bn254;
use ark_ec::{pairing::Pairing, AffineRepr, CurveGroup};
use ark_ff::{Field, One, PrimeField, Zero};

use crate::fr_utils::fq_from_split;
use crate::proof::{Proof, CONST_PROOF_SIZE_LOG_N, NUMBER_OF_ENTITIES};
use crate::transcript::Transcript;
use crate::vk::{g2_generator, g2_kzg_srs, VerificationKey};

pub const NUMBER_UNSHIFTED: usize = 35;

fn convert_proof_point(pp: &crate::proof::G1ProofPoint) -> G1Affine {
    let x = fq_from_split(&pp.x_0, &pp.x_1);
    let y = fq_from_split(&pp.y_0, &pp.y_1);
    G1Affine::new_unchecked(x, y)
}

fn negate_g1(p: G1Affine) -> G1Affine {
    use std::ops::Neg;
    p.neg()
}

fn batch_mul(bases: &[G1Affine], scalars: &[Fr]) -> G1Affine {
    assert_eq!(bases.len(), scalars.len());
    let mut acc = G1Projective::zero();
    for (base, scalar) in bases.iter().zip(scalars.iter()) {
        if !scalar.is_zero() {
            let bi = scalar.into_bigint();
            acc += base.mul_bigint(bi);
        }
    }
    acc.into_affine()
}

/// Compute powers of r: [r, r^2, r^4, ..., r^{2^{N-1}}]
fn compute_squares(r: Fr) -> [Fr; CONST_PROOF_SIZE_LOG_N] {
    let mut squares = [Fr::zero(); CONST_PROOF_SIZE_LOG_N];
    squares[0] = r;
    for i in 1..CONST_PROOF_SIZE_LOG_N {
        squares[i] = squares[i - 1] * squares[i - 1];
    }
    squares
}

/// Reconstruct A_l(r^{2^l}) evaluations for l = 0..logN-1.
fn compute_fold_pos_evaluations(
    sumcheck_u_challenges: &[Fr; CONST_PROOF_SIZE_LOG_N],
    batched_eval_accumulator: Fr,
    gemini_evaluations: &[Fr; CONST_PROOF_SIZE_LOG_N],
    gemini_eval_challenge_powers: &[Fr; CONST_PROOF_SIZE_LOG_N],
    log_size: usize,
) -> [Fr; CONST_PROOF_SIZE_LOG_N] {
    let mut fold_pos_evaluations = [Fr::zero(); CONST_PROOF_SIZE_LOG_N];
    let mut acc = batched_eval_accumulator;

    // Loop from i = CONST_PROOF_SIZE_LOG_N down to 1
    for i in (1..=CONST_PROOF_SIZE_LOG_N).rev() {
        let challenge_power = gemini_eval_challenge_powers[i - 1];
        let u = sumcheck_u_challenges[i - 1];

        let numerator = challenge_power * acc * Fr::from(2u64)
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

    // Total commitment array size = NUMBER_OF_ENTITIES + CONST_PROOF_SIZE_LOG_N + 2 = 70
    const TOTAL: usize = NUMBER_OF_ENTITIES + CONST_PROOF_SIZE_LOG_N + 2;

    let mut scalars = [Fr::zero(); TOTAL];
    let mut commitments = [G1Affine::default(); TOTAL];

    // Denominators at powers_of_r[0]
    let pos_inv_denom = (t.shplonk_z - powers_of_r[0])
        .inverse()
        .expect("shplonk_z == r");
    let neg_inv_denom = (t.shplonk_z + powers_of_r[0])
        .inverse()
        .expect("shplonk_z == -r");

    let unshifted_scalar = pos_inv_denom + t.shplonk_nu * neg_inv_denom;
    let shifted_scalar = t.gemini_r.inverse().expect("gemini_r == 0")
        * (pos_inv_denom - t.shplonk_nu * neg_inv_denom);

    // commitments[0] = shplonkQ, scalar = 1
    scalars[0] = Fr::one();
    commitments[0] = convert_proof_point(&proof.shplonk_q);

    let mut batching_challenge = Fr::one();
    let mut batched_evaluation = Fr::zero();

    // Unshifted commitments [1..=NUMBER_UNSHIFTED]
    for i in 1..=NUMBER_UNSHIFTED {
        scalars[i] = -unshifted_scalar * batching_challenge;
        batched_evaluation += proof.sumcheck_evaluations[i - 1] * batching_challenge;
        batching_challenge *= t.rho;
    }

    // Shifted commitments [NUMBER_UNSHIFTED+1..=NUMBER_OF_ENTITIES]
    for i in NUMBER_UNSHIFTED + 1..=NUMBER_OF_ENTITIES {
        scalars[i] = -shifted_scalar * batching_challenge;
        batched_evaluation += proof.sumcheck_evaluations[i - 1] * batching_challenge;
        batching_challenge *= t.rho;
    }

    // VK commitments (indices 1..=27) match sumcheck eval indices 0..26
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

    // Proof point commitments (unshifted) [28..=35]
    commitments[28] = convert_proof_point(&proof.w1);
    commitments[29] = convert_proof_point(&proof.w2);
    commitments[30] = convert_proof_point(&proof.w3);
    commitments[31] = convert_proof_point(&proof.w4);
    commitments[32] = convert_proof_point(&proof.z_perm);
    commitments[33] = convert_proof_point(&proof.lookup_inverses);
    commitments[34] = convert_proof_point(&proof.lookup_read_counts);
    commitments[35] = convert_proof_point(&proof.lookup_read_tags);

    // Proof point commitments (shifted) [36..=40]
    commitments[36] = convert_proof_point(&proof.w1);
    commitments[37] = convert_proof_point(&proof.w2);
    commitments[38] = convert_proof_point(&proof.w3);
    commitments[39] = convert_proof_point(&proof.w4);
    commitments[40] = convert_proof_point(&proof.z_perm);

    // Fold evaluations A_l(r^{2^l})
    let fold_pos_evaluations = compute_fold_pos_evaluations(
        &t.sumcheck_u_challenges,
        batched_evaluation,
        &proof.gemini_a_evaluations,
        &powers_of_r,
        log_n,
    );

    // Initial constant term accumulator (from A_0(r) and A_0(-r))
    let mut constant_term_accumulator =
        fold_pos_evaluations[0] * pos_inv_denom
            + proof.gemini_a_evaluations[0] * t.shplonk_nu * neg_inv_denom;

    // batchingChallenge for gemini fold loop starts at nu^2
    let mut batching_challenge = t.shplonk_nu * t.shplonk_nu;

    // Gemini fold commitments [NUMBER_OF_ENTITIES+1 .. NUMBER_OF_ENTITIES+CONST_PROOF_SIZE_LOG_N]
    // = indices [41..68]
    let mut pos_inv = pos_inv_denom;
    let mut neg_inv = neg_inv_denom;

    for i in 0..CONST_PROOF_SIZE_LOG_N - 1 {
        let dummy_round = i >= log_n - 1;
        let idx = NUMBER_OF_ENTITIES + 1 + i;

        if !dummy_round {
            pos_inv = (t.shplonk_z - powers_of_r[i + 1])
                .inverse()
                .expect("shplonk_z == r^{2^{i+1}}");
            neg_inv = (t.shplonk_z + powers_of_r[i + 1])
                .inverse()
                .expect("shplonk_z == -r^{2^{i+1}}");

            let scaling_factor_pos = batching_challenge * pos_inv;
            let scaling_factor_neg = batching_challenge * t.shplonk_nu * neg_inv;

            scalars[idx] = -scaling_factor_neg + (-scaling_factor_pos);

            let accum_contrib = scaling_factor_neg * proof.gemini_a_evaluations[i + 1]
                + scaling_factor_pos * fold_pos_evaluations[i + 1];
            constant_term_accumulator += accum_contrib;

            batching_challenge *= t.shplonk_nu * t.shplonk_nu;
        }
        // scalar stays zero for dummy rounds

        commitments[idx] = convert_proof_point(&proof.gemini_fold_comms[i]);
    }

    // G1(1, 2) — the standard BN254 G1 generator
    let g1_one_two = G1Affine::new_unchecked(Fq::from(1u64), Fq::from(2u64));

    let g1_idx = NUMBER_OF_ENTITIES + CONST_PROOF_SIZE_LOG_N;
    commitments[g1_idx] = g1_one_two;
    scalars[g1_idx] = constant_term_accumulator;

    let quotient_commitment = convert_proof_point(&proof.kzg_quotient);
    let kzg_idx = NUMBER_OF_ENTITIES + CONST_PROOF_SIZE_LOG_N + 1;
    commitments[kzg_idx] = quotient_commitment;
    scalars[kzg_idx] = t.shplonk_z;

    // P_0 = batchMul(commitments, scalars)
    let p0 = batch_mul(&commitments, &scalars);
    // P_1 = negate(kzgQuotient)
    let p1 = negate_g1(quotient_commitment);

    // Pairing check: e(P_0, G2_gen) * e(P_1, G2_vk) = 1
    let g2_gen = g2_generator();
    let g2_vk = g2_kzg_srs();

    let result = Bn254::multi_pairing([p0, p1], [g2_gen, g2_vk]);
    result.is_zero()
}
