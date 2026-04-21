use crate::fr::Fr;
use crate::proof::{Proof, BATCHED_RELATION_PARTIAL_LENGTH};
use crate::relations::{accumulate_relation_evaluations, accumulate_relation_evaluations_raw, NUMBER_OF_SUBRELATIONS};
use crate::transcript::Transcript;

/// Barycentric Lagrange denominators (from Solidity BARYCENTRIC_LAGRANGE_DENOMINATORS)
fn barycentric_lagrange_denominators() -> [Fr; BATCHED_RELATION_PARTIAL_LENGTH] {
    [
        fr_from_hex("30644e72e131a029b85045b68181585d2833e84879b9709143e1f593efffec51"),
        fr_from_hex("00000000000000000000000000000000000000000000000000000000000002d0"),
        fr_from_hex("30644e72e131a029b85045b68181585d2833e84879b9709143e1f593efffff11"),
        fr_from_hex("0000000000000000000000000000000000000000000000000000000000000090"),
        fr_from_hex("30644e72e131a029b85045b68181585d2833e84879b9709143e1f593efffff71"),
        fr_from_hex("00000000000000000000000000000000000000000000000000000000000000f0"),
        fr_from_hex("30644e72e131a029b85045b68181585d2833e84879b9709143e1f593effffd31"),
        fr_from_hex("00000000000000000000000000000000000000000000000000000000000013b0"),
    ]
}

fn fr_from_hex(hex: &str) -> Fr {
    let hex = if hex.len() < 64 {
        // left-pad with zeros
        let pad = 64 - hex.len();
        let mut s = [b'0'; 64];
        let src = hex.as_bytes();
        s[pad..].copy_from_slice(src);
        // Safety: we filled it with ASCII digits
        let mut bytes = [0u8; 32];
        for (byte, pair) in bytes.iter_mut().zip(s.chunks(2)) {
            *byte = nibble(pair[0]) << 4 | nibble(pair[1]);
        }
        return Fr::from_be_bytes(&bytes);
    } else {
        hex
    };
    let b = hex.as_bytes();
    let mut bytes = [0u8; 32];
    for (byte, pair) in bytes.iter_mut().zip(b.chunks(2)) {
        *byte = nibble(pair[0]) << 4 | nibble(pair[1]);
    }
    Fr::from_be_bytes(&bytes)
}

fn nibble(b: u8) -> u8 {
    match b {
        b'0'..=b'9' => b - b'0',
        b'a'..=b'f' => b - b'a' + 10,
        b'A'..=b'F' => b - b'A' + 10,
        _ => 0,
    }
}

fn check_sum(round_univariate: &[Fr; BATCHED_RELATION_PARTIAL_LENGTH], round_target: Fr) -> bool {
    round_univariate[0] + round_univariate[1] == round_target
}

fn compute_next_target_sum(
    u: &[Fr; BATCHED_RELATION_PARTIAL_LENGTH],
    challenge: Fr,
) -> Fr {
    let d = barycentric_lagrange_denominators();
    let c0 = challenge;
    let c1 = challenge - Fr::from_u64(1);
    let c2 = challenge - Fr::from_u64(2);
    let c3 = challenge - Fr::from_u64(3);
    let c4 = challenge - Fr::from_u64(4);
    let c5 = challenge - Fr::from_u64(5);
    let c6 = challenge - Fr::from_u64(6);
    let c7 = challenge - Fr::from_u64(7);

    let numerator = c0 * c1 * c2 * c3 * c4 * c5 * c6 * c7;

    let i0 = (d[0] * c0).inverse().unwrap_or(Fr::zero());
    let i1 = (d[1] * c1).inverse().unwrap_or(Fr::zero());
    let i2 = (d[2] * c2).inverse().unwrap_or(Fr::zero());
    let i3 = (d[3] * c3).inverse().unwrap_or(Fr::zero());
    let i4 = (d[4] * c4).inverse().unwrap_or(Fr::zero());
    let i5 = (d[5] * c5).inverse().unwrap_or(Fr::zero());
    let i6 = (d[6] * c6).inverse().unwrap_or(Fr::zero());
    let i7 = (d[7] * c7).inverse().unwrap_or(Fr::zero());

    let target = u[0] * i0
        + u[1] * i1
        + u[2] * i2
        + u[3] * i3
        + u[4] * i4
        + u[5] * i5
        + u[6] * i6
        + u[7] * i7;
    target * numerator
}

