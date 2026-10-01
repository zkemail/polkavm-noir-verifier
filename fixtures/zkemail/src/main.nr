
mod x_handle_regex;
mod sender_domain_regex;
use poseidon::poseidon;
use std::{collections::bounded_vec::BoundedVec, hash::pedersen_hash};
use zkemail::{
    dkim::RSAPubkey,
    hash::poseidon_large,
    headers::body_hash::get_body_hash,
    partial_hash::partial_sha256_var_end, remove_soft_line_breaks::remove_soft_line_breaks,
    masking::mask_text, Sequence, utils::pack_bytes,
};
fn main(
    header: BoundedVec<u8, 960>,
    pubkey: RSAPubkey<18>,
    signature: [Field; 18],
    prover_address: [Field; 1],
    dkim_header_sequence: Sequence,
    body: BoundedVec<u8, 1920>,
    body_hash_index: u32,
    partial_body_hash: [u32; 8],
    partial_body_real_length: u64,
    decoded_body: BoundedVec<u8, 1920>,
    command: [Field; 20],
    x_handle_match_start: u32,
    x_handle_match_length: u32,
    x_handle_current_states: [Field; 64],
    x_handle_next_states: [Field; 64],
    x_handle_capture_group_1_id: [Field; 64],
    x_handle_capture_group_1_start: [Field; 64],
    x_handle_capture_group_start_indices: [Field; 1],
    sender_domain_match_start: u32,
    sender_domain_match_length: u32,
    sender_domain_current_states: [Field; 64],
    sender_domain_next_states: [Field; 64],
    sender_domain_capture_group_1_id: [Field; 64],
    sender_domain_capture_group_1_start: [Field; 64],
    sender_domain_capture_group_start_indices: [Field; 1],
) -> pub ([Field; 2], Field, Field, Field, [Field; 1] , [Field; 20], BoundedVec<u8, 64>, BoundedVec<u8, 64>) {
    // check the body and header lengths are within bounds
    assert(header.len() <= 960);
    assert(body.len() <= 1920);
    // DKIM Verification
    let header_hash: [u8; 32] = pubkey.verify_dkim_signature(header, signature);
    let header_hash: [Field; 2] = pack_bytes::<32, 16>(header_hash);
    // Body Hash Verification
    let signed_body_hash = get_body_hash(header, dkim_header_sequence, body_hash_index);
    let computed_body_hash: [u8; 32] = partial_sha256_var_end(
        partial_body_hash,
        body.storage(),
        body.len() as u64,
        partial_body_real_length,
    );
    assert(
        signed_body_hash == computed_body_hash,
        "SHA256 hash computed over body does not match body hash found in DKIM-signed header",
    );
    assert(
        remove_soft_line_breaks(body.storage(), decoded_body.storage()),
        "Decoded body does not properly remove soft line breaks",
    );
    // NOTE: the DKIM signature covers header[0..header.len()) and the body hash covers
    // body[0..body.len()). Everything a regex below reads must come from those signed bytes.
    // See `assert_zero_padded` for why each check is needed.
    assert_zero_padded(body);
    assert_zero_padded(decoded_body);
    assert(
        decoded_body.len() <= body.len(),
        "Decoded body cannot be longer than the encoded body",
    );
    assert(
        x_handle_match_start + x_handle_match_length <= decoded_body.len(),
        "Body regex match must lie within the signed body",
    );
    assert(
        sender_domain_match_start + sender_domain_match_length <= header.len(),
        "Header regex match must lie within the signed header",
    );

    // NOTE: zkemail.nr >= 2.0.0 (zkemail/zkemail.nr#62) returns [modulus_hash, redc_hash]. Earlier
    // versions hashed only the 2048-bit modulus, so redc was not bound to the public key hash. A
    // verifier consumer must check both values against the DKIM key it trusts.
    let pubkey_hash = pubkey.hash();
    let email_nullifier = poseidon::bn254::hash_1([poseidon_large(signature)]);
    let x_handle_capture_1 = x_handle_regex::regex_match::<1920, 64>(
        decoded_body.storage(),
        x_handle_match_start,
        x_handle_match_length,
        x_handle_current_states,
        x_handle_next_states,
        x_handle_capture_group_1_id,
        x_handle_capture_group_1_start,
        x_handle_capture_group_start_indices,
    );
    let sender_domain_capture_1 = sender_domain_regex::regex_match::<960, 64>(
        header.storage(),
        sender_domain_match_start,
        sender_domain_match_length,
        sender_domain_current_states,
        sender_domain_next_states,
        sender_domain_capture_group_1_id,
        sender_domain_capture_group_1_start,
        sender_domain_capture_group_start_indices,
    );
    (pubkey_hash, email_nullifier, header_hash[0], header_hash[1], prover_address, command, x_handle_capture_1, sender_domain_capture_1)
}

/**
 * Asserts that every byte of `vec`'s storage at index >= len() is zero.
 *
 * REASON: BoundedVec storage past len() is a free prover input. sha256_var / partial_sha256_var_end
 * only hash bytes [0, len()), and zk-regex's select_subarray only bounds a match by the array
 * capacity, not by len(). remove_soft_line_breaks also runs over the whole storage, so unsigned
 * bytes past body.len() flow into decoded_body. Without this check (and the match bounds in
 * main) a regex can match bytes that were never signed, and the public outputs no longer
 * describe the signed email. Do not remove these checks to save gates (they cost ~2 gates/byte).
 */
fn assert_zero_padded<let N: u32>(vec: BoundedVec<u8, N>) {
    let storage = vec.storage();
    for i in 0..N {
        assert((i < vec.len()) | (storage[i] == 0), "BoundedVec storage past len() must be zero");
    }
}
