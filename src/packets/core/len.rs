use anyhow::Result;

use crate::kernel::{Arg, Packet, Runtime, Value};

// [len] -> length of last value (string/doc)
// [len@var|"text"] -> length of the provided arg
pub fn handle(rt: &mut Runtime, p: &Packet) -> Result<Value> {
    let v = match p.arg.as_ref() {
        Some(arg) => rt.resolve_arg(arg)?,
        None => rt.last.clone(),
    };

    let n = match v {
        Value::Str(s) => s.chars().count() as f64,
        Value::Doc(d) => {
            if d.json.is_array() {
                d.json.as_array().map(|a| a.len()).unwrap_or(0) as f64
            } else if d.json.is_object() {
                d.json.as_object().map(|o| o.len()).unwrap_or(0) as f64
            } else if d.json.is_string() {
                d.json.as_str().map(|s| s.chars().count()).unwrap_or(0) as f64
            } else {
                0.0
            }
        }
        Value::Num(_) | Value::Bool(_) | Value::Unit => 0.0,
    };
    Ok(Value::Num(n))
}
