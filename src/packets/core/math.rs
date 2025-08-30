use std::str::FromStr;
use anyhow::Result;
use meval::Expr;

use crate::kernel::{Runtime, Value, Packet};
use crate::kernel::ast::Arg;

pub fn handle(rt: &mut Runtime, p: &Packet) -> Result<Value> {
    // Accept @<number>  -> return number
    // Accept @<ident>   -> if numeric var exists, return it; else treat ident text as expr
    // Accept @"a+b"     -> evaluate as expression with current numeric vars
    let expr_text = match p.arg.as_ref() {
        Some(Arg::Number(n)) => return Ok(Value::Num(*n)),
        Some(Arg::Ident(id)) => {
            if let Some(Value::Num(n)) = rt.get_var(id) { return Ok(Value::Num(n)); }
            id.clone() // treat as expression string: allows [math@counter+1]
        }
        Some(Arg::Str(s)) => s.clone(),
        _ => anyhow::bail!("math needs @<number|ident|expr>"),
    };

    // bind numeric vars into math context
    let mut ctx = meval::Context::new();
    for (k, v) in &rt.vars {
        if let Value::Num(n) = v { ctx.var(k.clone(), *n); }
    }

    let expr = Expr::from_str(&expr_text)?;
    let val  = expr.eval_with_context(ctx)?;
    Ok(Value::Num(val))
}
