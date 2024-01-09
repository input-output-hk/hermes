// cspell: words PCHAR pchar BCHAR bchar SESC sesc SCHAR schar fffd fffe

use cddl_parser::{
    self,
    cddl_test::{CDDLTestParser, Parser, Rule},
};

#[test]
/// Test if the `WHITESPACE` rule passes properly.
fn check_whitespace() {
    let whitespace = vec![" ", "\t", "\r", "\n", "\r\n"];

    let not_whitespace = "not";

    for ws in whitespace {
        let parse = CDDLTestParser::parse(Rule::WHITESPACE, ws);
        assert!(parse.is_ok());
    }

    let parse = CDDLTestParser::parse(Rule::WHITESPACE, not_whitespace);
    assert!(parse.is_err());
}

#[test]
/// Test if the `PCHAR` rule passes properly.
fn check_pchar() {
    for x in ('\u{0}'..='\u{ff}').map(char::from) {
        let test = format!("{x}");
        let parse = CDDLTestParser::parse(Rule::PCHAR, &test);
        if x < ' ' || x == '\u{7f}' {
            assert!(parse.is_err());
        } else {
            assert!(parse.is_ok());
        }
    }

    let parse = CDDLTestParser::parse(Rule::ASCII_VISIBLE, "\r");
    assert!(parse.is_err());
}

#[test]
/// Test if the `BCHAR` rule passes properly.
fn check_bchar() {
    for x in ('\u{0}'..='\u{ff}').map(char::from) {
        let test = format!("{x}");
        let parse = CDDLTestParser::parse(Rule::BCHAR, &test);
        if x != '\n' && x != '\r' && x < ' ' || x == '\u{27}' || x == '\u{5c}' || x == '\u{7f}' {
            assert!(parse.is_err());
        } else {
            assert!(parse.is_ok());
        }
    }

    let parse = CDDLTestParser::parse(Rule::ASCII_VISIBLE, "\r");
    assert!(parse.is_err());
}

#[test]
/// Test if the `SESC` rule passes properly.
fn check_sesc() {
    for x in (' '..='\u{ff}').map(char::from) {
        let test = format!("\\{x}");
        let parse = CDDLTestParser::parse(Rule::SESC, &test);
        if x == '\u{7f}' {
            assert!(parse.is_err());
        } else {
            assert!(parse.is_ok());
        }
    }

    let parse = CDDLTestParser::parse(Rule::ASCII_VISIBLE, "\r");
    assert!(parse.is_err());
}

#[test]
/// Test if the `ASCII_VISIBLE` rule passes properly.
fn check_ascii_visible() {
    for x in (b' '..=b'~').map(char::from) {
        let test = x.to_string();
        let parse = CDDLTestParser::parse(Rule::ASCII_VISIBLE, &test);
        assert!(parse.is_ok());
    }

    let parse = CDDLTestParser::parse(Rule::ASCII_VISIBLE, "\r");
    assert!(parse.is_err());

    let parse = CDDLTestParser::parse(Rule::ASCII_VISIBLE, "\u{80}");
    assert!(parse.is_err());
}

#[test]
/// Test if the `SCHAR_ASCII_VISIBLE` rule passes properly.
fn check_schar_ascii_visible() {
    let invalids = "\"\\";
    for x in (b' '..=b'~').map(char::from) {
        let test = x.to_string();
        let parse = CDDLTestParser::parse(Rule::SCHAR_ASCII_VISIBLE, &test);
        if invalids.contains(x) {
            assert!(parse.is_err());
        } else {
            assert!(parse.is_ok());
        }
    }

    let parse = CDDLTestParser::parse(Rule::SCHAR_ASCII_VISIBLE, "\r");
    assert!(parse.is_err());

    let parse = CDDLTestParser::parse(Rule::SCHAR_ASCII_VISIBLE, "\u{80}");
    assert!(parse.is_err());
}

#[test]
/// Test if the `BCHAR_ASCII_VISIBLE` rule passes properly.
fn check_bchar_ascii_visible() {
    let invalids = "'\\";
    for x in (b' '..=b'~').map(char::from) {
        let test = x.to_string();
        let parse = CDDLTestParser::parse(Rule::BCHAR_ASCII_VISIBLE, &test);
        if invalids.contains(x) {
            assert!(parse.is_err());
        } else {
            assert!(parse.is_ok());
        }
    }

    let parse = CDDLTestParser::parse(Rule::BCHAR_ASCII_VISIBLE, "\r");
    assert!(parse.is_err());

    let parse = CDDLTestParser::parse(Rule::BCHAR_ASCII_VISIBLE, "\u{80}");
    assert!(parse.is_err());
}

#[test]
/// Test if the `UNICODE_CHAR` rule passes properly.
fn check_unicode() {
    let parse = CDDLTestParser::parse(Rule::UNICODE_CHAR, "\r");
    assert!(parse.is_err());

    let parse = CDDLTestParser::parse(Rule::UNICODE_CHAR, "\u{80}");
    assert!(parse.is_ok());

    let parse = CDDLTestParser::parse(Rule::UNICODE_CHAR, "\u{10fffd}");
    assert!(parse.is_ok());

    let parse = CDDLTestParser::parse(Rule::UNICODE_CHAR, "\u{7ffff}");
    assert!(parse.is_ok());

    let parse = CDDLTestParser::parse(Rule::UNICODE_CHAR, "\u{10fffe}");
    assert!(parse.is_err());
}
