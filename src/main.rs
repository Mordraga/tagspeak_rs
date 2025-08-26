mod interpreter;
mod kernel;
mod packets;
mod router;

use kernel::Runtime;
use std::env;
use std::fs;
use std::path::Path;

fn main() -> anyhow::Result<()> {
    // path arg or default
    let path = env::args()
        .nth(1)
        .unwrap_or_else(|| "examples/smoke.tgsk".to_string());
    println!("Running file: {}", &path);
    let src = fs::read_to_string(&path)?;

    let ast = router::parse(&src)?;
    let mut rt = Runtime::from_entry(Path::new(&path))?;
    let _ = rt.eval(&ast)?;
    Ok(())
}
