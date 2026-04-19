use ark_bn254::{Fq, Fr};
use ark_ff::PrimeField;
use tiny_keccak::{Hasher, Keccak};

pub fn fr_from_be_bytes(bytes: &[u8; 32]) -> Fr {
    Fr::from_be_bytes_mod_order(bytes)
}

pub fn fr_to_be_bytes(fr: Fr) -> [u8; 32] {
    let bi = fr.into_bigint();
    let mut bytes = [0u8; 32];
    bytes[0..8].copy_from_slice(&bi.0[3].to_be_bytes());
    bytes[8..16].copy_from_slice(&bi.0[2].to_be_bytes());
    bytes[16..24].copy_from_slice(&bi.0[1].to_be_bytes());
    bytes[24..32].copy_from_slice(&bi.0[0].to_be_bytes());
    bytes
}

/// Reconstruct Fq from split encoding: x = x_0 | (x_1 << 136)
/// 136 bits = 17 bytes (clean byte boundary)
/// combined[0..15] = x_1_be[17..32]
/// combined[15..32] = x_0_be[15..32]
pub fn fq_from_split(x_0_be: &[u8; 32], x_1_be: &[u8; 32]) -> Fq {
    let mut combined = [0u8; 32];
    combined[0..15].copy_from_slice(&x_1_be[17..32]);
    combined[15..32].copy_from_slice(&x_0_be[15..32]);
    Fq::from_be_bytes_mod_order(&combined)
}

pub fn keccak256(input: &[u8]) -> [u8; 32] {
    let mut hasher = Keccak::v256();
    hasher.update(input);
    let mut output = [0u8; 32];
    hasher.finalize(&mut output);
    output
}

/// Split a 256-bit challenge into two 128-bit halves (both reduced mod p).
/// first  = challenge & 0xFFFF...FF (low 128 bits)
/// second = challenge >> 128         (high 128 bits)
pub fn split_challenge(challenge: Fr) -> (Fr, Fr) {
    let bytes = fr_to_be_bytes(challenge);
    // bytes[0..16] = high 128 bits, bytes[16..32] = low 128 bits (big-endian)
    let mut lo_bytes = [0u8; 32];
    lo_bytes[16..32].copy_from_slice(&bytes[16..32]);
    let mut hi_bytes = [0u8; 32];
    hi_bytes[16..32].copy_from_slice(&bytes[0..16]);
    (fr_from_be_bytes(&lo_bytes), fr_from_be_bytes(&hi_bytes))
}
