use anyhow::Result;
use crate::kernel::{Runtime, Value, Packet};

use super::util;

// [myth] goal: numeric greater-than
pub fn handle(rt: &mut Runtime, p: &Packet) -> Result<Value> {
    let (a, b) = util::parse_two(rt, p)?;
    let an = a.try_num().ok_or_else(|| anyhow::anyhow!("gt needs numbers"))?;
    let bn = b.try_num().ok_or_else(|| anyhow::anyhow!("gt needs numbers"))?;
    Ok(Value::Bool(an > bn))
}
