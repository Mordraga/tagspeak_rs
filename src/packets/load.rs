use anyhow::{Result, Context};
use std::fs;
use std::path::Path;

use serde_json::Value as JsonValue;
use serde_yaml::Value as YamlValue;
use toml::Value as TomlValue;

use crate::kernel::ast::Arg;
use crate::kernel::fs_guard::resolve;
use crate::kernel::values::Document;
use crate::kernel::{Packet, Runtime, Value};

pub fn handle(rt: &mut Runtime, p: &Packet) -> Result<Value> {
    let raw = match &p.arg {
        Some(Arg::Str(s)) => s,
        _ => anyhow::bail!("load needs @<path>"),
    };

    let root = rt
        .effective_root
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("no red.tgsk root"))?;

    let anchor_from_root = raw.starts_with('/');
    let rel_path = if anchor_from_root { &raw[1..] } else { raw.as_str() };

    let candidate = if anchor_from_root {
        Path::new(rel_path).to_path_buf()
    } else {
        rt.cwd.join(rel_path)
    };

    // 👇 DEBUG #1: before resolve
    #[cfg(debug_assertions)]
    eprintln!(
        "[load] raw={:?}\n  cwd={}\n  root={}\n  rel={}\n  candidate={}",
        raw,
        rt.cwd.display(),
        root.display(),
        rel_path,
        candidate.display()
    );

    let path = resolve(root, &candidate)?;

    // 👇 DEBUG #2: after resolve (final absolute path)
    #[cfg(debug_assertions)]
    eprintln!("[load] resolved={}", path.display());

    let content = fs::read_to_string(&path)
        .with_context(|| format!("reading {}", path.display()))?;
    let meta = fs::metadata(&path)
        .with_context(|| format!("stat {}", path.display()))?;
    let mtime = meta.modified()?;
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_string();

    let json: JsonValue = match ext.as_str() {
        "yaml" | "yml" => {
            let val: YamlValue = serde_yaml::from_str(&content)?;
            serde_json::to_value(val)?
        }
        "toml" => {
            let val: TomlValue = toml::from_str(&content)?;
            serde_json::to_value(val)?
        }
        "json" | "" => serde_json::from_str(&content)?,
        _ => anyhow::bail!("format_unsupported"),
    };

    let doc = Document::new(json, path, ext, mtime, root.clone());
    Ok(Value::Doc(doc))
}


#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use serde_json::json;

    #[test]
    fn loads_file_within_red_root() {
        let base = std::env::temp_dir().join(format!("tgsk_load_test_{}", std::process::id()));
        let _ = fs::remove_dir_all(&base);
        fs::create_dir_all(base.join("sub")).unwrap();
        fs::create_dir_all(base.join("something")).unwrap();
        fs::write(base.join("red.tgsk"), "").unwrap();
        fs::write(base.join("something/config.json"), "{\"hi\":1}").unwrap();
        let script = base.join("sub").join("main.tgsk");
        fs::write(&script, "[load@/something/config.json]>[save@cfg]").unwrap();

        let ast = crate::router::parse(&fs::read_to_string(&script).unwrap()).unwrap();
        let mut rt = Runtime::from_entry(&script).unwrap();
        let _ = rt.eval(&ast).unwrap();
        match rt.get_var("cfg") {
            Some(Value::Doc(doc)) => assert_eq!(doc.json, json!({"hi":1})),
            other => panic!("unexpected value: {:?}", other),
        }

        fs::remove_dir_all(base).unwrap();
    }

    #[test]
    fn loads_yaml_file_within_red_root() {
        let base = std::env::temp_dir().join(format!("tgsk_load_yaml_test_{}", std::process::id()));
        let _ = fs::remove_dir_all(&base);
        fs::create_dir_all(base.join("sub")).unwrap();
        fs::create_dir_all(base.join("something")).unwrap();
        fs::write(base.join("red.tgsk"), "").unwrap();
        fs::write(base.join("something/config.yaml"), "hi: 1\n").unwrap();
        let script = base.join("sub").join("main.tgsk");
        fs::write(&script, "[load@/something/config.yaml]>[save@cfg]").unwrap();

        let ast = crate::router::parse(&fs::read_to_string(&script).unwrap()).unwrap();
        let mut rt = Runtime::from_entry(&script).unwrap();
        let _ = rt.eval(&ast).unwrap();
        match rt.get_var("cfg") {
            Some(Value::Doc(doc)) => assert_eq!(doc.json, json!({"hi":1})),
            other => panic!("unexpected value: {:?}", other),
        }

        fs::remove_dir_all(base).unwrap();
    }

    #[test]
    fn loads_toml_file_within_red_root() {
        let base = std::env::temp_dir().join(format!("tgsk_load_toml_test_{}", std::process::id()));
        let _ = fs::remove_dir_all(&base);
        fs::create_dir_all(base.join("sub")).unwrap();
        fs::create_dir_all(base.join("something")).unwrap();
        fs::write(base.join("red.tgsk"), "").unwrap();
        fs::write(base.join("something/config.toml"), "hi = 1\n").unwrap();
        let script = base.join("sub").join("main.tgsk");
        fs::write(&script, "[load@/something/config.toml]>[save@cfg]").unwrap();

        let ast = crate::router::parse(&fs::read_to_string(&script).unwrap()).unwrap();
        let mut rt = Runtime::from_entry(&script).unwrap();
        let _ = rt.eval(&ast).unwrap();
        match rt.get_var("cfg") {
            Some(Value::Doc(doc)) => assert_eq!(doc.json, json!({"hi":1})),
            other => panic!("unexpected value: {:?}", other),
        }

        fs::remove_dir_all(base).unwrap();
    }
}

