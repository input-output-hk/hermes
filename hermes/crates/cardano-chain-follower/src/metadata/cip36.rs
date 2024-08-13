//! Decoder and Validator for CIP36 Metadata

use std::sync::Arc;

use ed25519_dalek::Verifier;
use minicbor::Decoder;
use pallas::ledger::traverse::MultiEraTx;
use tracing::debug;

use super::{
    DecodedMetadata, DecodedMetadataItem, DecodedMetadataValues, RawAuxData, ValidationReport,
};
use crate::Network;

/// CIP36 Metadata Label
pub const LABEL: u64 = 61284;
/// CIP36 Metadata Signature label
pub const SIG_LABEL: u64 = 61285;

/// Project Catalyst Purpose
pub const PROJECT_CATALYST_PURPOSE: u64 = 0;

/// Signdata Preamble = `{ 61284: ?? }`
/// CBOR Decoded =
/// A1       # map(1)
/// 19 EF64  # unsigned(61284)
pub const SIGNDATA_PREAMBLE: [u8; 4] = [0xA1, 0x19, 0xEF, 0x64];

/// Ed25519 Public Key
type Ed25519PubKey = ed25519_dalek::VerifyingKey;

/// Voting Public Key - Also known as Delegation in the CIP36 Specification
#[derive(Clone, Debug)]
pub struct VotingPubKey {
    /// Ed25519 Public Key
    pub voting_pk: Ed25519PubKey,
    /// Weight of the Voting Public Key
    pub weight: u32,
}

/// CIP 36 Registration Data.
#[derive(Clone, Debug, Default)]
pub struct Cip36 {
    /// Is this CIP36 or CIP15 format.
    #[allow(clippy::struct_field_names)]
    pub cip36: Option<bool>,
    /// Voting Keys (Called Delegations in the CIP-36 Spec)
    /// If No Voting Keys could be decoded, this will be an empty array.
    pub voting_keys: Vec<VotingPubKey>,
    /// Stake Address to associate with the Voting Keys
    pub stake_pk: Option<Ed25519PubKey>,
    /// Payment Address to associate with the Voting Keys
    /// No Payment key decoded will be an empty vec.
    pub payment_addr: Vec<u8>,
    /// Is the address able to be paid to? (Can't be a script or Stake address)
    pub payable: bool,
    /// Raw Nonce (Nonce that has not had slot correction applied)
    pub raw_nonce: u64,
    /// Nonce (Nonce that has been slot corrected)
    pub nonce: u64,
    /// Registration Purpose (Always 0 for Catalyst)
    pub purpose: u64,
    /// Signature Validates
    pub signed: bool,
    /// Strict Catalyst Validated
    pub strict_catalyst: bool,
}

