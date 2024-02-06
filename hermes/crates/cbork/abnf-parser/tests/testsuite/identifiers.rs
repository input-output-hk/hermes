use abnf_parser::{
    self,
    abnf_test::Rule,
};

use crate::common::*;

pub(crate) const RULENAME_PASSES: &[&str] = &[
    
];

pub(crate) const RULENAME_FAILS: &[&str] = &[
    
];

#[test]
/// Test if the `rulename` rule passes properly.
fn check_rulename() {
    check_tests_rule(Rule::rulename_TEST, RULENAME_PASSES, RULENAME_FAILS)
}