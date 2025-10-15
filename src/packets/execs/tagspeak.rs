use anyhow::{Result, bail};
use std::fs;
use std::path::{Path, PathBuf};

use crate::kernel::fs_guard::resolve;
use crate::kernel::{Arg, Packet, Runtime, Value};

enum Subcommand {
    Run,
    Build,
}

pub fn handle(rt: &mut Runtime, p: &Packet) -> Result<Value> {
    if rt.effective_root.is_none() {
        bail!("E_NO_RED: [tagspeak] requires a red.tgsk root");
    }

    let (cmd, raw_arg) = parse_command(rt, p)?;

    match cmd {
        Subcommand::Run => run(rt, raw_arg),
        Subcommand::Build => build(rt, raw_arg),
    }
}

fn parse_command(rt: &Runtime, p: &Packet) -> Result<(Subcommand, String)> {
    let verb = if matches!(p.ns.as_deref(), Some("tagspeak")) {
        p.op.trim()
    } else if let Some(rest) = p.op.trim().strip_prefix("tagspeak") {
        let rest = rest.trim_start();
        if rest.is_empty() {
            bail!("tagspeak packet needs a subcommand like `run` or `build`");
        }
        rest
    } else {
        bail!("unknown tagspeak form: {}", p.op);
    };

    let cmd = match verb {
        "run" => Subcommand::Run,
        "build" => Subcommand::Build,
        other => bail!("unknown tagspeak subcommand '{}'", other),
    };

    let raw = match &p.arg {
        Some(arg) => coerce_to_string(rt, arg)?,
        None => bail!("tagspeak {} expects @<path>", verb),
    };

    if raw.trim().is_empty() {
        bail!("tagspeak {} expects a non-empty path", verb);
    }

    Ok((cmd, raw))
}

fn coerce_to_string(rt: &Runtime, arg: &Arg) -> Result<String> {
    Ok(match arg {
        Arg::Str(s) => s.clone(),
        Arg::Ident(id) => match rt.get_var(id) {
            Some(Value::Str(s)) => s,
            Some(Value::Num(n)) => n.to_string(),
            Some(_) => bail!("variable '{}' does not contain a string path", id),
            None => bail!("variable '{}' not found for tagspeak packet", id),
        },
        Arg::Number(n) => n.to_string(),
        Arg::CondSrc(_) => bail!("conditions are not valid paths for tagspeak packets"),
    })
}

fn run(rt: &mut Runtime, raw_path: String) -> Result<Value> {
    let pkt = Packet {
        ns: None,
        op: "run".to_string(),
        arg: Some(Arg::Str(raw_path)),
        body: None,
    };
    super::run::handle(rt, &pkt)
}

fn build(rt: &Runtime, raw_path: String) -> Result<Value> {
    let resolved = resolve_within_root(rt, &raw_path)?;

    if !resolved.exists() {
        bail!(
            "tagspeak build target not found: {} (resolved {})",
            root_relative(rt, &resolved),
            resolved.display()
        );
    }

    if resolved
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("")
        != "tgsk"
    {
        bail!("tagspeak build expects a .tgsk file");
    }

    let src = fs::read_to_string(&resolved)?;
    crate::router::parse(&src)?;

    let rel = root_relative(rt, &resolved);
    Ok(Value::Str(format!("build_ok {}", rel)))
}

fn resolve_within_root(rt: &Runtime, raw: &str) -> Result<PathBuf> {
    let root = rt
        .effective_root
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("no red.tgsk root"))?;

    let trimmed = raw.trim();
    let rel = if trimmed.starts_with('/') {
        &trimmed[1..]
    } else {
        trimmed
    };

    let candidate = if trimmed.starts_with('/') {
        Path::new(rel).to_path_buf()
    } else {
        rt.cwd.join(rel)
    };

    resolve(root, &candidate)
}

fn root_relative(rt: &Runtime, abs: &Path) -> String {
    let root = match rt.effective_root.as_ref() {
        Some(r) => r,
        None => return abs.display().to_string(),
    };

    let rel_path = abs.strip_prefix(root).unwrap_or(abs);
    let mut out = String::from("/");
    let mut first = true;
    for part in rel_path.iter() {
        if !first {
            out.push('/');
        }
        first = false;
        let segment = part.to_string_lossy().replace('\\', "/");
        out.push_str(&segment);
    }
    if first {
        out.push('.');
    }
    out
}
