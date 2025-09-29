use anyhow::{Result, bail};
use reqwest::Url;
use reqwest::blocking::{Client, Response};

use crate::kernel::config;
use crate::kernel::{Arg, Packet, Runtime, Value};

fn detect_method(op: &str) -> Option<&str> {
    if let Some(rest) = op.strip_prefix("http(") {
        if let Some(end) = rest.find(')') {
            return Some(&rest[..end]);
        }
    }
    None
}

fn allowed_url(cfg: &config::Config, url: &Url) -> bool {
    if cfg.net_allow.is_empty() {
        return false;
    }
    let full = url.as_str();
    let host = url.host_str().unwrap_or("");
    for pat in &cfg.net_allow {
        let p = pat.trim();
        if p.is_empty() {
            continue;
        }
        // scheme+prefix match
        if p.starts_with("http://") || p.starts_with("https://") {
            if full.starts_with(p) {
                return true;
            }
        } else if p.starts_with("*.") {
            let suf = &p[2..];
            if host.ends_with(suf) {
                return true;
            }
        } else {
            // bare host
            if host.eq_ignore_ascii_case(p) {
                return true;
            }
        }
    }
    false
}

pub fn handle(rt: &mut Runtime, p: &Packet) -> Result<Value> {
    if rt.effective_root.is_none() {
        bail!("E_BOX_REQUIRED: no red.tgsk");
    }

    let cfg = config::load(rt.effective_root.as_deref());
    if !cfg.net_enabled {
        bail!("E_NET_DENY: network disabled by default; enable in .tagspeak.toml [network]");
    }

    let method = detect_method(&p.op)
        .ok_or_else(|| anyhow::anyhow!("http needs method: http(get|post|put|delete)"))?;
    let url_s = match &p.arg {
        Some(Arg::Str(s)) => s.clone(),
        Some(Arg::Ident(id)) => id.clone(),
        _ => bail!("http needs @<url>"),
    };
    let url = Url::parse(&url_s).map_err(|_| anyhow::anyhow!("invalid_url"))?;
    if !allowed_url(&cfg, &url) {
        bail!("E_BOX_VIOLATION: url not allowed by .tagspeak.toml [network.allow]");
    }

    let client = Client::new();
    let mut req = match method.to_ascii_lowercase().as_str() {
        "get" => client.get(url.clone()),
        "post" => client.post(url.clone()),
        "put" => client.put(url.clone()),
        "delete" => client.delete(url.clone()),
        other => bail!(format!("http_method_unsupported:{other}")),
    };

    // Structured body support: [key(header.X)@v], [key(json)@{...}], [key(body)@"raw"]
    if let Some(body) = &p.body {
        use crate::kernel::Node;
        for node in body {
            if let Node::Packet(pkt) = node {
                let op = pkt.op.as_str();
                if op.starts_with("key(") && op.ends_with(')') {
                    if let Some(name) = op.get(4..op.len() - 1) {
                        if name.starts_with("header.") {
                            let h = &name[7..];
                            let val = match pkt.arg.as_ref() {
                                Some(Arg::Str(s)) => s.clone(),
                                Some(Arg::Ident(i)) => i.clone(),
                                Some(Arg::Number(n)) => n.to_string(),
                                _ => String::new(),
                            };
                            if !h.is_empty() {
                                req = req.header(h, val);
                            }
                        } else if name == "json" {
                            if let Some(arg) = pkt.arg.as_ref() {
                                let j = arg_to_json(rt, arg)?;
                                req = req.json(&j);
                            }
                        } else if name == "body" {
                            if let Some(Arg::Str(s)) = pkt.arg.as_ref() {
                                req = req.body(s.clone());
                            }
                        }
                    }
                }
            }
        }
    }

    // Timeout default 5s (can extend via config later)
    let resp = req.timeout(std::time::Duration::from_millis(5000)).send();
    let resp = match resp {
        Ok(r) => r,
        Err(e) => bail!(format!("E_HTTP: {e}")),
    };
    let out = handle_response(resp)?;
    Ok(out)
}

fn handle_response(mut resp: Response) -> Result<Value> {
    let status = resp.status();
    let ctype = resp
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_lowercase();

    // Prefer JSON if indicated
    if ctype.contains("application/json") || ctype.contains("+json") {
        let val: serde_json::Value = resp
            .json()
            .map_err(|e| anyhow::anyhow!(format!("E_HTTP_JSON:{e}")))?;
        // Wrap as Value::Str JSON string (consistent with [exec(json)])
        return Ok(Value::Str(serde_json::to_string(&val)?));
    }

    // Fallback to text
    let text = resp.text().unwrap_or_default();
    if !status.is_success() {
        bail!(format!("E_HTTP_STATUS:{}", status.as_u16()));
    }
    Ok(Value::Str(text))
}

fn value_to_json(v: Value) -> Result<serde_json::Value> {
    Ok(match v {
        Value::Unit => serde_json::Value::Null,
        Value::Bool(b) => serde_json::Value::Bool(b),
        Value::Num(n) => serde_json::Value::Number(
            serde_json::Number::from_f64(n).ok_or_else(|| anyhow::anyhow!("invalid number"))?,
        ),
        Value::Str(s) => serde_json::from_str(&s).unwrap_or(serde_json::Value::String(s)),
        Value::Doc(d) => d.json,
    })
}

fn arg_to_json(rt: &Runtime, arg: &Arg) -> Result<serde_json::Value> {
    Ok(match arg {
        Arg::Number(n) => {
            if n.fract() == 0.0 && *n >= (i64::MIN as f64) && *n <= (i64::MAX as f64) {
                serde_json::Value::Number(serde_json::Number::from((*n as i64)))
            } else {
                serde_json::Value::Number(
                    serde_json::Number::from_f64(*n)
                        .ok_or_else(|| anyhow::anyhow!("invalid number"))?,
                )
            }
        }
        Arg::Str(s) => serde_json::from_str(s).unwrap_or(serde_json::Value::String(s.clone())),
        Arg::Ident(id) => match id.as_str() {
            "true" => serde_json::Value::Bool(true),
            "false" => serde_json::Value::Bool(false),
            "null" => serde_json::Value::Null,
            other => {
                if let Some(v) = rt.get_var(other) {
                    value_to_json(v)?
                } else {
                    serde_json::Value::String(other.to_string())
                }
            }
        },
        _ => serde_json::Value::Null,
    })
}
