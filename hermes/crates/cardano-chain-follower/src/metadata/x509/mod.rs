//! x509 metadata
//! Doc Reference: <https://github.com/input-output-hk/catalyst-CIPs/tree/x509-envelope-metadata/CIP-XXXX>
//! CDDL Reference: <https://github.com/input-output-hk/catalyst-CIPs/blob/x509-envelope-metadata/CIP-XXXX/x509-envelope.cddl>

mod rbac;

use std::io::Read;

use minicbor::{decode, Decode, Decoder};
use rbac::X509RbacMetadata;
use strum::FromRepr;

/// Enum of compression algorithms used to compress chunks.
#[derive(FromRepr, Debug, PartialEq)]
#[repr(u8)]
pub enum CompressionAlgorithm {
    /// Raw data, no compression.
    Raw = 10,
    /// Brotli compression.
    Brotli = 11,
    /// Zstd compression.
    Zstd = 12,
}

/// Struct of x509 chunks.
#[derive(Debug, PartialEq)]
struct X509Chunks {
    /// The compression algorithm used to compress the data.
    chunk_type: CompressionAlgorithm,
    /// The decompressed data.
    chunk_data: X509RbacMetadata,
}

impl X509Chunks {
    /// Create new instance of `X509Chunks`.
    fn new(chunk_type: CompressionAlgorithm, chunk_data: X509RbacMetadata) -> Self {
        Self {
            chunk_type,
            chunk_data,
        }
    }
}

impl Decode<'_, ()> for X509Chunks {
    fn decode(d: &mut Decoder, _ctx: &mut ()) -> Result<Self, decode::Error> {
        // Determine the algorithm
        let algorithm = CompressionAlgorithm::from_repr(d.u8()?)
            .ok_or(decode::Error::message("Invalid chunk data type"))?;

        // Decompress the data
        let decompressed = decompress(d, &algorithm)
            .map_err(|e| decode::Error::message(format!("Failed to decompress {e}")))?;

        println!("Decompressed data: {:?}", hex::encode(&decompressed));
        // Decode the decompressed data.
        let mut decoder = Decoder::new(&decompressed);
        let chunk_data = X509RbacMetadata::decode(&mut decoder, &mut ())
            .map_err(|e| decode::Error::message(format!("Failed to decode {e}")))?;

        Ok(X509Chunks {
            chunk_type: algorithm,
            chunk_data,
        })
    }
}

/// Decompress the data using the given algorithm.
fn decompress(d: &mut Decoder, algorithm: &CompressionAlgorithm) -> anyhow::Result<Vec<u8>> {
    let chunk_len = d
        .array()
        .map_err(|e| anyhow::anyhow!(e.to_string()))?
        .ok_or(anyhow::anyhow!("Error indefinite array in X509Chunks"))?;
    // Vector containing the concatenated chunks
    let mut concat_chunk = vec![];
    for _ in 0..chunk_len {
        let chunk_data = d.bytes().map_err(|e| anyhow::anyhow!(e.to_string()))?;
        concat_chunk.extend_from_slice(chunk_data);
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
            println!("Decompressed data: {:?}", hex::encode(&buffer));
        },
    }
    Ok(buffer)
}

/// x509 metadatum.
#[derive(Debug, PartialEq)]
pub(crate) struct X509Metadatum {
    /// `UUIDv4` Purpose .
    purpose: [u8; 16], // (bytes .size 16)
    /// Transaction inputs hash.
    txn_inputs_hash: [u8; 16], // bytes .size 16
    /// Optional previous transaction ID.
    prv_tx_id: Option<[u8; 32]>, // bytes .size 32
    /// x509 chunks.
    x509_chunks: X509Chunks, // chunk_type => [ + x509_chunk ]
    /// Validation signature.
    validation_signature: Vec<u8>, // bytes size (1..64)
}

#[allow(clippy::module_name_repetitions)]
impl X509Metadatum {
    /// Create a new instance of `X509Metadatum`.
    fn new() -> Self {
        Self {
            purpose: [0; 16],
            txn_inputs_hash: [0; 16],
            prv_tx_id: None,
            x509_chunks: X509Chunks::new(CompressionAlgorithm::Raw, X509RbacMetadata::new()),
            validation_signature: vec![],
        }
    }

    /// Set the purpose.
    fn set_purpose(&mut self, purpose: [u8; 16]) {
        self.purpose = purpose;
    }

    /// Set the transaction inputs hash.
    fn set_txn_inputs_hash(&mut self, txn_inputs_hash: [u8; 16]) {
        self.txn_inputs_hash = txn_inputs_hash;
    }

