//! Implementation of BIP39.

use std::vec;

use bip39::{Language, Mnemonic};
use ed25519_bip32::XPrv;
use hmac::Hmac;
use pbkdf2::pbkdf2;
use rand::RngCore;
use sha2::{Digest, Sha256, Sha512};

use crate::runtime_extensions::bindings::hermes::crypto::api::Errno;

/// Generate a new extended private key (`XPrv`) from a mnemonic and passphrase.
///
/// # Arguments
///
/// - `mnemonic`: A string representing the mnemonic.
/// - `passphrase`: An optional string representing the passphrase.
///
/// # Returns
///
/// Returns the `XPrv` extended private key as a `Result`.
/// If the conversion is successful, it returns `Ok` with the extended private key
/// (`XPrv`).
///
/// # Errors
///
/// - `InvalidMnemonic`: If the mnemonic should be either 12, 15, 18, 21, or 24.
pub(crate) fn mnemonic_to_xprv(mnemonic: &str, passphrase: &str) -> Result<XPrv, Errno> {
    /// 4096 is the number of iterations for PBKDF2.
    const ITER: u32 = 4096;

    // Parse will detect language and check mnemonic valid length
    // 12, 15, 18, 21, 24 are valid mnemonic length
    let mnemonic = Mnemonic::parse(mnemonic).map_err(|_| Errno::InvalidMnemonic)?;

    let entropy = mnemonic.to_entropy();

    // This implementation follows SLIP-0023 - Cardano HD master node derivation
    // from a master seed.
    // https://github.com/satoshilabs/slips/blob/master/slip-0023.md
    let mut pbkdf2_result = [0; 96];
    let _ = pbkdf2::<Hmac<Sha512>>(passphrase.as_bytes(), &entropy, ITER, &mut pbkdf2_result);

    Ok(XPrv::normalize_bytes_force3rd(pbkdf2_result))
}

/// Generate a new mnemonic.
/// Number of bytes entropy required is calculated by `word_count` / 3 * 4.
/// eg. 24 words will have 32 bytes entropy or 256 bits entropy.
/// Number of checksum required is calculated by `word_count` / 3.
/// eg. 24 words will have 8 checksum bits.
///
/// # Arguments
///
/// - `word_count`: The number of words in the mnemonic. Must be a multiple of 3 and in
///   the range of 12 - 24.
/// - `prefix`: A vector of strings representing the prefix, empty if no prefix.
/// - `language`: The language of the mnemonic.
///
/// # Returns
///
/// Returns the mnemonic of type `String` as a `Result`.
/// If the conversion is successful, it returns `Ok` with the mnemonic.
/// If there is an error during the computation, it returns `Err` with an `Errno`.
///
/// # Errors
///
/// - `InvalidMnemonicLength`: If the word count is not a multiple of 3 or not in the
///   range of 12 - 24.
/// - `PrefixTooLong`: If the prefix is longer than the maximum allowed length, max is 3.
/// - `WordNotFound`: If a word in the mnemonic is not found in the word list.
pub(crate) fn generate_new_mnemonic(
    word_count: usize, prefix: Vec<String>, language: Option<String>,
) -> Result<Vec<String>, Errno> {
    // Check word count
    if is_invalid_word_count(word_count) {
        return Err(Errno::InvalidMnemonicLength);
    }

    // Number of prefix word should be <= 3.
    if prefix.len() > 3 {
        return Err(Errno::PrefixTooLong);
    }

    let language = language.map_or_else(
        || string_to_language("English"),
        |lang| string_to_language(&lang),
    )?;

    let prefix_index_bits = get_prefix_index_bits(prefix, language)?;

    // Create an vec that will hold binary conversion of entropy.
    let mut entropy_bits = Vec::new();
    // Add the prefix index bit to the bit entropy.
    entropy_bits.extend_from_slice(&prefix_index_bits);

    let entropy = generate_entropy(word_count)?;

    // Convert bytes entropy to bits.
    byte_to_bit(entropy, &mut entropy_bits, word_count);

    let check_sum_bits = get_check_sum_bits(&entropy_bits, word_count);
    // Add the checksum bits to the end of bit entropy.
    entropy_bits.extend_from_slice(&check_sum_bits);

    let word_indices = get_word_indices(&entropy_bits, word_count);
    let mnemonic_list = get_mnemonic_from_indices(word_indices, language);

    Ok(mnemonic_list)
}

