//! Errors module.

use std::fmt::Display;

/// Errors struct which holds a collection of errors
#[derive(thiserror::Error, Debug)]
pub(crate) struct Errors(Vec<anyhow::Error>);

impl Display for Errors {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Errors:")?;
        for err in &self.0 {
            writeln!(f, "- {err}")?;
        }
        Ok(())
    }
}

impl Errors {
    /// Create a new empty `Errors`
    pub(crate) fn new() -> Self {
        Self(Vec::new())
    }

    /// Add an error to the `Errors`
    pub(crate) fn add_err(&mut self, err: anyhow::Error) {
        self.0.push(err);
    }

    /// Return errors if `Errors` is not empty or return `Ok(val)`
    pub(crate) fn return_result<T>(self, val: T) -> anyhow::Result<T> {
        if self.0.is_empty() {
            Ok(val)
        } else {
            Err(self.into())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_errors() {
        let mut errors = Errors::new();
        errors.add_err(anyhow::anyhow!("error 1"));
        errors.add_err(anyhow::anyhow!("error 2"));

        assert_eq!(errors.to_string(), "Errors:\n- error 1\n- error 2\n");
    }
}
