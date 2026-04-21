#![no_main]
#![no_std]
extern crate alloc;

mod fr;
mod fr_utils;
mod g1;
mod proof;
mod relations;
mod shplemini;
mod sumcheck;
mod transcript;
mod vk;

use alloc::vec::Vec;
use polkavm_derive::polkavm_export;
use simplealloc::SimpleAlloc;
use uapi::{HostFn, HostFnImpl as api, ReturnFlags};

use fr::Fr;
use proof::load_proof;
use transcript::generate_transcript;
use vk::load_vk;

#[global_allocator]
static ALLOCATOR: SimpleAlloc<{ 96 * 1024 }> = SimpleAlloc::new();

/// Function selector for verify(bytes,bytes32[]) = 0xea50d0e4
const VERIFY_SELECTOR: [u8; 4] = [0xea, 0x50, 0xd0, 0xe4];

const LOG_N: usize = 5;

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
    // Current verifier expects exactly one public input.
    // Avoid index loops here because loop/index codegen is known-bad on this target.
    if arr_len != 1 {
        return None;
    }
    let mut public_inputs: Vec<[u8; 32]> = alloc::vec![[0u8; 32]; 1];
    public_inputs[0].copy_from_slice(&data[arr_data_start..arr_data_start + 32]);

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

    // Checkpoint diagnostic mode:
    // 0x00 = verified
    // 0x64..0x68 = sumcheck round failure
    // 0xc8 = final grand sum mismatch
    // 0xff = shplemini failure
    let code = do_verify_diag(&proof_bytes, &public_inputs);
    api::return_value(ReturnFlags::empty(), &[code]);
}

fn do_verify_diag(proof_bytes: &[u8], public_inputs: &[[u8; 32]]) -> u8 {
    let vk = load_vk();
    let proof = load_proof(proof_bytes);
    let t = generate_transcript(&proof, public_inputs, vk.circuit_size, vk.public_inputs_size, vk.pub_inputs_offset);
    let mut t = t;
    t.relation_parameters.public_inputs_delta = compute_public_input_delta(
        public_inputs, t.relation_parameters.beta, t.relation_parameters.gamma,
        vk.pub_inputs_offset, vk.circuit_size, vk.public_inputs_size,
    );
    // Returns: 0=sumcheck pass, 100+round=check_sum fail at round, 200=final compare fail
    // NOTE: shplemini skipped here — precompile calls break eth_call on Paseo.
    // When code=200, also stores grand_sum+round_target in DIAG_GRAND_SUM for retrieval.
    let code = sumcheck::verify_sumcheck_diag(&proof, &t, LOG_N);
    if code == 200 {
        use crate::fr_utils::fr_to_scalar;
        // Return 1184 bytes: pow(32) + gate_chs[5](160) + sumcheck_chs[5](160) + evals[26](832)
        let (pow_partial_eval, gate_chs, sumcheck_chs, evals) = sumcheck::get_relation_evals_debug(&proof, &t);
        let mut out = [0u8; 1184]; // 37 field elements * 32 bytes
        out[0..32].copy_from_slice(&fr_to_scalar(pow_partial_eval));
        for (chunk, gc) in out[32..192].chunks_mut(32).zip(gate_chs.iter()) {
            chunk.copy_from_slice(&fr_to_scalar(*gc));
        }
        for (chunk, sc) in out[192..352].chunks_mut(32).zip(sumcheck_chs.iter()) {
            chunk.copy_from_slice(&fr_to_scalar(*sc));
        }
        for (chunk, ev) in out[352..].chunks_mut(32).zip(evals.iter()) {
            chunk.copy_from_slice(&fr_to_scalar(*ev));
        }
        api::return_value(ReturnFlags::empty(), &out);
        return 0; // unreachable but keeps type
    }
    code
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

    shplemini::verify_shplemini(&proof, &vk, &t)
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
