use anyhow::{bail, Result};

use crate::kernel::ast::{CmpBase, Comparator};
use crate::kernel::{Arg, Packet, Runtime, Value};
use crate::kernel::boolops::cmp_eval;

// Canonical comparator packets:
// [eq@rhs], [ne@rhs], [lt@rhs], [le@rhs], [gt@rhs], [ge@rhs]
// Compares last value against rhs and returns a bool.
pub fn handle(rt: &mut Runtime, p: &Packet) -> Result<Value> {
    let rhs = match p.arg.as_ref() {
        Some(Arg::Number(n)) => Value::Num(*n),
        Some(Arg::Str(s)) => Value::Str(s.clone()),
        Some(Arg::Ident(id)) => rt.get_var(id).unwrap_or(Value::Unit),
        _ => bail!("comparator needs @<rhs> (number|string|ident)"),
    };

    let lhs = rt.last.clone();
    let op = p.op.as_str();
    let (base, include_eq, negate) = match op {
        "eq" => (CmpBase::Eq, false, false),
        "ne" => (CmpBase::Eq, false, true),
        "lt" => (CmpBase::Lt, false, false),
        "le" => (CmpBase::Lt, true, false),
        "gt" => (CmpBase::Gt, false, false),
        "ge" => (CmpBase::Gt, true, false),
        _ => bail!("unknown_comparator"),
    };
    let cmp = Comparator { base, include_eq, negate };
    let out = cmp_eval(&cmp, &lhs, &rhs)?;
    Ok(Value::Bool(out))
}

