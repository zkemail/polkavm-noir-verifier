import * as fs from 'fs';
import * as path from 'path';

// --- CLI args ---
function parseArgs(): { sol: string; out: string; build: boolean } {
  const args = process.argv.slice(2);
  let sol = '', out = '', build = false;
  for (let i = 0; i < args.length; i++) {
    if (args[i] === '--sol' && args[i + 1]) sol = args[++i];
    else if (args[i] === '--out' && args[i + 1]) out = args[++i];
    else if (args[i] === '--build') build = true;
  }
  if (!sol || !out) {
    console.error('Usage: ts-node generate_verifier.ts --sol <HonkVerifier.sol> --out <output-dir> [--build]');
    process.exit(1);
  }
  return { sol, out, build };
}

// --- Solidity parser ---
interface G1Point { x: string; y: string }
interface ParsedSol {
  N: number;
  LOG_N: number;
  NUMBER_OF_PUBLIC_INPUTS: number;
  points: Map<string, G1Point>; // Solidity field name -> point
}

function parseSolidity(src: string): ParsedSol {
  const constMatch = (name: string): number => {
    const m = src.match(new RegExp(`uint256\\s+constant\\s+${name}\\s*=\\s*(\\d+)`));
    if (!m) throw new Error(`Cannot find constant ${name} in HonkVerifier.sol`);
    return parseInt(m[1], 10);
  };

  const N = constMatch('N');
  const LOG_N = constMatch('LOG_N');
  const NUMBER_OF_PUBLIC_INPUTS = constMatch('NUMBER_OF_PUBLIC_INPUTS');

  // Extract G1 points from loadVerificationKey()
  // Pattern: fieldName: Honk.G1Point({\s*x: uint256(0x...), \s*y: uint256(0x...)
  const pointRegex = /(\w+):\s*Honk\.G1Point\(\{\s*x:\s*uint256\(0x([0-9a-fA-F]+)\),\s*y:\s*uint256\(0x([0-9a-fA-F]+)\)\s*\}\)/g;
  const points = new Map<string, G1Point>();
  let m;
  while ((m = pointRegex.exec(src)) !== null) {
    const name = m[1];
    // Pad hex to 64 chars (left-pad with zeros)
    const x = m[2].padStart(64, '0');
    const y = m[3].padStart(64, '0');
    points.set(name, { x, y });
  }

  // We expect 27 points
  const expectedPoints = [
    'ql', 'qr', 'qo', 'q4', 'qm', 'qc', 'qArith', 'qDeltaRange', 'qElliptic',
    'qAux', 'qLookup', 'qPoseidon2External', 'qPoseidon2Internal',
    's1', 's2', 's3', 's4', 't1', 't2', 't3', 't4',
    'id1', 'id2', 'id3', 'id4', 'lagrangeFirst', 'lagrangeLast',
  ];
  for (const p of expectedPoints) {
    if (!points.has(p)) throw new Error(`Missing G1 point: ${p}`);
  }

  return { N, LOG_N, NUMBER_OF_PUBLIC_INPUTS, points };
}

// --- Solidity -> Rust field name mapping ---
const SOL_TO_RUST: Record<string, string> = {
  ql: 'ql', qr: 'qr', qo: 'qo', q4: 'q4', qm: 'qm', qc: 'qc',
  qArith: 'q_arith', qDeltaRange: 'q_delta_range', qElliptic: 'q_elliptic',
  qAux: 'q_aux', qLookup: 'q_lookup',
  qPoseidon2External: 'q_poseidon2_external', qPoseidon2Internal: 'q_poseidon2_internal',
  s1: 's1', s2: 's2', s3: 's3', s4: 's4',
  t1: 't1', t2: 't2', t3: 't3', t4: 't4',
  id1: 'id1', id2: 'id2', id3: 'id3', id4: 'id4',
  lagrangeFirst: 'lagrange_first', lagrangeLast: 'lagrange_last',
};

