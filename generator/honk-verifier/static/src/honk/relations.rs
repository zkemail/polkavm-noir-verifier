/// UltraHonk sub-relation evaluations (26 sub-relations).
///
/// Direct translation from HonkVerifier.sol `accumulateRelationEvaluations()`.
/// Source: Aztec/Barretenberg `bb write_solidity_verifier` output.
///
/// The 26 sub-relations correspond to:
///   - Arithmetic (2), Permutation (2), Lookup (2), DeltaRange (4),
///     Elliptic (2), Auxiliary (6), Poseidon2External (4), Poseidon2Internal (4)
///
/// Each sub-relation formula is identical to the Solidity implementation.
/// The scaling by alpha powers and pow evaluation matches TranscriptLib.
use super::fr::{
    Fr, FR_LIMB_2_68, FR_NEG_HALF, FR_NEG_ONE, FR_NEG_THREE, FR_NEG_TWO, FR_NINE, FR_SEVENTEEN,
    FR_SUBLIMB_2_14, FR_THREE, FR_TWO, POSEIDON_DIAG,
};
use super::proof::NUMBER_OF_ENTITIES;
use super::transcript::{RelationParameters, NUMBER_OF_ALPHAS};

pub const NUMBER_OF_SUBRELATIONS: usize = 26;

const Q_M: usize = 0;
const Q_C: usize = 1;
const Q_L: usize = 2;
const Q_R: usize = 3;
const Q_O: usize = 4;
const Q_4: usize = 5;
const Q_LOOKUP: usize = 6;
const Q_ARITH: usize = 7;
const Q_RANGE: usize = 8;
const Q_ELLIPTIC: usize = 9;
const Q_AUX: usize = 10;
const Q_POSEIDON2_EXTERNAL: usize = 11;
const Q_POSEIDON2_INTERNAL: usize = 12;
const SIGMA_1: usize = 13;
const SIGMA_2: usize = 14;
const SIGMA_3: usize = 15;
const SIGMA_4: usize = 16;
const ID_1: usize = 17;
const ID_2: usize = 18;
const ID_3: usize = 19;
const ID_4: usize = 20;
const TABLE_1: usize = 21;
const TABLE_2: usize = 22;
const TABLE_3: usize = 23;
const TABLE_4: usize = 24;
const LAGRANGE_FIRST: usize = 25;
const LAGRANGE_LAST: usize = 26;
const W_L: usize = 27;
const W_R: usize = 28;
const W_O: usize = 29;
const W_4: usize = 30;
const Z_PERM: usize = 31;
const LOOKUP_INVERSES: usize = 32;
const LOOKUP_READ_COUNTS: usize = 33;
const LOOKUP_READ_TAGS: usize = 34;
const W_L_SHIFT: usize = 35;
const W_R_SHIFT: usize = 36;
const W_O_SHIFT: usize = 37;
const W_4_SHIFT: usize = 38;
const Z_PERM_SHIFT: usize = 39;

#[inline]
fn w(p: &[Fr; NUMBER_OF_ENTITIES], wire: usize) -> Fr {
    p[wire]
}

/// Ultra Arithmetic Relation (2 subrelations: evals[0], evals[1])
fn accumulate_arithmetic(
    p: &[Fr; NUMBER_OF_ENTITIES],
    evals: &mut [Fr; NUMBER_OF_SUBRELATIONS],
    domain_sep: Fr,
) {
    let q_arith = w(p, Q_ARITH);

    {
        let mut accum = (q_arith - FR_THREE) * (w(p, Q_M) * w(p, W_R) * w(p, W_L)) * FR_NEG_HALF;
        accum = accum
            + w(p, Q_L) * w(p, W_L)
            + w(p, Q_R) * w(p, W_R)
            + w(p, Q_O) * w(p, W_O)
            + w(p, Q_4) * w(p, W_4)
            + w(p, Q_C);
        accum = accum + (q_arith - Fr::ONE) * w(p, W_4_SHIFT);
        accum = accum * q_arith * domain_sep;
        evals[0] = accum;
    }

    {
        let mut accum = w(p, W_L) + w(p, W_4) - w(p, W_L_SHIFT) + w(p, Q_M);
        accum = accum * (q_arith - FR_TWO) * (q_arith - Fr::ONE) * q_arith * domain_sep;
        evals[1] = accum;
    }
}

