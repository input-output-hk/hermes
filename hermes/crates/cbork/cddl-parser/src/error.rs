use derive_more::{Display, From};

use crate::parser::rfc_8610::Rule;

/// Represents an error that may occur during CDDL parsing pipeline.
#[derive(Display, Debug, From)]
pub enum CDDLError {
    ParsingRFC8610(pest::error::Error<Rule>)
}
