use std::vec;

use crate::runtime_extensions::bindings::hermes::{
    crypto::api::Errno, crypto::api::MnemonicPhrase,
};
use anyhow::Error;
use cryptoxide::{
    digest::Digest,
    hmac::Hmac,
    pbkdf2::pbkdf2,
    sha2::{Sha256, Sha512},
};

use bip39::{Language, Mnemonic};
use ed25519_bip32::XPrv;
use rand::{CryptoRng, RngCore};

#[allow(dead_code)]
pub(crate) fn mnemonic_to_xprv(mnemonic: &str, passphrase: Option<&str>) -> Result<XPrv, Error> {
    // Automatically detect language and check mnemonic valid length
    // 12, 15, 18, 21, 24 are valid mnemonic length
    let mnemonic = match Mnemonic::parse(mnemonic) {
        Ok(mnemonic) => mnemonic,
        Err(e) => {
            return Err(Error::new(e));
        },
    };

    let entropy = mnemonic.to_entropy();

    println!("entropy {:?}", entropy);
    let mut pbkdf2_result = [0; 96];

    const ITER: u32 = 4096;
    let passphrase = passphrase.unwrap_or_default();
    let passphrase_byte: &[u8] = passphrase.as_bytes();

    let mut mac = Hmac::new(Sha512::new(), passphrase_byte);
    pbkdf2(&mut mac, &entropy, ITER, &mut pbkdf2_result);

    Ok(XPrv::normalize_bytes_force3rd(pbkdf2_result))
}

#[allow(dead_code)]
fn is_invalid_word_count(word_count: usize) -> bool {
    word_count < 12 || word_count % 3 != 0 || word_count > 24
}

pub(crate) fn _gen_new_mnemonic() -> Result<MnemonicPhrase, Error> {
    todo!()
}

#[allow(dead_code)]
pub fn generate_in_with<R>(
    _rng: &mut R, word_count: usize, prefix: Vec<&str>, language: Language,
) -> Result<Mnemonic, Errno>
where
    R: RngCore + CryptoRng,
{
    // Check word count
    if is_invalid_word_count(word_count) {
        return Err(Errno::InvalidMnemonicLength);
    }

    // Prefix length is optional, but if defined, must be 3 characters or less.
    if prefix.len() > 3 {
        return Err(Errno::PrefixTooLong);
    }

    let prefix_index_bits = get_prefix_index_bits(prefix, language);

    println!("prefix_index {:?}", &prefix_index_bits);

    // Create an vec that will hold binary conversion of entropy.
    let mut bits_entropy = vec![];
    match prefix_index_bits {
        Ok(prefix_index_bits) => {
            // Add the prefix index bit to the bit entropy
            bits_entropy.extend_from_slice(&prefix_index_bits);
        },
        Err(e) => return Err(e),
    }

    let entropy = generate_entropy(word_count);
    println!("prefix_index after add to bit entropy{:?}", bits_entropy);

    // Entropy is in bytes, so convert to bits
    for byte in entropy {
        for j in (0..8).rev() {
            if bits_entropy.len() >= word_count / 3 * 4 * 8 {
                break;
            }
            bits_entropy.push((byte >> j) & 1);
        }
    }
    println!(
        "bits_entropy length {}, bits_entropy {:?}",
        bits_entropy.len(),
        bits_entropy
    );

    let check_sum_bits = get_check_sum_bits(&bits_entropy, word_count);
    bits_entropy.extend_from_slice(&check_sum_bits);

    println!("bits_entropy after adding checksum bits {:?}", bits_entropy);

    let word_index_vec = get_word_index_vec(&bits_entropy, word_count);
    println!("word index {:?}", word_index_vec);

    let mnemonic = get_mnemonic_from_indexs(word_index_vec, language);
    println!("mnemonic {:?}", mnemonic);

    println!("words {}", mnemonic.join(" "));
    todo!()
}

fn get_prefix_index_bits(prefix_list: Vec<&str>, language: Language) -> Result<Vec<u8>, Errno> {
    let mut prefix_index: Vec<u8> = Vec::new();
    for word in prefix_list {
        match language.find_word(word) {
            Some(index) => {
                for b in decimal_to_binary_array(index) {
                    prefix_index.push(b)
                }
            },
            None => return Err(Errno::WordNotFound),
        }
    }
    Ok(prefix_index)
}

fn generate_entropy(word_count: usize) -> Vec<u8> {
    // Number of entropy calculate from mnemonic word.
    let entropy_num = (word_count / 3) * 4;
    // Maximum length of mnemonic is 24 words which is 32 entropy.
    let mut entropy_num_max = [0u8; 32];
    // Random number
    let mut rng = rand::thread_rng();
    // Fill the random number into entropy_num.
    rng.fill_bytes(&mut entropy_num_max[0..entropy_num]);
    entropy_num_max[0..entropy_num].to_vec()
}