/// Permutation Relation (2 subrelations: evals[2], evals[3])
fn accumulate_permutation(
    p: &[Fr; NUMBER_OF_ENTITIES],
    rp: &RelationParameters,
    evals: &mut [Fr; NUMBER_OF_SUBRELATIONS],
    domain_sep: Fr,
) {
    let grand_product_numerator = {
        let a = w(p, W_L) + w(p, ID_1) * rp.beta + rp.gamma;
        let b = w(p, W_R) + w(p, ID_2) * rp.beta + rp.gamma;
        let c = w(p, W_O) + w(p, ID_3) * rp.beta + rp.gamma;
        let d = w(p, W_4) + w(p, ID_4) * rp.beta + rp.gamma;
        a * b * c * d
    };
    let grand_product_denominator = {
        let a = w(p, W_L) + w(p, SIGMA_1) * rp.beta + rp.gamma;
        let b = w(p, W_R) + w(p, SIGMA_2) * rp.beta + rp.gamma;
        let c = w(p, W_O) + w(p, SIGMA_3) * rp.beta + rp.gamma;
        let d = w(p, W_4) + w(p, SIGMA_4) * rp.beta + rp.gamma;
        a * b * c * d
    };

    {
        let acc = (w(p, Z_PERM) + w(p, LAGRANGE_FIRST)) * grand_product_numerator
            - (w(p, Z_PERM_SHIFT) + w(p, LAGRANGE_LAST) * rp.public_inputs_delta)
                * grand_product_denominator;
        evals[2] = acc * domain_sep;
    }

    evals[3] = w(p, LAGRANGE_LAST) * w(p, Z_PERM_SHIFT) * domain_sep;
}

/// Log-Derivative Lookup Relation (2 subrelations: evals[4], evals[5])
fn accumulate_log_derivative_lookup(
    p: &[Fr; NUMBER_OF_ENTITIES],
    rp: &RelationParameters,
    evals: &mut [Fr; NUMBER_OF_SUBRELATIONS],
    domain_sep: Fr,
) {
    let write_term = w(p, TABLE_1)
        + rp.gamma
        + w(p, TABLE_2) * rp.eta
        + w(p, TABLE_3) * rp.eta_two
        + w(p, TABLE_4) * rp.eta_three;

    let read_term = {
        let d1 = w(p, W_L) + rp.gamma + w(p, Q_R) * w(p, W_L_SHIFT);
        let d2 = w(p, W_R) + w(p, Q_M) * w(p, W_R_SHIFT);
        let d3 = w(p, W_O) + w(p, Q_C) * w(p, W_O_SHIFT);
        d1 + d2 * rp.eta + d3 * rp.eta_two + w(p, Q_O) * rp.eta_three
    };

    let read_inverse = w(p, LOOKUP_INVERSES) * write_term;
    let write_inverse = w(p, LOOKUP_INVERSES) * read_term;

    let inverse_exists_xor =
        w(p, LOOKUP_READ_TAGS) + w(p, Q_LOOKUP) - w(p, LOOKUP_READ_TAGS) * w(p, Q_LOOKUP);

    evals[4] = (read_term * write_term * w(p, LOOKUP_INVERSES) - inverse_exists_xor) * domain_sep;

    evals[5] = w(p, Q_LOOKUP) * read_inverse - w(p, LOOKUP_READ_COUNTS) * write_inverse;
}

/// Delta Range Relation (4 subrelations: evals[6..9])
fn accumulate_delta_range(
    p: &[Fr; NUMBER_OF_ENTITIES],
    evals: &mut [Fr; NUMBER_OF_SUBRELATIONS],
    domain_sep: Fr,
) {
    let minus_one = FR_NEG_ONE;
    let minus_two = FR_NEG_TWO;
    let minus_three = FR_NEG_THREE;

    let delta_1 = w(p, W_R) - w(p, W_L);
    let delta_2 = w(p, W_O) - w(p, W_R);
    let delta_3 = w(p, W_4) - w(p, W_O);
    let delta_4 = w(p, W_L_SHIFT) - w(p, W_4);

    evals[6] = delta_1
        * (delta_1 + minus_one)
        * (delta_1 + minus_two)
        * (delta_1 + minus_three)
        * w(p, Q_RANGE)
        * domain_sep;

    evals[7] = delta_2
        * (delta_2 + minus_one)
        * (delta_2 + minus_two)
        * (delta_2 + minus_three)
        * w(p, Q_RANGE)
        * domain_sep;

    evals[8] = delta_3
        * (delta_3 + minus_one)
        * (delta_3 + minus_two)
        * (delta_3 + minus_three)
        * w(p, Q_RANGE)
        * domain_sep;

    evals[9] = delta_4
        * (delta_4 + minus_one)
        * (delta_4 + minus_two)
        * (delta_4 + minus_three)
        * w(p, Q_RANGE)
        * domain_sep;
}

