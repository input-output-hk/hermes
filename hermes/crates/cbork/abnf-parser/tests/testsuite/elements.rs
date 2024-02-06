use abnf_parser::{
    self,
    abnf_test::Rule,
};

use crate::common::*;

pub(crate) const ELEMENT_PASSES: &[&str] = &[
    
];

pub(crate) const ELEMENT_FAILS: &[&str] = &[
    
];

pub(crate) const GROUP_PASSES: &[&str] = &[
    
];

pub(crate) const GROUP_FAILS: &[&str] = &[
    
];

pub(crate) const OPTION_PASSES: &[&str] = &[
    
];

pub(crate) const OPTION_FAILS: &[&str] = &[
    
];

#[test]
/// Test if the `element` rule passes properly.
fn check_element() {
    check_tests_rule(Rule::element_TEST, ELEMENT_PASSES, ELEMENT_FAILS)
}

#[test]
/// Test if the `group` rule passes properly.
fn check_group() {
    check_tests_rule(Rule::group_TEST, GROUP_PASSES, GROUP_FAILS)
}

#[test]
/// Test if the `option` rule passes properly.
fn check_option() {
    check_tests_rule(Rule::option_TEST, OPTION_PASSES, OPTION_FAILS)
}