// values.rs
use std::path::PathBuf;
use std::time::SystemTime;

use serde_json::Value as JsonValue;

#[derive(Clone, Debug)]
pub enum Value {
    Unit,
    Bool(bool),
    Num(f64),
    Str(String),
    Doc(Document),
}

#[derive(Clone, Debug)]
pub struct Document {
    pub json: JsonValue,
    pub path: PathBuf,
    pub ext: String,
    pub mtime: SystemTime,
    pub root: PathBuf,
    pub last_json: JsonValue,
}

impl Value {
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(b) => Some(*b),
            Value::Num(n) => Some(*n != 0.0 && !n.is_nan()),
            Value::Str(s) => Some(!s.is_empty()),
            Value::Unit => Some(false),
            Value::Doc(_) => Some(true),
        }
    }
    pub fn try_num(&self) -> Option<f64> {
        match self {
            Value::Num(n) => Some(*n),
            Value::Str(s) => s.parse().ok(),
            _ => None,
        }
    }
}

impl Document {
    pub fn new(json: JsonValue, path: PathBuf, ext: String, mtime: SystemTime, root: PathBuf) -> Self {
        Self {
            last_json: json.clone(),
            json,
            path,
            ext,
            mtime,
            root,
        }
    }
}
