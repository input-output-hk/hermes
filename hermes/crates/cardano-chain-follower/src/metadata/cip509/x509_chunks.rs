//! X509 chunks handler where compressed chunks are decompressed and decoded.

use std::io::Read;

use minicbor::{decode, Decode, Decoder};
use strum::FromRepr;

use super::{decode_helper::decode_u8, rbac::Cip509RbacMetadata};
use crate::metadata::cip509::decode_helper::{decode_array_len, decode_bytes};

/// Enum of compression algorithms used to compress chunks.
#[derive(FromRepr, Debug, PartialEq, Clone, Default)]
#[repr(u8)]
pub enum CompressionAlgorithm {
    /// Raw data, no compression.
    #[default]
    Raw = 10,
    /// Brotli compression.
    Brotli = 11,
    /// Zstd compression.
    Zstd = 12,
}

/// Struct of x509 chunks.
#[derive(Debug, PartialEq, Clone, Default)]
pub struct X509Chunks(pub Cip509RbacMetadata);

#[allow(dead_code)]
impl X509Chunks {
    /// Create new instance of `X509Chunks`.
    fn new(chunk_data: Cip509RbacMetadata) -> Self {
        Self(chunk_data)
    }
}

impl Decode<'_, ()> for X509Chunks {
    fn decode(d: &mut Decoder, _ctx: &mut ()) -> Result<Self, decode::Error> {
        // Determine the algorithm
        let algo = decode_u8(d, "algorithm in X509Chunks")?;
        let algorithm = CompressionAlgorithm::from_repr(algo)
            .ok_or(decode::Error::message("Invalid chunk data type"))?;

        // Decompress the data
        let decompressed = decompress(d, &algorithm)
            .map_err(|e| decode::Error::message(format!("Failed to decompress {e}")))?;

        // Decode the decompressed data.
        let mut decoder = Decoder::new(&decompressed);
        let chunk_data = Cip509RbacMetadata::decode(&mut decoder, &mut ())
            .map_err(|e| decode::Error::message(format!("Failed to decode {e}")))?;

        Ok(X509Chunks(chunk_data))
    }
}

/// Decompress the data using the given algorithm.
fn decompress(d: &mut Decoder, algorithm: &CompressionAlgorithm) -> anyhow::Result<Vec<u8>> {
    let chunk_len = decode_array_len(d, "decompression in X509Chunks")?;
    // Vector containing the concatenated chunks
    let mut concat_chunk = vec![];
    for _ in 0..chunk_len {
        let chunk_data = decode_bytes(d, "decompression in X509Chunks")?;
        concat_chunk.extend_from_slice(&chunk_data);
    }

    let mut buffer = vec![];

    match algorithm {
        CompressionAlgorithm::Raw => {
            buffer.extend_from_slice(concat_chunk.as_slice());
        },
        CompressionAlgorithm::Zstd => {
            zstd::stream::copy_decode(concat_chunk.as_slice(), &mut buffer)?;
        },
        CompressionAlgorithm::Brotli => {
            let mut decoder = brotli::Decompressor::new(concat_chunk.as_slice(), 4096);
            decoder
                .read_to_end(&mut buffer)
                .map_err(|_| anyhow::anyhow!("Failed to decompress using Brotli algorithm"))?;
        },
    }
    Ok(buffer)
}
