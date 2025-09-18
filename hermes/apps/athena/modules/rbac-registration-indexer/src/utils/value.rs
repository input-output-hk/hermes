//! Hermes SQLite value conversion.

use crate::hermes::sqlite::api::Value;

// ------ Rust types to SQLite value conversion ------

impl From<String> for Value {
    fn from(v: String) -> Self {
        Value::Text(v)
    }
}

impl From<Vec<u8>> for Value {
    fn from(v: Vec<u8>) -> Self {
        Value::Blob(v)
    }
}

impl TryFrom<u64> for Value {
    type Error = anyhow::Error;

    fn try_from(v: u64) -> Result<Self, Self::Error> {
        match i64::try_from(v) {
            Ok(i) => Ok(Value::Int64(i)),
            Err(_) => Err(anyhow::anyhow!("u64 value too large for i64")),
        }
    }
}

impl From<u16> for Value {
    fn from(v: u16) -> Self {
        let i = i64::from(v);
        Value::Int64(i)
    }
}

// Generic option conversion
impl<T> From<Option<T>> for Value
where
    T: Into<Value>,
{
    fn from(opt: Option<T>) -> Self {
        opt.map(|v| v.into()).unwrap_or(Value::Null)
    }
}
