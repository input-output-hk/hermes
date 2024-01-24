use cddl_parser::{
    self,
    cddl_test::{CDDLTestParser, Parser, Rule},
};

pub const S_PASSES: &[&str] = &[" ", "  ", " \t \t", " \t  \r \n \r\n   "];
pub const S_FAILS: &[&str] = &[" a ", "zz", " \t d \t", " \t  \r \n \t \r\n  x"];
pub const TEXT_PASSES: &[&str] = &[r#""""#, r#""abc""#, "\"abc\\n\""];
pub const TEXT_FAILS: &[&str] = &["", "''", "\"abc\n\""];

pub fn passes_tests_rule(rule_type: Rule, test_data: &[&str]) {
    for test in test_data {
        let parse = CDDLTestParser::parse(rule_type, test);
        assert!(parse.is_ok());
    }
}

pub fn fails_tests_rule(rule_type: Rule, test_data: &[&str]) {
    for test in test_data {
        let parse = CDDLTestParser::parse(rule_type, test);
        assert!(parse.is_err());
    }
}

#[test]
/// Test if the `S` rule passes properly.
/// This uses a special rule in the Grammar to test whitespace exhaustively.
fn check_s() {
    let passes = S_PASSES;
    let fails = S_FAILS;

    passes_tests_rule(Rule::S_TEST, passes);
    fails_tests_rule(Rule::S_TEST, fails);
}

#[test]
/// Test if the `text` rule passes properly.
fn check_text() {
    let passes = TEXT_PASSES;
    let fails = TEXT_FAILS;

    passes_tests_rule(Rule::text_TEST, passes);
    fails_tests_rule(Rule::text_TEST, fails);
}
