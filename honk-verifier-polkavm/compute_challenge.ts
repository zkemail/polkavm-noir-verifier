import { ethers } from 'ethers';
import * as fs from 'fs';
import * as path from 'path';

const P = BigInt("0x30644e72e131a029b85045b68181585d2833e84879b9709143e1f593f0000001");

function modP(x: bigint): bigint {
    return ((x % P) + P) % P;
}

function frAdd(a: bigint, b: bigint): bigint { return modP(a + b); }
function frMul(a: bigint, b: bigint): bigint { return modP(a * b); }
function frInv(a: bigint): bigint { return modP(modExp(a, P - 2n, P)); }
function frSub(a: bigint, b: bigint): bigint { return modP(a - b); }

function modExp(base: bigint, exp: bigint, mod: bigint): bigint {
    let result = 1n;
    base = base % mod;
    while (exp > 0n) {
        if (exp % 2n === 1n) result = result * base % mod;
        exp >>= 1n;
        base = base * base % mod;
    }
    return result;
}

function keccak256(data: Uint8Array): Uint8Array {
    const hex = ethers.keccak256(data);
    return ethers.getBytes(hex);
}

function u64ToBe32(v: bigint): Uint8Array {
    const buf = new Uint8Array(32);
    for (let i = 0; i < 8; i++) {
        buf[31 - i] = Number(v & 0xffn);
        v >>= 8n;
    }
    return buf;
}

function hashU256s(values: Uint8Array[]): Uint8Array {
    const total = new Uint8Array(values.reduce((s, v) => s + v.length, 0));
    let off = 0;
    for (const v of values) { total.set(v, off); off += v.length; }
    return keccak256(total);
}

function splitChallenge(h: Uint8Array): [bigint, bigint] {
    const lo_bytes = new Uint8Array(32);
    const hi_bytes = new Uint8Array(32);
    lo_bytes.set(h.slice(16, 32), 16);
    hi_bytes.set(h.slice(0, 16), 16);
    const lo = modP(BigInt('0x' + Buffer.from(lo_bytes).toString('hex')));
    const hi = modP(BigInt('0x' + Buffer.from(hi_bytes).toString('hex')));
    return [lo, hi];
}

const proofData = fs.readFileSync('/Users/benceharomi/Projects/kusama-demo/honk-verifier-polkavm/../circuit/target/proof');
const pubInputData = fs.readFileSync('/Users/benceharomi/Projects/kusama-demo/honk-verifier-polkavm/../circuit/target/public_inputs');

function readFrBe(offset: number): bigint {
    return BigInt('0x' + proofData.slice(offset, offset+32).toString('hex'));
}

function readG1pp(offset: number): [Uint8Array, Uint8Array, Uint8Array, Uint8Array] {
    return [proofData.slice(offset, offset+32), proofData.slice(offset+32, offset+64),
            proofData.slice(offset+64, offset+96), proofData.slice(offset+96, offset+128)];
}

const [w1x0, w1x1, w1y0, w1y1] = readG1pp(0x000);
const [w2x0, w2x1, w2y0, w2y1] = readG1pp(0x080);
const [w3x0, w3x1, w3y0, w3y1] = readG1pp(0x100);
const [lrc0, lrc1, lrc2, lrc3] = readG1pp(0x180);
const [lrt0, lrt1, lrt2, lrt3] = readG1pp(0x200);
const [w4x0, w4x1, w4y0, w4y1] = readG1pp(0x280);
const [li0, li1, li2, li3] = readG1pp(0x300);
const [zp0, zp1, zp2, zp3] = readG1pp(0x380);

// Round 0: eta
const round0 = [u64ToBe32(32n), u64ToBe32(1n), u64ToBe32(1n), pubInputData.slice(0,32),
    w1x0, w1x1, w1y0, w1y1, w2x0, w2x1, w2y0, w2y1, w3x0, w3x1, w3y0, w3y1];
let prevH = hashU256s(round0.map(v => v instanceof Uint8Array ? v : v));
let prev = modP(BigInt('0x' + Buffer.from(prevH).toString('hex')));
let prevBytes = new Uint8Array(32);
// prev to 32-byte big-endian
{ let tmp = prev; for(let i=31;i>=0;i--) { prevBytes[i]=Number(tmp&0xffn); tmp>>=8n; } }
const [eta, eta2] = splitChallenge(prevBytes);
prevH = keccak256(prevBytes);
prev = modP(BigInt('0x' + Buffer.from(prevH).toString('hex')));
{ let tmp = prev; for(let i=31;i>=0;i--) { prevBytes[i]=Number(tmp&0xffn); tmp>>=8n; } }

