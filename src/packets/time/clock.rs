use anyhow::{Result, bail};
use chrono::{DateTime, Datelike, Local, SecondsFormat, Timelike, Utc};

use crate::kernel::ast::Arg;
use crate::kernel::{Packet, Runtime, Value};

pub fn handle_utc(_rt: &mut Runtime, p: &Packet) -> Result<Value> {
    let now = Utc::now();
    resolve_component(now, p.arg.as_ref())
}

pub fn handle_local(_rt: &mut Runtime, p: &Packet) -> Result<Value> {
    let now = Local::now();
    resolve_component(now, p.arg.as_ref())
}

fn resolve_component<Tz>(dt: DateTime<Tz>, arg: Option<&Arg>) -> Result<Value>
where
    Tz: chrono::TimeZone,
    Tz::Offset: Send + Sync,
{
    match arg {
        None => Ok(Value::Str(dt.to_rfc3339_opts(SecondsFormat::Millis, true))),
        Some(component) => {
            let key = match component {
                Arg::Ident(id) => id.as_str(),
                Arg::Str(s) => s.as_str(),
                _ => bail!("time component must be identifier or string"),
            };
            component_value(&dt, key)
        }
    }
}

fn component_value<Tz>(dt: &DateTime<Tz>, key: &str) -> Result<Value>
where
    Tz: chrono::TimeZone,
    Tz::Offset: Send + Sync,
{
    let lower = key.to_ascii_lowercase();
    let val = match lower.as_str() {
        "year" => Value::Num(dt.year() as f64),
        "month" | "mon" => Value::Num(dt.month() as f64),
        "day" | "date" => Value::Num(dt.day() as f64),
        "hour" | "hr" => Value::Num(dt.hour() as f64),
        "minute" | "min" => Value::Num(dt.minute() as f64),
        "second" | "sec" => Value::Num(dt.second() as f64),
        "millisecond" | "ms" | "millis" => Value::Num(dt.timestamp_subsec_millis() as f64),
        "microsecond" | "us" | "micros" => Value::Num(dt.timestamp_subsec_micros() as f64),
        "nanosecond" | "ns" | "nanos" => Value::Num(dt.timestamp_subsec_nanos() as f64),
        "weekday" | "dow" => Value::Str(dt.weekday().to_string()),
        "yearday" | "ordinal" => Value::Num(dt.ordinal() as f64),
        "unix" | "timestamp" => Value::Num(dt.timestamp() as f64),
        "iso" => Value::Str(dt.to_rfc3339()),
        _ => bail!("unknown time component '{key}'"),
    };
    Ok(val)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{kernel::Runtime, router};
    use anyhow::Error;

    #[test]
    fn utc_returns_iso_string() -> Result<()> {
        let script = "[UTC]";
        let node = router::parse(script).map_err(Error::new)?;
        let mut rt = Runtime::new()?;
        let value = rt.eval(&node)?;
        match value {
            Value::Str(s) => {
                assert!(s.contains('T'), "expected ISO timestamp, got {s}");
            }
            other => bail!("expected string, got {other:?}"),
        }
        Ok(())
    }

    #[test]
    fn local_second_component_is_in_range() -> Result<()> {
        let script = "[local@sec]";
        let node = router::parse(script).map_err(Error::new)?;
        let mut rt = Runtime::new()?;
        let value = rt.eval(&node)?;
        match value {
            Value::Num(n) => {
                assert!(n >= 0.0 && n < 60.0, "second component out of range: {n}");
            }
            other => bail!("expected numeric second, got {other:?}"),
        }
        Ok(())
    }
}
