use cddl_parser::{
    self,
    cddl_test::{CDDLTestParser, Parser, Rule},
};

pub const S_PASSES: &[&str] = &[" ", "  ", " \t \t", " \t  \r \n \r\n   "];
pub const S_FAILS: &[&str] = &[" a ", "zz", " \t d \t", " \t  \r \n \t \r\n  x"];
pub const TEXT_PASSES: &[&str] = &[r#""""#, r#""abc""#, "\"abc\\n\""];
pub const TEXT_FAILS: &[&str] = &["", "''", "\"abc\n\""];

#[test]
/// Test if the `S` rule passes properly.
/// This uses a special rule in the Grammar to test whitespace exhaustively.
fn check_s() {
    let tests = S_PASSES;
    let fails = S_FAILS;

    for test in tests {
        let parse = CDDLTestParser::parse(Rule::S_TEST, test);
        assert!(parse.is_ok());
    }

    for test in fails {
        let parse = CDDLTestParser::parse(Rule::S_TEST, test);
        assert!(parse.is_err());
    }
}

#[test]
/// Test if the `text` rule passes properly.
fn check_text() {
    let test = TEXT_PASSES;
    let fail = TEXT_FAILS;

    for test in test {
        let parse = CDDLTestParser::parse(Rule::text_TEST, test);
        assert!(parse.is_ok());
    }

    for test in fail {
        let parse = CDDLTestParser::parse(Rule::text_TEST, test);
        assert!(parse.is_err());
    }
}