/// Check if the word count is invalid.
/// Valid word count is a multiple of 3 and in the range of 12 - 24.
/// Returns true if the word count is invalid, otherwise false.
fn is_invalid_word_count(word_count: usize) -> bool {
    word_count < 12 || word_count % 3 != 0 || word_count > 24
}

/// Get the index bits of the prefix words from a BIP39 dictionary.
fn get_prefix_index_bits(prefix_list: Vec<String>, language: Language) -> Result<Vec<u8>, Errno> {
    let mut prefix_index: Vec<u8> = Vec::new();
    for word in prefix_list {
        match language.find_word(&word) {
            Some(index) => {
                for b in decimal_to_binary_array(index) {
                    prefix_index.push(b);
                }
            },
            None => return Err(Errno::WordNotFound),
        }
    }
    Ok(prefix_index)
}

/// Generate entropy bytes and return the value.
///
/// Calculation:
/// Each word in a BIP39 mnemonic phrase represents 11 bits of information. The total
/// number of bits in the mnemonic phrase (including both entropy and checksum) is
/// therefore:
///
/// `total_bits` = `word_count` * 11
///
/// The total number of bits includes both the entropy bits and the checksum bits. The
/// length of the checksum is defined as:
///
/// `checksum_len` = `entropy_bits` / 32
///
/// Therefore, the total number of bits can be written as:
///
/// `total_bits` = `entropy_bits` + `checksum_len`
///              = `entropy_bits` + `entropy_bits` / 32
///              = `entropy_bits` * (1 + 1/32)
///              = `entropy_bits` * 33 / 32
///
/// Since `total_bits` is also equal to `word_count * 11`, we have:
///
/// `word_count` * 11 = `entropy_bits` * 33 / 32
///
/// Solving for `entropy_bits`, we get:
///
/// `entropy_bits` = `word_count` * 11 * 32 / 33
///
/// To find the number of entropy bytes, we need to divide the entropy bits by 8:
///
/// `total_entropy_bytes` = `entropy_bits` / 8
///
/// Simplifying further, we get:
///
/// `total_entropy_bytes` = (`word_count` * 11 * 32 / 33) / 8
///                       = `word_count` * 11 * 4 / 33
///                       = `word_count` * 4 / 3
///
/// However, since the number of mnemonic words is always a multiple of 3
/// (in BIP39, valid word counts are 12, 15, 18, 21, or 24),
/// we can simplify this to:
///
/// `total_entropy_bytes` = `word_count` * 4 / 3
///
/// Note that if entropy bits is needed, multiply the `total_entropy_bytes` by 8.
fn generate_entropy(word_count: usize) -> Result<Vec<u8>, Errno> {
    // Number of bytes entropy calculate from mnemonic word.
    let total_entropy_bytes = word_count.saturating_mul(4) / 3;
    // Maximum length of mnemonic is 24 words which is 32 bytes entropy.
    let mut total_entropy_bytes_max = [0u8; 32];
    // Random number
    let mut rng = rand::thread_rng();
    // Fill the random number into total_entropy_bytes.
    if let Some(slice) = total_entropy_bytes_max.get_mut(0..total_entropy_bytes) {
        rng.fill_bytes(slice);
    }

    if let Some(slice) = total_entropy_bytes_max.get(0..total_entropy_bytes) {
        Ok(slice.to_vec())
    } else {
        Err(Errno::GenerateEntropyFailed)
    }
}

/// Generate checksum bits from entropy bits.
#[allow(clippy::indexing_slicing)]
fn get_check_sum_bits(entropy_bits: &[u8], word_count: usize) -> Vec<u8> {
    let checksum_len = word_count / 3;
    // Convert entropy_bits to bytes, so it works with SHA256 hasher.
    let entropy_bytes = bits_to_bytes(entropy_bits);

    let hash_result = Sha256::digest(entropy_bytes);

    // Retrieve the first `checksum_len` checksum bits from the hash result.
    let mut checksum_bits = Vec::new();
    for i in 0..checksum_len {
        checksum_bits.push(hash_result[0] >> (7usize.saturating_sub(i)) & 1);
    }
    checksum_bits
}

