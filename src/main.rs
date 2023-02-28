use anyhow::{Context, Result};
use clap::Parser;

/// Search for a pattern in a file and display the lines that contain it.
#[derive(Parser)]
struct Cli {
    /// The template to look for in ~/.config/gedent/templates
    template: String,
    /// The path to the xyz file to read
    path: std::path::PathBuf,
}

fn main() -> Result<()> {
    let args = Cli::parse();
    let content = std::fs::read_to_string(&args.path)
        .with_context(|| format!("could not read file: {}", &args.path.display()))?;
    for line in content.lines() {
        println!("{}", line)
    }
    Ok(())
}
