use std::fmt::Debug;

pub use pest::Parser;
use pest_derive::Parser;

extern crate derive_more;
use derive_more::{Display, From};

// Parser with DEBUG rules.  These rules are only used in tests.
#[derive(Parser)]
#[grammar = "grammar/cddl.pest"]
#[grammar = "grammar/cddl_test.pest"]  // Ideally this would only be used in tests.
pub struct CDDLParser;

#[derive(Display, Debug, From)]
pub struct CDDLError(pest::error::Error<Rule>);

// CDDL Standard Postlude - read from an external file
const POSTLUDE: &str = include_str!("grammar/postlude.cddl");

pub fn parse_cddl(input: &str) -> Result<(), Box<CDDLError>> {
    let result = CDDLParser::parse(Rule::cddl, input);

    match result {
        Ok(c) => println!("{c:?}"),
        Err(e) => {
            println!("{e:?}");
            println!("{e}");
            return Err(Box::new(CDDLError::from(e)));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::{parse_cddl, POSTLUDE};

    #[test]
    fn it_works() {
        let result = parse_cddl(POSTLUDE);

        match result {
            Ok(c) => println!("{c:?}"),
            Err(e) => {
                println!("{e:?}");
                println!("{e}");
            }
        }
    }
}