// Order of fields in the VK struct (matches existing vk.rs load_vk() assignment order)
const VK_FIELD_ORDER = [
  'ql', 'qr', 'qo', 'q4', 'qm', 'qc', 'q_arith', 'q_delta_range', 'q_elliptic',
  'q_aux', 'q_lookup', 'q_poseidon2_external', 'q_poseidon2_internal',
  's1', 's2', 's3', 's4', 't1', 't2', 't3', 't4',
  'id1', 'id2', 'id3', 'id4', 'lagrange_first', 'lagrange_last',
];

// Reverse map: Rust name -> Solidity name
const RUST_TO_SOL: Record<string, string> = {};
for (const [sol, rust] of Object.entries(SOL_TO_RUST)) {
  RUST_TO_SOL[rust] = sol;
}

// --- Generate vk.rs ---
function generateVkRs(parsed: ParsedSol): string {
  const pointAssignments = VK_FIELD_ORDER.map(rustName => {
    const solName = RUST_TO_SOL[rustName];
    const pt = parsed.points.get(solName)!;
    return `    vk.${rustName} = g1(\n        "${pt.x}",\n        "${pt.y}",\n    );`;
  }).join('\n');

  return `extern crate alloc;
use alloc::boxed::Box;
use alloc::alloc::{alloc_zeroed, Layout};
use crate::honk::g1::G1Point;

/// G2 point in EVM pairing precompile format: (x_im, x_re, y_im, y_re) each 32 bytes.
pub type G2Point = [u8; 128];

pub struct VerificationKey {
    pub circuit_size: u64,
    pub log_circuit_size: u64,
    pub public_inputs_size: u64,
    pub pub_inputs_offset: u64,
    pub qm: G1Point,
    pub qc: G1Point,
    pub ql: G1Point,
    pub qr: G1Point,
    pub qo: G1Point,
    pub q4: G1Point,
    pub q_lookup: G1Point,
    pub q_arith: G1Point,
    pub q_delta_range: G1Point,
    pub q_elliptic: G1Point,
    pub q_aux: G1Point,
    pub q_poseidon2_external: G1Point,
    pub q_poseidon2_internal: G1Point,
    pub s1: G1Point,
    pub s2: G1Point,
    pub s3: G1Point,
    pub s4: G1Point,
    pub id1: G1Point,
    pub id2: G1Point,
    pub id3: G1Point,
    pub id4: G1Point,
    pub t1: G1Point,
    pub t2: G1Point,
    pub t3: G1Point,
    pub t4: G1Point,
    pub lagrange_first: G1Point,
    pub lagrange_last: G1Point,
}

fn hex32(s: &str) -> [u8; 32] {
    let b = s.as_bytes();
    let len = b.len();
    let mut out = [0u8; 32];
    if len == 64 {
        for i in 0..32 {
            out[i] = (nibble(b[i * 2]) << 4) | nibble(b[i * 2 + 1]);
        }
    } else {
        // left-pad with zeros
        let pad = 64 - len;
        // first \`pad\` nibbles are zero
        let pad_bytes = pad / 2; // full zero bytes from padding
        // remaining bytes come from the string
        for i in 0..32 {
            let nibble_idx = i * 2; // index in the full 64-nibble sequence
            let hi = if nibble_idx < pad { 0 } else { nibble(b[nibble_idx - pad]) };
            let lo_idx = nibble_idx + 1;
            let lo = if lo_idx < pad { 0 } else { nibble(b[lo_idx - pad]) };
            out[i] = (hi << 4) | lo;
            let _ = pad_bytes;
        }
    }
    out
}

fn nibble(b: u8) -> u8 {
    match b {
        b'0'..=b'9' => b - b'0',
        b'a'..=b'f' => b - b'a' + 10,
        b'A'..=b'F' => b - b'A' + 10,
        _ => 0,
    }
}

fn g1(x: &str, y: &str) -> G1Point {
    G1Point { x: hex32(x), y: hex32(y) }
}

pub fn load_vk() -> Box<VerificationKey> {
    // Allocate zeroed VK directly on the heap to avoid ~1.8KB stack allocation in do_verify.
    let layout = Layout::new::<VerificationKey>();
    let ptr = unsafe { alloc_zeroed(layout) as *mut VerificationKey };
    assert!(!ptr.is_null());
    let vk = unsafe { &mut *ptr };
    vk.circuit_size = ${parsed.N};
    vk.log_circuit_size = ${parsed.LOG_N};
    vk.public_inputs_size = ${parsed.NUMBER_OF_PUBLIC_INPUTS};
    vk.pub_inputs_offset = 1;
${pointAssignments}
    unsafe { Box::from_raw(ptr) }
}

/// G2 generator for KZG pairing check (EVM format: x_im, x_re, y_im, y_re).
pub fn g2_generator() -> G2Point {
    let mut g2 = [0u8; 128];
    // x_im
    g2[0..32].copy_from_slice(&hex32("198e9393920d483a7260bfb731fb5d25f1aa493335a9e71297e485b7aef312c2"));
    // x_re
    g2[32..64].copy_from_slice(&hex32("1800deef121f1e76426a00665e5c4479674322d4f75edadd46debd5cd992f6ed"));
    // y_im
    g2[64..96].copy_from_slice(&hex32("090689d0585ff075ec9e99ad690c3395bc4b313370b38ef355acdadcd122975b"));
    // y_re
    g2[96..128].copy_from_slice(&hex32("12c85ea5db8c6deb4aab71808dcb408fe3d1e7690c43d37b4ce6cc0166fa7daa"));
    g2
}

/// KZG SRS G2 point (EVM format).
pub fn g2_kzg_srs() -> G2Point {
    let mut g2 = [0u8; 128];
    // x_im
    g2[0..32].copy_from_slice(&hex32("260e01b251f6f1c7e7ff4e580791dee8ea51d87a358e038b4efe30fac09383c1"));
    // x_re
    g2[32..64].copy_from_slice(&hex32("0118c4d5b837bcc2bc89b5b398b5974e9f5944073b32078b7e231fec938883b0"));
    // y_im
    g2[64..96].copy_from_slice(&hex32("04fc6369f7110fe3d25156c1bb9a72859cf2a04641f99ba4ee413c80da6a5fe4"));
    // y_re
    g2[96..128].copy_from_slice(&hex32("22febda3c0c0632a56475b4214e5615e11e6dd3f96e6cea2854a87d4dacc5e55"));
    g2
}
`;
}

