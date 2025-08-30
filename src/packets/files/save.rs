use anyhow::{Result, bail};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use tempfile::NamedTempFile;

use crate::kernel::ast::Arg;
use crate::kernel::values::{Document, Value};
use crate::kernel::{Packet, Runtime};

pub fn handle(rt: &mut Runtime, p: &Packet) -> Result<Value> {
    let handle = match &p.arg {
        Some(Arg::Ident(id)) => id.as_str(),
        _ => bail!("save needs @<ident>"),
    };

    if let Some(Value::Doc(doc)) = rt.get_var(handle) {
        // already registered -> attempt write
        let mut doc = doc;
        if doc.json == doc.last_json {
            return Ok(Value::Doc(doc));
        }

        let current_mtime = fs::metadata(&doc.path)?.modified()?;
        if current_mtime != doc.mtime {
            bail!("changed_on_disk");
        }

        let bytes = encode(&doc)?;

        let tmp_path = temp_path(&doc.path);
        {
            let dir = tmp_path.parent().unwrap_or(Path::new("."));
            let mut tmp = NamedTempFile::new_in(dir)?;
            tmp.write_all(&bytes)?;
            tmp.persist(&tmp_path)?;
        }
        fs::rename(&tmp_path, &doc.path)?;
        let meta = fs::metadata(&doc.path)?;
        doc.mtime = meta.modified()?;
        doc.last_json = doc.json.clone();
        rt.set_var(handle, Value::Doc(doc.clone()))?;
        Ok(Value::Doc(doc))
    } else {
        // not yet registered -> register from last value
        match rt.last.clone() {
            Value::Doc(doc) => {
                rt.set_var(handle, Value::Doc(doc.clone()))?;
                Ok(Value::Doc(doc))
            }
            _ => bail!("save needs document in pipeline"),
        }
    }
}

fn encode(doc: &Document) -> Result<Vec<u8>> {
    let s = match doc.ext.as_str() {
        "yaml" | "yml" => serde_yaml::to_string(&doc.json)?,
        "toml" => toml::to_string_pretty(&doc.json)?,
        "json" | "" => {
            let raw = serde_json::to_string_pretty(&doc.json)?;
            cleanup_trailing_commas(&raw)
        }
        other => bail!("format_unsupported: {other}"),
    };
    Ok(s.into_bytes())
}

fn cleanup_trailing_commas(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let chars: Vec<char> = s.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == ',' {
            let mut j = i + 1;
            while j < chars.len() && chars[j].is_whitespace() {
                j += 1;
            }
            if j < chars.len() && (chars[j] == '}' || chars[j] == ']') {
                i += 1;
                continue;
            }
        }
        out.push(chars[i]);
        i += 1;
    }
    out
}

fn temp_path(path: &PathBuf) -> PathBuf {
    let mut tmp = path.clone();
    if let Some(ext) = tmp.extension() {
        let mut e = ext.to_os_string();
        e.push(".tmp");
        tmp.set_extension(e);
    } else {
        tmp.set_extension("tmp");
    }
    tmp
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn modifies_and_saves_file() {
        let base = std::env::temp_dir().join(format!("tgsk_save_test_{}", std::process::id()));
        let _ = fs::remove_dir_all(&base);
        fs::create_dir_all(base.join("sub")).unwrap();
        fs::write(base.join("red.tgsk"), "").unwrap();
        fs::write(base.join("config.json"), "{\"a\":{\"b\":1,\"c\":2}}").unwrap();
        let script = base.join("sub").join("main.tgsk");
        fs::write(&script, "[load@/config.json]>[save@cfg]>[mod@cfg]{[comp(a.b)@2][merge(a)@{\"d\":4}][ins(a.e)@5][del(a.c)]}>[save@cfg]").unwrap();

        let ast = crate::router::parse(&fs::read_to_string(&script).unwrap()).unwrap();
        let mut rt = Runtime::from_entry(&script).unwrap();
        let _ = rt.eval(&ast).unwrap();
        let out: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(base.join("config.json")).unwrap()).unwrap();
        assert_eq!(out["a"]["b"].as_f64(), Some(2.0));
        assert_eq!(out["a"]["d"].as_i64(), Some(4));
        assert_eq!(out["a"]["e"].as_f64(), Some(5.0));

        fs::remove_dir_all(base).unwrap();
    }

    #[test]
    fn deleting_last_key_writes_valid_json() {
        let base = std::env::temp_dir().join(format!("tgsk_trailing_test_{}", std::process::id()));
        let _ = fs::remove_dir_all(&base);
        fs::create_dir_all(base.join("sub")).unwrap();
        fs::write(base.join("red.tgsk"), "").unwrap();
        fs::write(base.join("config.json"), "{\"greeting\":\"hi\",\"test\":1}").unwrap();
        let script = base.join("sub").join("main.tgsk");
        fs::write(
            &script,
            "[load@/config.json]>[save@cfg]>[mod@cfg]{[del(test)]}>[save@cfg]",
        )
        .unwrap();

        let ast = crate::router::parse(&fs::read_to_string(&script).unwrap()).unwrap();
        let mut rt = Runtime::from_entry(&script).unwrap();
        let _ = rt.eval(&ast).unwrap();
        let content = fs::read_to_string(base.join("config.json")).unwrap();
        serde_json::from_str::<serde_json::Value>(&content).expect("valid json");
        assert!(!content.contains(",\n}"));

        fs::remove_dir_all(base).unwrap();
    }
}
