// cspell: words RULENAME

use abnf_parser::{self, abnf_test::Rule};

use crate::{common::*, groups::*, identifiers::*, values::*};

pub(crate) const ELEMENT_PASSES: &[&str] = &[];

pub(crate) const ELEMENT_FAILS: &[&str] = &[];

#[test]
/// Test if the `element` rule passes properly.
fn check_element() {
    let passes: Vec<_> = ELEMENT_PASSES
        .iter()
        .map(|x| (*x).to_string())
        .chain(RULENAME_PASSES.iter().map(|x| (*x).to_string()))
        .chain(OPTION_PASSES.iter().map(|x| (*x).to_string()))
        .chain(GROUP_PASSES.iter().map(|x| (*x).to_string()))
        .chain(BIN_VAL_PASSES.iter().map(|x| format!("%{x}")))
        .chain(DEC_VAL_PASSES.iter().map(|x| format!("%{x}")))
        .chain(HEX_VAL_PASSES.iter().map(|x| format!("%{x}")))
        .collect();
    let fails: Vec<_> = ELEMENT_FAILS
        .iter()
        .map(|x| (*x).to_string())
        .chain(RULENAME_FAILS.iter().map(|x| (*x).to_string()))
        .chain(OPTION_FAILS.iter().map(|x| (*x).to_string()))
        .chain(GROUP_FAILS.iter().map(|x| (*x).to_string()))
        .chain(BIN_VAL_FAILS.iter().map(|x| format!("%{x}")))
        .chain(DEC_VAL_FAILS.iter().map(|x| format!("%{x}")))
        .chain(HEX_VAL_FAILS.iter().map(|x| format!("%{x}")))
        .collect();

    check_tests_rule(
        Rule::element_TEST,
        &passes
            .iter()
            .map(std::string::String::as_str)
            .collect::<Vec<_>>(),
        &fails
            .iter()
            .map(std::string::String::as_str)
            .collect::<Vec<_>>(),
    );
}