/// Elliptic Curve Relation (2 subrelations: evals[10], evals[11])
fn accumulate_elliptic(
    p: &[Fr; NUMBER_OF_ENTITIES],
    evals: &mut [Fr; NUMBER_OF_SUBRELATIONS],
    domain_sep: Fr,
) {
    let grumpkin_b_neg = FR_SEVENTEEN;

    let x_1 = w(p, W_R);
    let y_1 = w(p, W_O);
    let x_2 = w(p, W_L_SHIFT);
    let y_2 = w(p, W_4_SHIFT);
    let y_3 = w(p, W_O_SHIFT);
    let x_3 = w(p, W_R_SHIFT);

    let q_sign = w(p, Q_L);
    let q_is_double = w(p, Q_M);

    let x_diff = x_2 - x_1;
    let y1_sqr = y_1 * y_1;

    {
        let y2_sqr = y_2 * y_2;
        let y1y2 = y_1 * y_2 * q_sign;
        let x_add_identity = (x_3 + x_2 + x_1) * x_diff * x_diff - y2_sqr - y1_sqr + y1y2 + y1y2;
        evals[10] = x_add_identity * domain_sep * w(p, Q_ELLIPTIC) * (Fr::one() - q_is_double);
    }

    {
        let y1_plus_y3 = y_1 + y_3;
        let y_diff = y_2 * q_sign - y_1;
        let y_add_identity = y1_plus_y3 * x_diff + (x_3 - x_1) * y_diff;
        evals[11] = y_add_identity * domain_sep * w(p, Q_ELLIPTIC) * (Fr::one() - q_is_double);
    }

    {
        let x_pow_4 = (y1_sqr + grumpkin_b_neg) * x_1;
        let y1_sqr_mul_4 = (y1_sqr + y1_sqr) * FR_TWO;
        let x1_pow_4_mul_9 = x_pow_4 * FR_NINE;
        let x_double_identity = (x_3 + x_1 + x_1) * y1_sqr_mul_4 - x1_pow_4_mul_9;
        evals[10] = evals[10] + x_double_identity * domain_sep * w(p, Q_ELLIPTIC) * q_is_double;
    }

    {
        let x1_sqr_mul_3 = (x_1 + x_1 + x_1) * x_1;
        let y_double_identity = x1_sqr_mul_3 * (x_1 - x_3) - (y_1 + y_1) * (y_1 + y_3);
        evals[11] = evals[11] + y_double_identity * domain_sep * w(p, Q_ELLIPTIC) * q_is_double;
    }
}

