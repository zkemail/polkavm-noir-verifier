use ark_bn254::Fr;
use ark_ff::PrimeField;

use crate::fr_utils::fr_from_be_bytes;

pub const CONST_PROOF_SIZE_LOG_N: usize = 28;
pub const NUMBER_OF_ENTITIES: usize = 40;
pub const BATCHED_RELATION_PARTIAL_LENGTH: usize = 8;

/// G1 proof point in split encoding.
/// x = x_0 | (x_1 << 136),  y = y_0 | (y_1 << 136)
#[derive(Clone, Copy, Default)]
pub struct G1ProofPoint {
    pub x_0: [u8; 32],
    pub x_1: [u8; 32],
    pub y_0: [u8; 32],
    pub y_1: [u8; 32],
}

pub struct Proof {
    pub w1: G1ProofPoint,
    pub w2: G1ProofPoint,
    pub w3: G1ProofPoint,
    pub w4: G1ProofPoint,
    pub z_perm: G1ProofPoint,
    pub lookup_read_counts: G1ProofPoint,
    pub lookup_read_tags: G1ProofPoint,
    pub lookup_inverses: G1ProofPoint,
    pub sumcheck_univariates: [[Fr; BATCHED_RELATION_PARTIAL_LENGTH]; CONST_PROOF_SIZE_LOG_N],
    pub sumcheck_evaluations: [Fr; NUMBER_OF_ENTITIES],
    pub gemini_fold_comms: [G1ProofPoint; CONST_PROOF_SIZE_LOG_N - 1],
    pub gemini_a_evaluations: [Fr; CONST_PROOF_SIZE_LOG_N],
    pub shplonk_q: G1ProofPoint,
    pub kzg_quotient: G1ProofPoint,
}

fn read_g1pp(data: &[u8]) -> G1ProofPoint {
    assert!(data.len() >= 128);
    let mut pp = G1ProofPoint::default();
    pp.x_0.copy_from_slice(&data[0..32]);
    pp.x_1.copy_from_slice(&data[32..64]);
    pp.y_0.copy_from_slice(&data[64..96]);
    pp.y_1.copy_from_slice(&data[96..128]);
    pp
}

fn read_fr(data: &[u8]) -> Fr {
    let mut buf = [0u8; 32];
    buf.copy_from_slice(&data[..32]);
    fr_from_be_bytes(&buf)
}

/// Parse proof from raw bytes (matching TranscriptLib.loadProof in Solidity).
pub fn load_proof(data: &[u8]) -> Proof {
    assert_eq!(data.len(), 14080, "expected 14080-byte proof");

    let w1 = read_g1pp(&data[0x000..]);
    let w2 = read_g1pp(&data[0x080..]);
    let w3 = read_g1pp(&data[0x100..]);
    let lookup_read_counts = read_g1pp(&data[0x180..]);
    let lookup_read_tags = read_g1pp(&data[0x200..]);
    let w4 = read_g1pp(&data[0x280..]);
    let lookup_inverses = read_g1pp(&data[0x300..]);
    let z_perm = read_g1pp(&data[0x380..]);

    let mut boundary = 0x400usize;

    let mut sumcheck_univariates =
        [[Fr::from(0u64); BATCHED_RELATION_PARTIAL_LENGTH]; CONST_PROOF_SIZE_LOG_N];
    for i in 0..CONST_PROOF_SIZE_LOG_N {
        for j in 0..BATCHED_RELATION_PARTIAL_LENGTH {
            sumcheck_univariates[i][j] = read_fr(&data[boundary..]);
            boundary += 32;
        }
    }

    let mut sumcheck_evaluations = [Fr::from(0u64); NUMBER_OF_ENTITIES];
    for i in 0..NUMBER_OF_ENTITIES {
        sumcheck_evaluations[i] = read_fr(&data[boundary..]);
        boundary += 32;
    }

    let mut gemini_fold_comms = [G1ProofPoint::default(); CONST_PROOF_SIZE_LOG_N - 1];
    for i in 0..CONST_PROOF_SIZE_LOG_N - 1 {
        gemini_fold_comms[i] = read_g1pp(&data[boundary..]);
        boundary += 128;
    }

    let mut gemini_a_evaluations = [Fr::from(0u64); CONST_PROOF_SIZE_LOG_N];
    for i in 0..CONST_PROOF_SIZE_LOG_N {
        gemini_a_evaluations[i] = read_fr(&data[boundary..]);
        boundary += 32;
    }

    let shplonk_q = read_g1pp(&data[boundary..]);
    boundary += 128;
    let kzg_quotient = read_g1pp(&data[boundary..]);

    Proof {
        w1,
        w2,
        w3,
        w4,
        z_perm,
        lookup_read_counts,
        lookup_read_tags,
        lookup_inverses,
        sumcheck_univariates,
        sumcheck_evaluations,
        gemini_fold_comms,
        gemini_a_evaluations,
        shplonk_q,
        kzg_quotient,
    }
}
