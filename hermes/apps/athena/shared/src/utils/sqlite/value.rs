//! Hermes SQLite value conversion.

use cardano_blockchain_types;

use crate::bindings::hermes::sqlite::api::Value;

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

impl From<cardano_blockchain_types::hashes::TransactionId> for Value {
    fn from(v: cardano_blockchain_types::hashes::TransactionId) -> Self {
        Value::Blob(v.into())
    }
}


impl From<cardano_blockchain_types::pallas_addresses::Address> for Value {
    fn from(v: cardano_blockchain_types::pallas_addresses::Address) -> Self {
        Value::Blob(v.to_vec())
    }
}

impl From<cardano_blockchain_types::pallas_addresses::StakeAddress> for Value {
    fn from(v: cardano_blockchain_types::pallas_addresses::StakeAddress) -> Self {
        cardano_blockchain_types::pallas_addresses::Address::Stake(v).into()
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

// ------ SQLite value to Rust types conversion ------

impl TryFrom<Value> for () {
    type Error = anyhow::Error;

    fn try_from(v: Value) -> Result<Self, Self::Error> {
        match v {
            Value::Null => Ok(()),
            _ => Err(anyhow::anyhow!("Value is not a null")),
        }
    }
}

impl TryFrom<Value> for String {
    type Error = anyhow::Error;

    fn try_from(v: Value) -> Result<Self, Self::Error> {
        match v {
            Value::Text(s) => Ok(s),
            _ => Err(anyhow::anyhow!("Value is not a Text")),
        }
    }
}

impl TryFrom<Value> for Vec<u8> {
    type Error = anyhow::Error;

    fn try_from(v: Value) -> Result<Self, Self::Error> {
        match v {
            Value::Blob(b) => Ok(b),
            _ => Err(anyhow::anyhow!("Value is not a Blob")),
        }
    }
}

impl TryFrom<Value> for u64 {
    type Error = anyhow::Error;

    fn try_from(v: Value) -> Result<Self, Self::Error> {
        match v {
            Value::Int64(i) => {
                u64::try_from(i).map_err(|_| anyhow::anyhow!("Cannot convert Int64 to u64"))
            },
            Value::Int32(i) => {
                u64::try_from(i).map_err(|_| anyhow::anyhow!("Cannot convert Int32 to u64"))
            },
            _ => Err(anyhow::anyhow!("Value is not an integer")),
        }
    }
}

impl TryFrom<Value> for u16 {
    type Error = anyhow::Error;

    fn try_from(v: Value) -> Result<Self, Self::Error> {
        match v {
            Value::Int64(i) => {
                u16::try_from(i).map_err(|_| anyhow::anyhow!("Cannot convert Int64 to u16"))
            },
            Value::Int32(i) => {
                u16::try_from(i).map_err(|_| anyhow::anyhow!("Cannot convert Int32 to u16"))
            },
            _ => Err(anyhow::anyhow!("Value is not an integer")),
        }
    }
}

impl<T> TryFrom<Value> for Option<T>
where
    T: TryFrom<Value, Error = anyhow::Error>,
{
    type Error = anyhow::Error;

    fn try_from(v: Value) -> Result<Self, Self::Error> {
        match v {
            Value::Null => Ok(None),
            other => {
                let t = T::try_from(other)?;
                Ok(Some(t))
            },
        }
    }
}

impl TryFrom<Value> for cardano_blockchain_types::hashes::TransactionId {
    type Error = anyhow::Error;

    fn try_from(v: Value) -> Result<Self, Self::Error> {
        match v {
            Value::Blob(b) => {
                let hash: cardano_blockchain_types::hashes::Blake2bHash<32> = b.try_into()?;
                Ok(hash.into())
            },
            _ => Err(anyhow::anyhow!("Value is not a Blob for TransactionId")),
        }
    }
}

impl TryFrom<Value> for cardano_blockchain_types::pallas_primitives::BigInt {
    type Error = anyhow::Error;

    fn try_from(v: Value) -> Result<Self, Self::Error> {
        match v {
            Value::Blob(b) => {
                cardano_blockchain_types::pallas_codec::minicbor::decode(&b).map_err(Into::into)
            },
            _ => Err(anyhow::anyhow!("Value is not a Blob for BigInt")),
        }
    }
}

impl TryFrom<Value> for cardano_blockchain_types::pallas_addresses::Address {
    type Error = anyhow::Error;

    fn try_from(v: Value) -> Result<Self, Self::Error> {
        match v {
            Value::Blob(b) => cardano_blockchain_types::pallas_addresses::Address::from_bytes(&b)
                .map_err(Into::into),
            _ => Err(anyhow::anyhow!("Value is not a Blob for Address")),
        }
    }
}

impl TryFrom<Value> for cardano_blockchain_types::pallas_addresses::StakeAddress {
    type Error = anyhow::Error;

    fn try_from(v: Value) -> Result<Self, Self::Error> {
        cardano_blockchain_types::pallas_addresses::Address::try_from(v).and_then(|v| {
            if let cardano_blockchain_types::pallas_addresses::Address::Stake(addr) = v {
                Ok(addr)
            } else {
                anyhow::bail!("Decoded as an Address, but not the StakeAddress")
            }
        })
    }
}

impl TryFrom<Value> for cardano_blockchain_types::pallas_primitives::PolicyId {
    type Error = anyhow::Error;

    fn try_from(v: Value) -> Result<Self, Self::Error> {
        match v {
            Value::Blob(b) => {
                cardano_blockchain_types::pallas_codec::minicbor::decode(&b).map_err(Into::into)
            },
            _ => Err(anyhow::anyhow!("Value is not a Blob for PolicyId")),
        }
    }
}

impl TryFrom<Value> for cardano_blockchain_types::pallas_primitives::Bytes {
    type Error = anyhow::Error;

    fn try_from(v: Value) -> Result<Self, Self::Error> {
        match v {
            Value::Blob(b) => Ok(b.into()),
            _ => Err(anyhow::anyhow!("Value is not a Blob for Bytes")),
        }
    }
}
