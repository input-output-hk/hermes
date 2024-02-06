use abnf_parser::{
    self,
    abnf_test::Rule,
};

use crate::common::*;

pub(crate) const RULE_PASSES: &[&str] = &[
    
];

pub(crate) const RULE_FAILS: &[&str] = &[
    
];

pub(crate) const DEFINED_AS_PASSES: &[&str] = &[
    
];

pub(crate) const DEFINED_AS_FAILS: &[&str] = &[
    
];

pub(crate) const ELEMENTS_PASSES: &[&str] = &[
    
];

pub(crate) const ELEMENTS_FAILS: &[&str] = &[
    
];

#[test]
/// Test if the `rule` rule passes properly.
fn check_rule() {
    check_tests_rule(Rule::rule_TEST, RULE_PASSES, RULE_FAILS)
}

#[test]
/// Test if the `defined_as` rule passes properly.
fn check_defined_as() {
    check_tests_rule(Rule::defined_as_TEST, DEFINED_AS_PASSES, DEFINED_AS_FAILS)
}

#[test]
/// Test if the `elements` rule passes properly.
fn check_elements() {
    check_tests_rule(Rule::elements_TEST, ELEMENTS_PASSES, ELEMENTS_FAILS)
}