    /// Set the previous transaction ID.
    fn set_prv_tx_id(&mut self, prv_tx_id: [u8; 32]) {
        self.prv_tx_id = Some(prv_tx_id);
    }

    /// Set the x509 chunks.
    fn set_x509_chunks(&mut self, x509_chunks: X509Chunks) {
        self.x509_chunks = x509_chunks;
    }

    /// Set the validation signature.
    fn set_validation_signature(&mut self, validation_signature: Vec<u8>) {
        self.validation_signature = validation_signature;
    }
}

/// Enum of x509 metadatum with its associated unsigned integer value.
#[allow(clippy::module_name_repetitions)]
#[derive(FromRepr, Debug, PartialEq)]
#[repr(u8)]
pub enum X509MetadatumInt {
    /// Purpose.
    Purpose = 0,
    /// Transaction inputs hash.
    TxInputsHash = 1,
    /// Previous transaction ID.
    PreviousTxId = 2,
    /// Validation signature.
    ValidationSignature = 99,
}

impl Decode<'_, ()> for X509Metadatum {
    fn decode(d: &mut Decoder, ctx: &mut ()) -> Result<Self, decode::Error> {
        let map_len = d.map()?.ok_or(decode::Error::message(
            "Error indefinite array in X509Metadatum",
        ))?;
        let mut x509_metadatum = X509Metadatum::new();
        for _ in 0..map_len {
            // Use probe to peak
            let key = d.probe().u8()?;

            if let Some(key) = X509MetadatumInt::from_repr(key) {
                // Consuming the int
                d.u8()?;
                match key {
                    X509MetadatumInt::Purpose => {
                        x509_metadatum.set_purpose(
                            d.bytes()?.try_into().map_err(|_| {
                                decode::Error::message("Invalid data size of Purpose")
                            })?,
                        );
                        println!("purpose: {:?}", x509_metadatum.purpose);
                    },
                    X509MetadatumInt::TxInputsHash => {
                        x509_metadatum.set_txn_inputs_hash(d.bytes()?.try_into().map_err(
                            |_| decode::Error::message("Invalid data size of TxInputsHash"),
                        )?);
                        println!("txn_inputs_hash: {:?}", x509_metadatum.txn_inputs_hash);
                    },
                    X509MetadatumInt::PreviousTxId => {
                        x509_metadatum.set_prv_tx_id(d.bytes()?.try_into().map_err(|_| {
                            decode::Error::message("Invalid data size of PreviousTxId")
                        })?);
                        println!("prv_tx_id: {:?}", x509_metadatum.prv_tx_id);
                    },
                    X509MetadatumInt::ValidationSignature => {
                        let validation_signature = d.bytes()?;
                        if validation_signature.is_empty() || validation_signature.len() > 64 {
                            return Err(decode::Error::message(
                                "Invalid data size of ValidationSignature",
                            ));
                        }
                        x509_metadatum.set_validation_signature(validation_signature.to_vec());
                    },
                }
            } else {
                // Handle the x509 chunks 10 11 12
                let x509_chunks = X509Chunks::decode(d, ctx)?;
                x509_metadatum.set_x509_chunks(x509_chunks);
            }
        }
        Ok(x509_metadatum)
    }
}

/// Decode any in CDDL, only support basic datatype
pub(crate) fn decode_any(d: &mut Decoder) -> Result<Vec<u8>, decode::Error> {
    match d.datatype()? {
        minicbor::data::Type::Bytes => Ok(d.bytes()?.to_vec()),
        minicbor::data::Type::String => Ok(d.str()?.as_bytes().to_vec()),
        minicbor::data::Type::Array => {
            let arr_len = d.array()?.ok_or(decode::Error::message(
                "Error indefinite length in decoding any",
            ))?;
            let mut buffer = vec![];
            for _ in 0..arr_len {
                buffer.extend_from_slice(&decode_any(d)?);
            }
            Ok(buffer)
        },
        minicbor::data::Type::U8
        | minicbor::data::Type::U16
        | minicbor::data::Type::U32
        | minicbor::data::Type::U64 => Ok(d.u64()?.to_be_bytes().to_vec()),
        minicbor::data::Type::I8
        | minicbor::data::Type::I16
        | minicbor::data::Type::I32
        | minicbor::data::Type::I64 => Ok(d.i64()?.to_be_bytes().to_vec()),
        _ => Err(decode::Error::message("Data type not supported")),
    }
}
