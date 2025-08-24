use anyhow::Result;
use crate::kernel::{Runtime, Value};
use crate::kernel::ast::{BExpr, Node};
use crate::kernel::boolops::cmp_eval;

fn eval_bexpr(rt: &mut Runtime, expr: &BExpr) -> Result<bool> {
    use BExpr::*;
    Ok(match expr {
        Cmp { lhs, cmp, rhs } => {
            let a = rt.eval(lhs)?;
            let b = rt.eval(rhs)?;
            cmp_eval(cmp, &a, &b)?
        }
        And(a, b) => eval_bexpr(rt, a)? && eval_bexpr(rt, b)?,
        Or(a, b) => eval_bexpr(rt, a)? || eval_bexpr(rt, b)?,
        Not(x) => !eval_bexpr(rt, x)?,
        Lit(n) => rt.eval(n)?.as_bool().unwrap_or(false),
    })
}

fn run_block(rt: &mut Runtime, nodes: &[Node]) -> Result<Value> {
    let mut last = Value::Unit;
    for n in nodes {
        last = rt.eval(n)?;
    }
    Ok(last)
}

pub fn exec(rt: &mut Runtime, cond: &BExpr, then_b: &[Node], else_b: &[Node]) -> Result<Value> {
    if eval_bexpr(rt, cond)? {
        run_block(rt, then_b)
    } else if else_b.is_empty() {
        Ok(Value::Unit)
    } else {
        run_block(rt, else_b)
    }
}

