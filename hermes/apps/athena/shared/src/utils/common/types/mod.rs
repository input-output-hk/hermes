//! Common types
//!
//! These should be simple types, not objects.
//! For example, types derived from strings or integers and vectors of simple types only.
//!
//! Objects are objects, and not types.
//!
//! Simple types can be enums, if the intended underlying type is simple, such as a string
//! or integer.

use cardano_blockchain_types::pallas_primitives;

pub(crate) mod array_types;
pub mod cardano;
pub(crate) mod generic;
pub(crate) mod headers;
pub(crate) mod string_types;

/// Converts `pallas_primitives::BigInt` to a `num_bigint::BigInt`.
///
/// This conversion correctly handles the CBOR standard encoding for Bignums (Tags 2 and
/// 3). For `BigUInt` (Tag 2), the bytes directly represent the magnitude (n).
/// For `BigNInt` (Tag 3), the bytes encode a value (n) from which the actual number is
/// derived as: Actual Value = -1 - n.
///
/// Reference: <https://datatracker.ietf.org/doc/html/rfc7049#page-16> (Section 2.4.2. Bignums)
#[must_use]
pub fn pallas_big_int_to_num_bigint(
    pallas_big_int: &pallas_primitives::BigInt
) -> num_bigint::BigInt {
    match pallas_big_int {
        pallas_primitives::BigInt::Int(int_val) => {
            let val: i128 = (*int_val).into();
            val.into()
        },
        pallas_primitives::BigInt::BigUInt(bytes) => {
            num_bigint::BigInt::from_bytes_be(num_bigint::Sign::Plus, bytes)
        },
        pallas_primitives::BigInt::BigNInt(bytes) => {
            let n = num_bigint::BigInt::from_bytes_be(num_bigint::Sign::Plus, bytes);
            -(n + num_bigint::BigInt::from(1))
        },
    }
}

#[cfg(test)]
mod tests {
    use cardano_blockchain_types::pallas_primitives::{BigInt, Int};
    use num_bigint::BigInt as NumBigInt;

    use crate::utils::common::types::pallas_big_int_to_num_bigint;

    #[test]
    fn test_int_conversion_zero() {
        let pallas_int = BigInt::Int(Int::from(0));
        let num_bigint = pallas_big_int_to_num_bigint(&pallas_int);
        assert_eq!(num_bigint, NumBigInt::from(0));
    }

    #[test]
    fn test_int_conversion_positive() {
        let pallas_int = BigInt::Int(Int::from(42));
        let num_bigint = pallas_big_int_to_num_bigint(&pallas_int);
        assert_eq!(num_bigint, NumBigInt::from(42));
    }

    #[test]
    fn test_int_conversion_negative() {
        let pallas_int = BigInt::Int(Int::from(-100));
        let num_bigint = pallas_big_int_to_num_bigint(&pallas_int);
        assert_eq!(num_bigint, NumBigInt::from(-100));
    }

    // --- Tests for BigInt::BigUInt (CBOR Tag 2) ---

    #[test]
    fn test_biguint_2_pow_64() {
        // Value: 2^64 (the example from RFC 7049, needs 9 bytes: 01 followed by 8 zeros)
        let bytes: Vec<u8> = vec![0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
        let pallas_biguint = BigInt::BigUInt(bytes.into());

        let num_bigint = pallas_big_int_to_num_bigint(&pallas_biguint);
        let expected = NumBigInt::from(2).pow(64);
        assert_eq!(num_bigint, expected, "Should correctly decode 2^64");
    }

    #[test]
    fn test_biguint_leading_zeros() {
        // Value: 256 (0x0100)
        let bytes_no_padding: Vec<u8> = vec![0x01, 0x00];
        let bytes_padded: Vec<u8> = vec![0x00, 0x01, 0x00]; // Leading zero
        let expected = NumBigInt::from(256);

        let num_bigint_a = pallas_big_int_to_num_bigint(&BigInt::BigUInt(bytes_no_padding.into()));
        let num_bigint_b = pallas_big_int_to_num_bigint(&BigInt::BigUInt(bytes_padded.into()));

        // Test that both encodings result in the same mathematical value
        assert_eq!(
            num_bigint_a, expected,
            "Should decode unpadded 0x0100 correctly"
        );
        assert_eq!(
            num_bigint_b, expected,
            "Should handle leading zeros (0x000100) correctly"
        );
    }

    // --- Tests for BigInt::BigNInt (CBOR Tag 3) ---

    #[test]
    fn test_bignint_minus_one() {
        // The value for n is 0. Actual Value = -1 - 0 = -1.
        let bytes: Vec<u8> = vec![]; // Empty byte array represents n=0
        let pallas_bignint = BigInt::BigNInt(bytes.into());

        let num_bigint = pallas_big_int_to_num_bigint(&pallas_bignint);
        assert_eq!(num_bigint, NumBigInt::from(-1), "Should decode -1");
    }

    #[test]
    fn test_bignint_minus_256() {
        // We want -256. Actual Value = -1 - n.
        // Therefore, n = -(Actual Value) - 1 = -(-256) - 1 = 256 - 1 = 255.
        // 255 is 0xFF in one byte.
        let bytes: Vec<u8> = vec![0xFF];
        let pallas_bignint = BigInt::BigNInt(bytes.into());

        let num_bigint = pallas_big_int_to_num_bigint(&pallas_bignint);
        assert_eq!(num_bigint, NumBigInt::from(-256), "Should decode -256");
    }

    #[test]
    fn test_bignint_leading_zeros() {
        // We want -17. Actual Value = -1 - n.
        // n = -(-17) - 1 = 16. 16 is 0x10.
        let bytes_no_padding: Vec<u8> = vec![0x10];
        let bytes_padded: Vec<u8> = vec![0x00, 0x10]; // Leading zero for n=16
        let expected = NumBigInt::from(-17);

        let num_bigint_a = pallas_big_int_to_num_bigint(&BigInt::BigNInt(bytes_no_padding.into()));
        let num_bigint_b = pallas_big_int_to_num_bigint(&BigInt::BigNInt(bytes_padded.into()));

        // Test that both encodings result in the same mathematical value
        assert_eq!(
            num_bigint_a, expected,
            "Should decode unpadded n=16 correctly"
        );
        assert_eq!(
            num_bigint_b, expected,
            "Should handle leading zeros for n=16 (0x0010) correctly"
        );
    }

    #[test]
    fn test_biguint_2_pow_64_from_rfc_example() {
        // Value: 2^64 (18446744073709551616). CBOR encodes the magnitude n.
        // n is represented by 9 bytes: 0x01 followed by eight 0x00 bytes.
        let bytes: Vec<u8> = vec![0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
        let pallas_biguint = BigInt::BigUInt(bytes.into());

        let num_bigint = pallas_big_int_to_num_bigint(&pallas_biguint);
        let expected = NumBigInt::from(2).pow(64);

        // This test ensures the 9-byte Big-Endian decoding is correct.
        assert_eq!(num_bigint, expected);
    }
}
