//! The arguments of a call as the `Input` of the generated command code.
//!
//! Pattern: Adapter — [`JsonInput`] answers the questions of `cligen::rt::Input` (named values by path) from the JSON
//! the client sent. The command code spells a path in kebab case (`other-uid`), the JSON uses the names of the contract
//! (`otherUid`), so every segment is compared in kebab form; a list element is its index (`ids.0`). What is wrongly typed
//! is a `BadValue` of that path, never a guess: the run is not started with such an argument (see [`super::fit`]).

use capnp_compiler::cligen::model::kebab;
use capnp_compiler::cligen::rt::{CallError, ErrorKind, Input};
use serde_json::Value;

use super::base64;

/// The arguments of one call.
pub struct JsonInput {
    root: Value,
}

impl JsonInput {
    /// The arguments object of a `tools/call`.
    #[must_use]
    pub fn new(arguments: Value) -> Self {
        Self { root: arguments }
    }

    fn at(&self, path: &str) -> Option<&Value> {
        let mut v = &self.root;
        if path.is_empty() {
            return Some(v);
        }
        for seg in path.split('.') {
            v = match v {
                Value::Object(m) => {
                    m.iter()
                        .find(|(k, _)| kebab(k) == seg || k.as_str() == seg)?
                        .1
                }
                Value::Array(a) => a.get(seg.parse::<usize>().ok()?)?,
                _ => return None,
            };
        }
        Some(v)
    }

    fn given(&self, path: &str) -> Option<&Value> {
        self.at(path).filter(|v| !v.is_null())
    }
}

fn bad(path: &str, what: &str) -> CallError {
    CallError::at(path, ErrorKind::BadValue(what.to_owned()))
}

impl Input for JsonInput {
    fn has(&self, path: &str) -> bool {
        self.given(path).is_some()
    }

    fn text(&mut self, path: &str) -> Result<Option<String>, CallError> {
        match self.given(path) {
            None => Ok(None),
            Some(Value::String(s)) => Ok(Some(s.clone())),
            Some(_) => Err(bad(path, "a text is expected")),
        }
    }

    fn int(&mut self, path: &str) -> Result<Option<i128>, CallError> {
        match self.given(path) {
            None => Ok(None),
            // a 64-bit integer is a string (the capnp-JSON codec writes and reads it so); the others are numbers
            Some(Value::String(s)) => s
                .parse::<i128>()
                .map(Some)
                .map_err(|_| bad(path, "an integer is expected")),
            Some(Value::Number(n)) => n
                .as_i64()
                .map(i128::from)
                .or_else(|| n.as_u64().map(i128::from))
                .map(Some)
                .ok_or_else(|| bad(path, "an integer is expected")),
            Some(_) => Err(bad(path, "an integer is expected")),
        }
    }

    fn float(&mut self, path: &str) -> Result<Option<f64>, CallError> {
        match self.given(path) {
            None => Ok(None),
            Some(Value::Number(n)) => Ok(n.as_f64()),
            Some(Value::String(s)) => match s.as_str() {
                "Infinity" => Ok(Some(f64::INFINITY)),
                "-Infinity" => Ok(Some(f64::NEG_INFINITY)),
                "NaN" => Ok(Some(f64::NAN)),
                _ => Err(bad(path, "a number is expected")),
            },
            Some(_) => Err(bad(path, "a number is expected")),
        }
    }

    fn boolean(&mut self, path: &str) -> Result<Option<bool>, CallError> {
        match self.given(path) {
            None => Ok(None),
            Some(Value::Bool(b)) => Ok(Some(*b)),
            Some(_) => Err(bad(path, "true or false is expected")),
        }
    }

    fn enumerant(&mut self, path: &str, names: &[&str]) -> Result<Option<usize>, CallError> {
        match self.given(path) {
            None => Ok(None),
            Some(Value::String(s)) => names
                .iter()
                .position(|n| n == s)
                .map(Some)
                .ok_or_else(|| bad(path, "not a name of this enumeration")),
            Some(_) => Err(bad(path, "a name of the enumeration is expected")),
        }
    }

    fn count(&mut self, path: &str) -> Result<Option<u32>, CallError> {
        match self.given(path) {
            None => Ok(None),
            Some(Value::Array(a)) => u32::try_from(a.len())
                .map(Some)
                .map_err(|_| bad(path, "too many elements")),
            Some(_) => Err(bad(path, "a list is expected")),
        }
    }

    fn data(&mut self, path: &str) -> Result<Option<Vec<u8>>, CallError> {
        match self.given(path) {
            None => Ok(None),
            Some(Value::String(s)) => base64::decode(s)
                .map(Some)
                .ok_or_else(|| bad(path, "base64 text is expected")),
            Some(_) => Err(bad(path, "base64 text is expected")),
        }
    }
}
