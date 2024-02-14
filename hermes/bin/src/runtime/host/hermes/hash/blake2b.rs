//! Implementation of blake2b and blake2bmac hash functions.

use blake2b_simd::Params;

use crate::runtime::extensions::hermes::{binary::api::Bstr, hash::api::Errno};

/// Implementation of blake2b given a buffer and outlen.
pub(crate) fn blake2b_impl(buf: &Bstr, outlen: Option<u8>) -> Result<Bstr, Errno> {
    // Default to 64 bytes Blake2b-512
    let outlen = outlen.unwrap_or(64) as usize;

    // outlen is set, but invalid when == 0
    if outlen == 0 {
        return Err(Errno::InvalidDigestByteLength);
    } else if outlen > 64 {
        return Err(Errno::HashTooBig);
    }
    let hash = Params::new()
        .hash_length(outlen)
        .to_state()
        .update(buf)
        .finalize();

    return Ok(Bstr::from(hash.as_bytes()));
}

/// Implementation of blake2bmac given a buffer, outlen, key, salt, and persona.
pub(crate) fn blake2bmac_impl(
    buf: &Bstr, outlen: Option<u8>, key: &Bstr, salt: Option<Bstr>, personal: Option<Bstr>,
) -> Result<Bstr, Errno> {
    // Default to 64 bytes Blake2b-512
    let outlen = outlen.unwrap_or(64) as usize;

    if key.len() > outlen {
        return Err(Errno::KeyTooBig);
    }

    // outlen is set, invalid when > 64
    // Omit outlen == 0, because it will failed because of key.len() > outlen
    if outlen > 64 {
        return Err(Errno::HashTooBig);
    }

    let salt = salt.unwrap_or_default();

    // salt length of blake2b should not exceeds 16 bytes
    if salt.len() > 16 {
        return Err(Errno::SaltTooBig);
    }

    let personal = personal.unwrap_or_default();

    // personal length of blake2b should not exceeds 16 bytes
    if personal.len() > 16 {
        return Err(Errno::PersonalTooBig);
    }

    let hash = Params::new()
        .hash_length(outlen)
        .key(key)
        .salt(&salt)
        .personal(&personal)
        .to_state()
        .update(buf)
        .finalize();

    return Ok(Bstr::from(hash.as_bytes()));
}

#[cfg(test)]
mod tests_blake2b {
    use hex_literal::hex;

    use super::*;

    #[test]
    fn blake2b_512() {
        let buf = Bstr::from("test test");
        let outlen = Some(64);

        let result = blake2b_impl(&buf, outlen).expect("Failed to hash blake2b-512");

        assert_eq!(
            result.as_ref(),
            hex!("8e27b2481dd1fe73d598104c03b1f67da60725abb73cf66e400177d73aee01e74b93f55adda27b0ad92e22e284b5e0cc95ad81b04b496bd58c4ae6bca5f56196")
        );
    }

    #[test]
    fn blake2b_256() {
        let buf = Bstr::from("test test");
        let outlen = Some(32);

        let result = blake2b_impl(&buf, outlen).expect("Failed to hash blake2b-256");

        assert_eq!(
            result.as_ref(),
            hex!("7f3dc1170e7017a1643d84d102429c4c7aec4ca99c016c32af18af997fed51f1")
        );
    }
    #[test]
    fn blake2b_512_with_default_outlen() {
        let buf = Bstr::from("test test");

        let result = blake2b_impl(&buf, None).expect("Failed to hash blake2b-512 default outlen");

        assert_eq!(
            result.as_ref(),
            hex!("8e27b2481dd1fe73d598104c03b1f67da60725abb73cf66e400177d73aee01e74b93f55adda27b0ad92e22e284b5e0cc95ad81b04b496bd58c4ae6bca5f56196")
        );
    }

    #[test]
    fn blake2b_zero_outlen_err() {
        let buf = Bstr::from("test test");
        let outlen = Some(0);

        let result =
            blake2b_impl(&buf, outlen).expect_err(Errno::InvalidDigestByteLength.message());
    }

    #[test]
    fn blake2b_hash_too_big_err() {
        let buf = Bstr::from("test test");
        let outlen = Some(100);

        let result = blake2b_impl(&buf, outlen).expect_err(Errno::HashTooBig.message());
    }
    #[test]
    fn blake2bmac_512() {
        let buf = Bstr::from("test test");
        let key = Bstr::from("key");
        let outlen = Some(64);

        let result =
            blake2bmac_impl(&buf, outlen, &key, None, None).expect("Failed to hash blake2bmac-512");

        assert_eq!(
        result.as_ref(),
        hex!("c28029cbab4e11d759e971d7e2a13dbe9ef60d2fa539cc03138b0432c3fdb2757b6c87383bd1074f5533c0c2ad2a5d2ac71bbd96f0f8fbb4c3ba0d4abb309115"));
    }

    #[test]
    fn blake2bmac_512_key_too_big_err() {
        let buf = Bstr::from("test test");
        let key = Bstr::from("key".repeat(22));
        let outlen = Some(10);

        let result =
            blake2bmac_impl(&buf, outlen, &key, None, None).expect_err(Errno::KeyTooBig.message());
    }

    #[test]
    fn blake2bmac_zero_outlen_key_too_big_err() {
        let buf = Bstr::from("test test");
        let key = Bstr::from("key");
        let outlen = Some(0);

        let result =
            blake2bmac_impl(&buf, outlen, &key, None, None).expect_err(Errno::KeyTooBig.message());
    }

    #[test]
    fn blake2bmac_hash_too_big_err() {
        let buf = Bstr::from("test test");
        let key = Bstr::from("key");
        let outlen = Some(100);

        let result =
            blake2bmac_impl(&buf, outlen, &key, None, None).expect_err(Errno::HashTooBig.message());
    }

    #[test]
    fn blake2bmac_salt_too_big_err() {
        let buf = Bstr::from("test test");
        let key = Bstr::from("key");
        let salt = Bstr::from("salt".repeat(6));
        let outlen = Some(64);

        let result = blake2bmac_impl(&buf, outlen, &key, Some(salt), None)
            .expect_err(Errno::SaltTooBig.message());
    }

    #[test]
    fn blake2bmac_personal_too_big_err() {
        let buf = Bstr::from("test test");
        let key = Bstr::from("key");
        let personal = Bstr::from("personal".repeat(16));
        let outlen = Some(64);

        let result = blake2bmac_impl(&buf, outlen, &key, None, Some(personal))
            .expect_err(Errno::PersonalTooBig.message());
    }
}
