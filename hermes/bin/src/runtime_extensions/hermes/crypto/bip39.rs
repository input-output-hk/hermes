//! Implementation of BIP39.

use std::vec;

use bip39::{Language, Mnemonic};
use cryptoxide::{
    digest::Digest,
    hmac::Hmac,
    pbkdf2::pbkdf2,
    sha2::{Sha256, Sha512},
};
use ed25519_bip32::XPrv;
use rand::RngCore;

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
    // Parse will detect language and check mnemonic valid length
    // 12, 15, 18, 21, 24 are valid mnemonic length
    let Ok(mnemonic) = Mnemonic::parse(mnemonic) else {
        return Err(Errno::InvalidMnemonic);
    };

    let entropy = mnemonic.to_entropy();

    // This implementation follows SLIP-0023 - Cardano HD master node derivation
    // from a master seed.
    // https://github.com/satoshilabs/slips/blob/master/slip-0023.md
    let mut pbkdf2_result = [0; 96];
    // 4096 is the number of iterations for PBKDF2.
    let passphrase_byte: &[u8] = passphrase.as_bytes();
    let mut mac = Hmac::new(Sha512::new(), passphrase_byte);
    pbkdf2(&mut mac, &entropy, 4096, &mut pbkdf2_result);

    Ok(XPrv::normalize_bytes_force3rd(pbkdf2_result))
}

/// Generate a new mnemonic.
/// Number of entropy required is calculated by `word_count` / 3 * 4.
/// eg. 24 words will have 32 entropy or 256 bits entropy.
/// Number of checksum required is calculated by `word_count` / 3 * 4 * 8 / 32.
/// eg. 24 words will have 8 checksum bits.
///
/// # Arguments
///
/// - `word_count`: The number of words in the mnemonic. Must be a multiple of
/// 3 and in the range of 12 - 24.
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

    let language = string_to_language(&language.unwrap_or("English".to_string()));

    let prefix_index_bits = match get_prefix_index_bits(prefix, language) {
        Ok(prefix_index_bits) => prefix_index_bits,
        Err(e) => return Err(e),
    };

    // Create an vec that will hold binary conversion of entropy.
    let mut bits_entropy = Vec::new();
    // Add the prefix index bit to the bit entropy.
    bits_entropy.extend_from_slice(&prefix_index_bits);

    let entropy = match generate_entropy(word_count) {
        Ok(entropy) => entropy,
        Err(e) => return Err(e),
    };

    // Convert bytes entropy to bits.
    for byte in entropy {
        for j in (0..8).rev() {
            // Should not exceed the word_count / 3 * 4 * 8
            // which is number of entropy bits for the mnemonic word count.
            if bits_entropy.len() >= word_count / 3 * 4 * 8 {
                break;
            }
            bits_entropy.push((byte >> j) & 1);
        }
    }

    let check_sum_bits = get_check_sum_bits(&bits_entropy, word_count);
    // Add the checksum bits to the end of bit entropy.
    bits_entropy.extend_from_slice(&check_sum_bits);

    let word_indices = get_word_indices(&bits_entropy, word_count);
    let mnemonic_list = get_mnemonic_from_indices(word_indices, language);

    Ok(mnemonic_list)
}

/// Check if the word count is valid.
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

/// Generate entropies and return the value.
fn generate_entropy(word_count: usize) -> Result<Vec<u8>, Errno> {
    // Number of entropy calculate from mnemonic word.
    let entropy_num = (word_count / 3) * 4;
    // Maximum length of mnemonic is 24 words which is 32 entropy.
    let mut entropy_num_max = [0u8; 32];
    // Random number
    let mut rng = rand::thread_rng();
    // Fill the random number into entropy_num.
    if let Some(slice) = entropy_num_max.get_mut(0..entropy_num) {
        rng.fill_bytes(slice);
    }

    if let Some(slice) = entropy_num_max.get(0..entropy_num) {
        Ok(slice.to_vec())
    } else {
        Err(Errno::GenerateEntropyFailed)
    }
}

/// Generate checksum bits from entropy bits.
fn get_check_sum_bits(entropy_bits: &[u8], word_count: usize) -> Vec<u8> {
    // Number of checksum bits to be added.
    let check_sum_num = word_count / 3 * 4 * 8 / 32;
    let mut hash_result = [0u8; 32];
    let mut hasher = Sha256::new();
    // Convert bits_entropy to bytes, so it works with SHA256 hasher.
    let bytes_entropy = bits_to_bytes(entropy_bits);
    hasher.input(&bytes_entropy);

    hasher.result(&mut hash_result);

    // Retrieve the first `check_sum_num` check sum bits from the hash result.
    let mut check_sum_bits = Vec::new();
    for i in 0..check_sum_num {
        check_sum_bits.push(hash_result[0] >> (7 - i) & 1);
    }
    check_sum_bits
}

