use crate::kernel::ast::CmpBase;
use crate::kernel::{Arg, BExpr, Comparator, Node, Packet, Runtime, Value};
use anyhow::Result;

// [myth] goal: branch like a choose-your-own-adventure (no paper cuts)
pub fn handle(_rt: &mut Runtime, _p: &crate::kernel::Packet) -> Result<Value> {
    Ok(Value::Unit)
}

pub fn parse_cond(src: &str) -> BExpr {
    let s = src.trim();
    if s.starts_with('(') && s.ends_with(')') {
        return parse_cond(&s[1..s.len() - 1]);
    }

    // try logical OR first (lowest precedence)
    for pat in ["||", "[or]"] {
        if let Some(idx) = s.find(pat) {
            let lhs = parse_cond(&s[..idx]);
            let rhs = parse_cond(&s[idx + pat.len()..]);
            return BExpr::Or(Box::new(lhs), Box::new(rhs));
        }
    }

    // then logical AND
    for pat in ["&&", "[and]"] {
        if let Some(idx) = s.find(pat) {
            let lhs = parse_cond(&s[..idx]);
            let rhs = parse_cond(&s[idx + pat.len()..]);
            return BExpr::And(Box::new(lhs), Box::new(rhs));
        }
    }

    // unary NOT
    for pat in ["!", "[not]"] {
        if s.starts_with(pat) {
            let inner = parse_cond(&s[pat.len()..]);
            return BExpr::Not(Box::new(inner));
        }
    }

    // comparison operators
    let ops: [(&str, Comparator); 18] = [
        (
            "[!=]",
            Comparator {
                base: CmpBase::Eq,
                include_eq: false,
                negate: true,
            },
        ),
        (
            "[ne]",
            Comparator {
                base: CmpBase::Eq,
                include_eq: false,
                negate: true,
            },
        ),
        (
            "!=",
            Comparator {
                base: CmpBase::Eq,
                include_eq: false,
                negate: true,
            },
        ),
        (
            "[>=]",
            Comparator {
                base: CmpBase::Gt,
                include_eq: true,
                negate: false,
            },
        ),
        (
            "[ge]",
            Comparator {
                base: CmpBase::Gt,
                include_eq: true,
                negate: false,
            },
        ),
        (
            ">=",
            Comparator {
                base: CmpBase::Gt,
                include_eq: true,
                negate: false,
            },
        ),
        (
            "[<=]",
            Comparator {
                base: CmpBase::Lt,
                include_eq: true,
                negate: false,
            },
        ),
        (
            "[le]",
            Comparator {
                base: CmpBase::Lt,
                include_eq: true,
                negate: false,
            },
        ),
        (
            "<=",
            Comparator {
                base: CmpBase::Lt,
                include_eq: true,
                negate: false,
            },
        ),
        (
            "[>]",
            Comparator {
                base: CmpBase::Gt,
                include_eq: false,
                negate: false,
            },
        ),
        (
            "[gt]",
            Comparator {
                base: CmpBase::Gt,
                include_eq: false,
                negate: false,
            },
        ),
        (
            ">",
            Comparator {
                base: CmpBase::Gt,
                include_eq: false,
                negate: false,
            },
        ),
        (
            "[<]",
            Comparator {
                base: CmpBase::Lt,
                include_eq: false,
                negate: false,
            },
        ),
        (
            "[lt]",
            Comparator {
                base: CmpBase::Lt,
                include_eq: false,
                negate: false,
            },
        ),
        (
            "<",
            Comparator {
                base: CmpBase::Lt,
                include_eq: false,
                negate: false,
            },
        ),
        (
            "[=]",
            Comparator {
                base: CmpBase::Eq,
                include_eq: false,
                negate: false,
            },
        ),
        (
            "[eq]",
            Comparator {
                base: CmpBase::Eq,
                include_eq: false,
                negate: false,
            },
        ),
        (
            "==",
            Comparator {
                base: CmpBase::Eq,
                include_eq: false,
                negate: false,
            },
        ),
    ];

    for (pat, cmp) in ops.iter() {
        if let Some(idx) = s.find(pat) {
            let lhs_src = s[..idx].trim();
            let rhs_src = s[idx + pat.len()..].trim();
            if let (Some(lhs), Some(rhs)) = (parse_atom(lhs_src), parse_atom(rhs_src)) {
                return BExpr::Cmp {
                    lhs: Box::new(lhs),
                    cmp: cmp.clone(),
                    rhs: Box::new(rhs),
                };
            } else {
                return BExpr::Lit(src.to_string());
            }
        }
    }

    BExpr::Lit(src.to_string())
}

