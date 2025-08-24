use anyhow::Result;
use crate::kernel::{Runtime, Value, Packet};

pub fn handle(_rt: &mut Runtime, _p: &Packet) -> Result<Value> {
    anyhow::bail!("if/else not implemented yet")
}
