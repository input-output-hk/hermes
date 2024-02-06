use abnf_parser::{
    self,
    abnf_test::Rule,
};

use crate::common::*;

pub(crate) const REPETITION_PASSES: &[&str] = &[
    
];

pub(crate) const REPETITION_FAILS: &[&str] = &[
    
];

pub(crate) const REPEAT_PASSES: &[&str] = &[
    
];

pub(crate) const REPEAT_FAILS: &[&str] = &[
    
];

#[test]
/// Test if the `repetition` rule passes properly.
fn check_repetition() {
    check_tests_rule(Rule::repetition_TEST, REPETITION_PASSES, REPETITION_FAILS)
}

#[test]
/// Test if the `repeat` rule passes properly.
fn check_repeat() {
    check_tests_rule(Rule::repeat_TEST, REPEAT_PASSES, REPEAT_FAILS)
}