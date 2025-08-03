use std::env;
use std::fs;
use std::path::{PathBuf};

mod interpreter;
mod packets;
mod router;



fn main() {
    let args: Vec<String> = env::args().collect();

    let filepath = if args.len() > 1 {
        expand_path(&args[1])
    } else {
        PathBuf::from("examples/test.tgsk")
    };

    match fs::read_to_string(&filepath) {
        Ok(contents) => {
            println!("Running file: {:?}", filepath);
            interpreter::run_lines(contents.lines().collect());

        }
        Err(e) => {
            eprintln!("❌ Failed to read '{}': {}", filepath.display(), e);
            std::process::exit(1);
        }
    }
}

// Expand ~ to home directory on Unix. No-op on Windows.
fn expand_path(input: &str) -> PathBuf {
    if cfg!(unix) && input.starts_with("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(&input[2..]);
        }
    }
    PathBuf::from(input)
}
