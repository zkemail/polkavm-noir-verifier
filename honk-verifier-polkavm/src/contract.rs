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
use ethabi::{decode, ParamType};
use polkavm_derive::polkavm_export;
use simplealloc::SimpleAlloc;
use uapi::{HostFn, HostFnImpl as api, ReturnFlags};

use fr::Fr;
use proof::load_proof;
use transcript::generate_transcript;
use vk::load_vk;

#[global_allocator]
static ALLOCATOR: SimpleAlloc<{ 512 * 1024 }> = SimpleAlloc::new();

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

fn handle_verify(length: usize) {
    // Read full calldata (selector + ABI-encoded args)
    let data_len = length.saturating_sub(4);
    let mut data = alloc::vec![0u8; data_len];
    if data_len > 0 {
        api::call_data_copy(&mut data, 4);
    }

    // Decode ABI: verify(bytes proof, bytes32[] publicInputs)
    let decoded = match decode(&[ParamType::Bytes, ParamType::Array(alloc::boxed::Box::new(ParamType::FixedBytes(32)))], &data) {
        Ok(d) => d,
        Err(_) => {
            api::return_value(ReturnFlags::REVERT, b"ABI_DECODE_FAILED");
            return;
        }
    };

    let proof_bytes = match &decoded[0] {
        ethabi::Token::Bytes(b) => b.clone(),
        _ => {
            api::return_value(ReturnFlags::REVERT, b"INVALID_PROOF_ARG");
            return;
        }
    };

    let public_inputs: Vec<[u8; 32]> = match &decoded[1] {
        ethabi::Token::Array(arr) => {
            arr.iter()
                .filter_map(|t| {
                    if let ethabi::Token::FixedBytes(fb) = t {
                        if fb.len() == 32 {
                            let mut b = [0u8; 32];
                            b.copy_from_slice(fb);
                            Some(b)
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                })
                .collect()
        }
        _ => {
            api::return_value(ReturnFlags::REVERT, b"INVALID_PI_ARG");
            return;
        }
    };

    let result = do_verify(&proof_bytes, &public_inputs);

    // Return ABI-encoded bool
    let mut ret = [0u8; 32];
    if result {
        ret[31] = 1;
    }
    api::return_value(ReturnFlags::empty(), &ret);
}

fn do_verify(proof_bytes: &[u8], public_inputs: &[[u8; 32]]) -> bool {
    let vk = load_vk();
    let proof = load_proof(proof_bytes);

    let mut t = generate_transcript(
        &proof,
        public_inputs,
        vk.circuit_size,
        vk.public_inputs_size,
        vk.pub_inputs_offset,
    );

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

    for i in 0..num_public_inputs as usize {
        let pub_input = Fr::from_be_bytes(&public_inputs[i]);
        numerator = numerator * (numerator_acc + pub_input);
        denominator = denominator * (denominator_acc + pub_input);
        numerator_acc = numerator_acc + beta;
        denominator_acc = denominator_acc - beta;
    }

    numerator * denominator.inverse().unwrap()
}
