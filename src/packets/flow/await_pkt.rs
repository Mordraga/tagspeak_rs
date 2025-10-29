use anyhow::{Result, bail};

use crate::kernel::ast::Arg;
use crate::kernel::{Packet, Runtime, Value};

pub fn handle(rt: &mut Runtime, p: &Packet) -> Result<Value> {
    let name = match p.arg.as_ref() {
        Some(Arg::Ident(id)) => id.clone(),
        Some(Arg::Str(s)) => s.clone(),
        _ => bail!("[await@name] expects identifier or string handle"),
    };
    rt.await_async_function(&name)
}
