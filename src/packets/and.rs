use anyhow::Result;
use crate::kernel::{Runtime, Value, Packet};

use super::util;

// [myth] goal: boolean conjunction
pub fn handle(rt: &mut Runtime, p: &Packet) -> Result<Value> {
    let (a, b) = util::parse_two(rt, p)?;
    Ok(Value::Bool(a.as_bool().unwrap_or(false) && b.as_bool().unwrap_or(false)))
}
