use crate::kernel::{Packet, Runtime, Value};
use crate::kernel::ast::Arg;
use anyhow::{Result, bail};

// [var@name] -> returns the current value of runtime variable `name` (or Unit if missing)
pub fn handle(rt: &mut Runtime, p: &Packet) -> Result<Value> {
    let name = match p.arg.as_ref() {
        Some(Arg::Ident(id)) => id.as_str(),
        Some(Arg::Str(s)) => s.as_str(),
        _ => bail!("var needs @<ident|\"name\">")
    };
    Ok(rt.get_var(name).unwrap_or(Value::Unit))
}

