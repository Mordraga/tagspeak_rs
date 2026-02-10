use anyhow::{Result, bail};

use crate::kernel::{Packet, Runtime, Value};

pub fn handle(rt: &mut Runtime, p: &Packet) -> Result<Value> {
    let (name, is_async) = parse_funct_descriptor(p)?;
    let body = p
        .body
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("funct requires a {{ ... }} body"))?;
    rt.register_tag(&name, body.clone(), is_async);
    if is_async {
        // async functions default last value to Unit to avoid leaking previous sync runs
        rt.last = Value::Unit;
    }
    Ok(Value::Unit)
}

fn parse_funct_descriptor(p: &Packet) -> Result<(String, bool)> {
    match p.ns.as_deref() {
        Some("funct") | None => Ok((p.op.clone(), false)),
        Some(ns) if ns.starts_with("fn(") && ns.ends_with(')') => {
            let name = ns[3..ns.len() - 1].trim();
            if name.is_empty() {
                bail!("fn() needs a name");
            }
            if !p.op.eq_ignore_ascii_case("async") {
                bail!("expected [fn(name):async] for async functions");
            }
            Ok((name.to_string(), true))
        }
        _ => bail!("unrecognized funct namespace"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{kernel::Runtime, router};

    #[test]
    fn registers_async_function() -> Result<()> {
        let script = "[fn(job):async]{[return@1]}";
        let node = router::parse(script).map_err(anyhow::Error::new)?;
        let mut rt = Runtime::new()?;
        rt.eval(&node)?;
        let func = rt.get_tag("job").expect("function registered");
        assert!(func.is_async);
        Ok(())
    }
}
