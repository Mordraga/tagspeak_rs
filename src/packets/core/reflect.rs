use anyhow::{Result, bail};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use crate::kernel::values::Document;
use crate::kernel::{Packet, Runtime, Value};

fn extract_group_block(content: &str, group: &str) -> Vec<String> {
    let mut out = Vec::new();
    let pat = format!("pub use {group}::{{");
    if let Some(start_idx) = content.find(&pat) {
        let after = &content[start_idx + pat.len()..];
        if let Some(end_rel) = after.find("};") {
            let inner = &after[..end_rel];
            for t in inner.split(',') {
                let t = t.trim();
                if t.is_empty() {
                    continue;
                }
                // skip allow/alias lines like `r#loop` re-exported as module
                out.push(t.to_string());
            }
        }
    }
    out
}

fn reflect_packets_from_mod_rs(
    root: &Path,
    override_path: Option<&Path>,
) -> Result<serde_json::Value> {
    let path = if let Some(p) = override_path {
        p.to_path_buf()
    } else {
        root.join("src").join("packets").join("mod.rs")
    };
    let content = fs::read_to_string(&path)?;

    // Accumulate tokens by group
    let mut core = BTreeSet::new();
    let mut files = BTreeSet::new();
    let mut flow = BTreeSet::new();
    let mut execs = BTreeSet::new();

    for it in extract_group_block(&content, "core") {
        core.insert(it);
    }
    for it in extract_group_block(&content, "files") {
        files.insert(it);
    }
    for it in extract_group_block(&content, "flow") {
        flow.insert(it);
    }
    for it in extract_group_block(&content, "execs") {
        execs.insert(it);
    }

    // Normalize tokens → canonical packet names
    let mut canon_core: BTreeSet<String> = BTreeSet::new();
    for t in &core {
        let name = t.trim_start_matches("r#").to_string();
        match name.as_str() {
            // skip internals; we list user-facing comparators instead
            "compare" => {
                for k in ["eq", "ne", "lt", "le", "gt", "ge"] {
                    canon_core.insert(k.to_string());
                }
            }
            other => {
                canon_core.insert(other.to_string());
            }
        }
    }

    let mut canon_files: BTreeSet<String> = BTreeSet::new();
    for t in &files {
        let name = t.trim_start_matches("r#").to_string();
        match name.as_str() {
            // expose query ops as get/exists instead of backend name
            "query" => {
                canon_files.insert("get".into());
                canon_files.insert("exists".into());
            }
            // map internal module name to packet op
            "modify" => {
                canon_files.insert("mod".into());
            }
            other => {
                canon_files.insert(other.to_string());
            }
        }
    }

    let mut canon_flow: BTreeSet<String> = BTreeSet::new();
    for t in &flow {
        let name = t.trim_start_matches("r#");
        match name {
            "loop" => {
                canon_flow.insert("loopN".into());
            }
            "conditionals" => { /* expose as if/or/else */ }
            other => {
                canon_flow.insert(other.to_string());
            }
        }
    }
    // Always include conditionals sugar
    for k in ["if", "or", "else"] {
        canon_flow.insert(k.into());
    }

    let mut canon_execs: BTreeSet<String> = BTreeSet::new();
    for t in &execs {
        canon_execs.insert(t.trim_start_matches("r#").to_string());
    }
    // Include sugar alias
    canon_execs.insert("yellow".into());

    // Helpers are defined as structural elements for [log]
    let helpers = vec!["key".to_string(), "sect".to_string()];

    // Build final structure
    let to_vec = |set: BTreeSet<String>| -> Vec<String> { set.into_iter().collect() };
    let canon = serde_json::json!({
        "core": to_vec(canon_core),
        "files": to_vec(canon_files),
        "flow": to_vec(canon_flow),
        "execs": to_vec(canon_execs),
        "helpers": helpers,
    });
    Ok(serde_json::json!({ "canon": canon }))
}

