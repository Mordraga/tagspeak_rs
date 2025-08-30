use std::path::Path;

#[derive(Clone, Debug, Default)]
pub struct Config {
    pub allow_exec: bool,
    pub exec_allowlist: Vec<String>,
    pub run_max_depth: usize,
    pub prompts_noninteractive: bool,
    pub require_yellow_run: bool,
}

fn parse_bool_env(key: &str) -> Option<bool> {
    std::env::var(key).ok().and_then(|v| {
        let s = v.to_lowercase();
        match s.as_str() {
            "1" | "true" | "yes" | "y" => Some(true),
            "0" | "false" | "no" | "n" => Some(false),
            _ => None,
        }
    })
}

fn parse_usize_env(key: &str) -> Option<usize> {
    std::env::var(key).ok()?.parse::<usize>().ok()
}

fn parse_list_env(key: &str) -> Option<Vec<String>> {
    std::env::var(key).ok().map(|s| {
        s.split(',')
            .map(|t| t.trim().to_string())
            .filter(|t| !t.is_empty())
            .collect::<Vec<_>>()
    })
}

pub fn load(root: Option<&Path>) -> Config {
    // Defaults
    let mut cfg = Config {
        allow_exec: false,
        exec_allowlist: Vec::new(),
        run_max_depth: 8,
        prompts_noninteractive: false,
        require_yellow_run: false,
    };

    // Read TOML if present
    if let Some(root) = root {
        let path = root.join(".tagspeak.toml");
        if let Ok(s) = std::fs::read_to_string(path) {
            if let Ok(val) = s.parse::<toml::Value>() {
                // security.allow_exec (bool)
                if let Some(b) = val
                    .get("security")
                    .and_then(|t| t.get("allow_exec"))
                    .and_then(|v| v.as_bool())
                {
                    cfg.allow_exec = b;
                }
                // security.exec_allowlist ([string])
                if let Some(list) = val
                    .get("security")
                    .and_then(|t| t.get("exec_allowlist"))
                    .and_then(|v| v.as_array())
                {
                    cfg.exec_allowlist = list
                        .iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect();
                }
                // run.max_depth (usize)
                if let Some(n) = val
                    .get("run")
                    .and_then(|t| t.get("max_depth"))
                    .and_then(|v| v.as_integer())
                {
                    if n > 0 { cfg.run_max_depth = n as usize; }
                }
                // run.require_yellow (bool)
                if let Some(b) = val
                    .get("run")
                    .and_then(|t| t.get("require_yellow"))
                    .and_then(|v| v.as_bool())
                {
                    cfg.require_yellow_run = b;
                }
                // prompts.noninteractive (bool)
                if let Some(b) = val
                    .get("prompts")
                    .and_then(|t| t.get("noninteractive"))
                    .and_then(|v| v.as_bool())
                {
                    cfg.prompts_noninteractive = b;
                }
            }
        }
    }

    // Env overrides
    if let Some(b) = parse_bool_env("TAGSPEAK_ALLOW_EXEC") { cfg.allow_exec = b; }
    if let Some(n) = parse_usize_env("TAGSPEAK_MAX_RUN_DEPTH") { if n > 0 { cfg.run_max_depth = n; } }
    if let Some(b) = parse_bool_env("TAGSPEAK_NONINTERACTIVE") { cfg.prompts_noninteractive = b; }
    if let Some(list) = parse_list_env("TAGSPEAK_EXEC_ALLOWLIST") { cfg.exec_allowlist = list; }

    cfg
}

