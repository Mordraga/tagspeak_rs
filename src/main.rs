mod interpreter;
mod kernel;
mod packets;
mod router;

use anyhow::{Result, anyhow};
use kernel::Runtime;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

fn main() -> Result<()> {
    // Simple CLI: `tagspeak init [dir]` or `tagspeak <file.tgsk>`
    let mut args = env::args();
    let _exe = args.next();
    match args.next() {
        Some(cmd) if cmd == "init" => {
            let dir = args.next();
            init_red(dir.as_deref())
        }
        Some(path) => run_script(&path),
        None => {
            // no args: guide the user
            eprintln!(
                "No input file provided. Usage:\n  tagspeak init [dir]\n  tagspeak <file.tgsk>"
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
    let ast = router::parse(&src)?;
    let mut rt = Runtime::from_entry(Path::new(&path))?;
    if rt.effective_root.is_none() {
        return Err(anyhow!(
            "No red.tgsk root detected.\nCreate one via `tagspeak init` in your project root."
        ));
    }
    let _ = rt.eval(&ast)?;
    Ok(())
}
