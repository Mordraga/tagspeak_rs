use anyhow::{Result, bail};

use crate::kernel::ast::{Arg, BExpr};
use crate::kernel::{Packet, Runtime, Value};
use crate::packets::conditionals::{eval_cond, parse_cond};

pub fn handle(rt: &mut Runtime, p: &Packet) -> Result<Value> {
    let raw = match p.arg.as_ref() {
        Some(Arg::Ident(id)) => id.as_str().to_string(),
        Some(Arg::Str(s)) => s.clone(),
        Some(Arg::CondSrc(s)) => s.clone(),
        Some(Arg::Number(n)) => return Ok(Value::Bool(*n != 0.0)),
        None => bail!("bool needs @<value>"),
    };

    let cond = parse_cond(&raw);
    if !matches!(cond, BExpr::Lit(ref s) if s == &raw) {
        return Ok(Value::Bool(eval_cond(rt, &cond)?));
    }

    let val = match raw.as_str() {
        "true" => true,
        "false" => false,
        other => match rt.get_var(other) {
            Some(Value::Bool(b)) => b,
            Some(Value::Num(n)) => n != 0.0,
            Some(Value::Str(s)) => !s.is_empty(),
            _ => false,
        },
    };
    Ok(Value::Bool(val))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{kernel::Runtime, router};

    #[test]
    fn parses_expressions() -> Result<()> {
        let script = "[bool@(1==1 && 2==2)]>[store@x]";
        let node = router::parse(script).map_err(anyhow::Error::new)?;
        let mut rt = Runtime::new()?;
        rt.eval(&node)?;
        assert_eq!(rt.get_var("x"), Some(Value::Bool(true)));
        Ok(())
    }
}
