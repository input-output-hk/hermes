//! Cbor decoding helper functions for CIP-509 metadata.

use minicbor::{data::Tag, decode, Decoder};

/// Helper function for decoding map.
pub(crate) fn decode_map_len(d: &mut Decoder, from: &str) -> Result<u64, decode::Error> {
    d.map()
        .map_err(|e| decode::Error::message(&format!("Failed to decode map in {from}: {e}")))?
        .ok_or(decode::Error::message(&format!(
            "Failed to decode map in {from}, unexpected indefinite length",
        )))
}

/// Helper function for decoding u8.
pub(crate) fn decode_u8(d: &mut Decoder, from: &str) -> Result<u8, decode::Error> {
    d.u8()
        .map_err(|e| decode::Error::message(&format!("Failed to decode u8 in {from}: {e}")))
}

/// Helper function for decoding u16.
pub(crate) fn decode_u16(d: &mut Decoder, from: &str) -> Result<u16, decode::Error> {
    d.u16()
        .map_err(|e| decode::Error::message(&format!("Failed to decode u16 in {from}: {e}")))
}

/// Helper function for decoding u32.
pub(crate) fn decode_u32(d: &mut Decoder, from: &str) -> Result<u32, decode::Error> {
    d.u32()
        .map_err(|e| decode::Error::message(&format!("Failed to decode u32 in {from}: {e}")))
}

/// Helper function for decoding u64.
pub(crate) fn decode_u64(d: &mut Decoder, from: &str) -> Result<u64, decode::Error> {
    d.u64()
        .map_err(|e| decode::Error::message(&format!("Failed to decode u64 in {from}: {e}")))
}

/// Helper function for decoding i8.
pub(crate) fn decode_i8(d: &mut Decoder, from: &str) -> Result<i8, decode::Error> {
    d.i8()
        .map_err(|e| decode::Error::message(&format!("Failed to decode i8 in {from}: {e}")))
}

/// Helper function for decoding i16.
pub(crate) fn decode_i16(d: &mut Decoder, from: &str) -> Result<i16, decode::Error> {
    d.i16()
        .map_err(|e| decode::Error::message(&format!("Failed to decode i16 in {from}: {e}")))
}

/// Helper function for decoding i32.
pub(crate) fn decode_i32(d: &mut Decoder, from: &str) -> Result<i32, decode::Error> {
    d.i32()
        .map_err(|e| decode::Error::message(&format!("Failed to decode i32 in {from}: {e}")))
}

/// Helper function for decoding i64.
pub(crate) fn decode_i64(d: &mut Decoder, from: &str) -> Result<i64, decode::Error> {
    d.i64()
        .map_err(|e| decode::Error::message(&format!("Failed to decode i64 in {from}: {e}")))
}

/// Helper function for decoding string.
pub(crate) fn decode_string(d: &mut Decoder, from: &str) -> Result<String, decode::Error> {
    d.str()
        .map(std::borrow::ToOwned::to_owned)
        .map_err(|e| decode::Error::message(&format!("Failed to decode string in {from}: {e}")))
}

/// Helper function for decoding bytes.
pub(crate) fn decode_bytes(d: &mut Decoder, from: &str) -> Result<Vec<u8>, decode::Error> {
    d.bytes()
        .map(<[u8]>::to_vec)
        .map_err(|e| decode::Error::message(&format!("Failed to decode bytes in {from}: {e}")))
}

/// Helper function for decoding array.
pub(crate) fn decode_array_len(d: &mut Decoder, from: &str) -> Result<u64, decode::Error> {
    d.array()
        .map_err(|e| decode::Error::message(&format!("Failed to decode array in {from}: {e}")))?
        .ok_or(decode::Error::message(&format!(
            "Failed to decode array in {from}, unexpected indefinite length",
        )))
}

/// Helper function for decoding tag.
pub(crate) fn decode_tag(d: &mut Decoder, from: &str) -> Result<Tag, decode::Error> {
    d.tag()
        .map_err(|e| decode::Error::message(&format!("Failed to decode tag in {from}: {e}")))
}

