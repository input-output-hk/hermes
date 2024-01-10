// cspell: words xdog intfloat hexfloat xabcp defp rstuvw

use cddl_parser::{
    self,
    cddl_test::{CDDLTestParser, Parser, Rule},
};

/// Note, the `text`, `bytes` and `id` tests are elsewhere.

#[test]
/// Test if the `uint` rule passes properly.
fn check_uint() {
    let tests = vec![
        "10",
        "101",
        "2034",
        "30456",
        "123456789",
        "0x123456789abcdefABCDEF",
        "0b0001110101010101",
        "0",
    ];

    let fails = vec![" a ", "zz", "0123zzz", "0xdog", "0b777"];

    for test in tests {
        let parse = CDDLTestParser::parse(Rule::uint_TEST, test);
        assert!(parse.is_ok());
    }

    for test in fails {
        let parse = CDDLTestParser::parse(Rule::uint_TEST, test);
        assert!(parse.is_err());
    }
}

#[test]
/// Test if the `uint` rule passes properly.
fn check_int() {
    let tests = vec![
        "10",
        "101",
        "2034",
        "30456",
        "123456789",
        "0x123456789abcdefABCDEF",
        "0b0001110101010101",
        "0",
        "-10",
        "-101",
        "-2034",
        "-30456",
        "-123456789",
        "-0x123456789abcdefABCDEF",
        "-0b0001110101010101",
        "-0",
    ];

    let fails = vec![" a ", "zz", "0123zzz", "0xdog", "0b777"];

    for test in tests {
        let parse = CDDLTestParser::parse(Rule::int_TEST, test);
        assert!(parse.is_ok());
    }

    for test in fails {
        let parse = CDDLTestParser::parse(Rule::int_TEST, test);
        assert!(parse.is_err());
    }
}

#[test]
/// Test if the `uint` rule passes properly.
fn check_intfloat() {
    let tests = vec![
        "10",
        "101",
        "2034",
        "30456",
        "123456789",
        "0",
        "-10",
        "-101",
        "-2034",
        "-30456",
        "-123456789",
        "123.456",
        "123.456",
        "123e+789",
        "123e-789",
        "123.456e+789",
        "123.456e-789",
    ];

    let fails = vec![" a ", "zz", "0123zzz", "0xdog", "0b777"];

    for test in tests {
        let parse = CDDLTestParser::parse(Rule::intfloat_TEST, test);
        assert!(parse.is_ok());
    }

    for test in fails {
        let parse = CDDLTestParser::parse(Rule::intfloat_TEST, test);
        assert!(parse.is_err());
    }
}

#[test]
/// Test if the `uint` rule passes properly.
fn check_hexfloat() {
    let tests = vec![
        "0xabcp+123",
        "-0xabcp+123",
        "0xabcp-123",
        "-0xabcp-123",
        "0xabc.defp+123",
        "-0xabc.defp+123",
        "0xabc.defp-123",
        "-0xabc.defp-123",
    ];

    let fails = vec![" a ", "zz", "0123zzz", "0xdog", "0b777"];

    for test in tests {
        let parse = CDDLTestParser::parse(Rule::hexfloat_TEST, test);
        assert!(parse.is_ok());
    }

    for test in fails {
        let parse = CDDLTestParser::parse(Rule::hexfloat_TEST, test);
        assert!(parse.is_err());
    }
}

#[test]
/// Test if the `number` rule passes properly.
fn check_number() {
    let tests = vec![
        "0xabcp+123",
        "-0xabcp+123",
        "0xabcp-123",
        "-0xabcp-123",
        "0xabc.defp+123",
        "-0xabc.defp+123",
        "0xabc.defp-123",
        "-0xabc.defp-123",
        "10",
        "101",
        "2034",
        "30456",
        "123456789",
        "0",
        "-10",
        "-101",
        "-2034",
        "-30456",
        "-123456789",
        "123.456",
        "123.456",
        "123e+789",
        "123e-789",
        "123.456e+789",
        "123.456e-789",
    ];

    let fails = vec![" a ", "zz", "0123zzz", "0xdog", "0b777"];

    for test in tests {
        let parse = CDDLTestParser::parse(Rule::number_TEST, test);
        assert!(parse.is_ok());
    }

    for test in fails {
        let parse = CDDLTestParser::parse(Rule::number_TEST, test);
        assert!(parse.is_err());
    }
}

#[test]
/// Test if the `uint` rule passes properly.
fn check_value() {
    let tests = vec![
        "0xabcp+123",
        "-0xabcp+123",
        "0xabcp-123",
        "-0xabcp-123",
        "0xabc.defp+123",
        "-0xabc.defp+123",
        "0xabc.defp-123",
        "-0xabc.defp-123",
        "10",
        "101",
        "2034",
        "30456",
        "123456789",
        "0",
        "-10",
        "-101",
        "-2034",
        "-30456",
        "-123456789",
        "123.456",
        "123.456",
        "123e+789",
        "123e-789",
        "123.456e+789",
        "123.456e-789",
        // Ideally we would define these somewhere central and just use them where needed.
        "h''",
        "b64''",
        "''",
        "h'00'",
        "h'00112233445566778899aabbccddeeff0123456789abcdef'",
        "h'0 1 2 3 4 5 6 7 8 9 a b c d e f'",
        "h' 0 1 2 3 4 5\r 6 7 \n 8 9 a\r\n\t b c d e f'",
        "b64'abcdefghijklmnopq   rstuvw   yz01\t23456789-_ABCDEFGHIJKLMNOPQRSTUVWXYZ~'",
        "b64'abcdefghijklmnopq   rstuvw   yz01\t23456789-_ABCDEFGHIJKLMNOPQRSTUVWXYZ'",
        "''",
        "'text\n that gets converted \\\' into a byte string...'",
        // Ideally we would define these somewhere central and just use them where needed.
        r#""""#,
        r#""abc""#,
        "\"abc\\n\"",
    ];

    let fails = vec![" a ", "zz", "0123zzz", "0xdog", "0b777"];

    for test in tests {
        let parse = CDDLTestParser::parse(Rule::value_TEST, test);
        assert!(parse.is_ok());
    }

    for test in fails {
        let parse = CDDLTestParser::parse(Rule::value_TEST, test);
        assert!(parse.is_err());
    }
}
