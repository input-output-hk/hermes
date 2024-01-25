// cspell: words xdog intfloat hexfloat xabcp defp rstuvw

use std::ops::Deref;

use cddl_parser::{self, cddl_test::Rule};

#[path = "common/mod.rs"]
#[allow(clippy::duplicate_mod)]
mod common;

mod byte_sequences;
use byte_sequences::BYTES_PASSES;
mod text_sequences;
use text_sequences::TEXT_PASSES;

pub(crate) const UINT_PASSES: &[&str] = &[
    "10",
    "101",
    "2034",
    "30456",
    "123456789",
    "0x123456789abcdefABCDEF",
    "0b0001110101010101",
    "0",
];

pub(crate) const UINT_FAILS: &[&str] = &[" a ", "zz", "0123zzz", "0xdog", "0b777"];

pub(crate) const INT_PASSES: &[&str] = &[
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

pub(crate) const INT_FAILS: &[&str] = &[" a ", "zz", "0123zzz", "0xdog", "0b777"];

pub(crate) const INTFLOAT_PASSES: &[&str] = &[
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

pub(crate) const INTFLOAT_FAILS: &[&str] = &[" a ", "zz", "0123zzz", "0xdog", "0b777"];

pub(crate) const HEXFLOAT_PASSES: &[&str] = &[
    "0xabcp+123",
    "-0xabcp+123",
    "0xabcp-123",
    "-0xabcp-123",
    "0xabc.defp+123",
    "-0xabc.defp+123",
    "0xabc.defp-123",
    "-0xabc.defp-123",
];

pub(crate) const HEXFLOAT_FAILS: &[&str] = &[" a ", "zz", "0123zzz", "0xdog", "0b777"];

pub(crate) const NUMBER_PASSES: &[&str] = &[
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

pub(crate) const NUMBER_FAILS: &[&str] = &[" a ", "zz", "0123zzz", "0xdog", "0b777"];

pub(crate) const VALUE_PASSES: &[&str] = &[];

pub(crate) const VALUE_FAILS: &[&str] = &[];

#[test]
/// Test if the `uint` rule passes properly.
fn check_uint() {
    common::check_tests_rule(Rule::uint_TEST, UINT_PASSES, UINT_FAILS);
}

#[test]
/// Test if the `uint` rule passes properly.
fn check_int() {
    common::check_tests_rule(Rule::int_TEST, INT_PASSES, INT_FAILS);
}

#[test]
/// Test if the `uint` rule passes properly.
fn check_intfloat() {
    common::check_tests_rule(Rule::intfloat_TEST, INTFLOAT_PASSES, INTFLOAT_FAILS);
}

#[test]
/// Test if the `uint` rule passes properly.
fn check_hexfloat() {
    common::check_tests_rule(Rule::hexfloat_TEST, HEXFLOAT_PASSES, HEXFLOAT_FAILS);
}

#[test]
/// Test if the `number` rule passes properly.
fn check_number() {
    common::check_tests_rule(Rule::number_TEST, NUMBER_PASSES, NUMBER_FAILS);
}

#[test]
/// Test if the `uint` rule passes properly.
fn check_value() {
    let passes: Vec<_> = VALUE_PASSES
        .iter()
        .chain(NUMBER_PASSES)
        .chain(BYTES_PASSES)
        .chain(TEXT_PASSES)
        .map(Deref::deref)
        .collect();
    let fails: Vec<_> = VALUE_FAILS
        .iter()
        .chain(NUMBER_FAILS)
        .map(Deref::deref)
        .collect();

    common::check_tests_rule(Rule::value_TEST, &passes, &fails);
}
