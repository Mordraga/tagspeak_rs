use crate::kernel::ast::CmpBase;
use crate::kernel::{Arg, BExpr, Comparator, Node, Packet, Runtime, Value};
use anyhow::Result;

// [myth] goal: branch like a choose-your-own-adventure (no paper cuts)
pub fn handle(_rt: &mut Runtime, _p: &crate::kernel::Packet) -> Result<Value> {
    Ok(Value::Unit)
}

pub fn parse_cond(src: &str) -> BExpr {
    fn token_to_node(tok: &str) -> Node {
        let tok = tok.trim();
        let arg = if let Ok(n) = tok.parse::<f64>() {
            Arg::Number(n)
        } else if tok
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.')
        {
            Arg::Ident(tok.to_string())
        } else {
            Arg::Str(tok.to_string())
        };
        Node::Packet(Packet {
            ns: None,
            op: "math".to_string(),
            arg: Some(arg),
            body: None,
        })
    }

    let specs = [
        ("[ge]", CmpBase::Gt, true, false),
        ("[le]", CmpBase::Lt, true, false),
        ("[ne]", CmpBase::Eq, false, true),
        ("[eq]", CmpBase::Eq, false, false),
        ("[gt]", CmpBase::Gt, false, false),
        ("[lt]", CmpBase::Lt, false, false),
        (">=", CmpBase::Gt, true, false),
        ("<=", CmpBase::Lt, true, false),
        ("!=", CmpBase::Eq, false, true),
        ("==", CmpBase::Eq, false, false),
        (">", CmpBase::Gt, false, false),
        ("<", CmpBase::Lt, false, false),
    ];

    for (pat, base, include_eq, negate) in specs.iter() {
        if let Some(idx) = src.find(pat) {
            let lhs = token_to_node(&src[..idx]);
            let rhs = token_to_node(&src[idx + pat.len()..]);
            return BExpr::Cmp {
                lhs: Box::new(lhs),
                cmp: Comparator {
                    base: base.clone(),
                    include_eq: *include_eq,
                    negate: *negate,
                },
                rhs: Box::new(rhs),
            };
        }
    }

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

    #[test]
    fn packet_eq_in_if() -> Result<()> {
        // [myth] goal: compare vars via [eq]
        let script = "[math@5]>[store@a]>[math@5]>[store@b]>".to_string()
            + "[if@([eq@a b])]>[then]{[math@1]>[store@res]}>"
            + "[else]>[then]{[math@0]>[store@res]}";
        let mut rt = Runtime::new();
        let node = router::parse(&script)?;
        rt.eval(&node)?;
        assert_eq!(rt.get_num("res"), Some(1.0));
        Ok(())
    }

    #[test]
    fn and_chain_with_stores() -> Result<()> {
        // [myth] goal: chain comparisons then and them
        let script = "[math@7]>[store@x]>[math@8]>[store@y]>".to_string()
            + "[gt@x 5]>[store@c1]>[lt@y 10]>[store@c2]>"
            + "[if@([and@c1 c2])]>[then]{[math@1]>[store@res]}>"
            + "[else]>[then]{[math@0]>[store@res]}";
        let mut rt = Runtime::new();
        let node = router::parse(&script)?;
        rt.eval(&node)?;
        assert_eq!(rt.get_num("res"), Some(1.0));
        Ok(())
    }

    #[test]
    fn stored_cmp_reuse() -> Result<()> {
        // [myth] goal: reuse stored boolean from comparator
        let script = "[math@5]>[store@x]>[math@6]>[store@y]>".to_string()
            + "[eq@x y]>[store@cmp]>"
            + "[if@([eq@cmp false])]>[then]{[math@1]>[store@res]}>"
            + "[else]>[then]{[math@0]>[store@res]}";
        let mut rt = Runtime::new();
        let node = router::parse(&script)?;
        rt.eval(&node)?;
        assert_eq!(rt.get_num("res"), Some(1.0));
        Ok(())
    }

    #[test]
    fn infix_cmp_falls_to_else() -> Result<()> {
        // [myth] goal: support infix comparators like 1>1 and 1[lt]1
        let script = r#"
            [if@(1>1)]>[then]{[math@10]>[store@x]}>
            [or@(1[lt]1)]>[then]{[math@20]>[store@x]}>
            [else]>[then]{[math@30]>[store@x]}
        "#;
        let mut rt = Runtime::new();
        let node = router::parse(script)?;
        rt.eval(&node)?;
        assert_eq!(rt.get_num("x"), Some(30.0));
        Ok(())
    }
}
