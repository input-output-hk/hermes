//! `JsonObject` Type.

use std::ops::{Deref, DerefMut};

use serde_json::{Map, Value};

/// `JSON` Object API definition
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub(crate) struct JsonObject(Map<String, Value>);

impl Deref for JsonObject {
    type Target = Map<String, Value>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for JsonObject {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl TryFrom<Value> for JsonObject {
    type Error = anyhow::Error;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        if let Value::Object(obj) = value {
            Ok(Self(obj))
        } else {
            anyhow::bail!("Provided JSON value not an object {value}")
        }
    }
}

impl From<JsonObject> for Value {
    fn from(value: JsonObject) -> Self {
        Value::Object(value.0)
    }
}

impl From<JsonObject> for Map<String, Value> {
    fn from(value: JsonObject) -> Self {
        value.0
    }
}
