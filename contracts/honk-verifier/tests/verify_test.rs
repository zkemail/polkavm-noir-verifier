// x86 integration tests for UltraHonk verification logic.
// These tests run on the host (not PolkaVM) via the _lib_test.rs interface.
// EC precompile calls (shplemini pairing check) are not available on x86,
// so we test sumcheck + transcript generation only.

#[path = "../src/_lib_test.rs"]
mod lib_test;

use lib_test::honk::fr::Fr;
use lib_test::honk::proof::load_proof;
use lib_test::honk::transcript::generate_transcript;
use lib_test::vk::load_vk;
use lib_test::sumcheck;

const LOG_N: usize = 5;

fn compute_public_input_delta(
    public_inputs: &[[u8; 32]],
    beta: Fr, gamma: Fr,
    offset: u64, n: u64, _num_public_inputs: u64,
) -> Fr {
    let mut numerator = Fr::one();
    let mut denominator = Fr::one();
    let mut numerator_acc = gamma + beta * Fr::from_u64(n + offset);
    let mut denominator_acc = gamma - beta * Fr::from_u64(offset + 1);
    for pi in public_inputs.iter() {
        let pub_input = Fr::from_be_bytes(pi);
        numerator = numerator * (numerator_acc + pub_input);
        denominator = denominator * (denominator_acc + pub_input);
        numerator_acc = numerator_acc + beta;
        denominator_acc = denominator_acc - beta;
    }
    numerator * denominator.inverse().unwrap()
}

#[test]
fn test_fr_sanity() {
    assert_eq!(Fr::one() * Fr::one(), Fr::one());
    assert_eq!(Fr::from_u64(2) * Fr::from_u64(3), Fr::from_u64(6));
    assert_eq!(Fr::from_u64(5) - Fr::from_u64(3), Fr::from_u64(2));
}

#[test]
fn test_sumcheck_passes() {
    let proof_bytes = std::fs::read("../../circuit/target/proof").expect("read proof");
    let pi_bytes = std::fs::read("../../circuit/target/public_inputs").expect("read public_inputs");
    let mut public_inputs: Vec<[u8; 32]> = Vec::new();
    for chunk in pi_bytes.chunks(32) {
        let mut buf = [0u8; 32];
        buf.copy_from_slice(chunk);
        public_inputs.push(buf);
    }

    let vk = load_vk();
    let proof = load_proof(&proof_bytes);
    let mut t = generate_transcript(
        &proof, &public_inputs,
        vk.circuit_size, vk.public_inputs_size, vk.pub_inputs_offset,
    );
    t.relation_parameters.public_inputs_delta = compute_public_input_delta(
        &public_inputs, t.relation_parameters.beta, t.relation_parameters.gamma,
        vk.pub_inputs_offset, vk.circuit_size, vk.public_inputs_size,
    );

    let result = sumcheck::verify_sumcheck_diag(&proof, &t, LOG_N);
    assert_eq!(result, 0, "sumcheck failed with code {}", result);
}
