use anyhow::{Result, bail};
use std::process::Command;

use crate::kernel::config;
use crate::kernel::{Arg, Packet, Runtime, Value};

enum ExecMode {
    Stdout,
    Stderr,
    Code,
    Json,
}

fn detect_mode(op: &str) -> ExecMode {
    if let Some(rest) = op.strip_prefix("exec(")
        && let Some(end) = rest.find(')')
    {
        match &rest[..end].to_lowercase() {
            s if s == "stderr" => return ExecMode::Stderr,
            s if s == "code" => return ExecMode::Code,
            s if s == "json" => return ExecMode::Json,
            _ => {}
        }
    }
    ExecMode::Stdout
}

pub fn handle(rt: &mut Runtime, p: &Packet) -> Result<Value> {
    if rt.effective_root.is_none() {
        anyhow::bail!("E_NO_RED: [exec] disabled without a red.tgsk root");
    }
    // Red no longer required for exec; keep per-action yellow consent elsewhere
    let cmdline = match &p.arg {
        Some(Arg::Str(s)) => s.clone(),
        Some(Arg::Ident(id)) => id.clone(),
        Some(Arg::Number(n)) => n.to_string(),
        _ => bail!("exec needs @<command>"),
    };

    // Hard gate: require being within a yellow block unless env override
    let depth = rt.get_num("__yellow_depth").unwrap_or(0.0);
    let allowed_env = std::env::var("TAGSPEAK_ALLOW_EXEC")
        .map(|v| matches!(v.as_str(), "1" | "true" | "yes" | "y"))
        .unwrap_or(false);
    if depth <= 0.0 && !allowed_env {
        bail!(
            "E_YELLOW_REQUIRED: wrap [exec] in [yellow]{{...}} or use [yellow:exec@...] (or set TAGSPEAK_ALLOW_EXEC=1)"
        );
    }

    let mode = detect_mode(&p.op);

    // Config-driven gating: allow_exec or allowlist can bypass yellow
    let cfg = config::load(rt.effective_root.as_deref());
    let depth = rt.get_num("__yellow_depth").unwrap_or(0.0);
    let allowed_by_cfg = {
        if cfg.allow_exec {
            true
        } else {
            // best-effort extract first token for allowlist match
            let first = cmdline.split_whitespace().next().unwrap_or("");
            cfg.exec_allowlist.iter().any(|c| c == first)
        }
    };
    if depth <= 0.0 && !allowed_by_cfg {
        // Still permit explicit env override if set
        let allowed_env = std::env::var("TAGSPEAK_ALLOW_EXEC")
            .map(|v| matches!(v.as_str(), "1" | "true" | "yes" | "y"))
            .unwrap_or(false);
        if !allowed_env {
            anyhow::bail!(
                "E_YELLOW_REQUIRED: wrap [exec] in [yellow]{{...}} or use [yellow:exec@...] (or set TAGSPEAK_ALLOW_EXEC=1)"
            );
        }
    }

    // Compute working directory: effective_root + cwd (if available)
    let current_dir = rt.effective_root.as_ref().map(|root| root.join(&rt.cwd));

    // Spawn via platform shell so we support pipelines and quoting
    let output = {
        #[cfg(target_os = "windows")]
        let mut cmd = {
            let mut c = Command::new("cmd");
            c.arg("/C").arg(&cmdline);
            c
        };
        #[cfg(not(target_os = "windows"))]
        let mut cmd = {
            let mut c = Command::new("sh");
            c.arg("-c").arg(&cmdline);
            c
        };

        if let Some(dir) = &current_dir {
            cmd.current_dir(dir);
        }

        cmd.output()?
    };

    let code = output.status.code().unwrap_or_default();
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    Ok(match mode {
        ExecMode::Stdout => Value::Str(stdout),
        ExecMode::Stderr => Value::Str(stderr),
        ExecMode::Code => Value::Num(code as f64),
        ExecMode::Json => {
            let obj = serde_json::json!({
                "code": code,
                "stdout": stdout,
                "stderr": stderr,
            });
            Value::Str(serde_json::to_string(&obj)?)
        }
    })
}
