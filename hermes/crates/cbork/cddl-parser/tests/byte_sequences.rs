// cspell: words hexpair rstuvw abcdefghijklmnopqrstuvwyz rstuvw Xhhb Bhcm

use cddl_parser::cddl_test::Rule;

mod common;
use common::byte_sequences::*;

#[test]
/// Test if the `HEX_PAIR` rule passes properly.
fn check_hexpair() {
    common::check_tests_rule(Rule::HEX_PAIR, HEXPAIR_PASSES, HEXPAIR_FAILS);
}

#[test]
/// Test if the `URL_BASE64` rule passes properly.
fn check_url_base64() {
    common::check_tests_rule(Rule::URL_BASE64_TEST, URL_BASE64_PASSES, URL_BASE64_FAILS);
}

#[test]
/// Test if the `bytes` rule passes properly.
fn check_bytes() {
    common::check_tests_rule(Rule::bytes_TEST, BYTES_PASSES, BYTES_FAILS);
}