impl Cip36 {
    /// Decode and validate CIP36/15 Metadata
    ///
    /// CIP15 is a subset of CIP36.
    ///
    /// See:
    /// * <https://cips.cardano.org/cip/CIP-36>
    /// * <https://github.com/cardano-foundation/CIPs/tree/master/CIP-0036>
    ///
    /// # Parameters
    /// * `decoded_metadata` - Decoded Metadata - Will be updated only if CIP36 Metadata
    ///   is found.
    /// * `slot` - Current Slot
    /// * `txn` - Transaction Aux data was attached to and to be validated/decoded
    ///   against. Not used for CIP36 Metadata.
    /// * `raw_aux_data` - Raw Auxiliary Data for the transaction.
    /// * `catalyst_strict` - Strict Catalyst Validation - otherwise Catalyst Specific
    ///   rules/workarounds are not applied.
    ///
    /// # Returns
    ///
    /// Nothing.  IF CIP36 Metadata is found it will be updated in `decoded_metadata`.
    #[allow(clippy::too_many_lines)]
    pub(crate) fn decode_and_validate(
        decoded_metadata: &DecodedMetadata, slot: u64, txn: &MultiEraTx, raw_aux_data: &RawAuxData,
        catalyst_strict: bool, chain: Network,
    ) {
        let k61284 = raw_aux_data.get_metadata(LABEL);
        let k61285 = raw_aux_data.get_metadata(SIG_LABEL);

        let mut cip36 = Cip36 {
            strict_catalyst: catalyst_strict,
            ..Default::default()
        };

        // If there is NO Cip36/Cip15 Metadata then nothing to decode or validate, so quickly
        // exit.
        if k61284.is_none() && k61285.is_none() {
            return;
        }

        // if let Some(reg) = k61284.as_ref() {
        //    debug!("CIP36 Metadata Detected: {slot}, {reg:02x?}");
        //}
        // if let Some(sig) = k61285.as_ref() {
        //    debug!("CIP36 Signature Detected: {slot}, {sig:02x?}");
        //}

        // Any Decode/Validation errors go here.
        let mut validation_report = ValidationReport::new();

        // Check if we actually have metadata to decode for the CIP36 Registration.
        let Some(raw_cip36) = k61284 else {
            cip36.decoding_failed(
                "No CIP36 Metadata found, but CIP36 Signature Metadata found.",
                &mut validation_report,
                decoded_metadata,
            );
            debug!("decoded 1: {decoded_metadata:?}");
            return;
        };

        let cip36_slice = raw_cip36.as_slice();

        let mut decoder = Decoder::new(cip36_slice);

        // It should be a definite map, get the number of entries in the map.
        let Some(cip36_map_entries) =
            cip36.decode_map_entries(&mut decoder, &mut validation_report, decoded_metadata)
        else {
            debug!("decoded 2: {decoded_metadata:?}");
            return;
        };

        let mut found_keys: Vec<u64> = Vec::new();

        for _entry in 0..cip36_map_entries {
            let Some(key) =
                cip36.decode_map_key(&mut decoder, &mut validation_report, decoded_metadata)
            else {
                debug!("decoded 3: {decoded_metadata:?} : {raw_cip36:02x?}");
                return;
            };

            if found_keys.contains(&key) {
                validation_report.push(format!("Duplicate key found in CIP36 Metadata: {key}"));
            } else {
                found_keys.push(key);
                match key {
                    1 => {
                        if cip36
                            .decode_voting_key(
                                &mut decoder,
                                &mut validation_report,
                                decoded_metadata,
                            )
                            .is_none()
                        {
                            debug!("decoded 4: {decoded_metadata:?} : {validation_report:?} : {raw_cip36:02x?}");
                            return;
                        }
                    },
                    2 => {
                        if cip36
                            .decode_stake_pub(
                                &mut decoder,
                                &mut validation_report,
                                decoded_metadata,
                            )
                            .is_none()
                        {
                            debug!("decoded 5: {decoded_metadata:?} : {validation_report:?} : {raw_cip36:02x?}");
                            return;
                        }
                    },
                    3 => {
                        if cip36
                            .decode_payment_address(
                                &mut decoder,
                                &mut validation_report,
                                decoded_metadata,
                                txn,
                                chain,
                            )
                            .is_none()
                        {
                            debug!("decoded 6: {decoded_metadata:?} : {validation_report:?} : {raw_cip36:02x?}");
                            return;
                        }
                    },
                    4 => {
                        if cip36
                            .decode_nonce(
                                &mut decoder,
                                &mut validation_report,
                                decoded_metadata,
                                slot,
                            )
                            .is_none()
                        {
                            debug!("decoded 7: {decoded_metadata:?} : {validation_report:?} : {raw_cip36:02x?}");
                            return;
                        }
                    },
                    5 => {
                        if cip36
                            .decode_purpose(&mut decoder, &mut validation_report, decoded_metadata)
                            .is_none()
                        {
                            debug!("decoded 8: {decoded_metadata:?} : {validation_report:?} : {raw_cip36:02x?}");
                            return;
                        }
                    },
                    _ => {
                        validation_report
                            .push(format!("Invalid key found in CIP36 Metadata: {key}"));
                    },
                }
            }
        }

        // Validate that all keys required to be present in the CIP36 Metadata are present.
        if !found_keys.contains(&1) {
            validation_report.push(
                "The CIP36 Metadata Voting Key/Delegation is missing from the data.".to_string(),
            );
        }
        if !found_keys.contains(&2) {
            validation_report
                .push("The CIP36 Metadata Stake Address is missing from the data.".to_string());
        }
        if !found_keys.contains(&3) {
            validation_report
                .push("The CIP36 Metadata Payment Address is missing from the data.".to_string());
        }
        if !found_keys.contains(&4) {
            validation_report
                .push("The CIP36 Metadata Nonce is missing from the data.".to_string());
        }

        if !decoded_metadata.0.is_empty() {
            debug!("decoded 9: {decoded_metadata:?}");
        }
        // If we get this far, decode the signature, and verify it.
        cip36.validate_signature(&raw_cip36, k61285, &mut validation_report, decoded_metadata);
    }

