use abnf_parser::{
    self,
    abnf_test::Rule,
};

use crate::common::*;
use crate::identifiers::*;
use crate::values::*;
use crate::groups::*;

pub(crate) const ELEMENT_PASSES: &[&str] = &[
    
];

pub(crate) const ELEMENT_FAILS: &[&str] = &[
    
];

#[test]
/// Test if the `element` rule passes properly.
fn check_element() {
    let passes: Vec<_> = ELEMENT_PASSES
        .iter()
        .map(|x| format!("{x}"))
        .chain(RULENAME_PASSES.into_iter().map(|x| format!("{x}")))
        .chain(OPTION_PASSES.into_iter().map(|x| format!("{x}")))
        .chain(GROUP_PASSES.into_iter().map(|x| format!("{x}")))
        .chain(BIN_VAL_PASSES.into_iter().map(|x| format!("%{x}")))
        .chain(DEC_VAL_PASSES.into_iter().map(|x| format!("%{x}")))
        .chain(HEX_VAL_PASSES.into_iter().map(|x| format!("%{x}")))
        .collect();
    let fails: Vec<_> = ELEMENT_FAILS
        .iter()
        .map(|x| format!("{x}"))
        .chain(RULENAME_FAILS.into_iter().map(|x| format!("{x}")))
        .chain(OPTION_FAILS.into_iter().map(|x| format!("{x}")))
        .chain(GROUP_FAILS.into_iter().map(|x| format!("{x}")))
        .chain(BIN_VAL_FAILS.into_iter().map(|x| format!("%{x}")))
        .chain(DEC_VAL_FAILS.into_iter().map(|x| format!("%{x}")))
        .chain(HEX_VAL_FAILS.into_iter().map(|x| format!("%{x}")))
        .collect();

        check_tests_rule(
            Rule::element_TEST,
            &passes.iter().map(|s| s.as_str()).collect::<Vec<_>>(),
            &fails.iter().map(|s| s.as_str()).collect::<Vec<_>>()
        )
}
