use cddl_parser::{
    self,
    cddl_test::{CDDLTestParser, Parser, Rule},
};

pub const COMMENT_PASSES: &[&str] = &["; A Comment \n", "; And another\r", ";more\r\n"];

pub const COMMENT_FAILS: &[&str] = &["not a comment\n"];

pub const WHITESPACE_COMMENT_PASSES: &[&str] = &[
    " ",
    "  ",
    " \t \t",
    " \t  \r \n \r\n   ",
    "; A Comment\r",
    " \t ; A Comment    \n",
    "; One Comment\n; Two Comments\n",
    "; One Comment  \n; Two Comments\r; Another Comment\r\n",
    "\t; One Comment \n\t; Two Comments\r; Another Comment\r\n",
    "\t; A Comment \n    ; Another Comment \t \r\n    \t  ; A Final Comment   \r\n",
];

pub const WHITESPACE_COMMENT_FAILS: &[&str] = &["not a comment"];

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
/// Test if the `COMMENT` rule passes properly.
fn check_comment() {
    let passes = COMMENT_PASSES;
    let fails = COMMENT_FAILS;

    passes_tests_rule(Rule::COMMENT_TEST, passes);
    fails_tests_rule(Rule::COMMENT_TEST, fails);
}

#[test]
/// Test if the `COMMENT` rule passes properly with whitespace.
/// This uses a special rule in the Grammar to test whitespace exhaustively.
fn check_whitespace_comments() {
    let passes = WHITESPACE_COMMENT_PASSES;
    let fails = WHITESPACE_COMMENT_FAILS;

    passes_tests_rule(Rule::COMMENT_TEST, passes);
    fails_tests_rule(Rule::COMMENT_TEST, fails);
}
