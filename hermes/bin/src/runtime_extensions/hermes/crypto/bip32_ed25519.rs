use std::vec;

use crate::runtime_extensions::bindings::hermes::crypto::api::MnemonicPhrase;
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

#[allow(dead_code)]
pub fn generate_in_with<R>(
    _rng: &mut R, word_count: usize, prefix: Vec<&str>, language: Language,
) -> Result<bip39::Mnemonic, bip39::Error>
where
    R: RngCore + CryptoRng,
{
    // // Check word count
    // if is_invalid_word_count(word_count) {
    //     return Err(bip39::Error::BadWordCount(word_count));
    // }

    // FIXME - Change error type
    // prefix length is optional, but if defined, must be 3 characters or less.
    if prefix.len() > 3 {
        return Err(bip39::Error::BadWordCount(word_count));
    }

    // Dont't know the size of prefix
    // Index is type of u16
    let mut prefix_index: Vec<u8> = Vec::new();

    for word in prefix {
        match language.find_word(word) {
            Some(index) => {
                for b in decimal_to_binary_array(index) {
                    prefix_index.push(b)
                }
            },
            None => return Err(bip39::Error::BadWordCount(word_count)),
        }
    }

    println!("prefix_index {:?}", &prefix_index);

    // Number of entropy
    // eg. 12 words contain 16 entropy
    let entropy_num = (word_count / 3) * 4;
    // FIXME - Make 24 a constant
    // Maximum length of mnemonic is 24 words which is 32 entropy
    let mut entropy_num_max = [0u8; (24 / 3) * 4];

    // Random number
    let mut rng = rand::thread_rng();
    // Fill the random number into entropy_num
    rng.fill_bytes(&mut entropy_num_max[0..entropy_num]);

    let entropy: &[u8] = &entropy_num_max[0..entropy_num];

    // Create an vec that will hold binary conversion of entropy.
    let mut bits_entropy = vec![];

    // Add the prefix index bit to the bit entropy
    for i in prefix_index.clone() {
        bits_entropy.push(i);
    }
    println!("prefix_index after add to bit entropy{:?}", bits_entropy);

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

    // Number of checksum bits to be added
    let check_sum_num = word_count / 3 * 4 * 8 / 32;
    let mut hash_result = [0u8; 32];
    let mut hasher = Sha256::new();
    // Convert bits_entropy to bytes
    let bytes_entropy = bits_to_bytes(&bits_entropy);
    hasher.input(&bytes_entropy);
    hasher.result(&mut hash_result);

    println!("SHA256 of entropy, hash_result {:?}", hash_result);

    // Adding the checksum bits to the bits_entropy
    for i in 0..check_sum_num {
        bits_entropy.push(hash_result[0] >> (7 - i) & 1);
    }

    println!("bits_entropy after adding checksum bits {:?}", bits_entropy);

    let mut words_index = [0u16; 24];

    // Seperate entropy bits into 11 bits and convert to decimal
    // This decimal will be used to get the word index
    for i in 0..word_count {
        let mut idx = 0;
        for j in 0..11 {
            if bits_entropy[i * 11 + j] > 0 {
                idx += 1 << (10 - j);
            }
        }
        words_index[i] = idx;
    }
    println!("word index {:?}", words_index);

    // Get the word from word index
    let word_list = language.word_list();
    let mut mnemonic: Vec<&str> = vec![];
    for word in words_index.iter() {
        mnemonic.push(word_list[*word as usize]);
    }

    println!("words {}", mnemonic.join(" "));
    todo!()
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

pub(crate) fn _gen_new_mnemonic() -> Result<MnemonicPhrase, Error> {
    todo!()
}

#[cfg(test)]
mod tests_bip32_ed25519 {
    use super::*;

    #[test]
    fn test_mnemonic_to_xprv() {
        // Entropy of this mnemonic db587e36f8ed4b1543e27781c6c82d2d5d9a064be4d4a6e0ecae788045fcffc3
        // swap sentence misery vault start melt auto exclude limb curve area follow super add convince once plunge alter clog valve affair wrist yellow girl
        let mnemonic = vec![
            "swap", "sentence", "misery", "vault", "start", "melt", "auto", "exclude", "limb",
            "curve", "area", "follow", "super", "add", "convince", "once", "plunge", "alter",
            "clog", "valve", "affair", "wrist", "yellow", "girl",
        ];
        let mnemonic = mnemonic.join(" ");
        let xprv = mnemonic_to_xprv(&mnemonic, None).expect("Failed to convert mnemonic to xprv");
        println!("test1 {:?}", xprv.chain_code());
    }

    #[test]
    fn test_generate_mnemonic() {
        let mut rng = rand::thread_rng();
        let dest = &mut [0u8; 32];
        rng.fill_bytes(dest);
        let prefix = vec!["project", "cat"];
        let mnemonic = generate_in_with(&mut rng, 24, prefix, Language::English)
            .expect("Failed to generate mnemonic");
        println!("test2 {:?}", mnemonic);
    }
}