    /// Decoding of the CIP36 metadata failed, and can not continue.
    fn decoding_failed(
        &self, reason: &str, validation_report: &mut ValidationReport,
        decoded_metadata: &DecodedMetadata,
    ) {
        validation_report.push(reason.into());
        decoded_metadata.0.insert(
            LABEL,
            Arc::new(DecodedMetadataItem {
                value: DecodedMetadataValues::Cip36(Arc::new(self.clone()).clone()),
                report: validation_report.clone(),
            }),
        );
    }

    /// Decode number of entries in the CIP36 metadata map.
    fn decode_map_entries(
        &self, decoder: &mut Decoder, validation_report: &mut ValidationReport,
        decoded_metadata: &DecodedMetadata,
    ) -> Option<u64> {
        let cip36_map_entries = match decoder.map() {
            Ok(None) => {
                self.decoding_failed(
                    "CIP36 Metadata was Indefinite Map, Invalid Encoding.",
                    validation_report,
                    decoded_metadata,
                );
                return None;
            },
            Ok(Some(entries)) => entries,
            Err(error) => {
                self.decoding_failed(
                    format!("CIP36 Metadata was error decoding Map: {error}").as_str(),
                    validation_report,
                    decoded_metadata,
                );
                return None;
            },
        };

        Some(cip36_map_entries)
    }

    /// Decode the Key of an entry in the CIP36 Metadata map.
    fn decode_map_key(
        &self, decoder: &mut Decoder, validation_report: &mut ValidationReport,
        decoded_metadata: &DecodedMetadata,
    ) -> Option<u64> {
        let key = match decoder.u64() {
            Ok(key) => key,
            Err(err) => {
                self.decoding_failed(
                    format!("CIP36 Metadata was error decoding Map Entry Key: {err}").as_str(),
                    validation_report,
                    decoded_metadata,
                );
                return None;
            },
        };

        Some(key)
    }

    /// Decode the Registration Purpose in the CIP36 Metadata map.
    fn decode_purpose(
        &mut self, decoder: &mut Decoder, validation_report: &mut ValidationReport,
        decoded_metadata: &DecodedMetadata,
    ) -> Option<u64> {
        let purpose = match decoder.u64() {
            Ok(key) => key,
            Err(err) => {
                self.decoding_failed(
                    format!("Error decoding Purpose Value: {err}").as_str(),
                    validation_report,
                    decoded_metadata,
                );
                return None;
            },
        };

        if self.strict_catalyst && purpose != PROJECT_CATALYST_PURPOSE {
            validation_report.push(format!("Registration contains unknown purpose: {purpose}"));
        }

        self.purpose = purpose;

        Some(purpose)
    }

    /// Decode the Registration Nonce in the CIP36 Metadata map.
    fn decode_nonce(
        &mut self, decoder: &mut Decoder, validation_report: &mut ValidationReport,
        decoded_metadata: &DecodedMetadata, slot: u64,
    ) -> Option<u64> {
        let raw_nonce = match decoder.u64() {
            Ok(key) => key,
            Err(err) => {
                self.decoding_failed(
                    format!("Error decoding Purpose Value: {err}").as_str(),
                    validation_report,
                    decoded_metadata,
                );
                return None;
            },
        };

        let nonce = if self.strict_catalyst && raw_nonce > slot {
            slot
        } else {
            raw_nonce
        };

        self.raw_nonce = raw_nonce;
        self.nonce = nonce;

        Some(nonce)
    }

