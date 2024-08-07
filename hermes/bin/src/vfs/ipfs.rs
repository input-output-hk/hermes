//! IPFS virtual file system.

use std::io::{Error, ErrorKind, Read, Result, Seek, SeekFrom, Write};

use hermes_ipfs::Cid;

use crate::{ipfs::HERMES_IPFS, runtime_extensions::bindings::hermes::ipfs::api::Errno};

#[allow(dead_code)]
/// IPFS virtual file.
struct IpfsVirtualFile(Cid);

impl Read for IpfsVirtualFile {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        // Read data from IPFS and store it in `buf`.
        let mut slice = &mut buf[..];
        slice.write_all(
            HERMES_IPFS
                .file_get(&self.0.into())
                .map_err(|e| {
                    if e == Errno::InvalidCid {
                        Error::from(ErrorKind::NotFound)
                    } else {
                        tracing::error!("Error reading IPFS file: {:?}", e);
                        Error::from(ErrorKind::Other)
                    }
                })?
                .as_slice(),
        )?;
        Ok(buf.len())
    }
}

impl Write for IpfsVirtualFile {
    fn write(&mut self, _buf: &[u8]) -> Result<usize> {
        // Write data to IPFS.
        Err(ErrorKind::Unsupported.into())
    }

    fn flush(&mut self) -> Result<()> {
        // Flush data to IPFS.
        Err(ErrorKind::Unsupported.into())
    }
}

impl Seek for IpfsVirtualFile {
    fn seek(&mut self, _pos: SeekFrom) -> Result<u64> {
        // Seek to a position in the IPFS file.
        Err(ErrorKind::Unsupported.into())
    }
}