/// Auxiliary Relation (evals[12..17])
fn accumulate_auxiliary(
    p: &[Fr; NUMBER_OF_ENTITIES],
    rp: &RelationParameters,
    evals: &mut [Fr; NUMBER_OF_SUBRELATIONS],
    domain_sep: Fr,
) {
    let limb_size = FR_LIMB_2_68;
    let sublimb_shift = FR_SUBLIMB_2_14;
    let minus_one = FR_NEG_ONE;

    let limb_subproduct = w(p, W_L) * w(p, W_R_SHIFT) + w(p, W_L_SHIFT) * w(p, W_R);

    let non_native_field_gate_2 = {
        let v = (w(p, W_L) * w(p, W_4) + w(p, W_R) * w(p, W_O) - w(p, W_O_SHIFT)) * limb_size
            - w(p, W_4_SHIFT)
            + limb_subproduct;
        v * w(p, Q_4)
    };

    let limb_subproduct2 = limb_subproduct * limb_size + w(p, W_L_SHIFT) * w(p, W_R_SHIFT);
    let non_native_field_gate_1 = (limb_subproduct2 - (w(p, W_O) + w(p, W_4))) * w(p, Q_O);
    let non_native_field_gate_3 =
        (limb_subproduct2 + w(p, W_4) - (w(p, W_O_SHIFT) + w(p, W_4_SHIFT))) * w(p, Q_M);

    let non_native_field_identity =
        (non_native_field_gate_1 + non_native_field_gate_2 + non_native_field_gate_3) * w(p, Q_R);

    let limb_accumulator_1 = {
        let mut v = w(p, W_R_SHIFT) * sublimb_shift;
        v = (v + w(p, W_L_SHIFT)) * sublimb_shift;
        v = (v + w(p, W_O)) * sublimb_shift;
        v = (v + w(p, W_R)) * sublimb_shift;
        v = v + w(p, W_L) - w(p, W_4);
        v * w(p, Q_4)
    };

    let limb_accumulator_2 = {
        let mut v = w(p, W_O_SHIFT) * sublimb_shift;
        v = (v + w(p, W_R_SHIFT)) * sublimb_shift;
        v = (v + w(p, W_L_SHIFT)) * sublimb_shift;
        v = (v + w(p, W_4)) * sublimb_shift;
        v = v + w(p, W_O) - w(p, W_4_SHIFT);
        v * w(p, Q_M)
    };

    let limb_accumulator_identity = (limb_accumulator_1 + limb_accumulator_2) * w(p, Q_O);

    let memory_record_check =
        w(p, W_O) * rp.eta_three + w(p, W_R) * rp.eta_two + w(p, W_L) * rp.eta + w(p, Q_C);
    let partial_record_check = memory_record_check;
    let memory_record_check = memory_record_check - w(p, W_4);

    let index_delta = w(p, W_L_SHIFT) - w(p, W_L);
    let record_delta = w(p, W_4_SHIFT) - w(p, W_4);

    let index_is_monotonically_increasing = index_delta * index_delta - index_delta;
    let adjacent_values_match_if_adjacent_indices_match =
        (index_delta * minus_one + Fr::one()) * record_delta;

    evals[13] = adjacent_values_match_if_adjacent_indices_match
        * (w(p, Q_L) * w(p, Q_R))
        * (w(p, Q_AUX) * domain_sep);

    evals[14] =
        index_is_monotonically_increasing * (w(p, Q_L) * w(p, Q_R)) * (w(p, Q_AUX) * domain_sep);

    let rom_consistency_check_identity = memory_record_check * (w(p, Q_L) * w(p, Q_R));

    let access_type = w(p, W_4) - partial_record_check;
    let access_check = access_type * access_type - access_type;

    let next_gate_access_type = {
        let v = w(p, W_O_SHIFT) * rp.eta_three
            + w(p, W_R_SHIFT) * rp.eta_two
            + w(p, W_L_SHIFT) * rp.eta;
        w(p, W_4_SHIFT) - v
    };

    let value_delta = w(p, W_O_SHIFT) - w(p, W_O);
    let adjacent_values_match_if_adjacent_indices_match_and_next_access_is_a_read_operation =
        (index_delta * minus_one + Fr::one())
            * value_delta
            * (next_gate_access_type * minus_one + Fr::one());

    let next_gate_access_type_is_boolean =
        next_gate_access_type * next_gate_access_type - next_gate_access_type;

    evals[15] = adjacent_values_match_if_adjacent_indices_match_and_next_access_is_a_read_operation
        * w(p, Q_ARITH)
        * (w(p, Q_AUX) * domain_sep);

    evals[16] = index_is_monotonically_increasing * w(p, Q_ARITH) * (w(p, Q_AUX) * domain_sep);

    evals[17] = next_gate_access_type_is_boolean * w(p, Q_ARITH) * (w(p, Q_AUX) * domain_sep);

    let ram_consistency_check_identity = access_check * w(p, Q_ARITH);

    let timestamp_delta = w(p, W_R_SHIFT) - w(p, W_R);
    let ram_timestamp_check_identity =
        (index_delta * minus_one + Fr::one()) * timestamp_delta - w(p, W_O);

    let memory_identity = rom_consistency_check_identity
        + ram_timestamp_check_identity * (w(p, Q_4) * w(p, Q_L))
        + memory_record_check * (w(p, Q_M) * w(p, Q_L))
        + ram_consistency_check_identity;

    let auxiliary_identity =
        (memory_identity + non_native_field_identity + limb_accumulator_identity)
            * (w(p, Q_AUX) * domain_sep);
    evals[12] = auxiliary_identity;
}

