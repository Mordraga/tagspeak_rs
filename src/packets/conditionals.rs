use crate::interpreter::Scanner;
use crate::kernel::ast::CmpBase;
use crate::kernel::{Arg, BExpr, Comparator, Runtime, Value};
use anyhow::{Result, bail};

// [myth] goal: branch like a choose-your-own-adventure (no paper cuts)
pub fn handle(rt: &mut Runtime, p: &crate::kernel::Packet) -> Result<Value> {
    let src = match &p.arg {
        Some(Arg::CondSrc(s)) => s,
        _ => bail!("if needs @(...)"),
    };

    // incoming `@(...)` looks like `[lhs][cmp][rhs]>{actions}>[result]`
    let (cond_part, rest) = split_cond(src);
    let cond = parse_cond(&cond_part);

    if eval_cond(rt, &cond)? {
        // run the action chain when the comparison is true
        if rest.trim().is_empty() {
            Ok(Value::Unit)
        } else {
            let node = crate::router::parse(&rest)?;
            rt.eval(&node)
        }
    } else {
        Ok(Value::Unit)
    }
}

fn split_cond(src: &str) -> (String, String) {
    // walk through the string and split on the first top‑level `>`
    let mut brackets = 0usize;
    let mut braces = 0usize;
    let mut parens = 0usize;
    for (i, ch) in src.char_indices() {
        match ch {
            '[' => brackets += 1,
            ']' => brackets -= 1,
            '{' => braces += 1,
            '}' => braces -= 1,
            '(' => parens += 1,
            ')' => parens -= 1,
            '>' if brackets == 0 && braces == 0 && parens == 0 => {
                let cond = src[..i].trim().to_string();
                let rest = src[i + 1..].trim().to_string();
                return (cond, rest);
            }
            _ => {}
        }
    }
    (src.trim().to_string(), String::new())
}

pub fn parse_cond(src: &str) -> BExpr {
    let mut sc = Scanner::new(src);
    sc.skip_comments_and_ws();
    if sc.peek() == Some('[') {
        if let Ok(lhs) = sc.read_until_balanced('[', ']') {
            sc.skip_comments_and_ws();
            if sc.peek() == Some('[') {
                if let Ok(op) = sc.read_until_balanced('[', ']') {
                    sc.skip_comments_and_ws();
                    if sc.peek() == Some('[') {
                        if let Ok(rhs) = sc.read_until_balanced('[', ']') {
                            if let Some(cmp) = parse_cmp(op.trim()) {
                                if let (Ok(ln), Ok(rn)) = (
                                    crate::router::parse(&format!("[{}]", lhs.trim())),
                                    crate::router::parse(&format!("[{}]", rhs.trim())),
                                ) {
                                    return BExpr::Cmp {
                                        lhs: Box::new(ln),
                                        cmp,
                                        rhs: Box::new(rn),
                                    };
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    BExpr::Lit(src.to_string())
}

fn parse_cmp(op: &str) -> Option<Comparator> {
    use CmpBase::*;
    let (base, include_eq, negate) = match op {
        "eq" | "=" => (Eq, false, false),
        "ne" | "!=" => (Eq, false, true),
        "gt" | ">" => (Gt, false, false),
        "ge" | ">=" => (Gt, true, false),
        "lt" | "<" => (Lt, false, false),
        "le" | "<=" => (Lt, true, false),
        _ => return None,
    };
    Some(Comparator {
        base,
        include_eq,
        negate,
    })
}

pub fn eval_cond(rt: &mut Runtime, cond: &BExpr) -> Result<bool> {
    match cond {
        BExpr::Lit(src) => {
            let node = crate::router::parse(src)?;
            let mut tmp = Runtime::new();
            tmp.vars = rt.vars.clone();
            tmp.tags = rt.tags.clone();
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
    fn if_chain_executes() -> Result<()> {
        let script = "[if@([math@1][gt][math@0]>{[math@2]>[store@x]}>[math@3])]";
        let mut rt = Runtime::new();
        let node = router::parse(script)?;
        rt.eval(&node)?;
        assert_eq!(rt.get_num("x"), Some(2.0));
        assert_eq!(rt.last, Value::Num(3.0));
        Ok(())
    }

    #[test]
    fn if_chain_skips() -> Result<()> {
        let script = "[if@([math@0][gt][math@1]>{[math@2]>[store@y]}>[math@3])]";
        let mut rt = Runtime::new();
        let node = router::parse(script)?;
        rt.eval(&node)?;
        assert!(rt.get_var("y").is_none());
        Ok(())
    }
}