// --- Calculate required heap size ---
// SimpleAlloc is a bump allocator (never frees). We must account for ALL allocations
// across the entire verify call: calldata copy, proof parsing, transcript generation
// (multiple Vecs that are never freed), and shplemini.
function calculateHeapKB(numPublicInputs: number): number {
  const CONST_PROOF_SIZE_LOG_N = 28;
  const NUMBER_OF_ENTITIES = 40;

  // calldata copy
  const dataVec = 64 + 32 + 14080 + 32 + 32 + numPublicInputs * 32;
  const proofBytesCopy = 14080;
  const pubInputsVec = numPublicInputs * 32;

  // VK + Proof + Transcript structs
  const structs = 1900 + 14080 + 3000;

  // Transcript Vec allocations (bump-allocated, never freed)
  const round0 = (3 + numPublicInputs + 12) * 32;
  const hashBufs = round0 + 13*32 + 9*32 + 3*32 + 5*32; // round0,1,alpha,singles,z
  const ucLoop = CONST_PROOF_SIZE_LOG_N * 9 * 32; // Vec per sumcheck round
  const rho = (NUMBER_OF_ENTITIES + 1) * 32 * 2; // vec + hash buf
  const gr = ((CONST_PROOF_SIZE_LOG_N - 1) * 4 + 1) * 32 * 2;
  const nu = (CONST_PROOF_SIZE_LOG_N + 1) * 32 * 2;

  // Shplemini
  const shplemini = (NUMBER_OF_ENTITIES + CONST_PROOF_SIZE_LOG_N + 2) * (32 + 64) + CONST_PROOF_SIZE_LOG_N * 32 * 2;

  const total = dataVec + proofBytesCopy + pubInputsVec + structs +
                hashBufs + ucLoop + rho + gr + nu + shplemini;

  // Add 25% margin, round up to nearest 4KB
  const withMargin = Math.ceil(total * 1.25);
  return Math.ceil(withMargin / 4096) * 4;
}

