use anyhow::Result;
use crate::kernel::{Runtime, Value, Packet};

use super::util;

// [myth] goal: check if two values are equal
pub fn handle(rt: &mut Runtime, p: &Packet) -> Result<Value> {
    let (a, b) = util::parse_two(rt, p)?;
    Ok(Value::Bool(equals(&a, &b)))
}

fn equals(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Num(x), Value::Num(y)) => x == y,
        (Value::Str(x), Value::Str(y)) => x == y,
        (Value::Bool(x), Value::Bool(y)) => x == y,
        (Value::Unit, Value::Unit) => true,
        _ => false,
    }
}