    /// Decode the Payment Address Metadata in the CIP36 Metadata map.
    fn decode_payment_address(
        &mut self, decoder: &mut Decoder, validation_report: &mut ValidationReport,
        decoded_metadata: &DecodedMetadata, _txn: &MultiEraTx, chain: Network,
    ) -> Option<usize> {
        let raw_address = match decoder.bytes() {
            Ok(address) => address,
            Err(err) => {
                self.decoding_failed(
                    format!("Error decoding Payment Address: {err}").as_str(),
                    validation_report,
                    decoded_metadata,
                );
                return None;
            },
        };

        let Some(header_byte) = raw_address.first() else {
            self.decoding_failed(
                "Error decoding Payment Address: Empty",
                validation_report,
                decoded_metadata,
            );
            return None;
        };

        // See: https://cips.cardano.org/cip/CIP-19 for details on address decoding.
        let network_tag = header_byte & 0x0F;
        let header_type = header_byte >> 4;
        match header_type {
            0..=3 => {
                if raw_address.len() != 57 {
                    validation_report.push(format!("Address Length {} != 57", raw_address.len()));
                }
            },
            4 | 5 => {
                if raw_address.len() < 29 {
                    validation_report
                        .push(format!("Pointer Address Length {} < 29", raw_address.len()));
                }
            },
            6 | 7 | 14 | 15 => {
                if raw_address.len() != 29 {
                    validation_report.push(format!(
                        "Pointer Address Length {} != 29",
                        raw_address.len()
                    ));
                }
            },
            _ => {
                validation_report.push(format!(
                    "Address Type {header_type} is invalid and unsupported"
                ));
            },
        }

        // Check address is for the correct network of the transaction.
        if header_type == 8 {
            validation_report.push("Byron Addresses are unsupported".to_string());
        } else {
            let valid = match chain {
                Network::Mainnet => network_tag == 1,
                Network::Preprod | Network::Preview => network_tag == 0,
            };
            if !valid {
                validation_report.push(format!(
                    "Network Tag {network_tag} does not match transactions Network ID"
                ));
            }
        }

        // Addresses are only payable if they are a normal payment address and not a script
        // address.
        self.payable = header_type <= 7 && (header_type & 0x1 == 0);
        self.payment_addr = raw_address.to_vec();

        Some(self.payment_addr.len())
    }

    /// Decode the Payment Address Metadata in the CIP36 Metadata map.
    fn decode_ed25519_pub_key(
        &mut self, decoder: &mut Decoder, validation_report: &mut ValidationReport,
        decoded_metadata: &DecodedMetadata, key_type: &str,
    ) -> Option<ed25519_dalek::VerifyingKey> {
        let pub_key = match decoder.bytes() {
            Ok(pub_key) => pub_key,
            Err(err) => {
                self.decoding_failed(
                    format!("Error decoding {key_type}: {err}").as_str(),
                    validation_report,
                    decoded_metadata,
                );
                return None;
            },
        };

        if pub_key.len() == ed25519_dalek::PUBLIC_KEY_LENGTH {
            // Safe to use `unwrap()` here because the length is fixed and we know it's 32 bytes
            // long.
            #[allow(clippy::unwrap_used)]
            let pub_key: [u8; ed25519_dalek::PUBLIC_KEY_LENGTH] = pub_key.try_into().unwrap();
            match ed25519_dalek::VerifyingKey::from_bytes(&pub_key) {
                Ok(pk) => return Some(pk),
                Err(error) => {
                    validation_report.push(format!("{key_type} not valid Ed25519: {error}"));
                },
            }
        } else {
            validation_report.push(format!(
                "{key_type} Length {} != {}",
                pub_key.len(),
                ed25519_dalek::PUBLIC_KEY_LENGTH
            ));
        }

        None
    }

    /// Decode the Staking Public Key in the CIP36 Metadata map.
    fn decode_stake_pub(
        &mut self, decoder: &mut Decoder, validation_report: &mut ValidationReport,
        decoded_metadata: &DecodedMetadata,
    ) -> Option<usize> {
        let pk = self.decode_ed25519_pub_key(
            decoder,
            validation_report,
            decoded_metadata,
            "Stake Public Key",
        )?;
        self.stake_pk = Some(pk);

        Some(self.stake_pk.as_slice().len())
    }