/// Get the word indices from the entropy bits.
fn get_word_indices(entropy_bits: &[u8], word_count: usize) -> Vec<u16> {
    let mut word_index_vec = Vec::new();

    // Separate entropy bits into 11 bits and convert to decimal.
    // This decimal will be used to get the word index.
    for i in 0..word_count {
        let mut idx = 0u16;
        for j in 0..11 {
            if let Some(value) = entropy_bits.get(i.saturating_mul(11).saturating_add(j)) {
                if *value > 0 {
                    idx = idx.saturating_add(1 << (10usize.saturating_sub(j)));
                }
            }
        }
        word_index_vec.push(idx);
    }
    word_index_vec
}

/// Get the mnemonic from the BIP39 dictionary using word indices.
fn get_mnemonic_from_indices(word_index_vec: Vec<u16>, language: Language) -> Vec<String> {
    let word_list = language.word_list();
    let mut mnemonic: Vec<String> = vec![];
    for word in word_index_vec {
        if let Some(word) = word_list.get(word as usize) {
            mnemonic.push((*(word)).to_string());
        }
    }
    mnemonic
}

/// Turns decimal into binary array of length 11.
fn decimal_to_binary_array(decimal: u16) -> [u8; 11] {
    let mut binary = [0u8; 11];
    let mut n = decimal;
    let mut index = 0;

    while n > 0 {
        if let Some(value) = binary.get_mut(index) {
            let bit = n % 2;
            index = index.saturating_add(1);
            *value = bit as u8;
            n /= 2;
        }
    }

    // If the number of bits is less than 11, fill the remaining bits with 0.
    while index < 11 {
        if let Some(value) = binary.get_mut(index) {
            *value = 0;
        }
        index = index.saturating_add(1);
    }
    binary.reverse();
    binary
}

/// Turns bit of array to bytes of arrays
fn bits_to_bytes(bits: &[u8]) -> Vec<u8> {
    bits.chunks(8)
        .map(|chunk| {
            chunk.iter().enumerate().fold(0, |acc, (i, &bit)| {
                acc | ((bit) << (7usize.saturating_sub(i)))
            })
        })
        .collect()
}

/// Convert string to BIP39 language.
fn string_to_language(s: &str) -> Result<Language, Errno> {
    match s {
        "English" => Ok(Language::English),
        "SimplifiedChinese" => Ok(Language::SimplifiedChinese),
        "TraditionalChinese" => Ok(Language::TraditionalChinese),
        "Czech" => Ok(Language::Czech),
        "French" => Ok(Language::French),
        "Italian" => Ok(Language::Italian),
        "Japanese" => Ok(Language::Japanese),
        "Korean" => Ok(Language::Korean),
        "Spanish" => Ok(Language::Spanish),
        _ => Err(Errno::UnsupportedLanguage),
    }
}

/// Convert bytes entropy to bits.
fn byte_to_bit(entropy: Vec<u8>, entropy_bits: &mut Vec<u8>, word_count: usize) {
    for byte in entropy {
        for j in (0..8).rev() {
            // Should not exceed the word_count / 3 * 32
            // which is number of entropy bits for the mnemonic word count.
            if entropy_bits.len() >= word_count.saturating_div(3).saturating_mul(32) {
                break;
            }
            entropy_bits.push((byte >> j) & 1);
        }
    }
}

#[cfg(test)]
mod tests_bip39 {
    use super::*;

    // English test vector is from https://cips.cardano.org/cip/CIP-0011
    // Entropy can be checked from https://iancoleman.io/bip39
    const MNEMONIC_ENG: &str = "prevent company field green slot measure chief hero apple task eagle sunset endorse dress seed";
    // Japanese test is from https://github.com/rust-bitcoin/rust-bip39/blob/master/src/lib.rs
    const MNEMONIC_JAPANESE: &str = "こころ いどう きあつ そうがんきょう へいあん せつりつ ごうせい はいち  いびき きこく あんい おちつく きこえる けんとう たいこ すすめる はっけん ていど はんおん いんさつ うなぎ しねま れいぼう みつかる";

