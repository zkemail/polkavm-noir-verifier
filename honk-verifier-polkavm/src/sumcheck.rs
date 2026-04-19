use crate::fr::Fr;
use crate::proof::{Proof, BATCHED_RELATION_PARTIAL_LENGTH};
use crate::relations::accumulate_relation_evaluations;
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
        for i in 0..32 {
            bytes[i] = nibble(s[i * 2]) << 4 | nibble(s[i * 2 + 1]);
        }
        return Fr::from_be_bytes(&bytes);
    } else {
        hex
    };
    let b = hex.as_bytes();
    let mut bytes = [0u8; 32];
    for i in 0..32 {
        bytes[i] = nibble(b[i * 2]) << 4 | nibble(b[i * 2 + 1]);
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
    round_univariates: &[Fr; BATCHED_RELATION_PARTIAL_LENGTH],
    challenge: Fr,
) -> Fr {
    let denoms = barycentric_lagrange_denominators();

    let mut numerator_value = Fr::one();
    for i in 0..BATCHED_RELATION_PARTIAL_LENGTH {
        numerator_value = numerator_value * (challenge - Fr::from_u64(i as u64));
    }

    let mut denominator_inverses = [Fr::zero(); BATCHED_RELATION_PARTIAL_LENGTH];
    for i in 0..BATCHED_RELATION_PARTIAL_LENGTH {
        let inv = denoms[i] * (challenge - Fr::from_u64(i as u64));
        denominator_inverses[i] = inv.inverse().unwrap_or(Fr::zero());
    }

    let mut target_sum = Fr::zero();
    for i in 0..BATCHED_RELATION_PARTIAL_LENGTH {
        target_sum = target_sum + round_univariates[i] * denominator_inverses[i];
    }
    target_sum * numerator_value
}

fn partially_evaluate_pow(gate_challenge: Fr, current_evaluation: Fr, round_challenge: Fr) -> Fr {
    let univariate_eval = Fr::one() + round_challenge * (gate_challenge - Fr::one());
    current_evaluation * univariate_eval
}

pub fn verify_sumcheck(proof: &Proof, t: &Transcript, log_n: usize) -> bool {
    let mut round_target = Fr::zero();
    let mut pow_partial_evaluation = Fr::one();

    for round in 0..log_n {
        let round_univariate = &proof.sumcheck_univariates[round];

        if !check_sum(round_univariate, round_target) {
            return false;
        }

        let round_challenge = t.sumcheck_u_challenges[round];
        round_target = compute_next_target_sum(round_univariate, round_challenge);
        pow_partial_evaluation = partially_evaluate_pow(
            t.gate_challenges[round],
            pow_partial_evaluation,
            round_challenge,
        );
    }

    let grand_honk_relation_sum = accumulate_relation_evaluations(
        &proof.sumcheck_evaluations,
        &t.relation_parameters,
        &t.alphas,
        pow_partial_evaluation,
    );

    grand_honk_relation_sum == round_target
}