fn reflect_packets_from_fs(root: &Path) -> Result<serde_json::Value> {
    use std::collections::BTreeSet;
    let base = root.join("src").join("packets");

    let mut core: BTreeSet<String> = BTreeSet::new();
    let mut files: BTreeSet<String> = BTreeSet::new();
    let mut flow: BTreeSet<String> = BTreeSet::new();
    let mut execs: BTreeSet<String> = BTreeSet::new();

    let scan = |dir: &Path| -> Vec<String> {
        let mut out = Vec::new();
        if let Ok(rd) = fs::read_dir(dir) {
            for ent in rd.flatten() {
                let p = ent.path();
                if p.is_file() {
                    if p.file_name().and_then(|s| s.to_str()) == Some("mod.rs") {
                        continue;
                    }
                    if let Some(stem) = p.file_stem().and_then(|s| s.to_str()) {
                        out.push(stem.to_string());
                    }
                }
            }
        }
        out
    };

    // Core
    for name in scan(&base.join("core")) {
        match name.as_str() {
            "compare" => {
                for k in ["eq", "ne", "lt", "le", "gt", "ge"] {
                    core.insert(k.into());
                }
            }
            other => {
                core.insert(other.into());
            }
        }
    }
    // Files
    for name in scan(&base.join("files")) {
        match name.as_str() {
            "modify" => {
                files.insert("mod".into());
            }
            "query" => {
                files.insert("get".into());
                files.insert("exists".into());
            }
            other => {
                files.insert(other.into());
            }
        }
    }
    // Flow
    for name in scan(&base.join("flow")) {
        match name.as_str() {
            "loop" => {
                flow.insert("loopN".into());
            }
            "conditionals" => { /* expose as sugar below */ }
            other => {
                flow.insert(other.into());
            }
        }
    }
    for k in ["if", "or", "else"] {
        flow.insert(k.into());
    }

    // Execs
    for name in scan(&base.join("execs")) {
        execs.insert(name);
    }
    execs.insert("yellow".into());

    // Helpers
    let helpers = vec!["key".to_string(), "sect".to_string()];

    let to_vec = |set: BTreeSet<String>| -> Vec<String> { set.into_iter().collect() };
    let canon = serde_json::json!({
        "core": to_vec(core),
        "files": to_vec(files),
        "flow": to_vec(flow),
        "execs": to_vec(execs),
        "helpers": helpers,
    });
    Ok(serde_json::json!({ "canon": canon }))
}

fn extract_name_from_sig(sig: &str) -> Option<String> {
    // sig like: [print], [log(json)@file]{...}, [store:rigid@x]
    if !sig.starts_with('[') {
        return None;
    }
    let inner = &sig[1..];
    let mut name = String::new();
    for ch in inner.chars() {
        if ch == '(' || ch == '@' || ch == ':' || ch == ']' {
            break;
        }
        name.push(ch);
    }
    if name.is_empty() { None } else { Some(name) }
}

fn clean_desc(mut s: &str) -> String {
    // Trim leading separators/dashes and whitespace
    s = s.trim();
    // strip common separators like "+-—–:>"
    while let Some(c) = s.chars().next() {
        if c.is_whitespace()
            || c == '-'
            || c == '—'
            || c == '–'
            || c == ':'
            || c == '>'
            || c == '�'
            || c == '+'
        {
            s = s.get(c.len_utf8()..).unwrap_or("").trim_start();
        } else {
            break;
        }
    }
    s.to_string()
}

