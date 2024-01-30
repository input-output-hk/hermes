// cspell: words xdog intfloat hexfloat xabcp defp rstuvw

use std::ops::Deref;

use cddl_parser::{self, cddl_test::Rule};

mod common;
use common::{byte_sequences::*, literal_values::*, text_sequences::*};

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