/// Decode any in CDDL, only support basic datatype
pub(crate) fn decode_any(d: &mut Decoder, from: &str) -> Result<Vec<u8>, decode::Error> {
    match d.datatype()? {
        minicbor::data::Type::Bytes => Ok(decode_bytes(d, &format!("{from} Any"))?),
        minicbor::data::Type::String => {
            Ok(decode_string(d, &format!("{from} Any"))?
                .as_bytes()
                .to_vec())
        },
        minicbor::data::Type::Array => {
            Ok(decode_array_len(d, &format!("{from} Any"))?
                .to_be_bytes()
                .to_vec())
        },
        minicbor::data::Type::U8 => {
            Ok(decode_u8(d, &format!("{from} Any"))?.to_be_bytes().to_vec())
        },
        minicbor::data::Type::U16 => {
            Ok(decode_u16(d, &format!("{from} Any"))?
                .to_be_bytes()
                .to_vec())
        },
        minicbor::data::Type::U32 => {
            Ok(decode_u32(d, &format!("{from} Any"))?
                .to_be_bytes()
                .to_vec())
        },
        minicbor::data::Type::U64 => {
            Ok(decode_u64(d, &format!("{from} Any"))?
                .to_be_bytes()
                .to_vec())
        },
        minicbor::data::Type::I8 => {
            Ok(decode_i8(d, &format!("{from} Any"))?.to_be_bytes().to_vec())
        },
        minicbor::data::Type::I16 => {
            Ok(decode_i16(d, &format!("{from} Any"))?
                .to_be_bytes()
                .to_vec())
        },
        minicbor::data::Type::I32 => {
            Ok(decode_i32(d, &format!("{from} Any"))?
                .to_be_bytes()
                .to_vec())
        },
        minicbor::data::Type::I64 => {
            Ok(decode_i64(d, &format!("{from} Any"))?
                .to_be_bytes()
                .to_vec())
        },
        _ => {
            Err(decode::Error::message(&format!(
                "{from} Any, Data type not supported"
            )))
        },
    }
}

#[cfg(test)]
mod tests {

    use minicbor::Encoder;

    use super::*;

    #[test]
    fn test_decode_any_bytes() {
        let mut buf = Vec::new();
        let mut e = Encoder::new(&mut buf);
        e.bytes(&[1, 2, 3, 4]).expect("Error encoding bytes");

        let mut d = Decoder::new(&buf);
        let result = decode_any(&mut d, "test").expect("Error decoding bytes");
        assert_eq!(result, vec![1, 2, 3, 4]);
    }

    #[test]
    fn test_decode_any_string() {
        let mut buf = Vec::new();
        let mut e = Encoder::new(&mut buf);
        e.str("hello").expect("Error encoding string");

        let mut d = Decoder::new(&buf);
        let result = decode_any(&mut d, "test").expect("Error decoding string");
        assert_eq!(result, b"hello".to_vec());
    }

    #[test]
    fn test_decode_any_array() {
        // The array should contain a supported type
        let mut buf = Vec::new();
        let mut e = Encoder::new(&mut buf);
        e.array(2).expect("Error encoding array");
        e.u8(1).expect("Error encoding u8");
        e.u8(2).expect("Error encoding u8");
        let mut d = Decoder::new(&buf);
        let result = decode_any(&mut d, "test").expect("Error decoding array");
        // The decode of array is just a length of the array
        assert_eq!(
            u64::from_be_bytes(result.try_into().expect("Error converting bytes to u64")),
            2
        );
    }

    #[test]
    fn test_decode_any_u32() {
        let mut buf = Vec::new();
        let mut e = Encoder::new(&mut buf);
        let num: u32 = 123_456_789;
        e.u32(num).expect("Error encoding u32");

        let mut d = Decoder::new(&buf);
        let result = decode_any(&mut d, "test").expect("Error decoding u32");
        assert_eq!(
            u32::from_be_bytes(result.try_into().expect("Error converting bytes to u32")),
            num
        );
    }

    #[test]
    fn test_decode_any_i32() {
        let mut buf = Vec::new();
        let mut e = Encoder::new(&mut buf);
        let num: i32 = -123_456_789;
        e.i32(num).expect("Error encoding i32");

        let mut d = Decoder::new(&buf);
        let result = decode_any(&mut d, "test").expect("Error decoding i32");
        assert_eq!(
            i32::from_be_bytes(result.try_into().expect("Error converting bytes to i32")),
            num
        );
    }

    #[test]
    fn test_decode_any_unsupported_type() {
        let mut buf = Vec::new();
        let mut e = Encoder::new(&mut buf);
        e.null().expect("Error encoding null"); // Encode a null type which is unsupported

        let mut d = Decoder::new(&buf);
        let result = decode_any(&mut d, "test");
        // Should print out the error message with the location of the error
        println!("{result:?}");
        assert!(result.is_err());
    }
}
