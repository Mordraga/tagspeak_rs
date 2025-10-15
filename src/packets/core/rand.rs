use anyhow::{Result, anyhow, bail};

use crate::kernel::{Packet, Runtime, Value};

// [rand] -> uniform float in [0,1)
// [rand(min,max)] -> random number between evaluated bounds (ints yield ints)
pub fn handle(rt: &mut Runtime, p: &Packet) -> Result<Value> {
    if let Some(ns) = &p.ns {
        bail!("rand does not support namespace '{}'.", ns);
    }

    if p.op == "rand" {
        if p.arg.is_some() {
            bail!("rand expects either no args or parentheses (min,max)");
        }
        return Ok(Value::Num(fastrand::f64()));
    }

    if p.op.starts_with("rand(") {
        let inner =
            crate::router::extract_paren(&p.op).ok_or_else(|| anyhow!("rand needs (min,max)"))?;
        let (min, max) = parse_bounds(rt, inner)?;
        return Ok(Value::Num(sample_between(min, max)?));
    }

    bail!("unknown rand form")
}

fn parse_bounds(rt: &mut Runtime, inner: &str) -> Result<(f64, f64)> {
    let parts: Vec<&str> = inner
        .split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();
    if parts.len() != 2 {
        bail!("rand expects exactly two bounds");
    }
    let min = eval_expr(rt, parts[0])?;
    let max = eval_expr(rt, parts[1])?;
    if min > max {
        bail!("rand bounds inverted");
    }
    Ok((min, max))
}

fn eval_expr(rt: &mut Runtime, expr: &str) -> Result<f64> {
    let trimmed = expr.trim();
    if trimmed.is_empty() {
        bail!("empty bound");
    }

    if let Ok(n) = trimmed.parse::<f64>() {
        return Ok(n);
    }

    if let Some(val) = rt.get_var(trimmed) {
        return value_to_num(&val).ok_or_else(|| anyhow!("non-numeric bound"));
    }

    if trimmed.starts_with('[') {
        let node = crate::router::parse(trimmed).map_err(anyhow::Error::new)?;
        let prev_last = rt.last.clone();
        let result = rt.eval(&node);
        let out = match result {
            Ok(v) => v,
            Err(err) => {
                rt.last = prev_last;
                return Err(err);
            }
        };
        rt.last = prev_last;
        return value_to_num(&out).ok_or_else(|| anyhow!("non-numeric bound"));
    }

    bail!("unsupported bound expression")
}

fn value_to_num(v: &Value) -> Option<f64> {
    match v {
        Value::Num(n) => Some(*n),
        Value::Str(s) => s.parse().ok(),
        Value::Bool(b) => Some(if *b { 1.0 } else { 0.0 }),
        Value::Unit => Some(0.0),
        Value::Doc(_) => None,
    }
}

fn sample_between(min: f64, max: f64) -> Result<f64> {
    if (min - max).abs() < f64::EPSILON {
        return Ok(min);
    }

    if min.fract() == 0.0 && max.fract() == 0.0 {
        let a = min as i64;
        let b = max as i64;
        if a > b {
            bail!("rand bounds inverted");
        }
        return Ok(fastrand::i64(a..=b) as f64);
    }

    let span = max - min;
    if span <= 0.0 {
        bail!("rand bounds inverted");
    }
    Ok(min + fastrand::f64() * span)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_rand_range() -> Result<()> {
        let mut rt = Runtime::new()?;
        let node = crate::router::parse("[rand]").map_err(anyhow::Error::new)?;
        let out = rt.eval(&node)?;
        match out {
            Value::Num(n) => {
                assert!(n >= 0.0 && n < 1.0);
            }
            _ => bail!("rand did not return number"),
        }
        Ok(())
    }

    #[test]
    fn bounded_rand_inclusive_ints() -> Result<()> {
        let mut rt = Runtime::new()?;
        let node = crate::router::parse("[rand(1,3)]").map_err(anyhow::Error::new)?;
        let out = rt.eval(&node)?;
        match out {
            Value::Num(n) => {
                assert!(n >= 1.0 && n <= 3.0);
            }
            _ => bail!("rand did not return number"),
        }
        Ok(())
    }

    #[test]
    fn rand_with_packet_bounds() -> Result<()> {
        let mut rt = Runtime::new()?;
        let script = "[msg@\"alpha\"]>[rand([len],10)]";
        let node = crate::router::parse(script).map_err(anyhow::Error::new)?;
        let out = rt.eval(&node)?;
        match out {
            Value::Num(n) => {
                assert!(n >= 5.0 && n <= 10.0);
            }
            _ => bail!("rand did not return number"),
        }
        Ok(())
    }
}
