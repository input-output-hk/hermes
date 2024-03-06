use crate::runtime_extensions::bindings::hermes::crypto::api::{MnemonicPhrase, Passphrase};
use anyhow::Error;
use cryptoxide::{hmac::Hmac, pbkdf2::pbkdf2, sha2::Sha512};

use bip39::Mnemonic;
use ed25519_bip32::XPrv;

#[allow(dead_code)]
pub(crate) fn mnemonic_to_xprv(mnemonic: MnemonicPhrase, passphrase: Option<Passphrase>) -> Result<XPrv, Error> {
    // Automatically detect language and check mnemonic valid length
    // 12, 15, 18, 21, 24 are valid mnemonic length
    let mnemonic = match Mnemonic::parse(mnemonic) {
        Ok(mnemonic) => mnemonic,
        Err(e) => {
            return Err(Error::new(e));
        },
    };

    let entropy = mnemonic.to_entropy();

    let mut pbkdf2_result = [0; 96];

    const ITER: u32 = 4096;
    let passphrase = passphrase.unwrap_or_default();
    let passphrase_byte: &[u8] = passphrase.as_bytes();

    let mut mac = Hmac::new(Sha512::new(), passphrase_byte);
    pbkdf2(&mut mac, &entropy, ITER, &mut pbkdf2_result);

    Ok(XPrv::normalize_bytes_force3rd(pbkdf2_result))
}


#[cfg(test)]
mod tests_bip32_ed25519 {
    use super::*;

    #[test]
    fn test_mnemonic_to_xprv() {
        let mnemonic = MnemonicPhrase::from("swap sentence misery vault start melt auto exclude limb curve area follow super add convince once plunge alter clog valve affair wrist yellow girl");
        let xprv = mnemonic_to_xprv(mnemonic.clone(), None).expect("Failed to convert mnemonic to xprv");
        println!("test1 {:?}", xprv.chain_code());
    }
}
