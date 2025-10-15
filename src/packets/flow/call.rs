use crate::kernel::ast::Arg;
use crate::kernel::ast::Node;
use crate::kernel::{Packet, Runtime, Value};
use anyhow::{Result, bail};

pub fn handle(rt: &mut Runtime, p: &Packet) -> Result<Value> {
    let name = match p.arg.as_ref() {
        Some(Arg::Ident(id)) => id.as_str().to_string(),
        Some(Arg::Str(s)) => s.clone(),
        _ => bail!("call needs @<name>"),
    };
    let body = rt
        .tags
        .get(&name)
        .ok_or_else(|| anyhow::anyhow!(format!("unknown funct '{name}'")))?
        .clone();
    // Evaluate the stored block in current runtime
    let out = rt.eval(&Node::Block(body))?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{kernel::Runtime, router};

    #[test]
    fn defines_and_calls_function() -> Result<()> {
        let script = "[funct:step]{[math@1+2]>[store@x]}>[call@step]>[print@x]";
        let node = router::parse(script).map_err(anyhow::Error::new)?;
        let mut rt = Runtime::new()?;
        rt.eval(&node)?;
        assert_eq!(rt.get_num("x"), Some(3.0));
        Ok(())
    }
}
