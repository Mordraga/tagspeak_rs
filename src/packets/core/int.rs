use anyhow::{Result, bail};

use crate::kernel::ast::Arg;
use crate::kernel::{Packet, Runtime, Value};

pub fn handle(rt: &mut Runtime, p: &Packet) -> Result<Value> {
    let expr = match p.arg.as_ref() {
        Some(Arg::Number(n)) => return Ok(Value::Num(*n)),
        Some(Arg::Ident(id)) => {
            if let Some(v) = rt.get_var(id) {
                return Ok(match v {
                    Value::Num(n) => Value::Num(n.trunc()),
                    Value::Str(s) => Value::Num(s.parse::<f64>()?.trunc()),
                    _ => bail!("non_numeric_var"),
                });
            }
            id.clone()
        }
        Some(Arg::Str(s)) => s.clone(),
        _ => bail!("int needs @<number|expr>"),
    };

    let mut buf = String::new();
    for part in expr.split('+') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        if let Ok(n) = part.parse::<i64>() {
            buf.push_str(&n.to_string());
        } else if let Some(v) = rt.get_var(part) {
            match v {
                Value::Num(n) => buf.push_str(&format!("{}", n.trunc() as i64)),
                Value::Str(s) => buf.push_str(&s),
                _ => bail!("non_numeric_var"),
            }
        } else {
            bail!("unknown_segment");
        }
    }
    let n: i64 = buf.parse()?;
    Ok(Value::Num(n as f64))
}
