use cddl_parser::{
    self,
    cddl_test::{CDDLTestParser, Parser, Rule},
};

/// # Panics
pub(crate) fn check_tests_rule(rule_type: Rule, passes: &[&str], fails: &[&str]) {
    for test in passes {
        let parse = CDDLTestParser::parse(rule_type, test);
        assert!(parse.is_ok());
    }

    for test in fails {
        let parse = CDDLTestParser::parse(rule_type, test);
        assert!(parse.is_err());
    }
}
