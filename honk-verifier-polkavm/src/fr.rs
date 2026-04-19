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
        // Reduce mod P if needed (simple: at most one subtraction)
        let limbs = reduce_once(&limbs, &P);
        // Convert to Montgomery: limbs * R2 / R = limbs * R mod P
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
        // Fermat: a^{P-2} mod P (slow but simple; P is small enough for verifier use)
        Some(pow(self, &P_MINUS_2))
    }

    /// Square: self * self
    pub fn square(self) -> Fr {
        Fr(mont_mul(&self.0, &self.0))
    }
}

/// P - 2, for Fermat inversion
const P_MINUS_2: [u64; 4] = [
    0x43e1f593effffffe,
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

/// Reduce once: if a >= m, return a - m, else a
fn reduce_once(a: &[u64; 4], m: &[u64; 4]) -> [u64; 4] {
    // Check if a >= m (compare from most significant limb)
    let geq = {
        let mut c = true;
        let mut eq = true;
        for i in (0..4).rev() {
            if eq {
                if a[i] > m[i] {
                    c = true;
                    eq = false;
                } else if a[i] < m[i] {
                    c = false;
                    eq = false;
                }
            }
        }
        c || eq
    };
    if geq {
        sub_mod(a, m, m)
    } else {
        *a
    }
}

/// Square-and-multiply exponentiation: base^exp mod P (exp in little-endian limbs)
fn pow(base: Fr, exp: &[u64; 4]) -> Fr {
    let mut result = Fr::ONE;
    let mut b = base;
    for limb in exp.iter() {
        let mut limb = *limb;
        for _ in 0..64 {
            if limb & 1 == 1 {
                result = result * b;
            }
            b = b.square();
            limb >>= 1;
        }
    }
    result
}
