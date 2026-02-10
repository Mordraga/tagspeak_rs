use anyhow::{Result, bail};

use crate::kernel::Packet;
use crate::kernel::ast::{Arg, BExpr, Node};
use crate::kernel::runtime::{FlowSignal, Runtime};
use crate::kernel::values::{Document, Value};
use crate::packets::flow::conditionals;

const DEFAULT_LOOP_MAX: usize = 1_000_000;

pub fn handle(rt: &mut Runtime, p: &Packet) -> Result<Value> {
    if matches!(p.ns.as_deref(), Some("loop")) {
        handle_namespaced(rt, p)
    } else {
        handle_basic(rt, p)
    }
}

fn handle_basic(rt: &mut Runtime, p: &Packet) -> Result<Value> {
    if !p.op.starts_with("loop") {
        bail!("use [loopN@tag], [loop:tag@N], or [loop@N]{{...}}");
    }

    let suffix = &p.op[4..];
    if suffix.is_empty() {
        // [loop@N]{ ... }
        let body = clone_body(p)?;
        let count = parse_count(rt, p.arg.as_ref())?;
        enforce_iteration_limit(count)?;
        run_counted_loop(rt, count, body)
    } else {
        // [loopN@tag]
        let count: usize = suffix
            .parse()
            .map_err(|_| anyhow::anyhow!("invalid loop count in [{}]", p.op))?;
        enforce_iteration_limit(count)?;
        let tag = parse_tag_arg(p.arg.as_ref())?;
        let body = resolve_tag_body(rt, &tag)?;
        run_counted_loop(rt, count, body)
    }
}

fn handle_namespaced(rt: &mut Runtime, p: &Packet) -> Result<Value> {
    let op_lower = p.op.to_ascii_lowercase();
    if op_lower == "forever" {
        let body = clone_body(p)?;
        run_forever_loop(rt, body)
    } else if op_lower.starts_with("until") {
        let cond = parse_loop_condition(p)?;
        let body = clone_body(p)?;
        run_until_loop(rt, cond, body)
    } else if op_lower.starts_with("each") {
        let spec = extract_each_spec(p)?;
        let body = clone_body(p)?;
        run_each_loop(rt, &spec, body)
    } else {
        // treat as tagged loop [loop:tag@N]
        let count = parse_count(rt, p.arg.as_ref())?;
        enforce_iteration_limit(count)?;
        let body = resolve_tag_body(rt, &p.op)?;
        run_counted_loop(rt, count, body)
    }
}

fn run_counted_loop(rt: &mut Runtime, count: usize, body: Vec<Node>) -> Result<Value> {
    let mut last = Value::Unit;
    for _ in 0..count {
        if rt.signal_active() {
            break;
        }
        last = rt.eval(&Node::Block(body.clone()))?;
        if handle_loop_signal(rt) {
            break;
        }
    }
    Ok(last)
}

fn run_forever_loop(rt: &mut Runtime, body: Vec<Node>) -> Result<Value> {
    let mut last = Value::Unit;
    let max_iters = max_loop_iterations();
    let mut iterations = 0usize;
    loop {
        if rt.signal_active() {
            break;
        }
        if iterations >= max_iters {
            bail!(
                "E_LOOP_OVERFLOW: loop exceeded max iteration budget of {}",
                max_iters
            );
        }
        iterations += 1;
        last = rt.eval(&Node::Block(body.clone()))?;
        if handle_loop_signal(rt) {
            break;
        }
    }
    Ok(last)
}

fn run_until_loop(rt: &mut Runtime, cond: BExpr, body: Vec<Node>) -> Result<Value> {
    let mut last = Value::Unit;
    let max_iters = max_loop_iterations();
    let mut iterations = 0usize;
    while !conditionals::eval_cond(rt, &cond)? {
        if rt.signal_active() {
            break;
        }
        if iterations >= max_iters {
            bail!(
                "E_LOOP_OVERFLOW: loop:until exceeded max iteration budget of {}",
                max_iters
            );
        }
        iterations += 1;
        last = rt.eval(&Node::Block(body.clone()))?;
        if handle_loop_signal(rt) {
            break;
        }
    }
    Ok(last)
}

