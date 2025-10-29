use anyhow::{Result, bail};
use std::thread;
use std::time::Duration;

use crate::kernel::ast::{Arg, Node};
use crate::kernel::{Packet, Runtime, Value};

pub fn handle(rt: &mut Runtime, p: &Packet) -> Result<Value> {
    let unit = p.op.to_ascii_lowercase();
    let amount = parse_amount(rt, p.arg.as_ref())?;
    let duration = duration_from_unit(&unit, amount)?;

    thread::sleep(duration);

    if let Some(body) = &p.body {
        rt.eval(&Node::Block(body.clone()))
    } else {
        Ok(Value::Unit)
    }
}

pub fn duration_from_unit(unit: &str, amount: f64) -> Result<Duration> {
    if amount.is_nan() || amount.is_sign_negative() {
        bail!("timeout length must be non-negative");
    }
    let seconds = match unit {
        "ms" | "millis" | "millisecond" | "milliseconds" => amount / 1000.0,
        "s" | "sec" | "secs" | "second" | "seconds" => amount,
        "m" | "min" | "mins" | "minute" | "minutes" => amount * 60.0,
        "hr" | "hour" | "hours" => amount * 60.0 * 60.0,
        "day" | "days" => amount * 60.0 * 60.0 * 24.0,
        other => bail!("unknown timeout unit '{other}'"),
    };
    Ok(Duration::from_secs_f64(seconds))
}

fn parse_amount(rt: &Runtime, arg: Option<&Arg>) -> Result<f64> {
    match arg {
        Some(Arg::Number(n)) => Ok(*n),
        Some(Arg::Ident(id)) => rt
            .get_var(id)
            .and_then(|v| v.try_num())
            .ok_or_else(|| anyhow::anyhow!("timeout needs numeric len")),
        Some(Arg::Str(s)) => s
            .parse::<f64>()
            .map_err(|_| anyhow::anyhow!("timeout requires numeric len")),
        _ => bail!("timeout requires @<length>"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{kernel::Runtime, router};

    #[test]
    fn timeout_executes_block_after_delay() -> Result<()> {
        let script =
            "[int@0]>[store@count]>[timeout:ms@1]{[math@count+1]>[store@count]}";
        let node = router::parse(script).map_err(anyhow::Error::new)?;
        let mut rt = Runtime::new()?;
        rt.eval(&node)?;
        assert_eq!(rt.get_num("count"), Some(1.0));
        Ok(())
    }
}
