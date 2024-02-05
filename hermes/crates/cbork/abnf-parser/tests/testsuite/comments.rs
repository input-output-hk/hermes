use abnf_parser::{
    self,
    abnf_test::Rule,
};

use crate::common::*;

#[test]
/// Test if the `COMMENT` rule passes properly.
fn check_comments() {
    let passes = vec![";\n", "; a\n", "; a\r\n", ";;\n", "; a87246h\t\t\r\n", "; And another\r"];
    let fails = vec![";", ";abc", "not a comment"];

    check_tests_rule(Rule::COMMENT_TEST, &passes, &fails)
}
