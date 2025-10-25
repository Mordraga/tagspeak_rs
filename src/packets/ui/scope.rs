use anyhow::{Result, bail};
use crate::kernel::{Packet, Runtime, Value};
use crate::kernel::ast::Arg;

// [scope@"Name"]{ ... }
// Inside the body, plain [store@var] writes become context-bound to the named scope.
// The adapter will set __ui_scope at render time so reads pick the right value.
pub fn handle(rt: &mut Runtime, p: &Packet) -> Result<Value> {
    let name = match p.arg.as_ref() {
        Some(Arg::Str(s)) => s.clone(),
        Some(Arg::Ident(id)) => id.clone(),
        _ => bail!("scope needs @\"name\" or @ident"),
    };
    let body = p.body.as_ref().ok_or_else(|| anyhow::anyhow!("scope requires a body"))?;

    // Save prev capture flag
    let prev = rt.get_var("__scope_capture");
    rt.set_var("__scope_capture", Value::Str(name))?;
    // Evaluate the body in this capture context
    let out = rt.eval(&crate::kernel::Node::Block(body.clone()))?;
    // Restore previous
    match prev {
        Some(v) => { rt.set_var("__scope_capture", v)?; }
        None => { rt.set_var("__scope_capture", Value::Unit)?; }
    }
    Ok(out)
}

