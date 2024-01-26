// cspell: words aname groupsocket typesocket groupsocket

use cddl_parser::{
    self,
    cddl_test::{CDDLTestParser, Parser, Rule},
};

mod common;
use common::identifiers::*;

#[test]
/// Check if the name components pass properly.
fn check_name_characters() {
    for x in ('\u{0}'..='\u{ff}').map(char::from) {
        let test = format!("{x}");
        let parse_start = CDDLTestParser::parse(Rule::NAME_START, &test);
        let parse_end = CDDLTestParser::parse(Rule::NAME_END, &test);

        if x.is_ascii_alphabetic() || matches!(x, '@' | '_' | '$') {
            assert!(parse_start.is_ok());
            assert!(parse_end.is_ok());
        } else if x.is_ascii_digit() {
            assert!(parse_start.is_err());
            assert!(parse_end.is_ok());
        } else {
            assert!(parse_start.is_err());
            assert!(parse_end.is_err());
        }
    }
}

#[test]
/// Test if the `id` rule passes properly.
fn check_id() {
    common::check_tests_rule(Rule::id_TEST, ID_PASSES, ID_FAILS);
}