/// Poseidon2 External Relation (4 subrelations: evals[18..21])
fn accumulate_poseidon_external(
    p: &[Fr; NUMBER_OF_ENTITIES],
    evals: &mut [Fr; NUMBER_OF_SUBRELATIONS],
    domain_sep: Fr,
) {
    let s1 = w(p, W_L) + w(p, Q_L);
    let s2 = w(p, W_R) + w(p, Q_R);
    let s3 = w(p, W_O) + w(p, Q_O);
    let s4 = w(p, W_4) + w(p, Q_4);

    let u1 = s1 * s1 * s1 * s1 * s1;
    let u2 = s2 * s2 * s2 * s2 * s2;
    let u3 = s3 * s3 * s3 * s3 * s3;
    let u4 = s4 * s4 * s4 * s4 * s4;

    let t0 = u1 + u2;
    let t1 = u3 + u4;
    let t2 = u2 + u2 + t1;
    let t3 = u4 + u4 + t0;
    let v4 = (t1 + t1) * FR_TWO + t3;
    let v2 = (t0 + t0) * FR_TWO + t2;
    let v1 = t3 + v2;
    let v3 = t2 + v4;

    let q_pos = w(p, Q_POSEIDON2_EXTERNAL) * domain_sep;
    evals[18] = evals[18] + q_pos * (v1 - w(p, W_L_SHIFT));
    evals[19] = evals[19] + q_pos * (v2 - w(p, W_R_SHIFT));
    evals[20] = evals[20] + q_pos * (v3 - w(p, W_O_SHIFT));
    evals[21] = evals[21] + q_pos * (v4 - w(p, W_4_SHIFT));
}

/// Poseidon2 Internal Relation (4 subrelations: evals[22..25])
fn accumulate_poseidon_internal(
    p: &[Fr; NUMBER_OF_ENTITIES],
    evals: &mut [Fr; NUMBER_OF_SUBRELATIONS],
    domain_sep: Fr,
) {
    let diag = &POSEIDON_DIAG;

    let s1 = w(p, W_L) + w(p, Q_L);
    let u1 = s1 * s1 * s1 * s1 * s1;
    let u2 = w(p, W_R);
    let u3 = w(p, W_O);
    let u4 = w(p, W_4);

    let u_sum = u1 + u2 + u3 + u4;
    let q_pos = w(p, Q_POSEIDON2_INTERNAL) * domain_sep;

    evals[22] = evals[22] + q_pos * (u1 * diag[0] + u_sum - w(p, W_L_SHIFT));
    evals[23] = evals[23] + q_pos * (u2 * diag[1] + u_sum - w(p, W_R_SHIFT));
    evals[24] = evals[24] + q_pos * (u3 * diag[2] + u_sum - w(p, W_O_SHIFT));
    evals[25] = evals[25] + q_pos * (u4 * diag[3] + u_sum - w(p, W_4_SHIFT));
}

fn scale_and_batch(
    evaluations: &[Fr; NUMBER_OF_SUBRELATIONS],
    alphas: &[Fr; NUMBER_OF_ALPHAS],
) -> Fr {
    let mut acc = evaluations[0];
    for (eval, alpha) in evaluations[1..].iter().zip(alphas.iter()) {
        acc = acc + *eval * *alpha;
    }
    acc
}

pub fn accumulate_relation_evaluations(
    purported_evals: &[Fr; NUMBER_OF_ENTITIES],
    rp: &RelationParameters,
    alphas: &[Fr; NUMBER_OF_ALPHAS],
    pow_partial_eval: Fr,
) -> Fr {
    let evals = accumulate_relation_evaluations_raw(purported_evals, rp, pow_partial_eval);
    scale_and_batch(&evals, alphas)
}

/// Returns the raw 26 sub-relation evaluations (before scale_and_batch).
/// Used for diagnostic purposes to identify which sub-relation is wrong.
pub fn accumulate_relation_evaluations_raw(
    purported_evals: &[Fr; NUMBER_OF_ENTITIES],
    rp: &RelationParameters,
    pow_partial_eval: Fr,
) -> [Fr; NUMBER_OF_SUBRELATIONS] {
    let mut evals = [Fr::zero(); NUMBER_OF_SUBRELATIONS];

    accumulate_arithmetic(purported_evals, &mut evals, pow_partial_eval);
    accumulate_permutation(purported_evals, rp, &mut evals, pow_partial_eval);
    accumulate_log_derivative_lookup(purported_evals, rp, &mut evals, pow_partial_eval);
    accumulate_delta_range(purported_evals, &mut evals, pow_partial_eval);
    accumulate_elliptic(purported_evals, &mut evals, pow_partial_eval);
    accumulate_auxiliary(purported_evals, rp, &mut evals, pow_partial_eval);
    accumulate_poseidon_external(purported_evals, &mut evals, pow_partial_eval);
    accumulate_poseidon_internal(purported_evals, &mut evals, pow_partial_eval);

    evals
}
