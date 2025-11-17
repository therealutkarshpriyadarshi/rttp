//! Database value types

use serde_json;

/// Database value type that can be used in queries
#[derive(Debug, Clone)]
pub enum Value {
    /// Null value
    Null,

    /// Boolean value
    Bool(bool),

    /// 16-bit integer
    I16(i16),

    /// 32-bit integer
    I32(i32),

    /// 64-bit integer
    I64(i64),

    /// 32-bit float
    F32(f32),

    /// 64-bit float
    F64(f64),

    /// String value
    String(String),

    /// Byte array
    Bytes(Vec<u8>),

    /// JSON value
    Json(serde_json::Value),
}

impl From<bool> for Value {
    fn from(v: bool) -> Self {
        Value::Bool(v)
    }
}

impl From<i16> for Value {
    fn from(v: i16) -> Self {
        Value::I16(v)
    }
}

impl From<i32> for Value {
    fn from(v: i32) -> Self {
        Value::I32(v)
    }
}

impl From<i64> for Value {
    fn from(v: i64) -> Self {
        Value::I64(v)
    }
}

impl From<f32> for Value {
    fn from(v: f32) -> Self {
        Value::F32(v)
    }
}

impl From<f64> for Value {
    fn from(v: f64) -> Self {
        Value::F64(v)
    }
}

impl From<String> for Value {
    fn from(v: String) -> Self {
        Value::String(v)
    }
}

impl From<&str> for Value {
    fn from(v: &str) -> Self {
        Value::String(v.to_string())
    }
}

impl From<Vec<u8>> for Value {
    fn from(v: Vec<u8>) -> Self {
        Value::Bytes(v)
    }
}

impl From<serde_json::Value> for Value {
    fn from(v: serde_json::Value) -> Self {
        Value::Json(v)
    }
}

impl<T> From<Option<T>> for Value
where
    T: Into<Value>,
{
    fn from(v: Option<T>) -> Self {
        match v {
            Some(val) => val.into(),
            None => Value::Null,
        }
    }
}
