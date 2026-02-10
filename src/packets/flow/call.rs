use anyhow::{Result, bail};

use crate::kernel::ast::{Arg, Node};
use crate::kernel::runtime::FlowSignal;
use crate::kernel::{Packet, Runtime, Value};

pub fn handle(rt: &mut Runtime, p: &Packet) -> Result<Value> {
    let name = match p.arg.as_ref() {
        Some(Arg::Ident(id)) => id.as_str().to_string(),
        Some(Arg::Str(s)) => s.clone(),
        _ => bail!("call needs @<name>"),
    };
    let func = rt
        .get_tag(&name)
        .ok_or_else(|| anyhow::anyhow!(format!("unknown funct '{name}'")))?
        .clone();
    let body = func.body;
    // Evaluate the stored block in current runtime with recursion depth guard
    if rt.call_depth >= rt.max_call_depth {
        bail!(
            "E_CALL_DEPTH_EXCEEDED: max recursion depth {} reached",
            rt.max_call_depth
        );
    }
    rt.call_depth += 1;
    let result = rt.eval(&Node::Block(body));
    rt.call_depth = rt.call_depth.saturating_sub(1);
    let out = result?;
    match rt.flow_signal.clone() {
        FlowSignal::Return(Some(val)) => {
            rt.take_signal();
            Ok(val)
        }
        FlowSignal::Return(None) => {
            rt.take_signal();
            Ok(out)
        }
        _ => Ok(out),
    }
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
