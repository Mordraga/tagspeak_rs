use anyhow::{Result, bail};
use crate::kernel::{Runtime, Value, Packet};
use crate::kernel::ast::{Arg, Node};

pub fn handle(rt: &mut Runtime, p: &Packet) -> Result<Value> {
    // count: support "loop3" AND "[loop@N]{...}"
    let count = if let Some(rest) = p.op.strip_prefix("loop") {
        if rest.is_empty() {
            // form: [loop@N]{...}
            match p.arg.as_ref() {
                Some(Arg::Number(n)) => *n as usize,
                Some(Arg::Ident(id)) => rt.get_var(id).and_then(|v| v.try_num()).unwrap_or(0.0) as usize,
                Some(Arg::Str(s))    => s.parse::<f64>().unwrap_or(0.0) as usize,
                _ => bail!("loop needs N: [loop@3]{{...}} or [loop3@tag]"),
            }
        } else {
            // form: [loop3@tag]
            rest.parse::<usize>()
                .map_err(|_| anyhow::anyhow!("invalid loop count in [{}]", p.op))?
        }
    } else {
        bail!("use [loop3@tag] or [loop@N]{{...}}");
    };

    // choose body: inline or tag
    if let Some(body) = &p.body {
        let mut last = Value::Unit;
        for _ in 0..count {
            last = rt.eval(&Node::Block(body.clone()))?;
        }
        return Ok(last);
    }

    // tag-based body
    let tag = match p.arg.as_ref() {
        Some(Arg::Ident(s)) | Some(Arg::Str(s)) => s.as_str(),
        _ => bail!("loopN needs @tag: [loop3@tag]"),
    };
    let body = rt.tags.get(tag)
        .ok_or_else(|| anyhow::anyhow!("unknown tag '{tag}' — define [funct:{tag}]{{...}} first"))?
        .clone();

    let mut last = Value::Unit;
    for _ in 0..count {
        last = rt.eval(&Node::Block(body.clone()))?;
    }
    Ok(last)
}
