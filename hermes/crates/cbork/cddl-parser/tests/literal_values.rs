// cspell: words xdog intfloat hexfloat xabcp defp rstuvw

use std::ops::Deref;

use cddl_parser::{
    self,
    cddl_test::{CDDLTestParser, Parser, Rule},
};

mod byte_sequences;
use byte_sequences::BYTES_PASSES;
mod text_sequences;
use text_sequences::TEXT_PASSES;

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

pub const VALUE_PASSES: &[&str] = &[];

pub const VALUE_FAILS: &[&str] = &[];

pub fn passes_tests_rule(rule_type: Rule, test_data: &[&str]) {
    for test in test_data {
        let parse = CDDLTestParser::parse(rule_type, test);
        assert!(parse.is_ok());
    }
}

pub fn fails_tests_rule(rule_type: Rule, test_data: &[&str]) {
    for test in test_data {
        let parse = CDDLTestParser::parse(rule_type, test);
        assert!(parse.is_err());
    }
}

#[test]
/// Test if the `uint` rule passes properly.
fn check_uint() {
    let passes = UINT_PASSES;
    let fails = UINT_FAILS;

    passes_tests_rule(Rule::uint_TEST, passes);
    fails_tests_rule(Rule::uint_TEST, fails);
}

#[test]
/// Test if the `uint` rule passes properly.
fn check_int() {
    let passes = INT_PASSES;
    let fails = INT_FAILS;

    passes_tests_rule(Rule::int_TEST, passes);
    fails_tests_rule(Rule::int_TEST, fails);
}

#[test]
/// Test if the `uint` rule passes properly.
fn check_intfloat() {
    let passes = INTFLOAT_PASSES;
    let fails = INTFLOAT_FAILS;

    passes_tests_rule(Rule::intfloat_TEST, passes);
    fails_tests_rule(Rule::intfloat_TEST, fails);
}

#[test]
/// Test if the `uint` rule passes properly.
fn check_hexfloat() {
    let passes = HEXFLOAT_PASSES;
    let fails = HEXFLOAT_FAILS;

    passes_tests_rule(Rule::hexfloat_TEST, passes);
    fails_tests_rule(Rule::hexfloat_TEST, fails);
}

#[test]
/// Test if the `number` rule passes properly.
fn check_number() {
    let passes = NUMBER_PASSES;
    let fails = NUMBER_FAILS;

    passes_tests_rule(Rule::number_TEST, passes);
    fails_tests_rule(Rule::number_TEST, fails);
}

#[test]
/// Test if the `uint` rule passes properly.
fn check_value() {
    let passes: Vec<_> = VALUE_PASSES
        .into_iter()
        .chain(NUMBER_PASSES.into_iter())
        .chain(BYTES_PASSES.into_iter())
        .chain(TEXT_PASSES.into_iter())
        .map(Deref::deref)
        .collect();
    let fails: Vec<_> = VALUE_FAILS
        .into_iter()
        .chain(NUMBER_FAILS.into_iter())
        .map(Deref::deref)
        .collect();

    passes_tests_rule(Rule::value_TEST, &passes);
    fails_tests_rule(Rule::value_TEST, &fails);
}
