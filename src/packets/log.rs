use anyhow::{Result, bail};
use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;

use crate::kernel::ast::Arg;
use crate::kernel::fs_guard::resolve;
use crate::kernel::{Packet, Runtime, Value};

fn to_json(v: &Value) -> serde_json::Value {
    match v {
        Value::Unit => serde_json::Value::Null,
        Value::Bool(b) => serde_json::Value::Bool(*b),
        Value::Num(n) => serde_json::Number::from_f64(*n)
            .map(serde_json::Value::Number)
            .unwrap_or(serde_json::Value::Null),
        Value::Str(s) => serde_json::Value::String(s.clone()),
        Value::Doc(d) => d.json.clone(),
    }
}

pub fn handle(rt: &mut Runtime, p: &Packet) -> Result<Value> {
    let raw = match &p.arg {
        Some(Arg::Str(s)) => s,
        _ => bail!("log needs @<path>"),
    };

    let root = rt
        .effective_root
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("no red.tgsk root"))?;

    let rel = if raw.starts_with('/') { &raw[1..] } else { raw.as_str() };
    let candidate = if raw.starts_with('/') {
        Path::new(rel).to_path_buf()
    } else {
        rt.cwd.join(rel)
    };
    let path = resolve(root, &candidate)?;

    let json = serde_json::to_string_pretty(&to_json(&rt.last))?;
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    writeln!(file, "{}", json)?;
    Ok(rt.last.clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use crate::router;

    #[test]
    fn writes_json() -> Result<()> {
        let base = std::env::temp_dir().join(format!("tgsk_log_test_{}", std::process::id()));
        let _ = fs::remove_dir_all(&base);
        fs::create_dir_all(base.join("sub"))?;
        fs::write(base.join("red.tgsk"), "")?;
        let script = base.join("sub").join("main.tgsk");
        fs::write(&script, "[msg@\"hi\"]>[log@/out.json]")?;
        let node = router::parse(&fs::read_to_string(&script)? )?;
        let mut rt = Runtime::from_entry(&script)?;
        rt.eval(&node)?;
        let content = fs::read_to_string(base.join("out.json"))?;
        assert!(content.contains("\"hi\""));
        fs::remove_dir_all(base)?;
        Ok(())
    }
}