    /// Decode an individual delegation entry from the CIP36 Metadata map.
    fn decode_delegation(
        &mut self, decoder: &mut Decoder, validation_report: &mut ValidationReport,
        decoded_metadata: &DecodedMetadata,
    ) -> Option<usize> {
        match decoder.array() {
            Ok(Some(2)) => {
                let vk = self.decode_ed25519_pub_key(
                    decoder,
                    validation_report,
                    decoded_metadata,
                    "Delegation Public Key",
                )?;
                let weight = match decoder.u32() {
                    Ok(weight) => weight,
                    Err(err) => {
                        self.decoding_failed(
                            format!("Error Decoding CIP36 Delegations Entry Weight: {err}.")
                                .as_str(),
                            validation_report,
                            decoded_metadata,
                        );
                        return None;
                    },
                };

                self.voting_keys.push(VotingPubKey {
                    voting_pk: vk,
                    weight,
                });
            },
            Ok(Some(entries)) => {
                self.decoding_failed(
                    format!("Error Decoding CIP36 Delegations Entry Array: Must have exactly 2 elements, had {entries}.").as_str(),
                    validation_report,
                    decoded_metadata,
                );
                return None;
            },
            Ok(None) => {
                self.decoding_failed(
                    "Error Decoding CIP36 Delegations Entry Array: Indefinite Array is invalid encoding.",
                    validation_report,
                    decoded_metadata,
                );
                return None;
            },
            Err(err) => {
                self.decoding_failed(
                    format!("Error Decoding CIP36 Delegations Entry Array: {err}").as_str(),
                    validation_report,
                    decoded_metadata,
                );
                return None;
            },
        }

        Some(self.voting_keys.len())
    }

    /// Decode the Voting Key(s) in the CIP36 Metadata map.
    fn decode_voting_key(
        &mut self, decoder: &mut Decoder, validation_report: &mut ValidationReport,
        decoded_metadata: &DecodedMetadata,
    ) -> Option<usize> {
        match decoder.datatype() {
            Ok(key_type) => {
                match key_type {
                    minicbor::data::Type::Bytes => {
                        // CIP 15 type registration (single voting key).
                        self.cip36 = Some(false);
                        let vk = self.decode_ed25519_pub_key(
                            decoder,
                            validation_report,
                            decoded_metadata,
                            "Voting Public Key",
                        )?;
                        self.voting_keys.push(VotingPubKey {
                            voting_pk: vk,
                            weight: 1,
                        });
                    },
                    minicbor::data::Type::Array => {
                        // CIP 36 type registration (multiple voting keys).
                        self.cip36 = Some(true);
                        match decoder.array() {
                            Ok(Some(entries)) => {
                                for _entry in 0..entries {
                                    self.decode_delegation(
                                        decoder,
                                        validation_report,
                                        decoded_metadata,
                                    )?;
                                }
                            },
                            Ok(None) => {
                                self.decoding_failed(
                                "Error Decoding CIP36 Delegations Array: Indefinite Array is invalid encoding.",
                                validation_report,
                                decoded_metadata,
                            );
                            },
                            Err(err) => {
                                self.decoding_failed(
                                    format!("Error Decoding CIP36 Delegations Array: {err}")
                                        .as_str(),
                                    validation_report,
                                    decoded_metadata,
                                );
                                return None;
                            },
                        }
                    },
                    _ => {
                        self.decoding_failed(
                            format!(
                                "Error inspecting Voting Key type: Unexpected CBOR Type {key_type}"
                            )
                            .as_str(),
                            validation_report,
                            decoded_metadata,
                        );
                    },
                }
            },
            Err(error) => {
                self.decoding_failed(
                    format!("Error inspecting Voting Key type: {error}").as_str(),
                    validation_report,
                    decoded_metadata,
                );
                return None;
            },
        }

        if self.strict_catalyst && self.voting_keys.len() != 1 {
            validation_report.push(format!(
                "Catalyst Supports only a single Voting Key per registration.  Found {}",
                self.voting_keys.len()
            ));
        }

        Some(self.voting_keys.len())
    }