fn parse_packet_docs_from(
    content: &str,
    source: &str,
    out: &mut BTreeMap<String, serde_json::Value>,
) {
    let mut section = String::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("### ") {
            // capture section title
            section = trimmed[4..].trim().to_string();
            continue;
        }
        if !(trimmed.starts_with('*') || trimmed.starts_with('-')) {
            continue;
        }
        // find code span between backticks
        let first_tick = trimmed.find('`');
        let second_tick = first_tick.and_then(|i| trimmed[i + 1..].find('`').map(|j| i + 1 + j));
        if let (Some(i), Some(j)) = (first_tick, second_tick) {
            let code = &trimmed[i + 1..j];
            if let Some(name) = extract_name_from_sig(code) {
                // desc is rest after closing backtick
                let rest = &trimmed[j + 1..];
                let desc = clean_desc(rest);
                let entry = serde_json::json!({
                    "section": section,
                    "desc": desc,
                    "source": source,
                    "sig": code,
                });
                // prefer longer descriptions when merging
                match out.get(&name) {
                    Some(prev) => {
                        let prev_len = prev
                            .get("desc")
                            .and_then(|v| v.as_str())
                            .map(|s| s.len())
                            .unwrap_or(0);
                        let cur_len = entry
                            .get("desc")
                            .and_then(|v| v.as_str())
                            .map(|s| s.len())
                            .unwrap_or(0);
                        if cur_len > prev_len {
                            out.insert(name, entry);
                        }
                    }
                    None => {
                        out.insert(name, entry);
                    }
                }
            }
        }
    }
}

fn reflect_packet_docs(root: &Path) -> BTreeMap<String, serde_json::Value> {
    let mut map: BTreeMap<String, serde_json::Value> = BTreeMap::new();
    let cand = [
        (root.join("docs").join("Tagspeak_101.md"), "Tagspeak_101.md"),
        (root.join("docs").join("README.md"), "README.md"),
    ];
    for (path, label) in cand {
        if let Ok(s) = fs::read_to_string(&path) {
            parse_packet_docs_from(&s, label, &mut map);
        }
    }
    map
}