// --- Generate contract.rs ---
function generateContractRs(parsed: ParsedSol): string {
  const numPub = parsed.NUMBER_OF_PUBLIC_INPUTS;

  // Generate the public input parsing: unrolled assignments for each public input
  let pubInputParsing: string;
  if (numPub === 1) {
    pubInputParsing = `    let mut public_inputs: Vec<[u8; 32]> = alloc::vec![[0u8; 32]; 1];
    public_inputs[0].copy_from_slice(&data[arr_data_start..arr_data_start + 32]);`;
  } else {
    const lines = [`    let mut public_inputs: Vec<[u8; 32]> = alloc::vec![[0u8; 32]; ${numPub}];`];
    for (let i = 0; i < numPub; i++) {
      lines.push(`    public_inputs[${i}].copy_from_slice(&data[arr_data_start + ${i * 32}..arr_data_start + ${(i + 1) * 32}]);`);
    }
    pubInputParsing = lines.join('\n');
  }

  const heapKB = calculateHeapKB(numPub);

  return `#![no_main]
#![no_std]
extern crate alloc;

mod honk;
mod sumcheck;
mod vk;

use alloc::vec::Vec;
use polkavm_derive::polkavm_export;
use simplealloc::SimpleAlloc;
use uapi::{HostFn, HostFnImpl as api, ReturnFlags};

use honk::fr::Fr;
use honk::proof::load_proof;
use honk::transcript::generate_transcript;
use vk::load_vk;

#[global_allocator]
static ALLOCATOR: SimpleAlloc<{ ${heapKB} * 1024 }> = SimpleAlloc::new(); // ${heapKB}KB heap for ${numPub} public inputs

/// Function selector for verify(bytes,bytes32[]) = 0xea50d0e4
const VERIFY_SELECTOR: [u8; 4] = [0xea, 0x50, 0xd0, 0xe4];

const LOG_N: usize = ${parsed.LOG_N};

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    unsafe {
        core::arch::asm!("unimp");
        core::hint::unreachable_unchecked();
    }
}

#[no_mangle]
#[polkavm_export]
pub extern "C" fn deploy() {}

#[no_mangle]
#[polkavm_export]
pub extern "C" fn call() {
    let length = api::call_data_size() as usize;
    if length < 4 {
        api::return_value(ReturnFlags::REVERT, b"INPUT_TOO_SHORT");
        return;
    }

    let mut selector = [0u8; 4];
    api::call_data_copy(&mut selector, 0);

    match selector {
        VERIFY_SELECTOR => handle_verify(length),
        _ => api::return_value(ReturnFlags::REVERT, b"UNKNOWN_FUNCTION"),
    }
}

/// Read a uint256 from 32 bytes big-endian as a usize.
/// Returns None if the value is too large to be a reasonable offset/length.
fn read_u256_as_usize(bytes: &[u8]) -> Option<usize> {
    // High 24 bytes must be zero for a sane value
    for b in bytes[0..24].iter() {
        if *b != 0 {
            return None;
        }
    }
    let mut val = [0u8; 8];
    val.copy_from_slice(&bytes[24..32]);
    Some(u64::from_be_bytes(val) as usize)
}

/// Manual ABI decode of verify(bytes proof, bytes32[] publicInputs).
///
/// ABI layout after the 4-byte selector (data = calldata[4..]):
///   [0x00..0x20]  offset_to_proof   (should be 0x40 = 64)
///   [0x20..0x40]  offset_to_array
///   [offset_to_proof..+32]  proof_length
///   [offset_to_proof+32..+proof_length]  proof bytes
///   [offset_to_array..+32]  array_length
///   [offset_to_array+32..]  array_length × 32-byte elements
fn parse_verify_args(data: &[u8]) -> Option<(Vec<u8>, Vec<[u8; 32]>)> {
    if data.len() < 64 {
        return None;
    }
    let bytes_offset = read_u256_as_usize(&data[0..32])?;
    let arr_offset = read_u256_as_usize(&data[32..64])?;

    // Parse proof bytes
    if data.len() < bytes_offset.checked_add(32)? {
        return None;
    }
    let bytes_len = read_u256_as_usize(&data[bytes_offset..bytes_offset + 32])?;
    let proof_start = bytes_offset.checked_add(32)?;
    let proof_end = proof_start.checked_add(bytes_len)?;
    if data.len() < proof_end {
        return None;
    }
    let proof_bytes = data[proof_start..proof_end].to_vec();

    // Parse bytes32[] array
    if data.len() < arr_offset.checked_add(32)? {
        return None;
    }
    let arr_len = read_u256_as_usize(&data[arr_offset..arr_offset + 32])?;
    let arr_data_start = arr_offset.checked_add(32)?;
    let arr_data_end = arr_data_start.checked_add(arr_len.checked_mul(32)?)?;
    if data.len() < arr_data_end {
        return None;
    }
    // Verifier expects exactly ${numPub} public input(s).
    if arr_len != ${numPub} {
        return None;
    }
${pubInputParsing}

    Some((proof_bytes, public_inputs))
}

fn handle_verify(length: usize) {
    let data_len = length.saturating_sub(4);
    let mut data = alloc::vec![0u8; data_len];
    if data_len > 0 {
        api::call_data_copy(&mut data, 4);
    }

    let (proof_bytes, public_inputs) = match parse_verify_args(&data) {
        Some(x) => x,
        None => {
            api::return_value(ReturnFlags::REVERT, b"ABI_DECODE_FAILED");
            return;
        }
    };

    let ok = do_verify(&proof_bytes, &public_inputs);
    if ok {
        api::return_value(ReturnFlags::empty(), &[1u8]); // 0x01 = verified
    } else {
        api::return_value(ReturnFlags::empty(), &[0u8]); // 0x00 = failed
    }
}

fn do_verify(proof_bytes: &[u8], public_inputs: &[[u8; 32]]) -> bool {
    // All large structs boxed to keep do_verify's stack frame minimal (~50 bytes of pointers).
    let vk = load_vk();           // Box<VerificationKey> ~1.8KB on heap
    let proof = load_proof(proof_bytes); // Box<Proof> ~14KB on heap

    let t = generate_transcript(
        &proof,
        public_inputs,
        vk.circuit_size,
        vk.public_inputs_size,
        vk.pub_inputs_offset,
    );                            // Box<Transcript> ~2.9KB on heap

    let mut t = t;
    t.relation_parameters.public_inputs_delta = compute_public_input_delta(
        public_inputs,
        t.relation_parameters.beta,
        t.relation_parameters.gamma,
        vk.pub_inputs_offset,
        vk.circuit_size,
        vk.public_inputs_size,
    );

    if !sumcheck::verify_sumcheck(&proof, &t, LOG_N) {
        return false;
    }

    honk::shplemini::verify_shplemini(&proof, &vk, &t)
}

fn compute_public_input_delta(
    public_inputs: &[[u8; 32]],
    beta: Fr,
    gamma: Fr,
    offset: u64,
    n: u64,
    num_public_inputs: u64,
) -> Fr {
    let mut numerator = Fr::one();
    let mut denominator = Fr::one();

    let mut numerator_acc = gamma + beta * Fr::from_u64(n + offset);
    let mut denominator_acc = gamma - beta * Fr::from_u64(offset + 1);

    for pi in public_inputs.iter() {
        let pub_input = Fr::from_be_bytes(pi);
        numerator = numerator * (numerator_acc + pub_input);
        denominator = denominator * (denominator_acc + pub_input);
        numerator_acc = numerator_acc + beta;
        denominator_acc = denominator_acc - beta;
    }

    numerator * denominator.inverse().unwrap()
}
`;
}

