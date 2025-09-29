use anyhow::{Result, bail};
use std::fs;
use std::path::Path;

use serde_yaml::Value as YamlValue;
use toml::Value as TomlValue;

use crate::kernel::ast::Arg;
use crate::kernel::fs_guard::resolve;
use crate::kernel::values::{Document, Value};
use crate::kernel::{Packet, Runtime};

pub fn handle(rt: &mut Runtime, p: &Packet) -> Result<Value> {
    let raw = match &p.arg {
        Some(Arg::Str(s)) => s,
        _ => anyhow::bail!("load needs @<path>"),
    };

    let root = rt
        .effective_root
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("no red.tgsk root"))?;

    let rel_path = if raw.starts_with('/') {
        &raw[1..]
    } else {
        raw.as_str()
    };
    let candidate = if raw.starts_with('/') {
        Path::new(rel_path).to_path_buf()
    } else {
        rt.cwd.join(rel_path)
    };

    let path = resolve(root, &candidate)?;
    let content = fs::read_to_string(&path)?;
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    // Parse into a canonical JSON value for in-memory editing
    let json_val: serde_json::Value = match ext.as_str() {
        "yaml" | "yml" => {
            let yv: YamlValue = serde_yaml::from_str(&content)?;
            // Convert via serde to JSON value
            serde_json::to_value(yv)?
        }
        "toml" => {
            let tv: TomlValue = toml::from_str(&content)?;
            serde_json::to_value(tv)?
        }
        "json" | "" => serde_json::from_str(&content).unwrap_or(serde_json::Value::Null),
        other => bail!("unsupported_ext:{other}"),
    };

    let meta = fs::metadata(&path)?;
    let mtime = meta.modified()?;
    let doc = Document::new(json_val, path.clone(), ext, mtime, root.clone());
    Ok(Value::Doc(doc))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn loads_file_within_red_root() {
        let base = std::env::temp_dir().join(format!("tgsk_load_test_{}", std::process::id()));
        let _ = fs::remove_dir_all(&base);
        fs::create_dir_all(base.join("sub")).unwrap();
        fs::create_dir_all(base.join("something")).unwrap();
        fs::write(base.join("red.tgsk"), "").unwrap();
        fs::write(base.join("something/config.json"), "{\"hi\":1}").unwrap();
        let script = base.join("sub").join("main.tgsk");
        fs::write(&script, "[load@/something/config.json]>[store@cfg]").unwrap();

        let ast = crate::router::parse(&fs::read_to_string(&script).unwrap()).unwrap();
        let mut rt = Runtime::from_entry(&script).unwrap();
        let _ = rt.eval(&ast).unwrap();
        match rt.get_var("cfg") {
            Some(Value::Doc(d)) => assert_eq!(d.json["hi"].as_i64(), Some(1)),
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
        fs::write(&script, "[load@/something/config.yaml]>[store@cfg]").unwrap();

        let ast = crate::router::parse(&fs::read_to_string(&script).unwrap()).unwrap();
        let mut rt = Runtime::from_entry(&script).unwrap();
        let _ = rt.eval(&ast).unwrap();
        match rt.get_var("cfg") {
            Some(Value::Doc(d)) => assert_eq!(d.json["hi"].as_i64(), Some(1)),
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
        fs::write(&script, "[load@/something/config.toml]>[store@cfg]").unwrap();

        let ast = crate::router::parse(&fs::read_to_string(&script).unwrap()).unwrap();
        let mut rt = Runtime::from_entry(&script).unwrap();
        let _ = rt.eval(&ast).unwrap();
        match rt.get_var("cfg") {
            Some(Value::Doc(d)) => assert_eq!(d.json["hi"].as_i64(), Some(1)),
            other => panic!("unexpected value: {:?}", other),
        }

        fs::remove_dir_all(base).unwrap();
    }
}