fn run_each_loop(rt: &mut Runtime, spec: &str, body: Vec<Node>) -> Result<Value> {
    let (item_var, idx_var, handle_name) = parse_each_spec(spec)?;
    let doc = match rt.get_var(&handle_name) {
        Some(Value::Doc(d)) => d,
        _ => bail!("loop:each handle_unknown"),
    };
    if !doc.json.is_array() {
        bail!("loop:each requires an array handle");
    }
    let arr = doc.json.as_array().unwrap();
    let mut last = Value::Unit;
    for (idx, item) in arr.iter().enumerate() {
        let value = json_to_value(item, &doc);
        rt.set_var(&item_var, value.clone())?;
        rt.last = value.clone();
        if let Some(idx_name) = &idx_var {
            rt.set_var(idx_name, Value::Num(idx as f64))?;
        }
        last = rt.eval(&Node::Block(body.clone()))?;
        if handle_loop_signal(rt) {
            break;
        }
    }
    Ok(last)
}

fn handle_loop_signal(rt: &mut Runtime) -> bool {
    match rt.flow_signal.clone() {
        FlowSignal::None => false,
        FlowSignal::Break => {
            rt.take_signal();
            true
        }
        FlowSignal::Return(_) | FlowSignal::Interrupt(_) => true,
    }
}

fn clone_body(p: &Packet) -> Result<Vec<Node>> {
    p.body
        .as_ref()
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("loop requires a {{ ... }} body"))
}

fn parse_count(rt: &Runtime, arg: Option<&Arg>) -> Result<usize> {
    let raw = match arg {
        Some(Arg::Number(n)) => *n,
        Some(Arg::Ident(id)) => rt.get_var(id).and_then(|v| v.try_num()).unwrap_or(0.0),
        Some(Arg::Str(s)) => s.parse::<f64>().unwrap_or(0.0),
        _ => bail!("loop needs count: [loop@3]{{...}} or [loop:tag@3]"),
    };
    number_to_count(raw)
}

fn parse_tag_arg(arg: Option<&Arg>) -> Result<String> {
    match arg {
        Some(Arg::Ident(id)) => Ok(id.clone()),
        Some(Arg::Str(s)) => Ok(s.clone()),
        _ => bail!("loopN needs @tag: [loop3@tag] or [loop:tag@N]"),
    }
}

fn resolve_tag_body(rt: &Runtime, tag: &str) -> Result<Vec<Node>> {
    rt.get_tag(tag).cloned().map(|def| def.body).ok_or_else(|| {
        anyhow::anyhow!(format!(
            "unknown tag '{tag}' — define [funct:{tag}]{{...}} first"
        ))
    })
}

fn parse_loop_condition(p: &Packet) -> Result<BExpr> {
    if let Some(src) = crate::router::extract_paren(&p.op) {
        return Ok(conditionals::parse_cond(src));
    }
    if let Some(arg) = &p.arg {
        match arg {
            Arg::CondSrc(src) | Arg::Str(src) => {
                let inner = unwrap_parens(src);
                return Ok(conditionals::parse_cond(inner));
            }
            Arg::Ident(id) => return Ok(conditionals::parse_cond(id)),
            Arg::Number(n) => return Ok(conditionals::parse_cond(&n.to_string())),
        }
    }
    bail!("loop:until requires a condition");
}

fn unwrap_parens(src: &str) -> &str {
    let trimmed = src.trim();
    if trimmed.starts_with('(') && trimmed.ends_with(')') && trimmed.len() >= 2 {
        &trimmed[1..trimmed.len() - 1]
    } else {
        trimmed
    }
}

fn extract_each_spec(p: &Packet) -> Result<String> {
    if let Some(inner) = crate::router::extract_paren(&p.op) {
        return Ok(inner.trim().to_string());
    }
    if let Some(arg) = &p.arg {
        return match arg {
            Arg::Str(s) => Ok(s.trim().to_string()),
            Arg::Ident(id) => Ok(format!("it@{}", id)),
            Arg::CondSrc(src) => Ok(unwrap_parens(src).to_string()),
            Arg::Number(_) => bail!("loop:each spec cannot be numeric"),
        };
    }
    bail!("loop:each requires (item@handle) spec");
}