fn get_check_sum_bits(entropy_bits: &Vec<u8>, word_count: usize) -> Vec<u8> {
    // Number of checksum bits to be added.
    let check_sum_num = word_count / 3 * 4 * 8 / 32;
    let mut hash_result = [0u8; 32];
    let mut hasher = Sha256::new();
    // Convert bits_entropy to bytes, so it works with SHA256 hasher.
    let bytes_entropy = bits_to_bytes(entropy_bits);
    println!("bytes_entropy {:?}", bytes_entropy);
    hasher.input(&bytes_entropy);

    println!("hex str {}", hasher.clone().result_str());
    hasher.result(&mut hash_result);

    println!("SHA256 of entropy, hash_result {:?}", hash_result);
    // Retrieve the first check_sum_num check sum bits from the hash result.
    let mut check_sum_bits = Vec::new();
    for i in 0..check_sum_num {
        check_sum_bits.push(hash_result[0] >> (7 - i) & 1);
    }
    println!("check_sum_bits {:?}", check_sum_bits);
    check_sum_bits
}

fn get_word_index_vec(bits_entropy: &Vec<u8>, word_count: usize) -> Vec<u16> {
    let mut word_index_vec = Vec::new();

    // Seperate entropy bits into 11 bits and convert to decimal
    // This decimal will be used to get the word index
    for i in 0..word_count {
        let mut idx = 0;
        for j in 0..11 {
            if bits_entropy[i * 11 + j] > 0 {
                idx += 1 << (10 - j);
            }
        }
        word_index_vec.push(idx);
    }
    word_index_vec
}

fn get_mnemonic_from_indexs(word_index_vec: Vec<u16>, language: Language) -> Vec<&'static str> {
    let word_list = language.word_list();
    let mut mnemonic: Vec<&str> = vec![];
    for word in word_index_vec {
        mnemonic.push(word_list[word as usize]);
    }
    mnemonic
}

// Turns decimal into binary array of length 11
fn decimal_to_binary_array(decimal: u16) -> [u8; 11] {
    let mut binary = [0u8; 11];
    let mut n = decimal;
    let mut index = 0;

    while n > 0 {
        let bit = n % 2;
        binary[index] = bit as u8;
        index += 1;
        n /= 2;
    }

    // If the number of bits is less than 11, fill the remaining bits with 0
    while index < 11 {
        binary[index] = 0;
        index += 1;
    }

    binary.reverse();
    binary
}

// Turns bit of array to bytes of arrays
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

#[cfg(test)]
mod tests_bip32_ed25519 {
    use super::*;

    // Entropy of this mnemonic db587e36f8ed4b1543e27781c6c82d2d5d9a064be4d4a6e0ecae788045fcffc3
    const MNEMONIC_ENG: &str = "swap sentence misery vault start melt auto exclude limb curve area follow super add convince once plunge alter clog valve affair";

    const MNEMONIC_JAPANESE: &str = "こころ いどう きあつ そうがんきょう へいあん せつりつ ごうせい はいち  いびき きこく あんい おちつく きこえる けんとう たいこ すすめる はっけん ていど はんおん いんさつ うなぎ しねま れいぼう みつかる";

    #[test]
    fn test_eng_mnemonic_to_xprv() {
        // Valid mnemonic, shouldn't fail.
        mnemonic_to_xprv(&MNEMONIC_ENG, None).expect("Failed to convert English mnemonic to xprv");
    }

    #[test]
    fn test_jap_mnemonic_to_xprv() {
        // Valid mnemonic, shouldn't fail.
        mnemonic_to_xprv(&MNEMONIC_JAPANESE, None)
            .expect("Failed to convert Japanses mnemonic to xprv");
    }

    #[test]
    fn test_mnemonic_with_passphrase_to_xprv() {
        let passphrase = "test cat";
        // Valid mnemonic with passphrase, shouldn't fail.
        mnemonic_to_xprv(&MNEMONIC_ENG, Some(passphrase))
            .expect_err(&bip39::Error::BadWordCount(2).to_string());
    }

    #[test]
    fn test_mnemonic_to_xprv_invalid_length() {
        let mnemonic = "project cat test";
        mnemonic_to_xprv(&mnemonic, None).expect_err("Failed to convert mnemonic to xprv");
    }

    #[test]
    fn test_generate_mnemonic() {
        let mut rng = rand::thread_rng();
        let dest = &mut [0u8; 32];
        rng.fill_bytes(dest);
        let prefix = vec!["project", "cat"];
        let mnemonic = generate_in_with(&mut rng, 12, prefix, Language::English)
            .expect("Failed to generate mnemonic");
        println!("test2 {:?}", mnemonic);
    }
}
