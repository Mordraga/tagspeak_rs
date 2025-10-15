use crate::kernel::ast::Arg;
use crate::kernel::{Packet, Runtime, Value};
use crate::packets::conditionals::parse_cond;
use anyhow::{Result, bail};

pub fn handle(rt: &mut Runtime, p: &Packet) -> Result<Value> {
    let name = match p.arg.as_ref() {
        Some(Arg::Ident(id)) => id.as_str(),
        _ => bail!("store needs @<ident>"),
    };
    let val = rt.last.clone();

    match p.ns.as_deref() {
        Some("store") => {
            let mode = p.op.as_str();
            if mode == "rigid" {
                if rt.vars.contains_key(name) {
                    bail!("var_exists");
                }
                rt.set_var(name, val.clone())?;
                rt.rigid.insert(name.to_string());
            } else if mode == "fluid" {
                rt.set_var(name, val.clone())?;
            } else if mode.starts_with("context") {
                let mut src = mode.trim_start_matches("context").trim();
                if src.starts_with('(') && src.ends_with(')') {
                    src = &src[1..src.len() - 1];
                }
                let cond = parse_cond(src);
                rt.ctx_vars
                    .entry(name.to_string())
                    .or_default()
                    .push((cond, val.clone()));
            } else {
                bail!("unknown_store_mode");
            }
        }
        _ => {
            rt.set_var(name, val.clone())?;
        }
    }

    Ok(val)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{kernel::Runtime, kernel::values::Value, router};

    #[test]
    fn rigid_rejects_overwrite() {
        let script = "[msg@\"a\"]>[store:rigid@x]>[msg@\"b\"]>[store:rigid@x]";
        let node = router::parse(script).unwrap();
        let mut rt = Runtime::new().unwrap();
        assert!(rt.eval(&node).is_err());
    }

    #[test]
    fn fluid_allows_overwrite() -> Result<()> {
        let script = "[msg@\"a\"]>[store@x]>[msg@\"b\"]>[store:fluid@x]";
        let node = router::parse(script).map_err(anyhow::Error::new)?;
        let mut rt = Runtime::new()?;
        rt.eval(&node)?;
        assert_eq!(rt.get_var("x"), Some(Value::Str("b".into())));
        Ok(())
    }

    #[test]
    fn context_matches_conditions() -> Result<()> {
        let mut rt = Runtime::new()?;
        rt.last = Value::Str("apologetic".into());
        handle(
            &mut rt,
            &Packet {
                ns: Some("store".into()),
                op: "context(x==1)".into(),
                arg: Some(Arg::Ident("tone".into())),
                body: None,
            },
        )?;
        rt.last = Value::Str("neutral".into());
        handle(
            &mut rt,
            &Packet {
                ns: Some("store".into()),
                op: "context(1==1)".into(),
                arg: Some(Arg::Ident("tone".into())),
                body: None,
            },
        )?;
        rt.set_var("x", Value::Num(1.0))?;
        assert_eq!(rt.get_var("tone"), Some(Value::Str("apologetic".into())));
        rt.set_var("x", Value::Num(0.0))?;
        assert_eq!(rt.get_var("tone"), Some(Value::Str("neutral".into())));
        Ok(())
    }
}