// --- Generate sumcheck.rs ---
function generateSumcheckRs(parsed: ParsedSol): string {
  const logN = parsed.LOG_N;

  function generateRounds(includeCheckSum: boolean, returnOnFail: boolean): string {
    const rounds: string[] = [];
    for (let i = 0; i < logN; i++) {
      if (includeCheckSum) {
        rounds.push(`    // Round ${i}
    {
        let u = &proof.sumcheck_univariates[${i}];
        if !check_sum(u, round_target) { return ${100 + i}; }
        let ch = t.sumcheck_u_challenges[${i}];
        round_target = compute_next_target_sum(u, ch);
        pow_partial_evaluation = partially_evaluate_pow(t.gate_challenges[${i}], pow_partial_evaluation, ch);
    }`);
      } else {
        rounds.push(`    // Round ${i}
    {
        let u = &proof.sumcheck_univariates[${i}];
        let ch = t.sumcheck_u_challenges[${i}];
        round_target = compute_next_target_sum(u, ch);
        pow_partial_evaluation = partially_evaluate_pow(t.gate_challenges[${i}], pow_partial_evaluation, ch);
    }`);
      }
    }
    return rounds.join('\n');
  }

  function generatePowRounds(): string {
    const rounds: string[] = [];
    for (let i = 0; i < logN; i++) {
      rounds.push(`    // Round ${i}
    {
        let ch = t.sumcheck_u_challenges[${i}];
        pow_partial_evaluation = partially_evaluate_pow(t.gate_challenges[${i}], pow_partial_evaluation, ch);
    }`);
    }
    return rounds.join('\n');
  }

  function generateGateChallengesArray(): string {
    const entries = [];
    for (let i = 0; i < logN; i++) {
      entries.push(`        t.gate_challenges[${i}],`);
    }
    return entries.join('\n');
  }

  function generateSumcheckChallengesArray(): string {
    const entries = [];
    for (let i = 0; i < logN; i++) {
      entries.push(`        t.sumcheck_u_challenges[${i}],`);
    }
    return entries.join('\n');
  }

  return `use crate::honk::fr::Fr;
use crate::honk::proof::{Proof, BATCHED_RELATION_PARTIAL_LENGTH};
use crate::honk::relations::{accumulate_relation_evaluations, accumulate_relation_evaluations_raw, NUMBER_OF_SUBRELATIONS};
use crate::honk::transcript::Transcript;

/// Barycentric Lagrange denominators (from Solidity BARYCENTRIC_LAGRANGE_DENOMINATORS)
fn barycentric_lagrange_denominators() -> [Fr; BATCHED_RELATION_PARTIAL_LENGTH] {
    [
        fr_from_hex("30644e72e131a029b85045b68181585d2833e84879b9709143e1f593efffec51"),
        fr_from_hex("00000000000000000000000000000000000000000000000000000000000002d0"),
        fr_from_hex("30644e72e131a029b85045b68181585d2833e84879b9709143e1f593efffff11"),
        fr_from_hex("0000000000000000000000000000000000000000000000000000000000000090"),
        fr_from_hex("30644e72e131a029b85045b68181585d2833e84879b9709143e1f593efffff71"),
        fr_from_hex("00000000000000000000000000000000000000000000000000000000000000f0"),
        fr_from_hex("30644e72e131a029b85045b68181585d2833e84879b9709143e1f593effffd31"),
        fr_from_hex("00000000000000000000000000000000000000000000000000000000000013b0"),
    ]
}

fn fr_from_hex(hex: &str) -> Fr {
    let hex = if hex.len() < 64 {
        // left-pad with zeros
        let pad = 64 - hex.len();
        let mut s = [b'0'; 64];
        let src = hex.as_bytes();
        s[pad..].copy_from_slice(src);
        // Safety: we filled it with ASCII digits
        let mut bytes = [0u8; 32];
        for (byte, pair) in bytes.iter_mut().zip(s.chunks(2)) {
            *byte = nibble(pair[0]) << 4 | nibble(pair[1]);
        }
        return Fr::from_be_bytes(&bytes);
    } else {
        hex
    };
    let b = hex.as_bytes();
    let mut bytes = [0u8; 32];
    for (byte, pair) in bytes.iter_mut().zip(b.chunks(2)) {
        *byte = nibble(pair[0]) << 4 | nibble(pair[1]);
    }
    Fr::from_be_bytes(&bytes)
}

fn nibble(b: u8) -> u8 {
    match b {
        b'0'..=b'9' => b - b'0',
        b'a'..=b'f' => b - b'a' + 10,
        b'A'..=b'F' => b - b'A' + 10,
        _ => 0,
    }
}

fn check_sum(round_univariate: &[Fr; BATCHED_RELATION_PARTIAL_LENGTH], round_target: Fr) -> bool {
    round_univariate[0] + round_univariate[1] == round_target
}

fn compute_next_target_sum(
    u: &[Fr; BATCHED_RELATION_PARTIAL_LENGTH],
    challenge: Fr,
) -> Fr {
    let d = barycentric_lagrange_denominators();
    let c0 = challenge;
    let c1 = challenge - Fr::from_u64(1);
    let c2 = challenge - Fr::from_u64(2);
    let c3 = challenge - Fr::from_u64(3);
    let c4 = challenge - Fr::from_u64(4);
    let c5 = challenge - Fr::from_u64(5);
    let c6 = challenge - Fr::from_u64(6);
    let c7 = challenge - Fr::from_u64(7);

    let numerator = c0 * c1 * c2 * c3 * c4 * c5 * c6 * c7;

    let i0 = (d[0] * c0).inverse().unwrap_or(Fr::zero());
    let i1 = (d[1] * c1).inverse().unwrap_or(Fr::zero());
    let i2 = (d[2] * c2).inverse().unwrap_or(Fr::zero());
    let i3 = (d[3] * c3).inverse().unwrap_or(Fr::zero());
    let i4 = (d[4] * c4).inverse().unwrap_or(Fr::zero());
    let i5 = (d[5] * c5).inverse().unwrap_or(Fr::zero());
    let i6 = (d[6] * c6).inverse().unwrap_or(Fr::zero());
    let i7 = (d[7] * c7).inverse().unwrap_or(Fr::zero());

    let target = u[0] * i0
        + u[1] * i1
        + u[2] * i2
        + u[3] * i3
        + u[4] * i4
        + u[5] * i5
        + u[6] * i6
        + u[7] * i7;
    target * numerator
}

fn partially_evaluate_pow(gate_challenge: Fr, current_evaluation: Fr, round_challenge: Fr) -> Fr {
    let univariate_eval = Fr::one() + round_challenge * (gate_challenge - Fr::one());
    current_evaluation * univariate_eval
}

/// Returns: 0 = success, 100+round = check_sum failed at that round, 200 = final compare failed.
pub fn get_denom0_pub() -> Fr {
    barycentric_lagrange_denominators()[0]
}

pub fn compute_round0_target_pub(
    round_univariates: &[Fr; BATCHED_RELATION_PARTIAL_LENGTH],
    challenge: Fr,
) -> Fr {
    compute_next_target_sum(round_univariates, challenge)
}

pub fn verify_sumcheck_diag(proof: &Proof, t: &Transcript, _log_n: usize) -> u8 {
    let mut round_target = Fr::zero();
    let mut pow_partial_evaluation = Fr::one();

${generateRounds(true, true)}

    let grand_honk_relation_sum = accumulate_relation_evaluations(
        &proof.sumcheck_evaluations,
        &t.relation_parameters,
        &t.alphas,
        pow_partial_evaluation,
    );

    if grand_honk_relation_sum != round_target { 200 } else { 0 }
}

/// Returns (grand_honk_relation_sum, round_target) as two Fr values for debugging.
pub fn get_grand_sum_debug(proof: &Proof, t: &Transcript, _log_n: usize) -> (Fr, Fr) {
    let mut round_target = Fr::zero();
    let mut pow_partial_evaluation = Fr::one();

${generateRounds(false, false)}

    let grand_sum = accumulate_relation_evaluations(
        &proof.sumcheck_evaluations,
        &t.relation_parameters,
        &t.alphas,
        pow_partial_evaluation,
    );

    (grand_sum, round_target)
}

/// Returns (pow_partial_evaluation, gate_challenges[0..${logN}], sumcheck_u_challenges[0..${logN}], raw 26 sub-relation evaluations).
pub fn get_relation_evals_debug(proof: &Proof, t: &Transcript) -> (Fr, [Fr; ${logN}], [Fr; ${logN}], [Fr; NUMBER_OF_SUBRELATIONS]) {
    let mut pow_partial_evaluation = Fr::one();

${generatePowRounds()}

    let gate_chs = [
${generateGateChallengesArray()}
    ];
    let sumcheck_chs = [
${generateSumcheckChallengesArray()}
    ];
    let evals = accumulate_relation_evaluations_raw(
        &proof.sumcheck_evaluations,
        &t.relation_parameters,
        pow_partial_evaluation,
    );
    (pow_partial_evaluation, gate_chs, sumcheck_chs, evals)
}

pub fn verify_sumcheck(proof: &Proof, t: &Transcript, log_n: usize) -> bool {
    verify_sumcheck_diag(proof, t, log_n) == 0
}
`;
}

