use crate::g1::G1Point;

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
        // first `pad` nibbles are zero
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

pub fn load_vk() -> VerificationKey {
    VerificationKey {
        circuit_size: 32,
        log_circuit_size: 5,
        public_inputs_size: 1,
        pub_inputs_offset: 1,
        ql: g1(
            "11ad6b3a3b872fc1acce8109c398fa1f5e58d01a844f1f9def88b1ff01351661",
            "129b7f90aa4a19397ee41fb4cfff9311926c0e3a5033bed4ae42b1159226a51a",
        ),
        qr: g1(
            "0b0e20d82e14a5912834e9b4f5153d7c6f434740af0666ee5c56a24f44567365",
            "28663be34020c00322279f5179a36b9413146cbd4b340ea96150d094df88f8fe",
        ),
        qo: g1(
            "09de4c0ce293ba3b96cbad52ea2027630b0d8ff43d9ef999a83cc1cd66bbf03c",
            "117dbcfeb68ed48d23660581568ebe7f66fab0bd8e7254d8848b80222ba7cf71",
        ),
        q4: g1(
            "0fd1274f8b384aa238ab5ba135dad4650b9bbed5a48f4a2c77047f577dbb1201",
            "0e8a8f5fb867080edb0391c1bdad0b6a118d7b3a8312c27935b42e49dd9948fe",
        ),
        qm: g1(
            "13868a317444a5efe378de0d47922f9f5496a4692e3fa8c261ba5428d3182515",
            "07eef6a540b0a0c21ca51267b655e658e21e16b14a5ff25006bf8f2d233ccdc1",
        ),
        qc: g1(
            "201802934e67604b1863e37e5e1a8985e42a4e829b8459ee77954c5e5218f66e",
            "07e813a7e75c707d424f4a166307d849541fdf467cd18f83e21cfb1987d203fe",
        ),
        q_arith: g1(
            "006a9540ee6a4d6fbfa534a9c112715fc3e6d6bfb63a367e21f41eb47fbc0c3e",
            "0abf801f8ca9ad0b88fbc5cfc98f0ef64b9959da4e8b9e83f4dba7135273b21c",
        ),
        q_delta_range: g1(
            "1618ea679c4ee1467267e50bb898148ef78d5de08341b5afdc0c863a59ab7e70",
            "23268ad7678b97fba97cc3e75da6cff9a3659c3b8a49046cce4062820e5c1116",
        ),
        q_elliptic: g1(
            "1a11684e6c135cbe0b0ccdb27df1434557e054c65df3af7487468bdfa2fd8325",
            "2a8f4fba8e6893b6d523e9572d7f4c60cadfef00619e58d7db7f2b28cec21202",
        ),
        q_aux: g1(
            "1469006b8b61c8d79301dfeeed1b752548d2ebff7ed60a3cda0c7db10c955c8a",
            "19c2b11ddeff8ffe68ab919e345670ab029bcbce4a91df6570722995d636f037",
        ),
        q_lookup: g1(
            "2453e056dc179bdc9164de8e3654ed72bd9f54e6e9b57129ce995b19cec0c90f",
            "15bc4680db7eb8100d97a8cfaeabcda043139c10e1840d9a1a7bba0989618a5a",
        ),
        q_poseidon2_external: g1(
            "0524c8e7146a41551c673b3139893fa365285bca50c9a1b5a476834f5f518c05",
            "0e3589731c046d57d3ae60d42e595d73cc5e8e83438261bdb144c99daa9fe18b",
        ),
        q_poseidon2_internal: g1(
            "1aebf53057be467f5c3ed0f88d90604a4c8d6886256adeca293661e04f1a3ddf",
            "2bb5fcc21332b83521c63599557c6473249908a160efa4921bc0e5c16da58b6f",
        ),
        s1: g1(
            "09f2420ca39f8c66bc86cef9ca2f1b433e6db34f2cea25534ba68b732cca8d99",
            "1fc9ba0ca1f14657f8a3ac1759839d2e7216fc6fd4e2e056497e1551b37128f8",
        ),
        s2: g1(
            "154709fd0743103813577e0b9c5936e55aee220dba359a01a3d06e72bd5e96ff",
            "21d04477bb3d674d315bb512c54ad4befb8a40157d14ce7ebb9f0c7057596627",
        ),
        s3: g1(
            "15a8a335f2e2564d2e2ff1945693daba560ade30935724b6a5c95053777a0312",
            "1b7d24b00a51747254648350adb880b5d79361c489c9c75193368a53c2699aa5",
        ),
        s4: g1(
            "13b5fe0957911479ccd4425f1aa0ffa4fbfacc2176b4c039c7efe1bc636a8ef7",
            "27137b8082abcb07905cec75809bcf814c306db5cca3649a8fced32db68d29f9",
        ),
        t1: g1(
            "21ba3aba551d4f6eb1224ddc6b1bd3222e6553035953037a679ab272fa3105b9",
            "2021ee9bf4036008c7c5360b19266d1256c709eafae29d6d2c39a34b1b77c86c",
        ),
        t2: g1(
            "292ec6f935caa1df0c8f63212e0116c906e487174acb4ffd415b2d78e05072e6",
            "1d3047e5faf396ecdff46211993558a4755d42ee0b5ee4323e4a3c8dd012d101",
        ),
        t3: g1(
            "0a0a057328da58331a5bdac41900c5160e17aff3205ebc50dd3e6ade8082c459",
            "2f1f6579ac435ccd3becb11cac21a42f71677010de466cea777c910d9a5d4d6e",
        ),
        t4: g1(
            "27456b3a666ff24c6452657437518f7b73e854ce6c763732122a3b923bc6797b",
            "2ecbc0db4ae72d05db96eb72034b26275a33325b05b2dd53c33662369bcdc4e0",
        ),
        id1: g1(
            "17f3c984982dcc1d481b2b7d28aef79f8c700e4fde693a3ee792993c6ef77765",
            "26c942b83f4fc94e3646bd782284850616a4f088de05db283ea650de5452510e",
        ),
        id2: g1(
            "09e8c4400c501df3a680c807d199dca4b23892f2cb9163a1d1c378e84cb1e0be",
            "1a4e20a630920734bafb9962078862b97a655014d546f5a0f3f4e0037c6bc584",
        ),
        id3: g1(
            "24f21e58367f93a52601c7dad83f39ad9be9235d46cc9fd4bed60251f55f5f24",
            "216267dc395e5d7edd5da1a108783205bf0e476d7aaccf36f2a478c65f23cc0d",
        ),
        id4: g1(
            "233e6668b534fa576290c61f0f618509482bec6ab6b2ddaa5591e81bfb216779",
            "0a2eb194ae8dcd350327ff9adc990ab750bdb767e6d3e8293ae433ef77f8719f",
        ),
        lagrange_first: g1(
            "0000000000000000000000000000000000000000000000000000000000000001",
            "0000000000000000000000000000000000000000000000000000000000000002",
        ),
        lagrange_last: g1(
            "09dfd2992ac1708f0dd1d28c2ad910d9cf21a1510948580f406bc9416113d620",
            "205f76eebda12f565c98c775c4e4f3534b5dcc29e57eed899b1a1a880534dcb9",
        ),
    }
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