fn parse_each_spec(spec: &str) -> Result<(String, Option<String>, String)> {
    let mut parts = spec.split('@');
    let left = parts
        .next()
        .ok_or_else(|| anyhow::anyhow!("loop:each missing variable before '@'"))?
        .trim();
    let handle = parts
        .next()
        .ok_or_else(|| anyhow::anyhow!("loop:each missing handle after '@'"))?
        .trim();
    if parts.next().is_some() {
        bail!("loop:each spec may only contain one '@'");
    }
    let mut vars = left.split(',').map(|s| s.trim()).filter(|s| !s.is_empty());
    let item = vars
        .next()
        .ok_or_else(|| anyhow::anyhow!("loop:each requires an item variable"))?
        .to_string();
    let idx = vars.next().map(|s| s.to_string());
    if vars.next().is_some() {
        bail!("loop:each accepts at most one index variable");
    }
    if handle.is_empty() {
        bail!("loop:each handle cannot be empty");
    }
    Ok((item, idx, handle.to_string()))
}

fn number_to_count(raw: f64) -> Result<usize> {
    if !raw.is_finite() || raw < 0.0 {
        bail!("loop count must be a non-negative number");
    }
    Ok(raw.floor() as usize)
}

fn enforce_iteration_limit(count: usize) -> Result<()> {
    let max = max_loop_iterations();
    if count > max {
        bail!("E_LOOP_OVERFLOW: count {} exceeds max {}", count, max);
    }
    Ok(())
}

fn max_loop_iterations() -> usize {
    std::env::var("TAGSPEAK_MAX_LOOP_ITERATIONS")
        .ok()
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(DEFAULT_LOOP_MAX)
}

fn json_to_value(v: &serde_json::Value, meta: &Document) -> Value {
    match v {
        serde_json::Value::Null => Value::Unit,
        serde_json::Value::Bool(b) => Value::Bool(*b),
        serde_json::Value::Number(n) => n.as_f64().map(Value::Num).unwrap_or(Value::Unit),
        serde_json::Value::String(s) => Value::Str(s.clone()),
        serde_json::Value::Array(_) | serde_json::Value::Object(_) => {
            let mut d = meta.clone();
            d.json = v.clone();
            Value::Doc(d)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::router;

    #[test]
    fn counted_loop_breaks_on_signal() -> Result<()> {
        let script = "[funct:step]{[math@var+1]>[store@var]>[break]}\
                      [int@0]>[store@var]>[loop@5]{[call@step]}";
        let node = router::parse(script).map_err(anyhow::Error::new)?;
        let mut rt = Runtime::new()?;
        rt.eval(&node)?;
        assert_eq!(rt.get_num("var"), Some(1.0));
        Ok(())
    }

    #[test]
    fn loop_until_stops() -> Result<()> {
        let script = "[int@0]>[store@count]\
                      [loop:until@(count>=3)]{[math@count+1]>[store@count]}";
        let node = router::parse(script).map_err(anyhow::Error::new)?;
        let mut rt = Runtime::new()?;
        rt.eval(&node)?;
        assert_eq!(rt.get_num("count"), Some(3.0));
        Ok(())
    }

    #[test]
    fn loop_forever_breaks_with_signal() -> Result<()> {
        let script = "[int@0]>[store@ticks]\
                      [loop:forever]{[math@ticks+1]>[store@ticks]>[break]}";
        let node = router::parse(script).map_err(anyhow::Error::new)?;
        let mut rt = Runtime::new()?;
        rt.eval(&node)?;
        assert_eq!(rt.get_num("ticks"), Some(1.0));
        Ok(())
    }

    #[test]
    fn loop_each_iterates_items() -> Result<()> {
        let script = "[parse(json)@[1,2,3]]>[store@arr]\
                      [loop:each(item, idx@arr)]{[store@last_item]}";
        let node = router::parse(script).map_err(anyhow::Error::new)?;
        let mut rt = Runtime::new()?;
        rt.eval(&node)?;
        assert_eq!(rt.get_num("idx"), Some(2.0));
        assert_eq!(rt.get_num("last_item"), Some(3.0));
        Ok(())
    }
}
