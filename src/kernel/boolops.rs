use crate::kernel::ast::{Comparator, CmpBase};
use crate::kernel::values::Value;

pub fn reduce_op_chain_is_valid() -> bool { true } // placeholder if needed

pub fn cmp_eval(cmp: &Comparator, a: &Value, b: &Value) -> anyhow::Result<bool> {
    use CmpBase::*;
    let mut out = match cmp.base {
        Eq => eq_values(a, b),
        Lt => order(a, b, |x, y| x < y)?,
        Gt => order(a, b, |x, y| x > y)?,
    };
    if matches!(cmp.base, Lt | Gt) && cmp.include_eq {
        out = out || eq_values(a, b);
    }
    if cmp.negate { out = !out; }
    Ok(out)
}

fn eq_values(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Num(x),  Value::Num(y))  => x == y,
        (Value::Str(x),  Value::Str(y))  => x == y,
        (Value::Bool(x), Value::Bool(y)) => x == y,
        _ => false,
    }
}

fn order<F: Fn(f64, f64) -> bool>(a: &Value, b: &Value, f: F) -> anyhow::Result<bool> {
    let xa = to_num(a)?;
    let xb = to_num(b)?;
    Ok(f(xa, xb))
}

fn to_num(v: &Value) -> anyhow::Result<f64> {
    match v {
        Value::Num(n) => Ok(*n),
        Value::Str(s) => s.parse::<f64>().map_err(|_| anyhow::anyhow!("non-numeric string")),
        _ => Err(anyhow::anyhow!("non-numeric value")),
    }
}