fn partially_evaluate_pow(gate_challenge: Fr, current_evaluation: Fr, round_challenge: Fr) -> Fr {
    let univariate_eval = Fr::one() + round_challenge * (gate_challenge - Fr::one());
    current_evaluation * univariate_eval
}

/// Returns: 0 = success, 100+round = check_sum failed at that round, 200 = final compare failed.
pub fn get_denom0_pub() -> Fr {
    barycentric_lagrange_denominators()[0]
}

pub fn compute_round0_target_pub(
    round_univariates: &[Fr; BATCHED_RELATION_PARTIAL_LENGTH],
    challenge: Fr,
) -> Fr {
    compute_next_target_sum(round_univariates, challenge)
}

pub fn verify_sumcheck_diag(proof: &Proof, t: &Transcript, _log_n: usize) -> u8 {
    let mut round_target = Fr::zero();
    let mut pow_partial_evaluation = Fr::one();

    // Round 0
    {
        let u = &proof.sumcheck_univariates[0];
        if !check_sum(u, round_target) { return 100; }
        let ch = t.sumcheck_u_challenges[0];
        round_target = compute_next_target_sum(u, ch);
        pow_partial_evaluation = partially_evaluate_pow(t.gate_challenges[0], pow_partial_evaluation, ch);
    }
    // Round 1
    {
        let u = &proof.sumcheck_univariates[1];
        if !check_sum(u, round_target) { return 101; }
        let ch = t.sumcheck_u_challenges[1];
        round_target = compute_next_target_sum(u, ch);
        pow_partial_evaluation = partially_evaluate_pow(t.gate_challenges[1], pow_partial_evaluation, ch);
    }
    // Round 2
    {
        let u = &proof.sumcheck_univariates[2];
        if !check_sum(u, round_target) { return 102; }
        let ch = t.sumcheck_u_challenges[2];
        round_target = compute_next_target_sum(u, ch);
        pow_partial_evaluation = partially_evaluate_pow(t.gate_challenges[2], pow_partial_evaluation, ch);
    }
    // Round 3
    {
        let u = &proof.sumcheck_univariates[3];
        if !check_sum(u, round_target) { return 103; }
        let ch = t.sumcheck_u_challenges[3];
        round_target = compute_next_target_sum(u, ch);
        pow_partial_evaluation = partially_evaluate_pow(t.gate_challenges[3], pow_partial_evaluation, ch);
    }
    // Round 4
    {
        let u = &proof.sumcheck_univariates[4];
        if !check_sum(u, round_target) { return 104; }
        let ch = t.sumcheck_u_challenges[4];
        round_target = compute_next_target_sum(u, ch);
        pow_partial_evaluation = partially_evaluate_pow(t.gate_challenges[4], pow_partial_evaluation, ch);
    }

    let grand_honk_relation_sum = accumulate_relation_evaluations(
        &proof.sumcheck_evaluations,
        &t.relation_parameters,
        &t.alphas,
        pow_partial_evaluation,
    );

    if grand_honk_relation_sum != round_target { 200 } else { 0 }
}

