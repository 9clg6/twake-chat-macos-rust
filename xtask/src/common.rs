//! What the generators of `cargo xtask` share: the error and the reading of JSON values.

use std::fmt;

use serde_json::{Map, Value};

/// Why a file could not be generated; the text says what is wrong.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error(String);

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

pub(crate) type Res<T> = Result<T, Error>;

pub(crate) fn fail<T>(message: String) -> Res<T> {
    Err(Error(message))
}

pub(crate) fn string<'a>(value: &'a Value, what: &str) -> Res<&'a str> {
    match value.as_str() {
        Some(text) => Ok(text),
        None => fail(format!("{what} is not a string")),
    }
}

pub(crate) fn object<'a>(value: &'a Value, what: &str) -> Res<&'a Map<String, Value>> {
    match value.as_object() {
        Some(map) => Ok(map),
        None => fail(format!("{what} is not an object")),
    }
}

/// The JSON document at the root, which must be an object.
pub(crate) fn root(json: &str, what: &str) -> Res<Map<String, Value>> {
    match serde_json::from_str::<Value>(json) {
        Ok(Value::Object(map)) => Ok(map),
        Ok(_) => fail(format!("{what} is not an object")),
        Err(error) => fail(format!("invalid JSON: {error}")),
    }
}
