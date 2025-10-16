mod error_style;
mod interpreter;
mod kernel;
mod packets;
mod router;

use anyhow::{Result, anyhow};
use kernel::Runtime;
use kernel::ast::{Arg, Packet as AstPacket};
use kernel::values::Value;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process;

fn main() {
    if let Err(err) = run_cli() {
        eprintln!("\n{err}");
        process::exit(1);
    }
}

fn run_cli() -> Result<()> {
    // Simple CLI: `tagspeak init [dir]` or `tagspeak <file.tgsk>`
    let mut args = env::args();
    let _exe = args.next();
    match args.next() {
        Some(cmd) if cmd == "init" => {
            let dir = args.next();
            init_red(dir.as_deref())
        }
        Some(cmd) if cmd == "run" => {
            let path = args
                .next()
                .ok_or_else(|| anyhow!("`tagspeak run` expects a <file.tgsk> argument"))?;
            run_script(&path)
        }
        Some(cmd) if cmd == "build" => {
            let path = args
                .next()
                .ok_or_else(|| anyhow!("`tagspeak build` expects a <file.tgsk> argument"))?;
            build_script(&path)
        }
        Some(cmd) if cmd == "help" => {
            let topic = args.next();
            run_help(topic.as_deref())
        }
        Some(cmd) if cmd == "lint" => {
            let path = args
                .next()
                .ok_or_else(|| anyhow!("`tagspeak lint` expects a <file.tgsk> argument"))?;
            lint_script(&path)
        }
        Some(path) => run_script(&path),
        None => {
            // no args: guide the user
            eprintln!(
                "No input file provided. Usage:\n  tagspeak init [dir]\n  tagspeak run <file.tgsk>\n  tagspeak build <file.tgsk>\n  tagspeak help [packet]\n  tagspeak lint <file.tgsk>\n  tagspeak <file.tgsk>"
            );
            Err(anyhow!("no_input"))
        }
    }
}

fn init_red(dir: Option<&str>) -> Result<()> {
    let target: PathBuf = match dir {
        Some(d) => PathBuf::from(d),
        None => env::current_dir()?,
    };
    if !target.exists() {
        fs::create_dir_all(&target)?;
    }
    let path = target.join("red.tgsk");
    if path.exists() {
        println!("red.tgsk already exists at {}", path.display());
        return Ok(());
    }
    let banner = "# TagSpeak project root\n# This file marks the sandbox boundary for file access and execution.\n# Keep it checked into version control.\n";
    fs::write(&path, banner)?;
    println!("Created {}", path.display());
    Ok(())
}

fn run_script(path: &str) -> Result<()> {
    println!("Running file: {}", &path);
    let src = fs::read_to_string(&path)?;
    let ast = router::parse(&src).map_err(anyhow::Error::new)?;
    let mut rt = Runtime::from_entry(Path::new(&path))?;
    if rt.effective_root.is_none() {
        return Err(anyhow!(
            "No red.tgsk root detected.\nCreate one via `tagspeak init` in your project root."
        ));
    }
    let _ = rt.eval(&ast)?;
    Ok(())
}

fn build_script(path: &str) -> Result<()> {
    let abs = fs::canonicalize(path)?;
    if abs.extension().and_then(|ext| ext.to_str()).unwrap_or("") != "tgsk" {
        return Err(anyhow!("build expects a .tgsk file"));
    }

    let src = fs::read_to_string(&abs)?;
    router::parse(&src).map_err(anyhow::Error::new)?;

    let rt = Runtime::from_entry(&abs)?;
    let root = rt.effective_root.as_ref().ok_or_else(|| {
        anyhow!("No red.tgsk root detected.\nCreate one via `tagspeak init` in your project root.")
    })?;
    let root_abs = fs::canonicalize(root)?;
    let pretty = root_relative_path(&root_abs, &abs);
    println!("build_ok {}", pretty);
    Ok(())
}

fn root_relative_path(root: &Path, file: &Path) -> String {
    if let Ok(rel) = file.strip_prefix(root) {
        let mut out = String::from("/");
        let mut first = true;
        for part in rel.iter() {
            if !first {
                out.push('/');
            }
            first = false;
            out.push_str(&part.to_string_lossy().replace('\\', "/"));
        }
        if first {
            out.push('.');
        }
        out
    } else {
        file.display().to_string()
    }
}

fn run_help(topic: Option<&str>) -> Result<()> {
    let mut rt = Runtime::new()?;
    let arg = topic
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| Arg::Str(s.to_string()));
    let packet = AstPacket {
        ns: None,
        op: "help".to_string(),
        arg,
        body: None,
    };
    match packets::core::help::handle(&mut rt, &packet)? {
        Value::Str(s) => {
            println!("{s}");
            Ok(())
        }
        other => Err(anyhow!(
            "help packet returned unexpected value: {:?}",
            other
        )),
    }
}

fn lint_script(path: &str) -> Result<()> {
    let abs = fs::canonicalize(path)?;
    if abs.extension().and_then(|ext| ext.to_str()).unwrap_or("") != "tgsk" {
        return Err(anyhow!("lint expects a .tgsk file"));
    }
    let src = fs::read_to_string(&abs)?;
    let mut rt = Runtime::from_entry(&abs)?;
    println!("Linting {}", abs.display());
    rt.last = Value::Str(src);
    let packet = AstPacket {
        ns: None,
        op: "lint".to_string(),
        arg: None,
        body: None,
    };
    match packets::core::lint::handle(&mut rt, &packet)? {
        Value::Str(s) => {
            println!("{s}");
            Ok(())
        }
        other => Err(anyhow!(
            "lint packet returned unexpected value: {:?}",
            other
        )),
    }
}
