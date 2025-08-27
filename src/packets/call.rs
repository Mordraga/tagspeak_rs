use anyhow::{bail, Result};
use crate::kernel::ast::Arg;
use crate::kernel::{Packet, Runtime, Value};

pub fn handle(rt: &mut Runtime, p: &Packet) -> Result<Value> {
    let name = match p.arg.as_ref() {
        Some(Arg::Ident(s)) | Some(Arg::Str(s)) => s.clone(),
        _ => bail!("call needs @function_name"),
    };

    let body = rt
        .tags
        .get(&name)
        .cloned()
        .ok_or_else(|| anyhow::anyhow!(format!("undefined function: {name}")))?;

    let mut last = Value::Unit;
    for node in body {
        last = rt.eval(&node)?;
    }
    Ok(last)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::router;
    use std::fs;

    #[test]
    fn calls_defined_function() -> Result<()> {
        let base = std::env::temp_dir().join(format!("tgsk_call_test_{}", std::process::id()));
        let _ = fs::remove_dir_all(&base);
        fs::create_dir_all(&base)?;
        fs::write(base.join("red.tgsk"), "")?;
        let script = base.join("main.tgsk");
        fs::write(&script, "[funct@hello]{[msg@\"hi\"]}>[call@hello]")?;
        let node = router::parse(&fs::read_to_string(&script)? )?;
        let mut rt = Runtime::from_entry(&script)?;
        let val = rt.eval(&node)?;
        assert_eq!(val, Value::Str("hi".to_string()));
        fs::remove_dir_all(base)?;
        Ok(())
    }
}
