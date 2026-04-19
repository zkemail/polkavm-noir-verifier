mod fr_utils;
mod proof;
mod relations;
mod shplemini;
mod sumcheck;
mod transcript;
mod vk;

use ark_bn254::Fr;
use ark_ff::{Field, One};

use proof::load_proof;
use transcript::generate_transcript;
use vk::load_vk;

const LOG_N: usize = 5;

fn main() {
    let proof_bytes = std::fs::read("../circuit/target/proof").expect("proof file not found");
    let pub_input_bytes =
        std::fs::read("../circuit/target/public_inputs").expect("public_inputs file not found");

    assert_eq!(pub_input_bytes.len() % 32, 0);
    let public_inputs: Vec<[u8; 32]> = pub_input_bytes
        .chunks(32)
        .map(|c| c.try_into().unwrap())
        .collect();

    let vk = load_vk();
    let proof = load_proof(&proof_bytes);

    let result = verify(&proof, &vk, &public_inputs);
    println!("Verification result: {}", result);
    if !result {
        std::process::exit(1);
    }
}

fn verify(proof: &proof::Proof, vk: &vk::VerificationKey, public_inputs: &[[u8; 32]]) -> bool {
    // Generate the Fiat-Shamir transcript
    let mut t = generate_transcript(
        proof,
        public_inputs,
        vk.circuit_size,
        vk.public_inputs_size,
        vk.pub_inputs_offset,
    );

    // Compute public input delta
    t.relation_parameters.public_inputs_delta = compute_public_input_delta(
        public_inputs,
        t.relation_parameters.beta,
        t.relation_parameters.gamma,
        vk.pub_inputs_offset,
        vk.circuit_size,
        vk.public_inputs_size,
    );

    // Sumcheck
    if !sumcheck::verify_sumcheck(proof, &t, LOG_N) {
        eprintln!("Sumcheck failed");
        return false;
    }

    // Shplemini / KZG
    if !shplemini::verify_shplemini(proof, vk, &t) {
        eprintln!("Shplemini failed");
        return false;
    }

    true
}

/// Compute the public input delta (matches BaseHonkVerifier.computePublicInputDelta).
fn compute_public_input_delta(
    public_inputs: &[[u8; 32]],
    beta: Fr,
    gamma: Fr,
    offset: u64,
    n: u64,
    num_public_inputs: u64,
) -> Fr {
    let mut numerator = Fr::one();
    let mut denominator = Fr::one();

    let mut numerator_acc = gamma + beta * Fr::from(n + offset);
    let mut denominator_acc = gamma - beta * Fr::from(offset + 1);

    for i in 0..num_public_inputs as usize {
        let pub_input = fr_from_be_bytes_mod_p(&public_inputs[i]);
        numerator *= numerator_acc + pub_input;
        denominator *= denominator_acc + pub_input;
        numerator_acc += beta;
        denominator_acc -= beta;
    }

    numerator * denominator.inverse().expect("denominator is zero")
}

fn fr_from_be_bytes_mod_p(bytes: &[u8; 32]) -> Fr {
    use ark_ff::PrimeField;
    Fr::from_be_bytes_mod_order(bytes)
}
