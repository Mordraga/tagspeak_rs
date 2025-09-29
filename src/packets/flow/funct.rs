use crate::kernel::{Packet, Runtime, Value};
use anyhow::{Result, bail};

pub fn handle(rt: &mut Runtime, p: &Packet) -> Result<Value> {
    // [funct:tag]{ ... }  => ns = Some("funct"), op = "tag", body = Some(...)
    let tag = p.op.as_str();
    let body = p
        .body
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("funct requires a {{ ... }} body"))?;
    if tag.is_empty() {
        bail!("funct needs a tag name: [funct:tag]{{...}}");
    }
    rt.register_tag(tag, body.clone());
    Ok(Value::Unit)
}