// Round 1: beta/gamma
prevH = hashU256s([prevBytes, lrc0,lrc1,lrc2,lrc3, lrt0,lrt1,lrt2,lrt3, w4x0,w4x1,w4y0,w4y1]);
prev = modP(BigInt('0x' + Buffer.from(prevH).toString('hex')));
{ let tmp = prev; for(let i=31;i>=0;i--) { prevBytes[i]=Number(tmp&0xffn); tmp>>=8n; } }
const [beta, gamma] = splitChallenge(prevBytes);
console.log('beta =', beta.toString(16));
console.log('gamma =', gamma.toString(16));

// Alpha
prevH = hashU256s([prevBytes, li0,li1,li2,li3, zp0,zp1,zp2,zp3]);
prev = modP(BigInt('0x' + Buffer.from(prevH).toString('hex')));
{ let tmp = prev; for(let i=31;i>=0;i--) { prevBytes[i]=Number(tmp&0xffn); tmp>>=8n; } }

// 12 alpha pairs + 1 last
for (let i = 1; i < 12; i++) {
    prevH = keccak256(prevBytes);
    prev = modP(BigInt('0x' + Buffer.from(prevH).toString('hex')));
    { let tmp = prev; for(let i=31;i>=0;i--) { prevBytes[i]=Number(tmp&0xffn); tmp>>=8n; } }
}
prevH = keccak256(prevBytes);
prev = modP(BigInt('0x' + Buffer.from(prevH).toString('hex')));
{ let tmp = prev; for(let i=31;i>=0;i--) { prevBytes[i]=Number(tmp&0xffn); tmp>>=8n; } }

// 28 gate challenges
for (let i = 0; i < 28; i++) {
    prevH = keccak256(prevBytes);
    prev = modP(BigInt('0x' + Buffer.from(prevH).toString('hex')));
    { let tmp = prev; for(let i=31;i>=0;i--) { prevBytes[i]=Number(tmp&0xffn); tmp>>=8n; } }
}

// Sumcheck challenges
const sumcheckChallenges: bigint[] = [];
for (let i = 0; i < 28; i++) {
    const uc = new Uint8Array(9 * 32);
    uc.set(prevBytes, 0);
    for (let j = 0; j < 8; j++) {
        const val = proofData.slice(0x400 + i*8*32 + j*32, 0x400 + i*8*32 + j*32 + 32);
        uc.set(val, 32 + j*32);
    }
    prevH = keccak256(uc);
    prev = modP(BigInt('0x' + Buffer.from(prevH).toString('hex')));
    { let tmp = prev; for(let k=31;k>=0;k--) { prevBytes[k]=Number(tmp&0xffn); tmp>>=8n; } }
    const [sc] = splitChallenge(prevBytes);
    sumcheckChallenges.push(sc);
}

console.log('sumcheck_u_challenges[0] =', sumcheckChallenges[0].toString(16));

// Compute barycentric interpolation at challenge[0]
const challenge = sumcheckChallenges[0];
const denoms = [P-5040n, 720n, P-240n, 144n, P-144n, 240n, P-720n, 5040n];
const u0 = Array.from({length:8}, (_,j) => readFrBe(0x400 + j*32));

let numerator_value = 1n;
for (let i = 0; i < 8; i++) {
    numerator_value = frMul(numerator_value, frSub(challenge, BigInt(i)));
}

const denom_invs: bigint[] = [];
for (let i = 0; i < 8; i++) {
    const d = frMul(denoms[i], frSub(challenge, BigInt(i)));
    denom_invs.push(d === 0n ? 0n : frInv(d));
}

let target_sum = 0n;
for (let i = 0; i < 8; i++) {
    target_sum = frAdd(target_sum, frMul(u0[i], denom_invs[i]));
}
const computed_target = frMul(target_sum, numerator_value);

const u1_0 = readFrBe(0x500);
const u1_1 = readFrBe(0x520);
const u1_sum = frAdd(u1_0, u1_1);

console.log('\ncomputed_target (round 0 → round 1) =', computed_target.toString(16));
console.log('u1[0] + u1[1] =', u1_sum.toString(16));
console.log('Match?', computed_target === u1_sum);
