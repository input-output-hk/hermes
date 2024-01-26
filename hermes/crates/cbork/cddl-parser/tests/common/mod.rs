use cddl_parser::{
    self,
    cddl_test::{CDDLTestParser, Parser, Rule},
};

pub(crate) mod byte_sequences;
pub(crate) mod comments;
pub(crate) mod group_elements;
pub(crate) mod identifiers;
pub(crate) mod literal_values;
pub(crate) mod rules;
pub(crate) mod text_sequences;
pub(crate) mod type_declarations;

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
