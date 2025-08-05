//! Errors module.

use std::fmt::Display;

/// Errors struct which holds a collection of errors
#[derive(thiserror::Error, Debug)]
pub(crate) struct Errors(Vec<anyhow::Error>);

impl Display for Errors {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        for err in &self.0 {
            write!(f, "- ")?;
            let err_str = err.to_string();
            let mut err_lines = err_str.lines();
            if let Some(first_line) = err_lines.next() {
                writeln!(f, "{first_line}")?;
                for line in err_lines {
                    writeln!(f, "  {line}")?;
                }
            }
        }
        Ok(())
    }
}

impl Errors {
    /// Create a new empty `Errors`
    pub(crate) fn new() -> Self {
        Self(Vec::new())
    }

    /// Returns `true` if the `Errors` contains no elements.
    pub(crate) fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Add an error to the `Errors`
    pub(crate) fn add_err<E>(
        &mut self,
        err: E,
    ) where
        E: Into<anyhow::Error>,
    {
        let err = err.into();
        match err.downcast::<Errors>() {
            Ok(errs) => self.0.extend(errs.0),
            Err(err) => self.0.push(err),
        }
    }

    /// Return a closure that adds an error to the `Errors`
    pub(crate) fn get_add_err_fn<E>(&mut self) -> impl FnOnce(E) + '_
    where E: Into<anyhow::Error> {
        |err| self.add_err(err)
    }

    /// Return errors if `Errors` is not empty or return `Ok(val)`
    pub(crate) fn return_result<T>(
        self,
        val: T,
    ) -> anyhow::Result<T> {
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
        let mut errors_1 = Errors::new();
        errors_1.add_err(anyhow::anyhow!("error 1"));
        errors_1.add_err(anyhow::anyhow!("error 2"));

        let mut errors_2 = Errors::new();
        errors_2.add_err(anyhow::anyhow!("error 3"));
        errors_2.add_err(anyhow::anyhow!("error 4"));

        let mut combined_errors = Errors::new();
        combined_errors.add_err(errors_1);
        combined_errors.add_err(errors_2);

        assert_eq!(
            combined_errors.to_string(),
            "- error 1\n- error 2\n- error 3\n- error 4\n"
        );
    }
}
