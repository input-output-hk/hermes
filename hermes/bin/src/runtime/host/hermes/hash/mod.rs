//! Host - Hash implementations
#![allow(unused_variables)]

use blake2::digest::{consts::U64, FixedOutput, Update, VariableOutput};
use blake2::{Blake2bMac, Blake2bVar};

use crate::runtime::extensions::{
    hermes::{
        binary::api::Bstr,
        hash::api::{Errno, Host},
    },
    HermesState, Stateful,
};

/// State
pub(crate) struct State {}

impl Stateful for State {
    fn new(_ctx: &crate::wasm::context::Context) -> Self {
        State {}
    }
}

fn blake2b_impl(buf: Bstr, outlen: Option<u8>) -> Result<Bstr, Errno> {
    // Default to 64 bytes Blake2b-512
    let outlen = outlen.unwrap_or(64) as usize;

    // outlen is set
    // invalid when == 0 or > 64
    if outlen == 0 {
        return Err(Errno::InvalidDigestByteLength);
    } else if outlen > 64 {
        return Err(Errno::HashTooBig);
    }

    // Create an vector of length outlen
    let mut output = vec![0u8; outlen];
    let mut hasher: Blake2bVar = match Blake2bVar::new(outlen) {
        Ok(hasher) => hasher,
        Err(_) => {
            return Err(Errno::HashTooBig);
        },
    };
    hasher.update(&buf);
    match hasher.finalize_variable(&mut output) {
        Ok(_) => {},
        Err(_) => {
            return Err(Errno::HashTooBig);
        },
    }
    return Ok(Bstr::from(output));
}

fn blake2bmac_impl(
    buf: Bstr, outlen: Option<u8>, key: Bstr, salt: Option<Bstr>, persona: Option<Bstr>,
) -> Result<Bstr, Errno> {
    // Convert outlen to usize
    let outlen = outlen.unwrap_or(64) as usize;

    // outlen is set
    // invalid when == 0 or > 64
    if outlen == 0 {
        return Err(Errno::InvalidDigestByteLength);
    } else if outlen > 64 {
        return Err(Errno::HashTooBig);
    }

    let salt = salt.unwrap_or_default();
    let persona = persona.unwrap_or_default();
    let mut hasher = match Blake2bMac::<U64>::new_with_salt_and_personal(&key, &salt, &persona) {
        Ok(hasher) => hasher,
        Err(_) => {
            return Err(Errno::HashTooBig);
        },
    };

    hasher.update(&buf);
    return Ok(Bstr::from(hasher.finalize_fixed().to_vec()));
}

impl Host for HermesState {
    /// Hash a binary buffer with BLAKE2s
    fn blake2s(&mut self, buf: Bstr, outlen: Option<u8>) -> wasmtime::Result<Result<Bstr, Errno>> {
        todo!()
    }

    /// Hash a binary buffer with `BLAKE2s` with `MAC` (Message Authentication Code) mode
    fn blake2smac(
        &mut self, buf: Bstr, outlen: Option<u8>, key: Bstr, salt: Option<Bstr>,
        persona: Option<Bstr>,
    ) -> wasmtime::Result<Result<Bstr, Errno>> {
        todo!()
    }

    /// Hash a binary buffer with `BLAKE2b`
    fn blake2b(&mut self, buf: Bstr, outlen: Option<u8>) -> wasmtime::Result<Result<Bstr, Errno>> {
        let hash = blake2b_impl(buf, outlen);
        match hash {
            Ok(hash) => Ok(Ok(hash)),
            Err(err) => Err(err.into()),
        }
    }

    /// Hash a binary buffer with `BLAKE2b` with `MAC` (Message Authentication Code) mode
    fn blake2bmac(
        &mut self, buf: Bstr, outlen: Option<u8>, key: Bstr, salt: Option<Bstr>,
        persona: Option<Bstr>,
    ) -> wasmtime::Result<Result<Bstr, Errno>> {
        let hash = blake2bmac_impl(buf, outlen, key, salt, persona);
        match hash {
            Ok(hash) => Ok(Ok(hash)),
            Err(err) => Err(err.into()),
        }
    }

