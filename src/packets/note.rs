use anyhow::Result;
use crate::kernel::{Runtime, Value, Packet};
use crate::kernel::ast::Arg;

pub fn handle(_rt: &mut Runtime, p: &Packet) -> Result<Value> {
    // Optional: store last value or emit trace; by default do nothing.
    if let Some(Arg::Str(s)) = &p.arg {
        // If you have tracing, you could: rt.trace_note(s);
        let _ = s; // silence warning
    }
    Ok(Value::Unit)
}
