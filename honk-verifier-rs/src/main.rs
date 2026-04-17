use std::fs;
use ultrahonk_no_std::{verify, ProofType};

fn main() {
    let proof_bytes = fs::read("../circuit/target/proof")
        .expect("failed to read proof");
    let vk_bytes = fs::read("../circuit/target/vk")
        .expect("failed to read vk");
    let pub_bytes = fs::read("../circuit/target/public_inputs")
        .expect("failed to read public_inputs");

    // Each public input is 32 bytes (one BN254 field element, big-endian)
    let pubs: Vec<[u8; 32]> = pub_bytes
        .chunks_exact(32)
        .map(|c| c.try_into().unwrap())
        .collect();

    println!("proof size:    {} bytes", proof_bytes.len());
    println!("vk size:       {} bytes", vk_bytes.len());
    println!("public inputs: {}", pubs.len());

    // No --zk flag used in bb prove → ProofType::Plain
    let proof = ProofType::Plain(proof_bytes.into_boxed_slice());

    match verify::<()>(&vk_bytes, &proof, &pubs) {
        Ok(()) => println!("✓ Proof verified successfully"),
        Err(e) => println!("✗ Verification failed: {:?}", e),
    }
}
