use anyhow::Result;

use crate::kernel::ast::Arg;
use crate::kernel::{Packet, Runtime, Value};

pub fn handle(rt: &mut Runtime, p: &Packet) -> Result<Value> {
    let expr = match p.arg.as_ref() {
        Some(Arg::Str(s)) => s.as_str(),
        Some(Arg::Ident(id)) => id.as_str(),
        _ => "",
    };
    let out = eval_concat(rt, expr)?;
    Ok(Value::Str(out))
}

fn eval_concat(rt: &Runtime, expr: &str) -> Result<String> {
    let mut out = String::new();
    for part in expr.split('+') {
        let part = part.trim();
        if part.is_empty() { continue; }
        if part.starts_with('"') && part.ends_with('"') {
            let s: String = serde_json::from_str(part)?;
            out.push_str(&s);
        } else if let Some(v) = rt.get_var(part) {
            match v {
                Value::Str(s) => out.push_str(&s),
                Value::Num(n) => out.push_str(&format!("{}", n)),
                Value::Bool(b) => out.push_str(if b { "true" } else { "false" }),
                Value::Unit => {},
                Value::Doc(_) => out.push_str("[doc]"),
            }
        } else {
            out.push_str(part);
        }
    }
    Ok(out)
}

