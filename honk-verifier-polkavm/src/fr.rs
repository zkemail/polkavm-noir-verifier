/// BN254 scalar field Fr arithmetic (Montgomery form, 256-bit limbs).
/// P = 0x30644e72e131a029b85045b68181585d2833e84879b9709143e1f593f0000001
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
    0x666ea36f7879462c,
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

    /// Square: self * self
    pub fn square(self) -> Fr {
        Fr(mont_mul(&self.0, &self.0))
    }
}

/// P - 2, for Fermat inversion
const P_MINUS_2: [u64; 4] = [
    0x43e1f593efffffff,
    0x2833e84879b97091,
    0xb85045b68181585d,
    0x30644e72e131a029,
];

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

/// Montgomery multiplication: computes (a * b) / R mod P using CIOS
fn mont_mul(a: &[u64; 4], b: &[u64; 4]) -> [u64; 4] {
    let mut t = [0u64; 5];
    for i in 0..4 {
        // t = t + a[i]*b + m*P where m = t[0] * INV mod 2^64
        let mut c: u128 = 0;
        for j in 0..4 {
            c += t[j] as u128 + a[i] as u128 * b[j] as u128;
            t[j] = c as u64;
            c >>= 64;
        }
        t[4] = t[4].wrapping_add(c as u64);

        let m = t[0].wrapping_mul(INV);
        c = t[0] as u128 + m as u128 * P[0] as u128;
        c >>= 64;
        for j in 1..4 {
            c += t[j] as u128 + m as u128 * P[j] as u128;
            t[j - 1] = c as u64;
            c >>= 64;
        }
        t[3] = t[4].wrapping_add(c as u64);
        t[4] = 0;
    }
    reduce_once(&[t[0], t[1], t[2], t[3]], &P)
}

/// Addition mod m
fn add_mod(a: &[u64; 4], b: &[u64; 4], m: &[u64; 4]) -> [u64; 4] {
    let mut carry = 0u128;
    let mut r = [0u64; 4];
    for i in 0..4 {
        carry += a[i] as u128 + b[i] as u128;
        r[i] = carry as u64;
        carry >>= 64;
    }
    // carry is 0 or 1; if result >= P, subtract P
    reduce_once(&r, m)
}

/// Subtraction mod m: a - b mod m
fn sub_mod(a: &[u64; 4], b: &[u64; 4], m: &[u64; 4]) -> [u64; 4] {
    let mut borrow = 0i128;
    let mut r = [0u64; 4];
    for i in 0..4 {
        let diff = a[i] as i128 - b[i] as i128 - borrow;
        r[i] = diff as u64;
        borrow = if diff < 0 { 1 } else { 0 };
    }
    if borrow != 0 {
        // Add m back
        let mut carry = 0u128;
        for i in 0..4 {
            carry += r[i] as u128 + m[i] as u128;
            r[i] = carry as u64;
            carry >>= 64;
        }
    }
    r
}

/// Compare a >= P (the BN254 Fr modulus), used for full reduction.
fn geq_p(a: &[u64; 4]) -> bool {
    for i in (0..4).rev() {
        if a[i] > P[i] {
            return true;
        } else if a[i] < P[i] {
            return false;
        }
    }
    true // equal
}

/// Reduce once: if a >= m, return a - m, else a.
/// Only correct when a < 2m (single reduction suffices for montgomery output).
fn reduce_once(a: &[u64; 4], m: &[u64; 4]) -> [u64; 4] {
    // Compare a >= m from most significant limb
    let geq = {
        let mut result = false; // a < m unless proven otherwise
        let mut decided = false;
        for i in (0..4).rev() {
            if !decided {
                if a[i] > m[i] {
                    result = true;
                    decided = true;
                } else if a[i] < m[i] {
                    result = false;
                    decided = true;
                }
            }
        }
        if !decided { true } else { result } // equal counts as >=
    };
    if geq {
        sub_mod(a, m, m)
    } else {
        *a
    }
}

/// Public test wrapper for pow (still used for diagnostic)
pub fn pow_pub(base: Fr, exp: &[u64; 4]) -> Fr {
    pow(base, exp)
}

/// Square-and-multiply exponentiation (kept for non-inverse uses if needed)
fn pow(base: Fr, exp: &[u64; 4]) -> Fr {
    let mut result = Fr::ONE;
    let mut b = base;
    for limb_idx in 0..4usize {
        let mut e = exp[limb_idx];
        for _ in 0..64usize {
            if e & 1 == 1 {
                result = result * b;
            }
            b = b.square();
            e >>= 1;
        }
    }
    result
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
    let mut carry: u128 = 0;
    let mut r = [0u64; 4];
    for i in 0..4 {
        carry += a[i] as u128 + b[i] as u128;
        r[i] = carry as u64;
        carry >>= 64;
    }
    r
}

/// Compare two 256-bit values: true if a >= b.
fn geq256(a: &[u64; 4], b: &[u64; 4]) -> bool {
    for i in (0..4).rev() {
        if a[i] > b[i] { return true; }
        if a[i] < b[i] { return false; }
    }
    true
}

/// Subtract two 256-bit values (assumes a >= b, no underflow).
fn sub256(a: &[u64; 4], b: &[u64; 4]) -> [u64; 4] {
    let mut borrow: i128 = 0;
    let mut r = [0u64; 4];
    for i in 0..4 {
        let diff = a[i] as i128 - b[i] as i128 - borrow;
        r[i] = diff as u64;
        borrow = if diff < 0 { 1 } else { 0 };
    }
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
