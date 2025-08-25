use anyhow::{Result, bail};
use crate::kernel::{Runtime, Value, Packet};
use crate::kernel::ast::Arg;

// [myth] goal: lightweight arg parsing helpers
pub fn parse_two(rt: &Runtime, p: &Packet) -> Result<(Value, Value)> {
    let arg_str = match p.arg.as_ref() {
        Some(Arg::Str(s)) => s.clone(),
        _ => bail!("needs @<lhs> <rhs>"),
    };
    let mut parts = arg_str.split_whitespace();
    let a = parts.next().ok_or_else(|| anyhow::anyhow!("needs two args"))?;
    let b = parts.next().ok_or_else(|| anyhow::anyhow!("needs two args"))?;
    if parts.next().is_some() { bail!("expected two args"); }
    Ok((resolve(rt, a), resolve(rt, b)))
}

pub fn parse_one(rt: &Runtime, p: &Packet) -> Result<Value> {
    match p.arg.as_ref() {
        Some(Arg::Str(s)) => {
            let mut parts = s.split_whitespace();
            let token = parts.next().ok_or_else(|| anyhow::anyhow!("needs arg"))?;
            if parts.next().is_some() { bail!("expected one arg" ); }
            Ok(resolve(rt, token))
        }
        Some(Arg::Ident(id)) => Ok(resolve(rt, id)),
        Some(Arg::Number(n)) => Ok(Value::Num(*n)),
        _ => bail!("needs @arg"),
    }
}

fn resolve(rt: &Runtime, tok: &str) -> Value {
    if let Ok(n) = tok.parse::<f64>() {
        Value::Num(n)
    } else if tok.eq_ignore_ascii_case("true") {
        Value::Bool(true)
    } else if tok.eq_ignore_ascii_case("false") {
        Value::Bool(false)
    } else if let Some(v) = rt.get_var(tok) {
        v
    } else {
        Value::Str(tok.to_string())
    }
}
