use ark_bn254::Fr;
use ark_ff::{Field, One, Zero};

use crate::proof::{Proof, BATCHED_RELATION_PARTIAL_LENGTH, CONST_PROOF_SIZE_LOG_N};
use crate::relations::accumulate_relation_evaluations;
use crate::transcript::{Transcript, NUMBER_OF_ALPHAS};

/// Barycentric evaluation constants (from Solidity BARYCENTRIC_LAGRANGE_DENOMINATORS)
fn barycentric_lagrange_denominators() -> [Fr; BATCHED_RELATION_PARTIAL_LENGTH] {
    use ark_ff::PrimeField;
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

fn fr_from_hex(hex64: &str) -> Fr {
    use ark_ff::PrimeField;
    let s = if hex64.len() < 64 {
        format!("{:0>64}", hex64)
    } else {
        hex64.to_string()
    };
    let mut bytes = [0u8; 32];
    for (i, chunk) in s.as_bytes().chunks(2).enumerate() {
        let hi = (chunk[0] as char).to_digit(16).unwrap() as u8;
        let lo = (chunk[1] as char).to_digit(16).unwrap() as u8;
        bytes[i] = (hi << 4) | lo;
    }
    Fr::from_be_bytes_mod_order(&bytes)
}

/// Check that round_univariate[0] + round_univariate[1] == round_target.
fn check_sum(
    round_univariate: &[Fr; BATCHED_RELATION_PARTIAL_LENGTH],
    round_target: Fr,
) -> bool {
    round_univariate[0] + round_univariate[1] == round_target
}

/// Barycentric interpolation: evaluate the degree-7 univariate at `challenge`.
fn compute_next_target_sum(
    round_univariates: &[Fr; BATCHED_RELATION_PARTIAL_LENGTH],
    challenge: Fr,
) -> Fr {
    let denoms = barycentric_lagrange_denominators();

    // B(x) = product_{i=0}^{7} (x - i)
    let mut numerator_value = Fr::one();
    for i in 0..BATCHED_RELATION_PARTIAL_LENGTH {
        numerator_value *= challenge - Fr::from(i as u64);
    }

    // denominatorInverses[i] = 1 / (LAGRANGE_DENOM[i] * (x - i))
    let mut denominator_inverses = [Fr::zero(); BATCHED_RELATION_PARTIAL_LENGTH];
    for i in 0..BATCHED_RELATION_PARTIAL_LENGTH {
        let inv = denoms[i] * (challenge - Fr::from(i as u64));
        denominator_inverses[i] = inv.inverse().unwrap_or(Fr::zero());
    }

    let mut target_sum = Fr::zero();
    for i in 0..BATCHED_RELATION_PARTIAL_LENGTH {
        target_sum += round_univariates[i] * denominator_inverses[i];
    }
    target_sum * numerator_value
}

/// POW partial evaluation: (1 + roundChallenge * (gateChallenge - 1)) * currentEvaluation
fn partially_evaluate_pow(gate_challenge: Fr, current_evaluation: Fr, round_challenge: Fr) -> Fr {
    let univariate_eval = Fr::one() + round_challenge * (gate_challenge - Fr::one());
    current_evaluation * univariate_eval
}

/// Verify the sumcheck protocol (matches BaseHonkVerifier.verifySumcheck).
pub fn verify_sumcheck(proof: &Proof, t: &Transcript, log_n: usize) -> bool {
    let mut round_target = Fr::zero();
    let mut pow_partial_evaluation = Fr::one();

    for round in 0..log_n {
        let round_univariate = &proof.sumcheck_univariates[round];

        if !check_sum(round_univariate, round_target) {
            eprintln!("Sumcheck round {} failed sum check", round);
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

    let verified = grand_honk_relation_sum == round_target;
    if !verified {
        eprintln!(
            "Sumcheck final check failed: grand_honk={:?} vs round_target={:?}",
            grand_honk_relation_sum, round_target
        );
    }
    verified
}
