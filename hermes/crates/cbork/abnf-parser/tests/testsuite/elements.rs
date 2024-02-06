use abnf_parser::{
    self,
    abnf_test::Rule,
};

use crate::common::*;

pub(crate) const ELEMENT_PASSES: &[&str] = &[
    
];

pub(crate) const ELEMENT_FAILS: &[&str] = &[
    
];

#[test]
/// Test if the `element` rule passes properly.
fn check_element() {
    check_tests_rule(Rule::element_TEST, ELEMENT_PASSES, ELEMENT_FAILS)
}
