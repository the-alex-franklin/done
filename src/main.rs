mod runtime;
mod transpiler;

use std::{env, fs};

fn main() -> anyhow::Result<()> {
    let path = env::args()
        .nth(1)
        .ok_or_else(|| anyhow::anyhow!("Usage: done <file.ts>"))?;

    let source = fs::read_to_string(&path)
        .map_err(|e| anyhow::anyhow!("Failed to read {path}: {e}"))?;

    let js = transpiler::transpile(&source)?;
    if env::var("DONE_PRINT_JS").is_ok() {
        println!("{js}");
        return Ok(());
    }
    runtime::run(&js)?;

    Ok(())
}
