use anyhow::Result;

use crate::kernel::{Arg, Packet, Runtime, Value};

// [env@NAME] -> returns environment variable as string, or Unit if missing
// Accepts @"NAME" or @NAME ident. Does not mutate state beyond last value.
pub fn handle(rt: &mut Runtime, p: &Packet) -> Result<Value> {
    let key = match &p.arg {
        Some(Arg::Str(s)) => s.clone(),
        Some(Arg::Ident(id)) => id.clone(),
        _ => String::new(),
    };
    if key.is_empty() {
        return Ok(Value::Unit);
    }
    match std::env::var(&key) {
        Ok(v) => Ok(Value::Str(v)),
        Err(_) => Ok(Value::Unit),
    }
}

