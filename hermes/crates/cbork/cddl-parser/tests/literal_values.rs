// cspell: words xdog intfloat hexfloat xabcp defp rstuvw

use cddl_parser::{
    self,
    cddl_test::{CDDLTestParser, Parser, Rule},
};

mod byte_sequences;
use byte_sequences::BYTES_PASSES;

/// Note, the `text`, `bytes` and `id` tests are elsewhere.

pub const UINT_PASSES: &[&str] = &[
    "10",
    "101",
    "2034",
    "30456",
    "123456789",
    "0x123456789abcdefABCDEF",
    "0b0001110101010101",
    "0",
];

pub const UINT_FAILS: &[&str] = &[" a ", "zz", "0123zzz", "0xdog", "0b777"];

pub const INT_PASSES: &[&str] = &[
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

pub const INT_FAILS: &[&str] = &[" a ", "zz", "0123zzz", "0xdog", "0b777"];

pub const INTFLOAT_PASSES: &[&str] = &[
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

pub const INTFLOAT_FAILS: &[&str] = &[" a ", "zz", "0123zzz", "0xdog", "0b777"];

pub const HEXFLOAT_PASSES: &[&str] = &[
    "0xabcp+123",
    "-0xabcp+123",
    "0xabcp-123",
    "-0xabcp-123",
    "0xabc.defp+123",
    "-0xabc.defp+123",
    "0xabc.defp-123",
    "-0xabc.defp-123",
];

pub const HEXFLOAT_FAILS: &[&str] = &[" a ", "zz", "0123zzz", "0xdog", "0b777"];

pub const NUMBER_PASSES: &[&str] = &[
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

pub const NUMBER_FAILS: &[&str] = &[" a ", "zz", "0123zzz", "0xdog", "0b777"];

pub const VALUE_PASSES: &[&str] = &[
    // Ideally we would define these somewhere central and just use them where needed.
    // r#""""#,
    // r#""abc""#,
    // "\"abc\\n\"",
];

pub const VALUE_FAILS: &[&str] = &[];

#[test]
/// Test if the `uint` rule passes properly.
fn check_uint() {
    let tests = UINT_PASSES;
    let fails = UINT_FAILS;

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
    let tests = INT_PASSES;
    let fails = INT_FAILS;

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
    let tests = INTFLOAT_PASSES;
    let fails = INTFLOAT_FAILS;

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
    let tests = HEXFLOAT_PASSES;
    let fails = HEXFLOAT_FAILS;

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
    let tests = NUMBER_PASSES;
    let fails = NUMBER_FAILS;

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
    let tests: Vec<_> = VALUE_PASSES
        .into_iter()
        .chain(NUMBER_PASSES.into_iter())
        .chain(BYTES_PASSES.into_iter())
        .collect();
    let fails: Vec<_> = VALUE_FAILS
        .into_iter()
        .chain(NUMBER_FAILS.into_iter())
        .collect();

    for test in tests {
        let parse = CDDLTestParser::parse(Rule::value_TEST, test);
        assert!(parse.is_ok());
    }

    for test in fails {
        let parse = CDDLTestParser::parse(Rule::value_TEST, test);
        assert!(parse.is_err());
    }
}
