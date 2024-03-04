use crate::runtime_extensions::bindings::hermes::crypto::api::{
     MnemonicPhrase, Passphrase,
};
use bip39::{Language, Mnemonic};
use ed25519_bip32::XPrv;
use hkdf::Hkdf;
use sha2::Sha512;
fn is_valid_mnemonic_length(mnemonic: MnemonicPhrase) -> bool {
    let word_count = mnemonic.split_whitespace().count();
    match word_count {
        12 | 15 | 18 | 21 | 24 => true,
        _ => false,
    }
}

fn determine_mnemonic_language(mnemonic: MnemonicPhrase) -> Language {
    let mnemonic = Mnemonic::language_of(mnemonic.as_str());
    Ok(mnemonic.language())
}

pub(crate) fn mnemonic_to_xprv(
    mnemonic: MnemonicPhrase, passphrase: Option<Passphrase>,
) -> XPrv {
    if !is_valid_mnemonic_length(mnemonic) {
        todo!();
    }

    let language = determine_mnemonic_language(mnemonic);

    let mnemonic = match Mnemonic::parse_in(language, mnemonic) {
        Ok(mnemonic) => mnemonic,
        Err(_) => todo!(),
    };

    let passphrase = passphrase.unwrap_or_default();

    let seed = mnemonic.to_seed(passphrase);

    // Chain code 32 bytes
    
}
