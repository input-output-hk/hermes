use crate::runtime_extensions::bindings::hermes::crypto::api::MnemonicPhrase;
use anyhow::Error;
use cryptoxide::{hmac::Hmac, pbkdf2::pbkdf2, sha2::Sha512};

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
    // Check word count
    if is_invalid_word_count(word_count) {
        return Err(bip39::Error::BadWordCount(word_count));
    }

    // FIXME - Change error type
    // prefix length is optional, but if defined, must be 3 characters or less.
    if prefix.len() > 3 {
        return Err(bip39::Error::BadWordCount(word_count));
    }

    let mut prefix_index: Vec<u16> = Vec::new();

    // FIXME - Change error type
    // If index of any prefix not found, return an error.
    for word in prefix.iter() {
        match language.find_word(word) {
            Some(index) => prefix_index.push(index),
            None => return Err(bip39::Error::BadWordCount(word_count)),
        }
    }

    // 3 words => 4 entropy segment
    // 1 segment contains 32 bits
    let entropy_segment = (word_count / 3) * 4;
    // FIXME - Make 24 a constant
    // Maximum length of mnemonic is 24 words => 32 entropy segment
    let mut entropy_segment_max = [0u8; (24 / 3) * 4];
   

    let mut rng = rand::thread_rng();
    rng.fill_bytes(&mut entropy_segment_max[0..entropy_segment]);

    println!("rng {:?}", entropy_segment_max);
    todo!()

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