pub fn handle(rt: &mut Runtime, p: &Packet) -> Result<Value> {
    // Modes:
    //   reflect(packets)[@</path/to/mod.rs|/dir>]
    //   reflect(vars)
    //   reflect(runtime)
    //   reflect(doc)[@handle]
    let mode = if let Some(rest) = p.op.strip_prefix("reflect(") {
        rest.trim_end_matches(')')
    } else {
        ""
    };

    let root = rt
        .effective_root
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("E_BOX_REQUIRED: no red.tgsk"))?;

    match mode {
        "packets" => {
            // Optional @path: file to mod.rs or directory containing src/packets/mod.rs
            let override_path: Option<PathBuf> =
                if let Some(crate::kernel::ast::Arg::Str(s)) = p.arg.as_ref() {
                    let rel = if s.starts_with('/') {
                        &s[1..]
                    } else {
                        s.as_str()
                    };
                    let candidate = Path::new(rel);
                    let requested = if rel.ends_with("mod.rs") {
                        candidate.to_path_buf()
                    } else {
                        candidate.join("src").join("packets").join("mod.rs")
                    };
                    Some(crate::kernel::fs_guard::resolve(root, &requested)?)
                } else {
                    None
                };

            let json = reflect_packets_from_mod_rs(root, override_path.as_deref())?;
            let doc = Document::new(
                json,
                root.join("docs").join("PACKETS.json"),
                "json".into(),
                SystemTime::now(),
                root.clone(),
            );
            Ok(Value::Doc(doc))
        }
        "packets_fs" => {
            let json = reflect_packets_from_fs(root)?;
            let doc = Document::new(
                json,
                root.join("docs").join("PACKETS.json"),
                "json".into(),
                SystemTime::now(),
                root.clone(),
            );
            Ok(Value::Doc(doc))
        }
        "packets_full" => {
            // Canonical tokens
            let canon = reflect_packets_from_mod_rs(root, None)?;
            // Parse docs
            let docs_map = reflect_packet_docs(root);

            // Build a set of known canonical names
            let mut known = BTreeSet::new();
            if let Some(c) = canon.get("canon") {
                for key in ["core", "files", "flow", "execs", "helpers"] {
                    if let Some(arr) = c.get(key).and_then(|v| v.as_array()) {
                        for v in arr {
                            if let Some(s) = v.as_str() {
                                known.insert(s.to_string());
                            }
                        }
                    }
                }
            }

            // Filter docs to canonical names only
            let mut details = serde_json::Map::new();
            for (k, v) in docs_map {
                if known.contains(&k) {
                    details.insert(k, v);
                }
            }

            let out = serde_json::json!({
                "canon": canon.get("canon").cloned().unwrap_or(serde_json::json!({})),
                "details": serde_json::Value::Object(details),
            });
            let doc = Document::new(
                out,
                root.join("docs").join("PACKETS.json"),
                "json".into(),
                SystemTime::now(),
                root.clone(),
            );
            Ok(Value::Doc(doc))
        }
        "packets_full_fs" => {
            // Canonical tokens from filesystem
            let canon = reflect_packets_from_fs(root)?;
            let docs_map = reflect_packet_docs(root);

            // Build a set of known canonical names
            use std::collections::BTreeSet;
            let mut known = BTreeSet::new();
            if let Some(c) = canon.get("canon") {
                for key in ["core", "files", "flow", "execs", "helpers"] {
                    if let Some(arr) = c.get(key).and_then(|v| v.as_array()) {
                        for v in arr {
                            if let Some(s) = v.as_str() {
                                known.insert(s.to_string());
                            }
                        }
                    }
                }
            }
            let mut details = serde_json::Map::new();
            for (k, v) in docs_map {
                if known.contains(&k) {
                    details.insert(k, v);
                }
            }
            let out = serde_json::json!({
                "canon": canon.get("canon").cloned().unwrap_or(serde_json::json!({})),
                "details": serde_json::Value::Object(details),
            });
            let doc = Document::new(
                out,
                root.join("docs").join("PACKETS.json"),
                "json".into(),
                SystemTime::now(),
                root.clone(),
            );
            Ok(Value::Doc(doc))
        }
        "vars" => {
            use serde_json::Value as J;
            let mut obj = serde_json::Map::new();
            for (k, v) in rt.vars.clone() {
                obj.insert(k, value_to_json_reflect(v)?);
            }
            let json = J::Object(obj);
            let doc = Document::new(
                json,
                root.join("_reflect_vars.json"),
                "json".into(),
                SystemTime::now(),
                root.clone(),
            );
            Ok(Value::Doc(doc))
        }
        "runtime" => {
            use serde_json::json;
            let cwd_s = format!("/{}", rt.cwd.display());
            let tags: Vec<String> = rt.tags.keys().cloned().collect();
            let rigid: Vec<String> = rt.rigid.iter().cloned().collect();
            let json = json!({
                "root": "/",
                "cwd": cwd_s,
                "tags": tags,
                "rigid": rigid,
            });
            let doc = Document::new(
                json,
                root.join("_reflect_runtime.json"),
                "json".into(),
                SystemTime::now(),
                root.clone(),
            );
            Ok(Value::Doc(doc))
        }
        "doc" => {
            // If @handle provided, use that; else expect last to be a doc
            let d = if let Some(crate::kernel::ast::Arg::Ident(id)) = p.arg.as_ref() {
                match rt.get_var(id) {
                    Some(Value::Doc(d)) => d,
                    _ => bail!("handle_unknown"),
                }
            } else {
                match &rt.last {
                    Value::Doc(d) => d.clone(),
                    _ => bail!("no_doc_last"),
                }
            };
            use serde_json::json;
            let rel_path = d.path.strip_prefix(root).unwrap_or(&d.path).to_path_buf();
            let json = json!({
                "path": format!("/{}", rel_path.display()),
                "ext": d.ext,
                "json": d.json,
            });
            let doc = Document::new(
                json,
                root.join("_reflect_doc.json"),
                "json".into(),
                SystemTime::now(),
                root.clone(),
            );
            Ok(Value::Doc(doc))
        }
        _ => bail!("reflect mode unsupported"),
    }
}

fn value_to_json_reflect(v: Value) -> Result<serde_json::Value> {
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
