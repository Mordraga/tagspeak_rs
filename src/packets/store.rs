use anyhow::Result;
use crate::kernel::{Runtime, Value, Packet};
use crate::kernel::ast::Arg;

pub fn handle(rt: &mut Runtime, p: &Packet) -> Result<Value> {
    let name = match p.arg.as_ref() {
        Some(Arg::Ident(id)) => id.as_str(),
        _ => anyhow::bail!("store needs @<ident>"),
    };
    let val = rt.last.clone(); // store the pipeline's last value
    rt.set_var(name, val.clone());
    Ok(val)
}
