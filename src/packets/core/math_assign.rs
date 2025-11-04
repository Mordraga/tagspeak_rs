use crate::kernel::ast::Arg;
use crate::kernel::{Packet, Runtime, Value};
use anyhow::{Result, anyhow, bail};
use meval::{Context, Expr};
use std::str::FromStr;

/// Increment numeric variable by 1.
pub fn handle_inc(rt: &mut Runtime, p: &Packet) -> Result<Value> {
    let name = extract_ident(p.arg.as_ref(), "inc")?;
    let current = require_num(rt, &name)?;
    let updated = current + 1.0;
    rt.set_num(&name, updated)?;
    Ok(Value::Num(updated))
}

/// Decrement numeric variable by 1.
pub fn handle_dec(rt: &mut Runtime, p: &Packet) -> Result<Value> {
    let name = extract_ident(p.arg.as_ref(), "dec")?;
    let current = require_num(rt, &name)?;
    let updated = current - 1.0;
    rt.set_num(&name, updated)?;
    Ok(Value::Num(updated))
}

/// Add/subtract inline sugar: `[mod@counter+=5]`, `[mod@counter-=2]`
pub fn handle_add_assign(rt: &mut Runtime, p: &Packet) -> Result<Value> {
    let arg = p
        .arg
        .as_ref()
        .ok_or_else(|| anyhow!("mod assignment needs @var+=expr or @var-=expr"))?;
    let assignment = parse_assignment(arg, &["+=", "-="])?;
    let current = require_num(rt, &assignment.name)?;
    let delta = eval_expr(rt, &assignment.expr)?;
    let updated = match assignment.op.as_str() {
        "+=" => current + delta,
        "-=" => current - delta,
        other => bail!("unsupported mod assignment operator '{other}'"),
    };
    rt.set_num(&assignment.name, updated)?;
    Ok(Value::Num(updated))
}

/// Multiply/divide inline sugar: `[mul@counter*=3]`, `[mul@counter/=2]`
pub fn handle_mul_assign(rt: &mut Runtime, p: &Packet) -> Result<Value> {
    let arg = p
        .arg
        .as_ref()
        .ok_or_else(|| anyhow!("mul assignment needs @var*=expr or @var/=expr"))?;
    let assignment = parse_assignment(arg, &["*=", "/="])?;
    let current = require_num(rt, &assignment.name)?;
    let factor = eval_expr(rt, &assignment.expr)?;
    let updated = match assignment.op.as_str() {
        "*=" => current * factor,
        "/=" => {
            if factor == 0.0 {
                bail!("division by zero");
            }
            current / factor
        }
        other => bail!("unsupported mul assignment operator '{other}'"),
    };
    rt.set_num(&assignment.name, updated)?;
    Ok(Value::Num(updated))
}

fn extract_ident(arg: Option<&Arg>, packet: &str) -> Result<String> {
    match arg {
        Some(Arg::Ident(id)) => Ok(id.clone()),
        Some(Arg::Str(s)) if is_ident_like(s) => Ok(s.clone()),
        _ => bail!("{packet} needs @<ident>"),
    }
}

fn require_num(rt: &Runtime, name: &str) -> Result<f64> {
    match rt.get_var(name) {
        Some(Value::Num(n)) => Ok(n),
        Some(Value::Str(s)) => {
            let parsed = s
                .parse::<f64>()
                .map_err(|_| anyhow!("{name} is not numeric"))?;
            Ok(parsed)
        }
        Some(_) => bail!("{name} is not numeric"),
        None => bail!("var_missing: {name}"),
    }
}

struct Assignment {
    name: String,
    op: String,
    expr: String,
}

fn parse_assignment(arg: &Arg, ops: &[&str]) -> Result<Assignment> {
    let raw = match arg {
        Arg::Str(s) => s.trim(),
        Arg::Ident(id) => id.as_str(),
        Arg::Number(_) => bail!("assignment target must be identifier"),
        Arg::CondSrc(_) => bail!("unsupported assignment form"),
    };
    for op in ops {
        if let Some(idx) = raw.find(op) {
            let (lhs, rhs) = raw.split_at(idx);
            let rhs = &rhs[op.len()..];
            let name = lhs.trim();
            if !is_ident_like(name) {
                bail!("invalid assignment target '{name}'");
            }
            let expr = rhs.trim();
            if expr.is_empty() {
                bail!("assignment expression missing for '{name}{op}'");
            }
            return Ok(Assignment {
                name: name.to_string(),
                op: op.to_string(),
                expr: expr.to_string(),
            });
        }
    }
    bail!("assignment needs one of {}", ops.join(", "));
}

fn eval_expr(rt: &Runtime, expr: &str) -> Result<f64> {
    let mut ctx = Context::new();
    for (key, val) in &rt.vars {
        if let Value::Num(n) = val {
            ctx.var(key.clone(), *n);
        }
    }
    let expr = Expr::from_str(expr)?;
    let result = expr.eval_with_context(ctx)?;
    Ok(result)
}

fn is_ident_like(s: &str) -> bool {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{kernel::Runtime, kernel::values::Value, router};

    fn eval_script(src: &str) -> Result<Runtime> {
        let node = router::parse(src).map_err(anyhow::Error::new)?;
        let mut rt = Runtime::new()?;
        rt.eval(&node)?;
        Ok(rt)
    }

    #[test]
    fn inc_increments_var() -> Result<()> {
        let rt = eval_script("[int@4]>[store@counter]>[inc@counter]")?;
        match rt.get_var("counter") {
            Some(Value::Num(n)) => assert_eq!(n, 5.0),
            other => panic!("unexpected value: {other:?}"),
        }
        Ok(())
    }

    #[test]
    fn mod_adds_inline() -> Result<()> {
        let rt = eval_script("[int@2]>[store@counter]>[mod@counter+=5]")?;
        assert_eq!(rt.get_num("counter"), Some(7.0));
        Ok(())
    }

    #[test]
    fn mod_handles_subtract() -> Result<()> {
        let rt = eval_script("[int@10]>[store@counter]>[mod@counter-=3]")?;
        assert_eq!(rt.get_num("counter"), Some(7.0));
        Ok(())
    }

    #[test]
    fn mul_handles_product() -> Result<()> {
        let rt = eval_script("[int@4]>[store@counter]>[mul@counter*=3]")?;
        assert_eq!(rt.get_num("counter"), Some(12.0));
        Ok(())
    }

    #[test]
    fn mul_handles_division() -> Result<()> {
        let rt = eval_script("[int@12]>[store@counter]>[mul@counter/=4]")?;
        assert_eq!(rt.get_num("counter"), Some(3.0));
        Ok(())
    }
}
