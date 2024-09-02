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

/// Helper function for decoding u64.
pub(crate) fn decode_u64(d: &mut Decoder, from: &str) -> Result<u64, decode::Error> {
    d.u64()
        .map_err(|e| decode::Error::message(&format!("Failed to decode u64 in {from}: {e}")))
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
pub(crate) fn decode_any(d: &mut Decoder) -> Result<Vec<u8>, decode::Error> {
    match d.datatype()? {
        minicbor::data::Type::Bytes => Ok(decode_bytes(d, "Any")?.clone()),
        minicbor::data::Type::String => Ok(decode_string(d, "Any")?.as_bytes().to_vec()),
        minicbor::data::Type::Array => {
            let arr_len = decode_array_len(d, "Any")?;
            let mut buffer = vec![];
            for _ in 0..arr_len {
                buffer.extend_from_slice(&decode_any(d)?);
            }
            Ok(buffer)
        },
        minicbor::data::Type::U8
        | minicbor::data::Type::U16
        | minicbor::data::Type::U32
        | minicbor::data::Type::U64 => Ok(decode_u64(d, "Any")?.to_be_bytes().to_vec()),
        minicbor::data::Type::I8
        | minicbor::data::Type::I16
        | minicbor::data::Type::I32
        | minicbor::data::Type::I64 => Ok(decode_i64(d, "Any")?.to_be_bytes().to_vec()),
        _ => Err(decode::Error::message("Data type not supported")),
    }
}
