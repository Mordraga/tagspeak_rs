use std::env;
use std::path::PathBuf;

mod interpreter;
mod router;
mod packets;

fn main() {
    // Get file path from args or use default
    let args: Vec<String> = env::args().collect();
    let filepath = if args.len() > 1 {
        expand_path(&args[1])
    } else {
        PathBuf::from("examples/test.tgsk")
    };

    println!("Running file: {:?}", filepath);

    // Pass file to interpreter
    if let Err(e) = interpreter::run_file(filepath) {
        eprintln!("❌ {}", e);
        std::process::exit(1);
    }
}

// Expand ~ to home directory on Unix, no-op on Windows
fn expand_path(input: &str) -> PathBuf {
    if cfg!(unix) && input.starts_with("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(&input[2..]);
        }
    }
    PathBuf::from(input)
}
