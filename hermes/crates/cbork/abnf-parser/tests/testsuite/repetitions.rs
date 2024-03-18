use abnf_parser::{self, abnf_test::Rule};

use crate::common::*;

pub(crate) const REPETITION_PASSES: &[&str] = &["1*1foo", "1foo", "1a", "1%b1", "*foo"];

pub(crate) const REPETITION_FAILS: &[&str] = &["1*1 foo", "1 foo", "* foo", "1", "1%", "1%b"];

pub(crate) const REPEAT_PASSES: &[&str] = &["*", "5*10", "*20", "5*10", "0*5", "5*", "*5", "0123"];

pub(crate) const REPEAT_FAILS: &[&str] = &[
    "+",
    "?",
    "0x1*",
    "0b110*",
    "5**10",
    "5 * 10",
    "5\t\n*\n10",
    "0x1*0b110",
    "0x1*0b110",
    "++",
    "??",
    // Fail cases for uint
    "0xG",   // Invalid hex digit
    "0b123", // Invalid binary digit
    "0*5*",  // Multiple '*' not allowed
    "0x1*0b110*",
    "0x",
    "0b",
];

#[test]
/// Test if the `repetition` rule passes properly.
fn check_repetition() {
    check_tests_rule(Rule::repetition_TEST, REPETITION_PASSES, REPETITION_FAILS);
}

#[test]
/// Test if the `repeat` rule passes properly.
fn check_repeat() {
    check_tests_rule(Rule::repeat_TEST, REPEAT_PASSES, REPEAT_FAILS);
}
