//! Processing for String Environment Variables

// cspell: words smhdwy

use std::{
    env::{self, VarError},
    fmt::{self, Display},
    str::FromStr,
};

use log::{error, info};
use strum::VariantNames;

/// An environment variable read as a string.
#[derive(Clone)]
pub(crate) struct StringEnvVar {
    /// Value of the env var.
    value: String,
    /// Whether the env var is displayed redacted or not.
    redacted: bool,
}

/// Ergonomic way of specifying if a env var needs to be redacted or not.
pub(super) enum StringEnvVarParams {
    /// The env var is plain and should not be redacted.
    Plain(String, Option<String>),
    /// The env var is redacted and should be redacted.
    Redacted(String, Option<String>),
}

impl From<&str> for StringEnvVarParams {
    fn from(s: &str) -> Self {
        StringEnvVarParams::Plain(String::from(s), None)
    }
}

impl From<String> for StringEnvVarParams {
    fn from(s: String) -> Self {
        StringEnvVarParams::Plain(s, None)
    }
}

impl From<(&str, bool)> for StringEnvVarParams {
    fn from((s, r): (&str, bool)) -> Self {
        if r {
            StringEnvVarParams::Redacted(String::from(s), None)
        } else {
            StringEnvVarParams::Plain(String::from(s), None)
        }
    }
}

impl From<(&str, bool, &str)> for StringEnvVarParams {
    fn from((s, r, c): (&str, bool, &str)) -> Self {
        if r {
            StringEnvVarParams::Redacted(String::from(s), Some(String::from(c)))
        } else {
            StringEnvVarParams::Plain(String::from(s), Some(String::from(c)))
        }
    }
}

/// An environment variable read as a string.
impl StringEnvVar {
    /// Read the env var from the environment.
    ///
    /// If not defined, read from a .env file.
    /// If still not defined, use the default.
    ///
    /// # Arguments
    ///
    /// * `var_name`: &str - the name of the env var
    /// * `default_value`: &str - the default value
    ///
    /// # Returns
    ///
    /// * Self - the value
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// #use cat_data_service::settings::StringEnvVar;
    ///
    /// let var = StringEnvVar::new("MY_VAR", "default");
    /// assert_eq!(var.as_str(), "default");
    /// ```
    pub(super) fn new(
        var_name: &str,
        param: StringEnvVarParams,
    ) -> Self {
        let (default_value, redacted, choices) = match param {
            StringEnvVarParams::Plain(s, c) => (s, false, c),
            StringEnvVarParams::Redacted(s, c) => (s, true, c),
        };

        match env::var(var_name) {
            Ok(value) => {
                let value = Self { value, redacted };
                info!("Env Var Defined; env={}, value={}", var_name, value);
                value
            },
            Err(err) => {
                let value = Self {
                    value: default_value,
                    redacted,
                };
                if err == VarError::NotPresent {
                    if let Some(choices) = choices {
                        info!(
                            "Env Var Defaulted; env={}, default={:?}, choices={:?}",
                            var_name, value, choices
                        );
                    } else {
                        info!("Env Var Defaulted; env={}, default={}", var_name, value);
                    }
                } else if let Some(choices) = choices {
                    info!(
                        "Env Var Error; env={}, default={}, choices={:?}, error={:?}",
                        var_name, value, choices, err
                    );
                } else {
                    info!(
                        "Env Var Error; env={}, default={}, error={:?}",
                        var_name, value, err
                    );
                }

                value
            },
        }
    }

    /// Convert an Envvar into the required Enum Type.
    pub(super) fn new_as_enum<T: FromStr + Display + VariantNames>(
        var_name: &str,
        default: T,
        redacted: bool,
    ) -> T
    where
        <T as std::str::FromStr>::Err: std::fmt::Display,
    {
        let mut choices = String::new();
        for name in T::VARIANTS {
            if choices.is_empty() {
                choices.push('[');
            } else {
                choices.push(',');
            }
            choices.push_str(name);
        }
        choices.push(']');

        let choice = StringEnvVar::new(
            var_name,
            (default.to_string().as_str(), redacted, choices.as_str()).into(),
        );

        let value = match T::from_str(choice.as_str()) {
            Ok(var) => var,
            Err(error) => {
                error!(
                    "Invalid choice. Using Default.; error={}, default={}, choices={:?}, choice={}",
                    error, default, choices, choice
                );
                default
            },
        };

        value
    }

    /// Get the read env var as a str.
    ///
    /// # Returns
    ///
    /// * &str - the value
    pub(crate) fn as_str(&self) -> &str {
        &self.value
    }
}

impl fmt::Display for StringEnvVar {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        if self.redacted {
            return write!(f, "REDACTED");
        }
        write!(f, "{}", self.value)
    }
}

impl fmt::Debug for StringEnvVar {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        if self.redacted {
            return write!(f, "REDACTED");
        }
        write!(f, "env: {}", self.value)
    }
}
