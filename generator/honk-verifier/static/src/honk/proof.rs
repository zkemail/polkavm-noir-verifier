/// UltraHonk proof deserialization.
///
/// Translated from Barretenberg's HonkVerifier.sol `TranscriptLib.loadProof()`.
/// Source: `bb write_solidity_verifier` output (Aztec/Barretenberg).
/// Proof layout: 8 G1 commitments (split-encoded) + sumcheck univariates
/// + sumcheck evaluations + Gemini fold commitments + evaluations
/// + Shplonk quotient + KZG quotient.
///
/// Constants CONST_PROOF_SIZE_LOG_N=28 and NUMBER_OF_ENTITIES=40 are
/// fixed for all UltraHonk circuits (defined in the Barretenberg prover).
extern crate alloc;
use super::fr::Fr;
use alloc::alloc::{alloc_zeroed, Layout};
use alloc::boxed::Box;

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
    Fr::from_be_bytes(&buf)
}

/// Parse proof from raw bytes directly into heap-allocated memory.
/// Uses alloc_zeroed to guarantee the Proof struct is never on the stack
/// (stack allocation would overflow the limited PolkaVM stack).
pub fn load_proof(data: &[u8]) -> Box<Proof> {
    // Allocate zeroed Proof on the heap directly — no stack allocation.
    let layout = Layout::new::<Proof>();
    let ptr = unsafe { alloc_zeroed(layout) as *mut Proof };
    assert!(!ptr.is_null());

    let p = unsafe { &mut *ptr };

    p.w1 = read_g1pp(&data[0x000..]);
    p.w2 = read_g1pp(&data[0x080..]);
    p.w3 = read_g1pp(&data[0x100..]);
    p.lookup_read_counts = read_g1pp(&data[0x180..]);
    p.lookup_read_tags = read_g1pp(&data[0x200..]);
    p.w4 = read_g1pp(&data[0x280..]);
    p.lookup_inverses = read_g1pp(&data[0x300..]);
    p.z_perm = read_g1pp(&data[0x380..]);

    // Use slice-advancing pattern (no integer index variable) to avoid compiler
    // bug where `for i in 0..N { use i }` generates wrong code on this target.
    let mut cur = &data[0x400..];

    for row in p.sumcheck_univariates.iter_mut() {
        for elem in row.iter_mut() {
            *elem = read_fr(cur);
            cur = &cur[32..];
        }
    }

    for elem in p.sumcheck_evaluations.iter_mut() {
        *elem = read_fr(cur);
        cur = &cur[32..];
    }

    for comm in p.gemini_fold_comms.iter_mut() {
        *comm = read_g1pp(cur);
        cur = &cur[128..];
    }

    for elem in p.gemini_a_evaluations.iter_mut() {
        *elem = read_fr(cur);
        cur = &cur[32..];
    }

    p.shplonk_q = read_g1pp(cur);
    cur = &cur[128..];
    p.kzg_quotient = read_g1pp(cur);
    let _ = cur;

    unsafe { Box::from_raw(ptr) }
}
