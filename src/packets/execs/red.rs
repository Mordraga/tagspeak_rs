use anyhow::Result;

use crate::kernel::{Packet, Runtime, Value};

pub fn handle(rt: &mut Runtime, p: &Packet) -> Result<Value> {
    // Script-level consent: presence of [red] in the script enables red for this run.
    // Optional @message is accepted but not required.
    if let Some(arg) = &p.arg {
        // Store the last red message as context (optional)
        if let crate::kernel::Arg::Str(s) | crate::kernel::Arg::Ident(s) = arg {
            rt.set_var("__red_message", Value::Str(s.clone()))?;
        }
    }
    rt.set_var("__red_enabled", Value::Bool(true))?;
    Ok(Value::Bool(true))
}