    #[test]
    fn test_eng_mnemonic_to_xprv() {
        // Valid mnemonic, shouldn't fail.
        mnemonic_to_xprv(MNEMONIC_ENG, "").unwrap();
    }

    #[test]
    fn test_jap_mnemonic_to_xprv() {
        // Valid mnemonic, shouldn't fail.
        mnemonic_to_xprv(MNEMONIC_JAPANESE, "").unwrap();
    }

    #[test]
    fn test_mnemonic_with_passphrase_to_xprv() {
        let passphrase = "test cat";
        // Valid mnemonic with passphrase, shouldn't fail.
        mnemonic_to_xprv(MNEMONIC_ENG, passphrase).unwrap();
    }

    #[test]
    fn test_mnemonic_to_xprv_invalid_length() {
        let mnemonic = "project cat test";
        mnemonic_to_xprv(mnemonic, "").expect_err(&bip39::Error::BadWordCount(2).to_string());
    }

    #[test]
    fn test_generate_mnemonic_prefix() {
        let mnemonic = generate_new_mnemonic(12, vec![], Some("English".to_string())).unwrap();
        Mnemonic::parse(mnemonic.join(" ")).unwrap();
        let mnemonic =
            generate_new_mnemonic(12, vec!["project".to_string()], Some("English".to_string()))
                .unwrap();
        Mnemonic::parse(mnemonic.join(" ")).unwrap();
        let mnemonic = generate_new_mnemonic(
            12,
            vec!["project".to_string(), "cat".to_string()],
            Some("English".to_string()),
        )
        .unwrap();
        Mnemonic::parse(mnemonic.join(" ")).unwrap();
        let mnemonic = generate_new_mnemonic(
            12,
            vec!["project".to_string(), "cat".to_string(), "test".to_string()],
            Some("English".to_string()),
        )
        .unwrap();
        Mnemonic::parse(mnemonic.join(" ")).unwrap();
        let mnemonic = generate_new_mnemonic(
            12,
            vec!["project".to_string(), "cat".to_string(), "test".to_string()],
            None,
        )
        .unwrap();
        Mnemonic::parse(mnemonic.join(" ")).unwrap();
    }
    #[test]
    // Disable unicode error for linter
    #[allow(clippy::unicode_not_nfc)]
    fn test_generate_mnemonic_prefix_japanese() {
        let mnemonic = generate_new_mnemonic(
            12,
            vec!["たいみんぐ".to_string(), "うけたまわる".to_string()],
            Some("Japanese".to_string()),
        )
        .unwrap();
        Mnemonic::parse(mnemonic.join(" ")).unwrap();
    }

    #[test]
    fn test_generate_mnemonic_validity() {
        for _ in 0..20 {
            let prefix = vec!["project".to_string(), "cat".to_string()];
            let mnemonic = generate_new_mnemonic(12, prefix, Some("English".to_string())).unwrap();
            Mnemonic::parse(mnemonic.join(" ")).unwrap();
        }
    }

    #[test]
    fn test_generate_mnemonic_with_prefix_too_long() {
        let prefix = vec![
            "project".to_string(),
            "cat".to_string(),
            "test".to_string(),
            "long".to_string(),
        ];
        generate_new_mnemonic(12, prefix, Some("English".to_string()))
            .expect_err(&format!("{:?}", Errno::PrefixTooLong));
    }

    #[test]
    fn test_generate_mnemonic_invalid_length() {
        generate_new_mnemonic(3, vec![], Some("English".to_string()))
            .expect_err(&format!("{:?}", Errno::InvalidMnemonicLength));
    }
    #[test]
    fn test_generate_mnemonic_prefix_word_not_found() {
        generate_new_mnemonic(12, vec!["abc".to_string()], Some("English".to_string()))
            .expect_err(&format!("{:?}", Errno::WordNotFound));
    }
}
