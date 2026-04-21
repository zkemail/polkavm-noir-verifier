#![no_main]
#![no_std]
extern crate alloc;

use simplealloc::SimpleAlloc;
use uapi::{HostFn, HostFnImpl as api, ReturnFlags, CallFlags};

#[global_allocator]
static ALLOCATOR: SimpleAlloc<{ 64 * 1024 }> = SimpleAlloc::new();

fn call_precompile(addr: u8, input: &[u8], output: &mut [u8]) -> u32 {
    let mut target = [0u8; 20];
    target[19] = addr;
    let mut out: &mut [u8] = output;
    match api::call(
        CallFlags::empty(),
        &target,
        api::ref_time_left() / 2,
        u64::MAX,
        &[0u8; 32],
        &[0u8; 32],
        input,
        Some(&mut out),
    ) {
        Ok(_) => 0,
        Err(e) => e as u32,
    }
}

// selector 0x00000001: ecAdd(G1, G1) -> returns status(1) + point(64)
fn test_ecadd() {
    let mut input = [0u8; 128];
    input[31] = 1; input[63] = 2; // G1 = (1, 2)
    input[95] = 1; input[127] = 2; // G1 = (1, 2)
    let mut output = [0u8; 64];
    let s = call_precompile(0x06, &input, &mut output);
    let mut ret = [0u8; 65];
    ret[0] = s as u8;
    ret[1..65].copy_from_slice(&output);
    api::return_value(ReturnFlags::empty(), &ret);
}

// selector 0x00000002: ecMul(G1, 2) -> returns status(1) + point(64)
fn test_ecmul() {
    let mut input = [0u8; 96];
    input[31] = 1; input[63] = 2; // G1 = (1, 2)
    input[95] = 2; // scalar = 2
    let mut output = [0u8; 64];
    let s = call_precompile(0x07, &input, &mut output);
    let mut ret = [0u8; 65];
    ret[0] = s as u8;
    ret[1..65].copy_from_slice(&output);
    api::return_value(ReturnFlags::empty(), &ret);
}

// selector 0x00000003: ecPairing with 2 pairs: e(G1,G2)*e(-G1,G2)==1
fn test_ecpairing() {
    // G2 generator (from official polkadot-sdk test vectors)
    let g2: [u8; 128] = [
        0x19,0x8e,0x93,0x93,0x92,0x0d,0x48,0x3a,0x72,0x60,0xbf,0xb7,0x31,0xfb,0x5d,0x25,
        0xf1,0xaa,0x49,0x33,0x35,0xa9,0xe7,0x12,0x97,0xe4,0x85,0xb7,0xae,0xf3,0x12,0xc2,
        0x18,0x00,0xde,0xef,0x12,0x1f,0x1e,0x76,0x42,0x6a,0x00,0x66,0x5e,0x5c,0x44,0x79,
        0x67,0x43,0x22,0xd4,0xf7,0x5e,0xda,0xdd,0x46,0xde,0xbd,0x5c,0xd9,0x92,0xf6,0xed,
        0x09,0x06,0x89,0xd0,0x58,0x5f,0xf0,0x75,0xec,0x9e,0x99,0xad,0x69,0x0c,0x33,0x95,
        0xbc,0x4b,0x31,0x33,0x70,0xb3,0x8e,0xf3,0x55,0xac,0xda,0xdc,0xd1,0x22,0x97,0x5b,
        0x12,0xc8,0x5e,0xa5,0xdb,0x8c,0x6d,0xeb,0x4a,0xab,0x71,0x80,0x8d,0xcb,0x40,0x8f,
        0xe3,0xd1,0xe7,0x69,0x0c,0x43,0xd3,0x7b,0x4c,0xe6,0xcc,0x01,0x66,0xfa,0x7d,0xaa,
    ];
    // -G1.y = Fq - 2
    let neg_y: [u8; 32] = [
        0x30,0x64,0x4e,0x72,0xe1,0x31,0xa0,0x29,0xb8,0x50,0x45,0xb6,0x81,0x81,0x58,0x5d,
        0x97,0x81,0x6a,0x91,0x68,0x71,0xca,0x8d,0x3c,0x20,0x8c,0x16,0xd8,0x7c,0xfd,0x45,
    ];
    let mut input = [0u8; 384];
    // Pair 1: (G1, G2)
    input[31] = 1; input[63] = 2;
    input[64..192].copy_from_slice(&g2);
    // Pair 2: (-G1, G2)
    input[223] = 1;
    input[224..256].copy_from_slice(&neg_y);
    input[256..384].copy_from_slice(&g2);

    let mut output = [0u8; 32];
    let s = call_precompile(0x08, &input, &mut output);
    let mut ret = [0u8; 33];
    ret[0] = s as u8;
    ret[1..33].copy_from_slice(&output);
    api::return_value(ReturnFlags::empty(), &ret);
}

#[no_mangle]
#[polkavm_derive::polkavm_export]
pub extern "C" fn deploy() {}

#[no_mangle]
#[polkavm_derive::polkavm_export]
pub extern "C" fn call() {
    let mut sel = [0u8; 4];
    if (api::call_data_size() as usize) < 4 { return; }
    api::call_data_copy(&mut sel, 0);
    match sel {
        [0,0,0,1] => test_ecadd(),
        [0,0,0,2] => test_ecmul(),
        [0,0,0,3] => test_ecpairing(),
        _ => {},
    }
}

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    unsafe { core::arch::asm!("unimp"); core::hint::unreachable_unchecked(); }
}
