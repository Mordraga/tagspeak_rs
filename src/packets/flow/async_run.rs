use anyhow::{Result, bail};

use crate::kernel::ast::Arg;
use crate::kernel::{Packet, Runtime, Value};

pub fn handle(rt: &mut Runtime, p: &Packet) -> Result<Value> {
    let has_body = p.body.is_some();
    let has_arg = p.arg.is_some();

    if has_body && has_arg {
        bail!("[async] accepts either a body or @function, not both");
    }

    if let Some(body) = &p.body {
        rt.spawn_async_block(body.clone())?;
        return Ok(Value::Unit);
    }

    if let Some(arg) = &p.arg {
        let name = match arg {
            Arg::Ident(id) => id.clone(),
            Arg::Str(s) => s.clone(),
            _ => bail!("[async@name] expects identifier or string handle"),
        };
        rt.enqueue_async_function(&name)?;
        return Ok(Value::Unit);
    }

    bail!("[async] needs either a {{ ... }} block or @function target");
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{kernel::Runtime, router};

    #[test]
    fn async_function_runs_via_await() -> Result<()> {
        let script = "\
[fn(ping):async]{[timeout:ms@1][return@42]}\
[async@ping]\
[await@ping]>[store@result]";
        let node = router::parse(script).map_err(anyhow::Error::new)?;
        let mut rt = Runtime::new()?;
        rt.eval(&node)?;
        assert_eq!(rt.get_num("result"), Some(42.0));
        Ok(())
    }
}