    /// Hash a binary buffer with BLAKE3
    fn blake3(
        &mut self, buf: Bstr, outlen: Option<u8>, key: Option<Bstr>,
    ) -> wasmtime::Result<Result<Bstr, Errno>> {
        todo!()
    }
}

#[cfg(test)]
mod tests_blake2b {
    use super::*;
    use hex_literal::hex;

    #[test]
    fn blake2b_512() {
        let buf = Bstr::from("test test");
        let outlen = Some(64);

        let result = blake2b_impl(buf, outlen).expect("Failed to hash blake2b-512");

        assert_eq!(
            result.as_ref(),
            hex!("8e27b2481dd1fe73d598104c03b1f67da60725abb73cf66e400177d73aee01e74b93f55adda27b0ad92e22e284b5e0cc95ad81b04b496bd58c4ae6bca5f56196")
        );
    }

    #[test]
    fn blake2b_256() {
        let buf = Bstr::from("test test");
        let outlen = Some(32);

        let result = blake2b_impl(buf, outlen).expect("Failed to hash blake2b-256");

        assert_eq!(
            result.as_ref(),
            hex!("7f3dc1170e7017a1643d84d102429c4c7aec4ca99c016c32af18af997fed51f1")
        );
    }
    #[test]
    fn blake2b_512_with_default_outlen() {
        let buf = Bstr::from("test test");

        let result = blake2b_impl(buf, None).expect("Failed to hash blake2b-512 default outlen");

        assert_eq!(
            result.as_ref(),
            hex!("8e27b2481dd1fe73d598104c03b1f67da60725abb73cf66e400177d73aee01e74b93f55adda27b0ad92e22e284b5e0cc95ad81b04b496bd58c4ae6bca5f56196")
        );
    }

    #[test]
    fn blake2b_0_outlen_err() {
        let buf = Bstr::from("test test");
        let outlen = Some(0);

        let result = blake2b_impl(buf, outlen);
        assert_eq!(result.unwrap_err(), Errno::InvalidDigestByteLength)
    }

    #[test]
    fn blake2b_hash_too_big_err() {
        let buf = Bstr::from("test test");
        let outlen = Some(100);

        let result = blake2b_impl(buf, outlen);
        assert_eq!(result.unwrap_err(), Errno::HashTooBig)
    }
    #[test]
    fn blake2bmac_512() {
        let buf = Bstr::from("test test");
        let key = Bstr::from("key");
        let outlen = Some(64);

        let result =
            blake2bmac_impl(buf, outlen, key, None, None).expect("Failed to hash blake2bmac-512");

        assert_eq!(
        result.as_ref(),
        hex!("c28029cbab4e11d759e971d7e2a13dbe9ef60d2fa539cc03138b0432c3fdb2757b6c87383bd1074f5533c0c2ad2a5d2ac71bbd96f0f8fbb4c3ba0d4abb309115")
    );
    }

    #[test]
    fn blake2bmac_512_salt_persona() {
        let buf = Bstr::from("test test");
        let key = Bstr::from("key");
        let salt = Bstr::from("salt");
        let persona = Bstr::from("persona");
        let outlen = Some(64);

        let result = blake2bmac_impl(buf, outlen, key, Some(salt), Some(persona))
            .expect("Failed to hash blake2bmac-512");

        assert_eq!(
        result.as_ref(),
        hex!("c28029cbab4e11d759e971d7e2a13dbe9ef60d2fa539cc03138b0432c3fdb2757b6c87383bd1074f5533c0c2ad2a5d2ac71bbd96f0f8fbb4c3ba0d4abb309115")
    );
    }

    #[test]
    fn blake2bmac_512_0_key() {
        let buf = Bstr::from("test test");
        let key = vec![];
        let outlen = Some(64);

        let result = blake2bmac_impl(buf, outlen, key, None, None)
            .expect("Failed to hash blake2bmac-512 with emprty key");

        assert_eq!(
            result.as_ref(),
            hex!("8e27b2481dd1fe73d598104c03b1f67da60725abb73cf66e400177d73aee01e74b93f55adda27b0ad92e22e284b5e0cc95ad81b04b496bd58c4ae6bca5f56196")
        );
    }
}
