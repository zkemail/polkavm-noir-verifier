#![no_main]
#![no_std]
extern crate alloc;
use alloc::boxed::Box;
use alloc::vec;
use alloc::vec::Vec;

use simplealloc::SimpleAlloc;

mod zk;
use zk::{poseidon, Fr};

mod leanimt;
use leanimt::LeanIMT;

pub struct MerkleContract {
    tree: LeanIMT<32>,
}

#[global_allocator]
pub static mut GLOBAL: SimpleAlloc<{ 1024 * 10 }> = SimpleAlloc::new();

use uapi::{input, HostFn, HostFnImpl as api, ReturnFlags, StorageFlags};

use ethabi::{decode, ethereum_types::U256, ParamType, Token};

const NAME: &[u8] = b"name";

// Define constants for all your function selectors

const HASH_SELECTOR: [u8; 4] = [0x56, 0x15, 0x58, 0xfe]; // hash(uint256[2])

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    unsafe {
        core::arch::asm!("unimp");
        core::hint::unreachable_unchecked();
    }
}

#[no_mangle]
#[polkavm_derive::polkavm_export]
pub extern "C" fn deploy() {}

/// This is the regular entry point when the contract is called.
#[no_mangle]
#[polkavm_derive::polkavm_export]
pub extern "C" fn call() {
    input!(selector: &[u8; 4],);
    let length = api::call_data_size();
    // Temporarily remove length check for debugging
    // if length > 256 {
    //     panic!("Input data too long");
    // }
    let mut sender = [0_u8; 20];
    api::origin(&mut sender);

    match selector {
        &HASH_SELECTOR => {
            input!(buffer: &[u8; 4 + 32 + 32],);
            let param_types = &[ParamType::FixedArray(Box::new(ParamType::Uint(256)), 2)];
            let decode_result = decode(param_types, &buffer[4..]).unwrap();

            if let Token::FixedArray(array_tokens) = &decode_result[0] {
                if array_tokens.len() == 2 {
                    if let (Token::Uint(a), Token::Uint(b)) = (&array_tokens[0], &array_tokens[1]) {
                        // Convert U256 to Fr (field elements for poseidon)
                        let mut a_bytes = [0u8; 32];
                        let mut b_bytes = [0u8; 32];
                        a.to_big_endian(&mut a_bytes);
                        b.to_big_endian(&mut b_bytes);

                        // Convert 32 bytes to 4 u64 limbs (little endian from the big endian bytes)
                        let mut a_limbs = [0u64; 4];
                        let mut b_limbs = [0u64; 4];

                        // Big endian bytes need to be processed correctly
                        for i in 0..4 {
                            // Take 8 bytes at a time from the big endian representation
                            let start_idx = (3 - i) * 8; // Reverse order for little endian limbs
                            a_limbs[i] = u64::from_be_bytes([
                                a_bytes[start_idx],
                                a_bytes[start_idx + 1],
                                a_bytes[start_idx + 2],
                                a_bytes[start_idx + 3],
                                a_bytes[start_idx + 4],
                                a_bytes[start_idx + 5],
                                a_bytes[start_idx + 6],
                                a_bytes[start_idx + 7],
                            ]);
                            b_limbs[i] = u64::from_be_bytes([
                                b_bytes[start_idx],
                                b_bytes[start_idx + 1],
                                b_bytes[start_idx + 2],
                                b_bytes[start_idx + 3],
                                b_bytes[start_idx + 4],
                                b_bytes[start_idx + 5],
                                b_bytes[start_idx + 6],
                                b_bytes[start_idx + 7],
                            ]);
                        }

                        // Create Fr elements from the limbs
                        let fr_a = Fr::from_limbs(a_limbs);
                        let fr_b = Fr::from_limbs(b_limbs);

                        // Hash using poseidon
                        let hash_result = poseidon(&[fr_a, fr_b]);

                        // Convert back to bytes (big endian for Ethereum)
                        let result_limbs = hash_result.to_limbs();
                        let mut result_bytes = [0u8; 32];

                        // Convert limbs back to big endian bytes
                        for i in 0..4 {
                            let limb_bytes = result_limbs[3 - i].to_be_bytes(); // Reverse order
                            for j in 0..8 {
                                result_bytes[i * 8 + j] = limb_bytes[j];
                            }
                        }
                        api::return_value(ReturnFlags::empty(), &result_bytes);
                    } else {
                        panic!("Invalid token types in array");
                    }
                } else {
                    panic!("Array must have exactly 2 elements");
                }
            } else {
                panic!("Failed to decode input as uint256[2]");
            }
        }
        _ => panic!("Unknown function"),
    }
}

pub fn get_string(length_key: &[u8], data_key: &[u8]) -> Vec<u8> {
    // max length is 256 * 256
    let mut length_bytes = [0u8; 2];
    let _ = api::get_storage(
        uapi::StorageFlags::empty(),
        length_key,
        &mut &mut length_bytes[..],
    );
    let length = u16::from_be_bytes(length_bytes);
    // read data according to the length
    let mut data = vec![0u8; length as usize];
    let _ = api::get_storage(uapi::StorageFlags::empty(), data_key, &mut &mut data[..]);
    // compute padding length
    let result_length = if length % 32 == 0 {
        length + 64
    } else {
        (length / 32 + 1) * 32 + 64
    };
    let mut result = vec![0u8; result_length as usize];
    // set index of string length
    result[31] = 0x20;
    // set length
    result[62..64].copy_from_slice(&length_bytes);
    result[64..(64 + data.len())].copy_from_slice(&data[..]);
    result
}

pub fn set_string(length_key: &[u8], data_key: &[u8], data: &[u8]) {
    let length = data.len() as u16;
    let _ = api::set_storage(
        uapi::StorageFlags::empty(),
        length_key,
        &length.to_be_bytes(),
    );
    api::set_storage(uapi::StorageFlags::empty(), data_key, &data[..]);
}
