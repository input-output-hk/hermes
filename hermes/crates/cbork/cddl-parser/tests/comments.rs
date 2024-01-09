use cddl_parser::{
    self,
    cddl_test::{CDDLTestParser, Parser, Rule},
};

#[test]
/// Test if the `COMMENT` rule passes properly.
fn check_comment() {
    let comment1 = "; A Comment \n";
    let comment2 = "; And another\r";
    let comment3 = ";more\r\n";
    let not_comment = "not a comment\n";

    let parse = CDDLTestParser::parse(Rule::COMMENT, comment1);
    assert!(parse.is_ok());

    let parse = CDDLTestParser::parse(Rule::COMMENT, comment2);
    assert!(parse.is_ok());

    let parse = CDDLTestParser::parse(Rule::COMMENT, comment3);
    assert!(parse.is_ok());

    let parse = CDDLTestParser::parse(Rule::COMMENT, not_comment);
    assert!(parse.is_err());
}

#[test]
/// Test if the `COMMENT` rule passes properly with whitespace.
///   This uses a special rule in the Grammar to test whitespace exhaustively.
fn check_whitespace_comments() {
    let tests = vec![
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

    let fails = vec!["not a comment"];

    for test in tests {
        let parse = CDDLTestParser::parse(Rule::COMMENT_TEST, test);
        assert!(parse.is_ok());
    }

    for test in fails {
        let parse = CDDLTestParser::parse(Rule::COMMENT_TEST, test);
        assert!(parse.is_err());
    }
}
