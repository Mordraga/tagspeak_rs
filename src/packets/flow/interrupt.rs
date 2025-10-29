use anyhow::Result;

use crate::kernel::runtime::FlowSignal;
use crate::kernel::{Packet, Runtime, Value};

pub fn handle(rt: &mut Runtime, p: &Packet) -> Result<Value> {
    let val = match p.arg.as_ref() {
        Some(arg) => rt.resolve_arg(arg)?,
        None => rt.last.clone(),
    };
    rt.set_signal(FlowSignal::Interrupt(Some(val.clone())));
    Ok(val)
}
