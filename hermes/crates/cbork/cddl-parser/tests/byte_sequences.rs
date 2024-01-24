// cspell: words hexpair rstuvw abcdefghijklmnopqrstuvwyz rstuvw

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

#[test]
/// Test if the `HEX_PAIR` rule passes properly.
fn check_hexpair() {
    let hex_pairs = HEXPAIR_PASSES;
    let not_hex_pairs = HEXPAIR_FAILS;

    for hp in hex_pairs {
        let parse = CDDLTestParser::parse(Rule::HEX_PAIR, hp);
        assert!(parse.is_ok());
    }

    for hp in not_hex_pairs {
        let parse = CDDLTestParser::parse(Rule::HEX_PAIR, hp);
        assert!(parse.is_err());
    }
}

#[test]
/// Test if the `URL_BASE64` rule passes properly.
fn check_url_base64() {
    let tests = URL_BASE64_PASSES;
    let fails = URL_BASE64_FAILS;

    for test in tests {
        let parse = CDDLTestParser::parse(Rule::URL_BASE64_TEST, test);
        assert!(parse.is_ok());
    }

    for test in fails {
        let parse = CDDLTestParser::parse(Rule::URL_BASE64_TEST, test);
        assert!(parse.is_err());
    }
}

#[test]
/// Test if the `bytes` rule passes properly.
fn check_bytes() {
    let test = BYTES_PASSES;
    let fail = BYTES_FAILS;

    for test in test {
        let parse = CDDLTestParser::parse(Rule::bytes_TEST, test);
        assert!(parse.is_ok());
    }

    for test in fail {
        let parse = CDDLTestParser::parse(Rule::bytes_TEST, test);
        assert!(parse.is_err());
    }
}