    /// Decode a signature from the Signature metadata in 61285
    /// Also checks that the signature is valid against the public key.
    #[allow(clippy::too_many_lines)]
    fn validate_signature(
        &mut self, metadata: &Arc<Vec<u8>>, sig_metadata: Option<Arc<Vec<u8>>>,
        validation_report: &mut ValidationReport, decoded_metadata: &DecodedMetadata,
    ) -> Option<bool> {
        // Check if we actually have metadata to decode for the CIP36 Registration.
        let Some(raw_cip36) = sig_metadata else {
            self.decoding_failed(
                "No CIP36 Signature found, but CIP36 Metadata found.",
                validation_report,
                decoded_metadata,
            );
            return None;
        };

        let cip36_slice = raw_cip36.as_slice();

        let mut decoder = Decoder::new(cip36_slice);

        match decoder.map() {
            Ok(Some(1)) => (), // Ok
            Ok(Some(x)) => {
                self.decoding_failed(
                    format!("CIP36 Signature Map decoding failed: Has {x} entries, should have 1.")
                        .as_str(),
                    validation_report,
                    decoded_metadata,
                );
                return None;
            },
            Ok(None) => {
                self.decoding_failed(
                    "CIP36 Signature Map is Indefinite. Decoding failed.",
                    validation_report,
                    decoded_metadata,
                );
                return None;
            },
            Err(err) => {
                self.decoding_failed(
                    format!("CIP36 Signature Map decoding failed: {err}").as_str(),
                    validation_report,
                    decoded_metadata,
                );
                return None;
            },
        }

        match decoder.u64() {
            Ok(1) => (), // Ok
            Ok(x) => {
                self.decoding_failed(
                    format!("CIP36 Signature Map decoding failed: Map entry was {x} MUST BE 1.")
                        .as_str(),
                    validation_report,
                    decoded_metadata,
                );
                return None;
            },
            Err(err) => {
                self.decoding_failed(
                    format!("CIP36 Signature Map Key decoding failed: {err}").as_str(),
                    validation_report,
                    decoded_metadata,
                );
                return None;
            },
        }

        let sig: ed25519_dalek::Signature = match decoder.bytes() {
            Ok(sig) => {
                match ed25519_dalek::Signature::from_slice(sig) {
                    Ok(sig) => sig,
                    Err(err) => {
                        self.decoding_failed(
                            format!("CIP36 Signature Decoding failed: {err}",).as_str(),
                            validation_report,
                            decoded_metadata,
                        );
                        return None;
                    },
                }
            },
            Err(error) => {
                self.decoding_failed(
                    format!("CIP36 Signature Decode error: {error}.",).as_str(),
                    validation_report,
                    decoded_metadata,
                );
                return None;
            },
        };

        // Ok, if we get this far then we have a valid CIP36 Signature.
        let Some(pk) = self.stake_pk else {
            self.decoding_failed(
                "CIP36 Signature Verification Failed, no Staking Public Key.",
                validation_report,
                decoded_metadata,
            );
            return None;
        };

        // Now we have both the Public Key and the signature. So calculate the hash of the
        // metadata.
        let hash = blake2b_simd::Params::new()
            .hash_length(32)
            .to_state()
            .update(&SIGNDATA_PREAMBLE)
            .update(metadata)
            .finalize();

        // debug!(
        //    "Hash = {:02x?}, pk = {:02x?}, sig = {:02x?}",
        //    hash.as_bytes(),
        //    pk.as_ref(),
        //    sig.to_bytes()
        //);

        if let Err(error) = pk.verify(hash.as_bytes(), &sig) {
            self.signed = false;
            self.decoding_failed(
                format!("CIP36 Signature Verification Failed: {error}").as_str(),
                validation_report,
                decoded_metadata,
            );
            return None;
        };

        // If we get this far then we have a valid CIP36 Signature (Doesn't mean there aren't
        // other issues).
        self.signed = true;

        // Record the fully validated Cip36 metadata
        decoded_metadata.0.insert(
            LABEL,
            Arc::new(DecodedMetadataItem {
                value: DecodedMetadataValues::Cip36(Arc::new(self.clone()).clone()),
                report: validation_report.clone(),
            }),
        );

        Some(true)
    }
}
