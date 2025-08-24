use anyhow::Result;
use crate::kernel::{Runtime, Value, BExpr};

// [myth] goal: branch like a choose-your-own-adventure (no paper cuts)
pub fn handle(_rt: &mut Runtime, _p: &crate::kernel::Packet) -> Result<Value> {
    Ok(Value::Unit)
}

pub fn parse_cond(src: &str) -> BExpr {
    BExpr::Lit(src.to_string())
}

pub fn eval_cond(rt: &mut Runtime, cond: &BExpr) -> Result<bool> {
    match cond {
        BExpr::Lit(src) => {
            let node = crate::router::parse(src)?;
            let mut tmp = Runtime::new();
            tmp.vars = rt.vars.clone();
            tmp.tags = rt.tags.clone();
            // [myth] goal: numbers <= 0 and empty strings are false
            Ok(tmp.eval(&node)?.as_bool().unwrap_or(false))
        }
        BExpr::And(a, b) => Ok(eval_cond(rt, a)? && eval_cond(rt, b)?),
        BExpr::Or(a, b) => Ok(eval_cond(rt, a)? || eval_cond(rt, b)?),
        BExpr::Not(e) => Ok(!eval_cond(rt, e)?),
        BExpr::Cmp { lhs, cmp, rhs } => {
            let mut tmp = Runtime::new();
            tmp.vars = rt.vars.clone();
            let lv = tmp.eval(lhs)?;
            let rv = tmp.eval(rhs)?;
            crate::kernel::boolops::cmp_eval(cmp, &lv, &rv)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::router;

    #[test]
    fn or_else_chain() -> Result<()> {
        // [myth] goal: ensure or/else pick first truthy branch
        let script = "[if@([math@0])]>[then]{[math@1]>[store@x]}>
                      [or@([math@1])]>[then]{[math@2]>[store@x]}>
                      [else]>[then]{[math@3]>[store@x]}";
        let mut rt = Runtime::new();
        let node = router::parse(script)?;
        rt.eval(&node)?;
        assert_eq!(rt.get_num("x"), Some(2.0));
        Ok(())
    }
}
