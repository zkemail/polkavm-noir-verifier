/// BN254 scalar field Fr arithmetic (Montgomery form, 256-bit limbs).
///
/// Field modulus P = 21888242871839275222246405745257275088548364400416034343698204186575808495617
///                 = 0x30644e72e131a029b85045b68181585d2833e84879b9709143e1f593f0000001
///
/// Implementation follows standard Montgomery multiplication algorithms.
/// See: Montgomery, P.L. (1985) "Modular multiplication without trial division"
/// Modular inverse uses binary extended GCD (constant-time-ish, avoids pow).
///
/// This is custom no_std code — no audited BN254 Fr library exists for PolkaVM.
/// Correctness is verified by on-chain tests: the full UltraHonk verifier
/// (which depends on every Fr operation) produces identical results to
/// Barretenberg's `bb verify` for valid and invalid proofs.
use core::ops::{Add, Mul, Neg, Sub};

/// P in 64-bit limbs (little-endian limb order)
pub const P: [u64; 4] = [
    0x43e1f593f0000001,
    0x2833e84879b97091,
    0xb85045b68181585d,
    0x30644e72e131a029,
];

/// -P^{-1} mod 2^64
const INV: u64 = 0xc2e1f593efffffff;

/// R = 2^256 mod P (Montgomery factor), little-endian limbs
pub const R: [u64; 4] = [
    0xac96341c4ffffffb,
    0x36fc76959f60cd29,
    0x666ea36f7879462e,
    0x0e0a77c19a07df2f,
];

/// R^2 mod P
pub const R2: [u64; 4] = [
    0x1bb8e645ae216da7,
    0x53fe3ab1e35c59e3,
    0x8c49833d53bb8085,
    0x0216d0b17f4e44a5,
];

/// Field element in Montgomery form: stores a·R mod P
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Fr(pub [u64; 4]);

impl Fr {
    pub const ZERO: Fr = Fr([0, 0, 0, 0]);
    pub const ONE: Fr = Fr(R); // 1·R mod P

    pub fn zero() -> Fr {
        Fr::ZERO
    }
    pub fn one() -> Fr {
        Fr::ONE
    }
    pub fn is_zero(self) -> bool {
        self.0 == [0, 0, 0, 0]
    }

    /// Construct from a small u64 (converts to Montgomery form)
    pub fn from_u64(v: u64) -> Fr {
        let raw = [v, 0, 0, 0];
        Fr(mont_mul(&raw, &R2))
    }

    /// Construct from big-endian 32-byte representation (reduces mod P, then to Montgomery)
    pub fn from_be_bytes(bytes: &[u8; 32]) -> Fr {
        let mut limbs = [0u64; 4];
        for i in 0..4 {
            let b = &bytes[(3 - i) * 8..(3 - i) * 8 + 8];
            limbs[i] = u64::from_be_bytes([b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7]]);
        }
        // Full reduction mod P: keccak outputs can be up to 2^256-1 ≈ 5.3*P,
        // so one subtraction is not sufficient. Loop until limbs < P.
        while geq_p(&limbs) {
            limbs = sub_mod(&limbs, &P, &P);
        }
        // Convert to Montgomery form: limbs * R mod P
        Fr(mont_mul(&limbs, &R2))
    }

    /// Return as big-endian 32 bytes (converts out of Montgomery)
    pub fn to_be_bytes(self) -> [u8; 32] {
        // Convert from Montgomery: self * 1 / R mod P
        let normal = mont_mul(&self.0, &[1, 0, 0, 0]);
        let mut out = [0u8; 32];
        for i in 0..4 {
            let b = normal[3 - i].to_be_bytes();
            out[i * 8..i * 8 + 8].copy_from_slice(&b);
        }
        out
    }

    pub fn inverse(self) -> Option<Fr> {
        if self.is_zero() {
            return None;
        }
        // Binary extended GCD (avoids pow() which is broken for large exponents on this target).
        // Convert from Montgomery form, invert in normal form, convert back.
        let normal = mont_mul(&self.0, &[1, 0, 0, 0]);
        let inv = mod_inverse_normal(&normal)?;
        // Convert back to Montgomery: inv * R = mont_mul(inv, R^2/R) = mont_mul(inv, R2)
        Some(Fr(mont_mul(&inv, &R2)))
    }

}

