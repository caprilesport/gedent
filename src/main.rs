#![allow(unused_variables, unused_imports)]
use anyhow::{Context, Result};
use clap::{Args, Parser, Subcommand, ValueEnum};
use gedent::generate_template;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    mode: Mode,
    // Verbosity options.
    #[clap(flatten)]
    verbosity: clap_verbosity_flag::Verbosity,
}

#[derive(Debug, Subcommand)]
enum Mode {
    ///Generate a new input based on a template and a xyz file
    Gen {
        /// The template to look for in ~/.config/gedent/templates
        template: String,
        //last arguments may be optional
        #[arg(last = true)]
        opt_args: Vec<String>,
    },
    /// prints the current configurations as well as the location of the config file
    Config {},
}

fn gen_template(template: String, opts: Vec<String>) -> Result<()> {
    println!(
        "generating input with template {} and extra args {:?}",
        template, opts
    );
    Ok(())
}

// main logic goes here
fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.mode {
        Mode::Gen { template, opt_args } => gen_template(template, opt_args)?,
        Mode::Config {} => {
            // TODO: find out how to read from a specific directory (should this be so hard? lol
            let path = std::path::Path::new("gedent.toml");
            get_config(path)?
        }
    };

    Ok(())
}