/// Returns (grand_honk_relation_sum, round_target) as two Fr values for debugging.
pub fn get_grand_sum_debug(proof: &Proof, t: &Transcript, _log_n: usize) -> (Fr, Fr) {
    let mut round_target = Fr::zero();
    let mut pow_partial_evaluation = Fr::one();

    // Round 0
    {
        let u = &proof.sumcheck_univariates[0];
        let ch = t.sumcheck_u_challenges[0];
        round_target = compute_next_target_sum(u, ch);
        pow_partial_evaluation = partially_evaluate_pow(t.gate_challenges[0], pow_partial_evaluation, ch);
    }
    // Round 1
    {
        let u = &proof.sumcheck_univariates[1];
        let ch = t.sumcheck_u_challenges[1];
        round_target = compute_next_target_sum(u, ch);
        pow_partial_evaluation = partially_evaluate_pow(t.gate_challenges[1], pow_partial_evaluation, ch);
    }
    // Round 2
    {
        let u = &proof.sumcheck_univariates[2];
        let ch = t.sumcheck_u_challenges[2];
        round_target = compute_next_target_sum(u, ch);
        pow_partial_evaluation = partially_evaluate_pow(t.gate_challenges[2], pow_partial_evaluation, ch);
    }
    // Round 3
    {
        let u = &proof.sumcheck_univariates[3];
        let ch = t.sumcheck_u_challenges[3];
        round_target = compute_next_target_sum(u, ch);
        pow_partial_evaluation = partially_evaluate_pow(t.gate_challenges[3], pow_partial_evaluation, ch);
    }
    // Round 4
    {
        let u = &proof.sumcheck_univariates[4];
        let ch = t.sumcheck_u_challenges[4];
        round_target = compute_next_target_sum(u, ch);
        pow_partial_evaluation = partially_evaluate_pow(t.gate_challenges[4], pow_partial_evaluation, ch);
    }

    let grand_sum = accumulate_relation_evaluations(
        &proof.sumcheck_evaluations,
        &t.relation_parameters,
        &t.alphas,
        pow_partial_evaluation,
    );

    (grand_sum, round_target)
}

/// Returns (pow_partial_evaluation, gate_challenges[0..5], sumcheck_u_challenges[0..5], raw 26 sub-relation evaluations).
pub fn get_relation_evals_debug(proof: &Proof, t: &Transcript) -> (Fr, [Fr; 5], [Fr; 5], [Fr; NUMBER_OF_SUBRELATIONS]) {
    let mut pow_partial_evaluation = Fr::one();

    // Round 0
    {
        let ch = t.sumcheck_u_challenges[0];
        pow_partial_evaluation = partially_evaluate_pow(t.gate_challenges[0], pow_partial_evaluation, ch);
    }
    // Round 1
    {
        let ch = t.sumcheck_u_challenges[1];
        pow_partial_evaluation = partially_evaluate_pow(t.gate_challenges[1], pow_partial_evaluation, ch);
    }
    // Round 2
    {
        let ch = t.sumcheck_u_challenges[2];
        pow_partial_evaluation = partially_evaluate_pow(t.gate_challenges[2], pow_partial_evaluation, ch);
    }
    // Round 3
    {
        let ch = t.sumcheck_u_challenges[3];
        pow_partial_evaluation = partially_evaluate_pow(t.gate_challenges[3], pow_partial_evaluation, ch);
    }
    // Round 4
    {
        let ch = t.sumcheck_u_challenges[4];
        pow_partial_evaluation = partially_evaluate_pow(t.gate_challenges[4], pow_partial_evaluation, ch);
    }

    let gate_chs = [
        t.gate_challenges[0],
        t.gate_challenges[1],
        t.gate_challenges[2],
        t.gate_challenges[3],
        t.gate_challenges[4],
    ];
    let sumcheck_chs = [
        t.sumcheck_u_challenges[0],
        t.sumcheck_u_challenges[1],
        t.sumcheck_u_challenges[2],
        t.sumcheck_u_challenges[3],
        t.sumcheck_u_challenges[4],
    ];
    let evals = accumulate_relation_evaluations_raw(
        &proof.sumcheck_evaluations,
        &t.relation_parameters,
        pow_partial_evaluation,
    );
    (pow_partial_evaluation, gate_chs, sumcheck_chs, evals)
}

pub fn verify_sumcheck(proof: &Proof, t: &Transcript, log_n: usize) -> bool {
    verify_sumcheck_diag(proof, t, log_n) == 0
}