impl Add for Fr {
    type Output = Fr;
    fn add(self, rhs: Fr) -> Fr {
        Fr(add_mod(&self.0, &rhs.0, &P))
    }
}

impl Sub for Fr {
    type Output = Fr;
    fn sub(self, rhs: Fr) -> Fr {
        Fr(sub_mod(&self.0, &rhs.0, &P))
    }
}

impl Neg for Fr {
    type Output = Fr;
    fn neg(self) -> Fr {
        if self.is_zero() {
            self
        } else {
            Fr(sub_mod(&P, &self.0, &P))
        }
    }
}

impl Mul for Fr {
    type Output = Fr;
    fn mul(self, rhs: Fr) -> Fr {
        Fr(mont_mul(&self.0, &rhs.0))
    }
}

/// Multiply-accumulate: acc + a*b + carry_in → (lo, hi).
/// #[inline(never)] forces LLVM to handle each u128 operation in isolation,
/// preventing register allocation bugs on RV64E (only 16 GP registers).
#[inline(never)]
fn mac(acc: u64, a: u64, b: u64, carry: u64) -> (u64, u64) {
    let r = acc as u128 + (a as u128) * (b as u128) + carry as u128;
    (r as u64, (r >> 64) as u64)
}

/// Montgomery multiplication: computes (a * b) / R mod P using CIOS.
/// Uses #[inline(never)] mac() to isolate each u128 op from LLVM's optimizer.
fn mont_mul(a: &[u64; 4], b: &[u64; 4]) -> [u64; 4] {
    let mut t0: u64 = 0;
    let mut t1: u64 = 0;
    let mut t2: u64 = 0;
    let mut t3: u64 = 0;
    let mut t4: u64 = 0;

    // i = 0
    {
        let ai = a[0];
        let (r, c) = mac(t0, ai, b[0], 0); t0 = r;
        let (r, c) = mac(t1, ai, b[1], c); t1 = r;
        let (r, c) = mac(t2, ai, b[2], c); t2 = r;
        let (r, c) = mac(t3, ai, b[3], c); t3 = r;
        t4 = t4.wrapping_add(c);
        let m = t0.wrapping_mul(INV);
        let (_, c) = mac(t0, m, P[0], 0);
        let (r, c) = mac(t1, m, P[1], c); t0 = r;
        let (r, c) = mac(t2, m, P[2], c); t1 = r;
        let (r, c) = mac(t3, m, P[3], c); t2 = r;
        t3 = t4.wrapping_add(c);
        t4 = 0;
    }
    // i = 1
    {
        let ai = a[1];
        let (r, c) = mac(t0, ai, b[0], 0); t0 = r;
        let (r, c) = mac(t1, ai, b[1], c); t1 = r;
        let (r, c) = mac(t2, ai, b[2], c); t2 = r;
        let (r, c) = mac(t3, ai, b[3], c); t3 = r;
        t4 = t4.wrapping_add(c);
        let m = t0.wrapping_mul(INV);
        let (_, c) = mac(t0, m, P[0], 0);
        let (r, c) = mac(t1, m, P[1], c); t0 = r;
        let (r, c) = mac(t2, m, P[2], c); t1 = r;
        let (r, c) = mac(t3, m, P[3], c); t2 = r;
        t3 = t4.wrapping_add(c);
        t4 = 0;
    }
    // i = 2
    {
        let ai = a[2];
        let (r, c) = mac(t0, ai, b[0], 0); t0 = r;
        let (r, c) = mac(t1, ai, b[1], c); t1 = r;
        let (r, c) = mac(t2, ai, b[2], c); t2 = r;
        let (r, c) = mac(t3, ai, b[3], c); t3 = r;
        t4 = t4.wrapping_add(c);
        let m = t0.wrapping_mul(INV);
        let (_, c) = mac(t0, m, P[0], 0);
        let (r, c) = mac(t1, m, P[1], c); t0 = r;
        let (r, c) = mac(t2, m, P[2], c); t1 = r;
        let (r, c) = mac(t3, m, P[3], c); t2 = r;
        t3 = t4.wrapping_add(c);
        t4 = 0;
    }
    // i = 3
    {
        let ai = a[3];
        let (r, c) = mac(t0, ai, b[0], 0); t0 = r;
        let (r, c) = mac(t1, ai, b[1], c); t1 = r;
        let (r, c) = mac(t2, ai, b[2], c); t2 = r;
        let (r, c) = mac(t3, ai, b[3], c); t3 = r;
        t4 = t4.wrapping_add(c);
        let m = t0.wrapping_mul(INV);
        let (_, c) = mac(t0, m, P[0], 0);
        let (r, c) = mac(t1, m, P[1], c); t0 = r;
        let (r, c) = mac(t2, m, P[2], c); t1 = r;
        let (r, c) = mac(t3, m, P[3], c); t2 = r;
        t3 = t4.wrapping_add(c);
    }

    reduce_once(&[t0, t1, t2, t3], &P)
}

