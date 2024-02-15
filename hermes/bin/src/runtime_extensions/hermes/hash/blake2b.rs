//! Implementation of blake2b and blake2bmac hash functions.

use blake2b_simd::Params;

use crate::runtime_extensions::bindings::hermes::{binary::api::Bstr, hash::api::Errno};

/// Implementation of the Blake2b hash function.
///
/// # Arguments
///
/// - `buf`: A reference to the byte string for which the hash is to be computed.
/// - `outlen`: Optional output length in bytes. If not specified,
/// it defaults to 64 bytes.
///
/// # Returns
///
/// Returns the `Blake2b` hash of the byte string as a `Result`.
/// If the hash computation is successful, it returns `Ok` with the hash value.
/// If there is an error during the computation, it returns `Err` with an `Errno`.
///
/// # Errors
///
/// - `InvalidDigestByteLength`: If `outlen` is 0.
/// - `HashTooBig`: If `outlen` is greater than 64.
pub(crate) fn blake2b_impl(buf: &Bstr, outlen: Option<u8>) -> Result<Bstr, Errno> {
    // Default to 64 bytes Blake2b-512
    let outlen: usize = outlen.unwrap_or(64).into();

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

    return Ok(hash.as_bytes().into());
}

/// Implementation of the Blake2b Message Authentication Code.
///
/// # Arguments
///
/// - `buf`: A reference to the byte string for which the blake2bMac is to be computed.
/// - `outlen`: Optional output length in bytes. If not specified,
/// it defaults to 64 bytes.
/// - `key`: A reference to the byte string used as the key for computing the blake2bMac.
/// - `salt`: Optional salt value. If not specified, it defaults to an empty byte string.
/// - `personal`: Optional personalization string. If not specified,
/// it defaults to an empty byte string.
///
/// # Returns
///
/// Returns the `Blake2bMac` hash of the byte string as a `Result`.
/// If the hash computation is successful, it returns `Ok` with the hash value.
/// If there is an error during the computation, it returns `Err` with an `Errno`.
///
/// # Errors
///
/// - `KeyTooBig`: If the length of the key exceeds the specified output length.
/// - `HashTooBig`: If the specified output length exceeds 64 bytes.
/// - `SaltTooBig`: If the length of the salt exceeds 16 bytes.
/// - `PersonalTooBig`: If the length of the personalization string exceeds 16 bytes.
pub(crate) fn blake2bmac_impl(
    buf: &Bstr, outlen: Option<u8>, key: &Bstr, salt: Option<Bstr>, personal: Option<Bstr>,
) -> Result<Bstr, Errno> {
    // Default to 64 bytes Blake2bMac-512
    let outlen: usize = outlen.unwrap_or(64).into();

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

    return Ok(hash.as_bytes().into());
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

        blake2b_impl(&buf, outlen).expect_err(Errno::InvalidDigestByteLength.message());
    }

    #[test]
    fn blake2b_hash_too_big_err() {
        let buf = Bstr::from("test test");
        let outlen = Some(100);

        blake2b_impl(&buf, outlen).expect_err(Errno::HashTooBig.message());
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

        blake2bmac_impl(&buf, outlen, &key, None, None).expect_err(Errno::KeyTooBig.message());
    }

    #[test]
    fn blake2bmac_zero_outlen_key_too_big_err() {
        let buf = Bstr::from("test test");
        let key = Bstr::from("key");
        let outlen = Some(0);

        blake2bmac_impl(&buf, outlen, &key, None, None).expect_err(Errno::KeyTooBig.message());
    }

    #[test]
    fn blake2bmac_hash_too_big_err() {
        let buf = Bstr::from("test test");
        let key = Bstr::from("key");
        let outlen = Some(100);

        blake2bmac_impl(&buf, outlen, &key, None, None).expect_err(Errno::HashTooBig.message());
    }

    #[test]
    fn blake2bmac_salt_too_big_err() {
        let buf = Bstr::from("test test");
        let key = Bstr::from("key");
        let salt = Bstr::from("salt".repeat(6));
        let outlen = Some(64);

        blake2bmac_impl(&buf, outlen, &key, Some(salt), None)
            .expect_err(Errno::SaltTooBig.message());
    }

    #[test]
    fn blake2bmac_personal_too_big_err() {
        let buf = Bstr::from("test test");
        let key = Bstr::from("key");
        let personal = Bstr::from("personal".repeat(16));
        let outlen = Some(64);

        blake2bmac_impl(&buf, outlen, &key, None, Some(personal))
            .expect_err(Errno::PersonalTooBig.message());
    }
}
