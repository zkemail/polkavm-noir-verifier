/// BN254 G1 point operations via EVM precompiles on PolkaVM (pallet-revive).
///
/// Delegates to the standard EVM precompiles:
///   ecAdd (0x06) — EIP-196: https://eips.ethereum.org/EIPS/eip-196
///   ecMul (0x07) — EIP-196
///   ecPairing (0x08) — EIP-197: https://eips.ethereum.org/EIPS/eip-197
///
/// These are native implementations in the Polkadot runtime (not interpreted),
/// making them much cheaper than pure-Rust EC arithmetic inside PolkaVM.
/// Point format: uncompressed affine (x, y), each 32 bytes big-endian (Fq).
use pallet_revive_uapi::{CallFlags, HostFn, HostFnImpl as api};

/// A BN254 G1 affine point (uncompressed, big-endian field elements).
#[derive(Clone, Copy, Debug)]
pub struct G1Point {
    pub x: [u8; 32],
    pub y: [u8; 32],
}

impl G1Point {
    /// The point at infinity (identity).
    pub fn infinity() -> Self {
        G1Point {
            x: [0u8; 32],
            y: [0u8; 32],
        }
    }

    /// True if this is the point at infinity.
    pub fn is_infinity(&self) -> bool {
        self.x == [0u8; 32] && self.y == [0u8; 32]
    }
}

/// Negate a G1 point: (x, -y) where -y = Q - y, Q = BN254 Fq modulus.
/// Q = 0x30644e72e131a029b85045b68181585d97816a916871ca8d3c208c16d87cfd47
pub fn negate(p: G1Point) -> G1Point {
    if p.is_infinity() {
        return p;
    }
    let q: [u8; 32] = [
        0x30, 0x64, 0x4e, 0x72, 0xe1, 0x31, 0xa0, 0x29, 0xb8, 0x50, 0x45, 0xb6, 0x81, 0x81, 0x58,
        0x5d, 0x97, 0x81, 0x6a, 0x91, 0x68, 0x71, 0xca, 0x8d, 0x3c, 0x20, 0x8c, 0x16, 0xd8, 0x7c,
        0xfd, 0x47,
    ];
    G1Point {
        x: p.x,
        y: sub_fq(&q, &p.y),
    }
}

/// Subtract two 32-byte big-endian field elements: a - b (assumes a >= b in the field)
fn sub_fq(a: &[u8; 32], b: &[u8; 32]) -> [u8; 32] {
    let mut result = [0u8; 32];
    let mut borrow: i32 = 0;
    for i in (0..32).rev() {
        let diff = a[i] as i32 - b[i] as i32 - borrow;
        if diff < 0 {
            result[i] = (diff + 256) as u8;
            borrow = 1;
        } else {
            result[i] = diff as u8;
            borrow = 0;
        }
    }
    result
}

fn precompile_address(addr: u8) -> [u8; 20] {
    let mut a = [0u8; 20];
    a[19] = addr;
    a
}

/// ecAdd via precompile 0x06: returns P + Q
pub fn ec_add(p: G1Point, q: G1Point) -> G1Point {
    if p.is_infinity() {
        return q;
    }
    if q.is_infinity() {
        return p;
    }

    let mut input = [0u8; 128];
    input[0..32].copy_from_slice(&p.x);
    input[32..64].copy_from_slice(&p.y);
    input[64..96].copy_from_slice(&q.x);
    input[96..128].copy_from_slice(&q.y);

    let mut output = [0u8; 64];
    call_precompile(0x06, &input, &mut output);

    G1Point {
        x: output[0..32].try_into().unwrap(),
        y: output[32..64].try_into().unwrap(),
    }
}

/// ecMul via precompile 0x07: returns scalar * P (scalar is big-endian 32 bytes)
pub fn ec_mul(p: G1Point, scalar: &[u8; 32]) -> G1Point {
    if p.is_infinity() {
        return G1Point::infinity();
    }
    // If scalar is zero, return infinity
    if scalar == &[0u8; 32] {
        return G1Point::infinity();
    }

    let mut input = [0u8; 96];
    input[0..32].copy_from_slice(&p.x);
    input[32..64].copy_from_slice(&p.y);
    input[64..96].copy_from_slice(scalar);

    let mut output = [0u8; 64];
    call_precompile(0x07, &input, &mut output);

    G1Point {
        x: output[0..32].try_into().unwrap(),
        y: output[32..64].try_into().unwrap(),
    }
}

/// ecPairing via precompile 0x08: returns true if product of pairings = 1.
/// Expects pairs: [(G1_0, G2_0), (G1_1, G2_1)]
/// G2 point format: (x_im, x_re, y_im, y_re) each 32 bytes (EVM order).
pub fn ec_pairing_check(p0: G1Point, g2_0: &[u8; 128], p1: G1Point, g2_1: &[u8; 128]) -> bool {
    let mut input = [0u8; 384]; // 2 pairs × 192 bytes each
                                // Pair 0: G1(64 bytes) + G2(128 bytes)
    input[0..32].copy_from_slice(&p0.x);
    input[32..64].copy_from_slice(&p0.y);
    input[64..192].copy_from_slice(g2_0);
    // Pair 1: G1(64 bytes) + G2(128 bytes)
    input[192..224].copy_from_slice(&p1.x);
    input[224..256].copy_from_slice(&p1.y);
    input[256..384].copy_from_slice(g2_1);

    let mut output = [0u8; 32];
    call_precompile(0x08, &input, &mut output);

    // Output is 1 (success) if pairing product = 1
    output[31] == 1
}

fn call_precompile(addr: u8, input: &[u8], output: &mut [u8]) {
    let target = precompile_address(addr);
    let gas = api::gas_left() / 2;
    let mut output_ref: &mut [u8] = output;
    let _ = api::call(
        CallFlags::empty(),
        &target,
        gas,
        u64::MAX,
        &[0u8; 32], // deposit
        &[0u8; 32], // value
        input,
        Some(&mut output_ref),
    );
}
