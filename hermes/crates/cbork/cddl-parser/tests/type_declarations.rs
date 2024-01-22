use cddl_parser::{
    self,
    cddl_test::{CDDLTestParser, Parser, Rule},
};  

pub const CTLOP_PASSES: &[&str] = &[
    ".$",
    ".@",
    "._",
    ".a",
    ".z",
    ".A",
    ".Z",
    ".$$",
    ".@@",
    ".__",
    ".a$",
    ".a@",
    ".a_",
    ".$0",
    ".@9",
    "._a",
    ".abc",
    ".aname",
    ".@aname",
    "._aname",
    ".$aname",
    ".a$name",
    ".a.name",
    ".@a.name",
    ".$a.name",
    "._a.name",
    ".$$",
    ".$$groupsocket",
    ".$",
    ".$typesocket",
];

pub const CTLOP_FAILS: &[&str] = &[
    "aname.", ".", "..", "aname.", "aname-", "aname%", "a%name4", "a^name5", "a name", "",
];

#[test]
/// Test if the `ctlop` rule passes properly.
/// This uses a special rule in the Grammar to test `ctlop` exhaustively.
fn check_ctlop() {
    let passes = CTLOP_PASSES;
    let fails = CTLOP_FAILS;

    for test in passes {
        let parse = CDDLTestParser::parse(Rule::ctlop_TEST, test);
        assert!(parse.is_ok());
    }
  
    for test in fails {
        let parse = CDDLTestParser::parse(Rule::ctlop_TEST, test);
        assert!(parse.is_err());
    }
}