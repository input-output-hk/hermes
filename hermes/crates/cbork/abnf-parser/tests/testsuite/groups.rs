// cspell: words abnf

use abnf_parser::{self, abnf_test::Rule};

use crate::common::*;

pub(crate) const GROUP_PASSES: &[&str] = &[
    "(foo)",
    "( foo )",
    "(Rule1 Rule2)",
    "(foo / bar)",
    "(elem foo blat)",
    "((elem) (foo) (blat))",
    "(((((foo)))))",
    "(((((foo / bar)))))",
];

pub(crate) const GROUP_FAILS: &[&str] = &["()", "((foo)", "(())", "())"];

pub(crate) const OPTION_PASSES: &[&str] = &[
    "[foo]",
    "[(foo)]",
    "[([(foo)])]",
    "[ foo ]",
    "[Rule1 Rule2]",
    "[foo / bar]",
    "[elem foo blat]",
    "[[elem] [foo] [blat]]",
    "[[[[[foo / bar]]]]]",
];

pub(crate) const OPTION_FAILS: &[&str] = &["[]", "[]]", "[]]"];

#[test]
/// Test if the `group` rule passes properly.
fn check_group() {
    check_tests_rule(Rule::group_TEST, GROUP_PASSES, GROUP_FAILS);
}

#[test]
/// Test if the `option` rule passes properly.
fn check_option() {
    check_tests_rule(Rule::option_TEST, OPTION_PASSES, OPTION_FAILS);
}