/// 4-limb subtraction: a - b, returns (result, borrow).
/// Uses only u64 operations (overflowing_sub) — no i128.
/// Matches PoseidonPolkaVM's sub4 approach.
fn sub4(a: &[u64; 4], b: &[u64; 4]) -> ([u64; 4], bool) {
    let mut r = [0u64; 4];
    let mut borrow: u64 = 0;

    let (d, b1) = a[0].overflowing_sub(b[0]);
    let (d2, b2) = d.overflowing_sub(borrow);
    r[0] = d2;
    borrow = (b1 as u64) + (b2 as u64);

    let (d, b1) = a[1].overflowing_sub(b[1]);
    let (d2, b2) = d.overflowing_sub(borrow);
    r[1] = d2;
    borrow = (b1 as u64) + (b2 as u64);

    let (d, b1) = a[2].overflowing_sub(b[2]);
    let (d2, b2) = d.overflowing_sub(borrow);
    r[2] = d2;
    borrow = (b1 as u64) + (b2 as u64);

    let (d, b1) = a[3].overflowing_sub(b[3]);
    let (d2, b2) = d.overflowing_sub(borrow);
    r[3] = d2;
    borrow = (b1 as u64) + (b2 as u64);

    (r, borrow != 0)
}

/// 4-limb addition: a + b, returns (result, carry).
/// Uses only u64 operations — no u128.
fn add4(a: &[u64; 4], b: &[u64; 4]) -> ([u64; 4], bool) {
    let mut r = [0u64; 4];
    let mut carry: u64 = 0;

    let (s, c1) = a[0].overflowing_add(b[0]);
    let (s2, c2) = s.overflowing_add(carry);
    r[0] = s2;
    carry = (c1 as u64) + (c2 as u64);

    let (s, c1) = a[1].overflowing_add(b[1]);
    let (s2, c2) = s.overflowing_add(carry);
    r[1] = s2;
    carry = (c1 as u64) + (c2 as u64);

    let (s, c1) = a[2].overflowing_add(b[2]);
    let (s2, c2) = s.overflowing_add(carry);
    r[2] = s2;
    carry = (c1 as u64) + (c2 as u64);

    let (s, c1) = a[3].overflowing_add(b[3]);
    let (s2, c2) = s.overflowing_add(carry);
    r[3] = s2;
    carry = (c1 as u64) + (c2 as u64);

    (r, carry != 0)
}

/// Addition mod m
fn add_mod(a: &[u64; 4], b: &[u64; 4], m: &[u64; 4]) -> [u64; 4] {
    let (r, carry) = add4(a, b);
    // If carry, or r >= m, subtract m
    let (sub_r, borrow) = sub4(&r, m);
    if carry || !borrow {
        sub_r
    } else {
        r
    }
}

