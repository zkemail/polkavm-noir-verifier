/// Utilities: split-point Fq decoding, Fr/Fq byte helpers.
use super::fr::Fr;

/// Decode a split-encoded G1 x (or y) coordinate.
/// In the proof: x = x_0 | (x_1 << 136), i.e. x_1 occupies bits 255..136
/// Since 136 = 17*8, byte layout: combined[0..15] = x_1_be[17..32], combined[15..32] = x_0_be[15..32].
/// Returns the 32-byte big-endian Fq representation.
pub fn fq_from_split(x_0_be: &[u8; 32], x_1_be: &[u8; 32]) -> [u8; 32] {
    let mut combined = [0u8; 32];
    // x_1 occupies the high 120 bits (15 bytes): combined[0..15] = x_1_be[17..32]
    combined[0..15].copy_from_slice(&x_1_be[17..32]);
    // x_0 occupies the low 136 bits (17 bytes): combined[15..32] = x_0_be[15..32]
    combined[15..32].copy_from_slice(&x_0_be[15..32]);
    combined
}

/// Convert Fr to big-endian 32-byte scalar (for ecMul input)
pub fn fr_to_scalar(fr: Fr) -> [u8; 32] {
    fr.to_be_bytes()
}

/// Split an Fr challenge into (lo, hi) for transcript use.
/// lo = bytes[16..32] as Fr, hi = bytes[0..16] as Fr
pub fn split_challenge(challenge: Fr) -> (Fr, Fr) {
    let b = challenge.to_be_bytes();
    let mut lo_bytes = [0u8; 32];
    let mut hi_bytes = [0u8; 32];
    lo_bytes[16..32].copy_from_slice(&b[16..32]);
    hi_bytes[16..32].copy_from_slice(&b[0..16]);
    (Fr::from_be_bytes(&lo_bytes), Fr::from_be_bytes(&hi_bytes))
}
