// cspell: words hexpair rstuvw abcdefghijklmnopqrstuvwyz rstuvw Xhhb Bhcm

use cddl_parser::{
    self,
    cddl_test::{CDDLTestParser, Parser, Rule},
};

pub const HEXPAIR_PASSES: &[&str] = &["00", "ab", "de", "0f", "f0"];

pub const HEXPAIR_FAILS: &[&str] = &["0", " 0", "0 ", "az", "0p"];

pub const URL_BASE64_PASSES: &[&str] = &[
    "abcdefghijklmnopq   rstuvw   yz01\t23456789-_ABCDEFGHIJKLMNOPQRSTUVWXYZ~",
    "abcdefghijklmnopqrstuvwyz0123456789-_ABCDEFGHIJKLMNOPQRSTUVWXYZ",
];

pub const URL_BASE64_FAILS: &[&str] = &[
    "abcdefghijklmnopq #  rstuvw   yz01\t23456789-_ABCDEFGHIJKLMNOPQRSTUVWXYZ~ ",
    "abcdefghijklmnopq $  rstuvw   yz01\t23456789-_ABCDEFGHIJKLMNOPQRSTUVWXYZ~\t",
    "abcdefghijklmnopq %  rstuvw   yz01\t23456789-_ABCDEFGHIJKLMNOPQRSTUVWXYZ~\n",
    "abcdefghijklmnopq ^  rstuvw   yz01\t23456789-_ABCDEFGHIJKLMNOPQRSTUVWXYZ~\r",
    "abcdefghijklmnopq &  rstuvw   yz01\t23456789-_ABCDEFGHIJKLMNOPQRSTUVWXYZ~\r\n",
];

pub const BYTES_PASSES: &[&str] = &[
    "h''",
    "b64''",
    "''",
    "h'00'",
    "h'63666F6FF6'",
    "h'68656c6c6f20776f726c64'",
    "h'4 86 56c 6c6f'",
    "h' 20776 f726c64'",
    "h'00112233445566778899aabbccddeeff0123456789abcdef'",
    "h'0 1 2 3 4 5 6 7 8 9 a b c d e f'",
    "h' 0 1 2 3 4 5\r 6 7 \n 8 9 a\r\n\t b c d e f'",
    "h'0 \n\n\r f'",
    "b64'aHR0cHM6Ly93d3cuZXhhbXBsZS5jb20vcGFnZT9wYXJhbTE9dmFsdWUxJnBhcmFtMj12YWx1ZTI~'",
    "'text\n that gets converted \\\' into a byte string...'",
];

pub const BYTES_FAILS: &[&str] = &[
    "h64",
    "b64",
    "\"\"",
    "h ''",
    "b64 ''",
    "h'001'",
    "b64'abcdefghijklmnopq #  rstuvw   yz01\t23456789-_ABCDEFGHIJKLMNOPQRSTUVWXYZ~'",
    "b64'abcdefghijklmnopq   & rstuvw   yz01\t23456789-_ABCDEFGHIJKLMNOPQRSTUVWXYZ'",
    "'\u{7}'",
];

/// # Panics
pub fn passes_tests_rule(rule_type: Rule, test_data: &[&str]) {
    for test in test_data {
        let parse = CDDLTestParser::parse(rule_type, test);
        assert!(parse.is_ok());
    }
}

/// # Panics
pub fn fails_tests_rule(rule_type: Rule, test_data: &[&str]) {
    for test in test_data {
        let parse = CDDLTestParser::parse(rule_type, test);
        assert!(parse.is_err());
    }
}

#[test]
/// Test if the `HEX_PAIR` rule passes properly.
fn check_hexpair() {
    let passes = HEXPAIR_PASSES;
    let fails = HEXPAIR_FAILS;

    passes_tests_rule(Rule::HEX_PAIR, passes);
    fails_tests_rule(Rule::HEX_PAIR, fails);
}

#[test]
/// Test if the `URL_BASE64` rule passes properly.
fn check_url_base64() {
    let passes = URL_BASE64_PASSES;
    let fails = URL_BASE64_FAILS;

    passes_tests_rule(Rule::URL_BASE64_TEST, passes);
    fails_tests_rule(Rule::URL_BASE64_TEST, fails);
}

#[test]
/// Test if the `bytes` rule passes properly.
fn check_bytes() {
    let passes = BYTES_PASSES;
    let fails = BYTES_FAILS;

    passes_tests_rule(Rule::bytes_TEST, passes);
    fails_tests_rule(Rule::bytes_TEST, fails);
}
