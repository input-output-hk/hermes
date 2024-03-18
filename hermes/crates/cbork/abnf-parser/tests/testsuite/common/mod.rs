use abnf_parser::{
    self,
    abnf_test::{ABNFTestParser, Parser, Rule},
};

/// # Panics
pub(crate) fn check_tests_rule(rule_type: Rule, passes: &[&str], fails: &[&str]) {
    for test in passes {
        let parse = ABNFTestParser::parse(rule_type, test);
        assert!(parse.is_ok());
    }

    for test in fails {
        let parse = ABNFTestParser::parse(rule_type, test);
        assert!(parse.is_err());
    }
}
