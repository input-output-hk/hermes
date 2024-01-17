// cspell: words hexpair rstuvw abcdefghijklmnopqrstuvwyz rstuvw

use cddl_parser::{
    self,
    cddl_test::{CDDLTestParser, Parser, Rule},
};

pub const BYTES_PASSES: &[&str] = &[
    "h''",
    "b64''",
    "''",
    "h'00'",
    "h'00112233445566778899aabbccddeeff0123456789abcdef'",
    "h'001'",
    "h'0 1 2 3 4 5 6 7 8 9 a b c d e f'",
    "h'0 \n\n\r f'",
    "''",
    "'text\n that gets converted \\\' into a byte string...'",
];

pub const BYTES_FAILS: &[&str] = &[
    "h64",
    "b64",
    "\"\"",
    "h ''",
    "h '0 \t f'",
    "h' 0 1 2 3 4 5\r 6 7 \n 8 9 a\r\n\t b c d e f'",
    "b64'abcdefghijklmnopq   rstuvw   yz01\t23456789-_ABCDEFGHIJKLMNOPQRSTUVWXYZ~'",
    "b64'abcdefghijklmnopq   rstuvw   yz01\t23456789-_ABCDEFGHIJKLMNOPQRSTUVWXYZ'",
    "'\u{7}'",
];

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
