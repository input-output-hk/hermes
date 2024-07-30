mod rbac;

use std::io::Read;

use minicbor::{decode, Decode, Decoder};
use rbac::X509RbacMetadata;
use strum::FromRepr;

#[derive(FromRepr, Debug, PartialEq)]
#[repr(u8)]
pub enum CompressionAlgorithm {
    Raw = 10,
    Brotli = 11,
    Zstd = 12,
}

#[derive(Debug, PartialEq)]
struct X509Chunks {
    chunk_type: CompressionAlgorithm,
    chunk_data: X509RbacMetadata,
}

impl X509Chunks {
    fn new(chunk_type: CompressionAlgorithm, chunk_data: X509RbacMetadata) -> Self {
        Self {
            chunk_type,
            chunk_data,
        }
    }
}

impl Decode<'_, ()> for X509Chunks {
    fn decode(d: &mut Decoder, _ctx: &mut ()) -> Result<Self, decode::Error> {
        let _map_len = d
            .map()?
            .ok_or(decode::Error::message("Data should be type Map"))?;

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

        Ok(X509Chunks::new(algorithm, chunk_data))
    }
}

pub fn decompress(d: &mut Decoder, algorithm: &CompressionAlgorithm) -> anyhow::Result<Vec<u8>> {
    let chunk_len = d
        .array()
        .map_err(|e| anyhow::anyhow!(e.to_string()))?
        .ok_or(anyhow::anyhow!("Error indefinite arrays"))?;
    let mut concat_chunk = vec![];
    for _ in 0..chunk_len {
        let chunk_data = d.bytes().map_err(|e| anyhow::anyhow!(e.to_string()))?;
        concat_chunk.extend_from_slice(&chunk_data);
    }

    let mut buffer = vec![];

    match algorithm {
        CompressionAlgorithm::Raw => {
            buffer.extend_from_slice(concat_chunk.as_slice());
        },
        CompressionAlgorithm::Zstd => {
            zstd::stream::copy_decode(concat_chunk.as_slice(), &mut buffer)?
        },
        CompressionAlgorithm::Brotli => {
            let mut decoder = brotli::Decompressor::new(concat_chunk.as_slice(), 4096);
            decoder
                .read_to_end(&mut buffer)
                .map_err(|_| anyhow::anyhow!("Failed to decompress"))?;
            println!("Decompressed data: {:?}", hex::encode(&buffer));
        },
    }
    Ok(buffer)
}

#[derive(Debug, PartialEq)]
pub(crate) struct X509Metadatum {
    purpose: [u8; 16],             // uuidv4 (bytes .size 16)
    txn_inputs_hash: [u8; 28],     // bytes .size 28
    prv_tx_id: Option<[u8; 32]>,   // bytes .size 32
    x509_chunks: X509Chunks,       // chunk_type => [ + x509_chunk ]
    validation_signature: Vec<u8>, // bytes size (1..64)
}

#[allow(dead_code)]
impl X509Metadatum {
    fn new() -> Self {
        Self {
            purpose: [0; 16],
            txn_inputs_hash: [0; 28],
            prv_tx_id: None,
            x509_chunks: X509Chunks::new(CompressionAlgorithm::Raw, X509RbacMetadata::new()),
            validation_signature: vec![],
        }
    }

    fn set_purpose(&mut self, purpose: [u8; 16]) {
        self.purpose = purpose;
    }

    fn set_txn_inputs_hash(&mut self, txn_inputs_hash: [u8; 28]) {
        self.txn_inputs_hash = txn_inputs_hash;
    }

    fn set_prv_tx_id(&mut self, prv_tx_id: [u8; 32]) {
        self.prv_tx_id = Some(prv_tx_id);
    }

    fn set_x509_chunks(&mut self, x509_chunks: X509Chunks) {
        self.x509_chunks = x509_chunks;
    }

    fn set_validation_signature(&mut self, validation_signature: Vec<u8>) {
        self.validation_signature = validation_signature;
    }
}

#[derive(FromRepr, Debug, PartialEq)]
#[repr(u8)]
pub enum X509MetadatumInt {
    Purpose = 0,
    TxInputsHash = 1,
    PreviousTxId = 2,
    ValidationSignature = 99,
}

#[allow(unused)]
impl Decode<'_, ()> for X509Metadatum {
    fn decode(d: &mut Decoder, ctx: &mut ()) -> Result<Self, decode::Error> {
        let map_len = d
            .map()?
            .ok_or(decode::Error::message("Data should be type Map"))?;
        let mut x509_metadatum = X509Metadatum::new();
        for data in 0..map_len {
            if d.datatype()? == minicbor::data::Type::U8 {
                match X509MetadatumInt::from_repr(d.u8()?)
                    .ok_or(decode::Error::message("Invalid x509 metatdatum"))?
                {
                    X509MetadatumInt::Purpose => {
                        x509_metadatum.set_purpose(
                            d.bytes()?
                                .try_into()
                                .map_err(|_| decode::Error::message("Invalid data size"))?,
                        );
                        println!("purpose: {:?}", x509_metadatum.purpose);
                    },
                    X509MetadatumInt::TxInputsHash => {
                        x509_metadatum.set_txn_inputs_hash(
                            d.bytes()?
                                .try_into()
                                .map_err(|_| decode::Error::message("Invalid data size"))?,
                        );
                        println!("txn_inputs_hash: {:?}", x509_metadatum.txn_inputs_hash);
                    },
                    X509MetadatumInt::PreviousTxId => {
                        x509_metadatum.set_prv_tx_id(
                            d.bytes()?
                                .try_into()
                                .map_err(|_| decode::Error::message("Invalid data size"))?,
                        );
                        println!("prv_tx_id: {:?}", x509_metadatum.prv_tx_id);
                    },

                    X509MetadatumInt::ValidationSignature => {
                        let validation_signature = d.bytes()?;
                        if validation_signature.len() < 1 || validation_signature.len() > 64 {
                            return Err(decode::Error::message("Invalid data size"));
                        }
                        x509_metadatum.set_validation_signature(validation_signature.to_vec());
                    },
                }
            } else if d.datatype()? == minicbor::data::Type::Map {
                let chunks = X509Chunks::decode(d, ctx)?;
                x509_metadatum.set_x509_chunks(chunks);
            }
        }
        Ok(x509_metadatum)
    }
}