/// Subtraction mod m: a - b mod m
fn sub_mod(a: &[u64; 4], b: &[u64; 4], m: &[u64; 4]) -> [u64; 4] {
    let (r, borrowed) = sub4(a, b);
    if borrowed {
        // Add m back
        let (r2, _) = add4(&r, m);
        r2
    } else {
        r
    }
}

/// Compare a >= P (the BN254 Fr modulus), used for full reduction.
fn geq_p(a: &[u64; 4]) -> bool {
    // Try subtracting P; if no borrow, a >= P
    let (_, borrow) = sub4(a, &P);
    !borrow
}

/// Reduce once: if a >= m, return a - m, else a.
/// Only correct when a < 2m (single reduction suffices for montgomery output).
fn reduce_once(a: &[u64; 4], m: &[u64; 4]) -> [u64; 4] {
    let (sub_r, borrow) = sub4(a, m);
    if !borrow {
        sub_r // a >= m, return a - m
    } else {
        *a // a < m, keep as is
    }
}

// ─── Binary extended GCD modular inverse ────────────────────────────────────

/// Right shift a 256-bit value (little-endian limbs) by 1 bit in place.
fn shr1_256(a: &mut [u64; 4]) {
    a[0] = (a[0] >> 1) | (a[1] << 63);
    a[1] = (a[1] >> 1) | (a[2] << 63);
    a[2] = (a[2] >> 1) | (a[3] << 63);
    a[3] >>= 1;
}

/// Add two 256-bit values; returns the result (overflow is discarded — only used
/// when we know the sum fits, i.e. at most P + P < 2^256).
fn add256(a: &[u64; 4], b: &[u64; 4]) -> [u64; 4] {
    let (r, _) = add4(a, b);
    r
}

/// Compare two 256-bit values: true if a >= b.
fn geq256(a: &[u64; 4], b: &[u64; 4]) -> bool {
    let (_, borrow) = sub4(a, b);
    !borrow
}

/// Subtract two 256-bit values (assumes a >= b, no underflow).
fn sub256(a: &[u64; 4], b: &[u64; 4]) -> [u64; 4] {
    let (r, _) = sub4(a, b);
    r
}

fn is_zero256(a: &[u64; 4]) -> bool {
    a[0] == 0 && a[1] == 0 && a[2] == 0 && a[3] == 0
}

fn is_one256(a: &[u64; 4]) -> bool {
    a[0] == 1 && a[1] == 0 && a[2] == 0 && a[3] == 0
}

/// Modular inverse in NORMAL (non-Montgomery) form via binary extended GCD.
/// Returns None if a == 0.  Assumes P is an odd prime.
fn mod_inverse_normal(a: &[u64; 4]) -> Option<[u64; 4]> {
    if is_zero256(a) { return None; }

    let mut u = *a;
    let mut v = P;
    // Bezout coefficients for u: s such that u * s ≡ a^{-1} (mod P) at the end.
    let mut s = [1u64, 0u64, 0u64, 0u64];
    let mut r = [0u64; 4];

    loop {
        if is_one256(&u) { return Some(reduce_once(&s, &P)); }
        if is_one256(&v) { return Some(reduce_once(&r, &P)); }
        if is_zero256(&u) || is_zero256(&v) { return None; } // shouldn't happen for prime P

        // Remove common factor of 2 from u, adjusting s accordingly.
        while u[0] & 1 == 0 {
            shr1_256(&mut u);
            // s must stay in [0, P); halve s; if s is odd, add P first to make it even.
            if s[0] & 1 == 1 { s = add256(&s, &P); }
            shr1_256(&mut s);
        }
        // Remove common factor of 2 from v, adjusting r accordingly.
        while v[0] & 1 == 0 {
            shr1_256(&mut v);
            if r[0] & 1 == 1 { r = add256(&r, &P); }
            shr1_256(&mut r);
        }

        // Both u and v are now odd.  Subtract smaller from larger.
        if geq256(&u, &v) {
            u = sub256(&u, &v);
            s = sub_mod(&s, &r, &P);
        } else {
            v = sub256(&v, &u);
            r = sub_mod(&r, &s, &P);
        }
    }
}
