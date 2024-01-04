use cddl_parser::{self, CDDLParser, Parser, Rule};

#[test]
/// Test if the `S` rule passes properly.
///   This uses a special rule in the Grammar to test whitespace exhaustively.
fn check_s() {
    let tests = vec![" ", "  ", " \t \t", " \t  \r \n \r\n   "];

    let fails = vec![" a ", "zz", " \t d \t", " \t  \r \n \t \r\n  x"];

    for test in tests {
        let parse = CDDLParser::parse(Rule::S_TEST, &test);
        assert!(parse.is_ok());
    }

    for test in fails {
        let parse = CDDLParser::parse(Rule::S_TEST, &test);
        assert!(parse.is_err());
    }
}

#[test]
/// Test if the `text` rule passes properly.
fn check_text() {
    let test = vec![
        r#""""#,
        r#""abc""#,
        "\"abc\\n\"",
    ];

    let fail = vec![
        "",
        "''",
        "\"abc\n\"",
    ];

    for test in test {
        let parse = CDDLParser::parse(Rule::text_TEST, &test);
        assert!(parse.is_ok());
    }

    for test in fail {
        let parse = CDDLParser::parse(Rule::text_TEST, &test);
        assert!(parse.is_err());
    }
}
