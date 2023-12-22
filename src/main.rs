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
    // Generate a new input based on a template and a xyz file
    Gen {
        // The template to look for in ~/.config/gedent/templates
        template: String,
        // Add some common parameters as flags (maybe?)
        // Last arguments are the required xyz files
        // Can i make this a flag maybe? -xyz
        #[arg(last = true)]
        opt_args: Vec<String>,
    },
    // Subcommand to deal with configurations
    // set, where, add. remove, get inspiration in gh
    Config {},
    // Subcommand to deal with templates:
    // list, print, edit
    // Subcommand for init gedent "repo"
    // Init {},
}

// main logic goes here
fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.mode {
        Mode::Gen { template, opt_args } => generate_template(template, opt_args)?,
        Mode::Config {} => {
            println!("Placeholder, subcommand to be added");
        }
    };

    Ok(())
}