// --- Copy template directory ---
function copyDir(src: string, dest: string) {
  fs.mkdirSync(dest, { recursive: true });
  for (const entry of fs.readdirSync(src, { withFileTypes: true })) {
    const srcPath = path.join(src, entry.name);
    const destPath = path.join(dest, entry.name);
    if (entry.isDirectory()) {
      copyDir(srcPath, destPath);
    } else {
      fs.copyFileSync(srcPath, destPath);
    }
  }
}

// --- Main ---
function main() {
  const { sol, out, build } = parseArgs();
  const solPath = path.resolve(sol);
  const outDir = path.resolve(out);

  console.log(`Reading ${solPath}...`);
  const solSrc = fs.readFileSync(solPath, 'utf8');
  const parsed = parseSolidity(solSrc);
  console.log(`  N=${parsed.N}, LOG_N=${parsed.LOG_N}, PUBLIC_INPUTS=${parsed.NUMBER_OF_PUBLIC_INPUTS}`);
  console.log(`  Found ${parsed.points.size} G1 points`);

  // Copy template
  const templateDir = path.join(__dirname, 'template');
  console.log(`Copying template to ${outDir}...`);
  copyDir(templateDir, outDir);

  // Generate circuit-specific files
  const srcDir = path.join(outDir, 'src');
  fs.mkdirSync(srcDir, { recursive: true });

  console.log('Generating vk.rs...');
  fs.writeFileSync(path.join(srcDir, 'vk.rs'), generateVkRs(parsed));

  console.log('Generating main.rs...');
  fs.writeFileSync(path.join(srcDir, 'main.rs'), generateContractRs(parsed));

  console.log('Generating sumcheck.rs...');
  fs.writeFileSync(path.join(srcDir, 'sumcheck.rs'), generateSumcheckRs(parsed));

  console.log(`\nDone! Output directory: ${outDir}`);
  console.log(`  Generic files: 7 Rust sources + configs + TS scripts`);
  console.log(`  Generated files: main.rs, vk.rs, sumcheck.rs`);

  if (build) {
    console.log('\nBuilding...');
    const { execSync } = require('child_process');
    try {
      execSync('cargo build --release', { cwd: outDir, stdio: 'inherit' });
      const binName = 'honk_verifier';
      execSync(
        `polkatool link --strip --min-stack-size 65536 --output ${binName}.polkavm target/riscv64emac-unknown-none-polkavm/release/${binName}.elf`,
        { cwd: outDir, stdio: 'inherit' }
      );
      console.log('Build complete!');
    } catch (e) {
      console.error('Build failed:', e);
      process.exit(1);
    }
  }
}

main();