fn parse_atom(tok: &str) -> Option<Node> {
    let t = tok.trim();
    if t.starts_with('[') {
        crate::router::parse(t).ok()
    } else if let Ok(n) = t.parse::<f64>() {
        Some(Node::Packet(Packet {
            ns: None,
            op: "math".into(),
            arg: Some(Arg::Number(n)),
            body: None,
        }))
    } else if t.starts_with('"') && t.ends_with('"') {
        // string literal
        let inner = t.trim_matches('"').to_string();
        Some(Node::Packet(Packet {
            ns: None,
            op: "msg".into(),
            arg: Some(Arg::Str(inner)),
            body: None,
        }))
    } else if is_ident_like(t) {
        // resolve identifier as a runtime variable (string/number/bool)
        Some(Node::Packet(Packet {
            ns: None,
            op: "var".into(),
            arg: Some(Arg::Ident(t.to_string())),
            body: None,
        }))
    } else {
        None
    }
}

fn is_ident_like(s: &str) -> bool {
    let mut it = s.chars();
    match it.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' => {}
        _ => return false,
    }
    it.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

pub fn eval_cond(rt: &mut Runtime, cond: &BExpr) -> Result<bool> {
    match cond {
        BExpr::Lit(src) => {
            let s = src.trim();
            if is_ident_like(s) {
                // Treat bare identifiers as variable truthiness
                Ok(rt.get_var(s).and_then(|v| v.as_bool()).unwrap_or(false))
            } else if let Ok(n) = s.parse::<f64>() {
                // Numeric literals: non-zero = true
                Ok(n != 0.0 && !n.is_nan())
            } else {
                let node = crate::router::parse(s).map_err(anyhow::Error::new)?;
                let mut tmp = Runtime::new()?;
                tmp.vars = rt.vars.clone();
                tmp.tags = rt.tags.clone();
                // [myth] goal: numbers <= 0 and empty strings are false
                Ok(tmp.eval(&node)?.as_bool().unwrap_or(false))
            }
        }
        BExpr::And(a, b) => Ok(eval_cond(rt, a)? && eval_cond(rt, b)?),
        BExpr::Or(a, b) => Ok(eval_cond(rt, a)? || eval_cond(rt, b)?),
        BExpr::Not(e) => Ok(!eval_cond(rt, e)?),
        BExpr::Cmp { lhs, cmp, rhs } => {
            let mut tmp = Runtime::new()?;
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
        let script = "[if@([math@0])]>[then]{[math@1]>[store@x]}>\
                      [or@([math@1])]>[then]{[math@2]>[store@x]}>\
                      [else]>[then]{[math@3]>[store@x]}";
        let mut rt = Runtime::new()?;
        let node = router::parse(script).map_err(anyhow::Error::new)?;
        rt.eval(&node)?;
        assert_eq!(rt.get_num("x"), Some(2.0));
        Ok(())
    }

    #[test]
    fn infix_comparators() -> Result<()> {
        let script = "[if@(1>1)]>[then]{[math@10]>[store@x]}>\
                      [or@(1[lt]1)]>[then]{[math@20]>[store@x]}>\
                      [else]>[then]{[math@30]>[store@x]}";
        let mut rt = Runtime::new()?;
        let node = router::parse(script).map_err(anyhow::Error::new)?;
        rt.eval(&node)?;
        assert_eq!(rt.get_num("x"), Some(30.0));
        Ok(())
    }

    #[test]
    fn inline_blocks_without_then() -> Result<()> {
        let script = "[int@0]>[store@x]\n\
                      [if@(x==0)]{[math@5]>[store@x]}[else]{[math@9]>[store@x]}";
        let mut rt = Runtime::new()?;
        let node = router::parse(script).map_err(anyhow::Error::new)?;
        rt.eval(&node)?;
        assert_eq!(rt.get_num("x"), Some(5.0));
        Ok(())
    }

    #[test]
    fn or_branch_inline_block() -> Result<()> {
        let script = "[if@(1==0)]{[math@1]>[store@y]}\
                      [or@(1==1)]{[math@2]>[store@y]}";
        let mut rt = Runtime::new()?;
        let node = router::parse(script).map_err(anyhow::Error::new)?;
        rt.eval(&node)?;
        assert_eq!(rt.get_num("y"), Some(2.0));
        Ok(())
    }
}
