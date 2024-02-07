// cspell: words abnf VCHAR vchar

use abnf_parser::{self, abnf_test::Rule};

use crate::common::*;

#[test]
/// Test if the `WHITESPACE` rule passes properly.
fn check_whitespace() {
    let passes = vec![" ", "\t"];
    let fails = vec!["not", "\r", "\n", "\r\n"];

    check_tests_rule(Rule::WHITESPACE_TEST, &passes, &fails);
}

#[test]
/// Test if the `VCHAR` rule passes properly.
fn check_vchar() {
    let passes: Vec<_> = (b'!'..=b'~').map(char::from).map(String::from).collect();
    let fails = vec!["\r", "\u{80}"];

    check_tests_rule(
        Rule::VCHAR_TEST,
        &passes
            .iter()
            .map(std::string::String::as_str)
            .collect::<Vec<_>>(),
        &fails,
    );
}
