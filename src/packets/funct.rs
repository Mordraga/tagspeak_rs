use anyhow::{Result, bail};
use crate::kernel::ast::Arg;
use crate::kernel::{Runtime, Value, Packet};

pub fn handle(rt: &mut Runtime, p: &Packet) -> Result<Value> {
    let tag = if p.ns.as_deref() == Some("funct") {
        p.op.clone()
    } else if p.ns.is_none() && p.op == "funct" {
        match p.arg.as_ref() {
            Some(Arg::Ident(s)) | Some(Arg::Str(s)) => s.clone(),
            _ => bail!("funct needs @name"),
        }
    } else {
        bail!("invalid funct packet");
    };

    let body = p
        .body
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("funct requires a {{ ... }} body"))?;
    if tag.is_empty() { bail!("funct needs a tag name"); }
    rt.register_tag(&tag, body.clone());
    Ok(Value::Unit)
}
