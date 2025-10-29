use anyhow::Result;

use crate::kernel::runtime::FlowSignal;
use crate::kernel::{Packet, Runtime, Value};

pub fn handle(rt: &mut Runtime, _p: &Packet) -> Result<Value> {
    let last = rt.last.clone();
    rt.set_signal(FlowSignal::Break);
    Ok(last)
}
