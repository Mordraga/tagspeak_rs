use anyhow::{Result, bail};
use std::thread;

use crate::kernel::ast::{Arg, Node};
use crate::kernel::runtime::{FlowSignal, Runtime};
use crate::kernel::{Packet, Value};
use crate::packets::time::timeout::duration_from_unit;

pub fn handle(rt: &mut Runtime, p: &Packet) -> Result<Value> {
    let unit = p.op.to_ascii_lowercase();
    let amount = parse_amount(rt, p.arg.as_ref())?;
    let duration = duration_from_unit(&unit, amount)?;
    let body = p
        .body
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("interval requires a {{ ... }} body"))?
        .clone();

    let mut child = rt.fork()?;
    thread::spawn(move || {
        loop {
            thread::sleep(duration);
            if let Err(err) = child.eval(&Node::Block(body.clone())) {
                eprintln!("interval loop error: {err:?}");
                break;
            }
            match child.flow_signal.clone() {
                FlowSignal::Break => {
                    child.take_signal();
                    break;
                }
                FlowSignal::Interrupt(_) => break,
                _ => {}
            }
        }
    });

    Ok(Value::Unit)
}

fn parse_amount(rt: &Runtime, arg: Option<&Arg>) -> Result<f64> {
    match arg {
        Some(Arg::Number(n)) => Ok(*n),
        Some(Arg::Ident(id)) => rt
            .get_var(id)
            .and_then(|v| v.try_num())
            .ok_or_else(|| anyhow::anyhow!("interval needs numeric len")),
        Some(Arg::Str(s)) => s
            .parse::<f64>()
            .map_err(|_| anyhow::anyhow!("interval requires numeric len")),
        _ => bail!("interval requires @<length>"),
    }
}
