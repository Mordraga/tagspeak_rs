use anyhow::Result;
use crate::kernel::{Runtime, Value, Packet};

use super::util;

// [myth] goal: boolean negation
pub fn handle(rt: &mut Runtime, p: &Packet) -> Result<Value> {
    let v = util::parse_one(rt, p)?;
    Ok(Value::Bool(!v.as_bool().unwrap_or(false)))
}
