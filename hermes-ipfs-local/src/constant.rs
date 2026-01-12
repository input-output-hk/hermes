//! Define constant needed for IPFS and Document Sync

/// Current document synchronization protocol version.
#[allow(dead_code)]
pub(crate) const PROTOCOL_VERSION: u8 = 1;

/// `CID` version that Doc Sync supports.
#[allow(dead_code)]
pub(crate) const CID_VERSION: u8 = 1;

/// `CID` multihash digest size that Doc Sync supports.
#[allow(dead_code)]
pub(crate) const CID_DIGEST_SIZE: u8 = 32;

/// Multihash SHA256.
#[allow(dead_code)]
pub(crate) const MULTIHASH_SHA256: u8 = 0x12;

/// Codec CBOR.
pub(crate) const CODEC_CBOR: u8 = 0x51;
