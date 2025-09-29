use crate::kernel::ast::Arg;
use crate::kernel::{Packet, Runtime, Value};
use anyhow::Result;

pub fn handle(_rt: &mut Runtime, p: &Packet) -> Result<Value> {
    // Optional: store last value or emit trace; by default do nothing.
    if let Some(Arg::Str(s)) = &p.arg {
        // If you have tracing, you could: rt.trace_note(s);
        let _ = s; // silence warning
    }
    Ok(Value::Unit)
}