/// Get the word indices from the entropy bits.
fn get_word_indices(bits_entropy: &[u8], word_count: usize) -> Vec<u16> {
    let mut word_index_vec = Vec::new();

    // Separate entropy bits into 11 bits and convert to decimal.
    // This decimal will be used to get the word index.
    for i in 0..word_count {
        let mut idx = 0;
        for j in 0..11 {
            if let Some(value) = bits_entropy.get(i * 11 + j) {
                if *value > 0 {
                    idx += 1 << (10 - j);
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
            index += 1;
            *value = bit as u8;
            n /= 2;
        }
    }

    // If the number of bits is less than 11, fill the remaining bits with 0.
    while index < 11 {
        if let Some(value) = binary.get_mut(index) {
            *value = 0;
        }
        index += 1;
    }
    binary.reverse();
    binary
}

/// Turns bit of array to bytes of arrays
fn bits_to_bytes(bits: &[u8]) -> Vec<u8> {
    bits.chunks(8)
        .map(|chunk| {
            chunk
                .iter()
                .enumerate()
                .fold(0, |acc, (i, &bit)| acc | ((bit) << (7 - i)))
        })
        .collect()
}

/// Convert string to BIP39 language.
fn string_to_language(s: &str) -> Language {
    match s {
        "SimplifiedChinese" => Language::SimplifiedChinese,
        "TraditionalChinese" => Language::TraditionalChinese,
        "Czech" => Language::Czech,
        "French" => Language::French,
        "Italian" => Language::Italian,
        "Japanese" => Language::Japanese,
        "Korean" => Language::Korean,
        "Spanish" => Language::Spanish,
        _ => Language::English,
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
        mnemonic_to_xprv(MNEMONIC_ENG, "").expect("Failed to convert English mnemonic to xprv");
    }

    #[test]
    fn test_jap_mnemonic_to_xprv() {
        // Valid mnemonic, shouldn't fail.
        mnemonic_to_xprv(MNEMONIC_JAPANESE, "")
            .expect("Failed to convert Japanese mnemonic to xprv");
    }

    #[test]
    fn test_mnemonic_with_passphrase_to_xprv() {
        let passphrase = "test cat";
        // Valid mnemonic with passphrase, shouldn't fail.
        mnemonic_to_xprv(MNEMONIC_ENG, passphrase)
            .expect("Failed to convert English mnemonic with passphrase to xprv");
    }

    #[test]
    fn test_mnemonic_to_xprv_invalid_length() {
        let mnemonic = "project cat test";
        mnemonic_to_xprv(mnemonic, "").expect_err(&bip39::Error::BadWordCount(2).to_string());
    }

    #[test]
    fn test_generate_mnemonic_prefix() {
        let mnemonic = generate_new_mnemonic(12, vec![], Some("English".to_string()))
            .expect("Failed to generate mnemonic");
        Mnemonic::parse(mnemonic.join(" ")).expect("Fail to parse mnemonic");
        let mnemonic =
            generate_new_mnemonic(12, vec!["project".to_string()], Some("English".to_string()))
                .expect("Failed to generate mnemonic");
        Mnemonic::parse(mnemonic.join(" ")).expect("Fail to parse mnemonic");
        let mnemonic = generate_new_mnemonic(
            12,
            vec!["project".to_string(), "cat".to_string()],
            Some("English".to_string()),
        )
        .expect("Failed to generate mnemonic");
        Mnemonic::parse(mnemonic.join(" ")).expect("Fail to parse mnemonic");
        let mnemonic = generate_new_mnemonic(
            12,
            vec!["project".to_string(), "cat".to_string(), "test".to_string()],
            Some("English".to_string()),
        )
        .expect("Failed to generate mnemonic");
        Mnemonic::parse(mnemonic.join(" ")).expect("Fail to parse mnemonic");
        let mnemonic = generate_new_mnemonic(
            12,
            vec!["project".to_string(), "cat".to_string(), "test".to_string()],
            None,
        )
        .expect("Failed to generate mnemonic");
        Mnemonic::parse(mnemonic.join(" ")).expect("Fail to parse mnemonic");
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
        .expect("Failed to generate mnemonic");
        Mnemonic::parse(mnemonic.join(" ")).expect("Fail to parse mnemonic");
    }

    #[test]
    fn test_generate_mnemonic_validity() {
        for _ in 0..20 {
            let prefix = vec!["project".to_string(), "cat".to_string()];
            let mnemonic = generate_new_mnemonic(12, prefix, Some("English".to_string()))
                .expect("Failed to generate mnemonic");
            Mnemonic::parse(mnemonic.join(" ")).expect("Fail to parse mnemonic");
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
