#![allow(unused_variables, unused_imports)]
use anyhow::{Context, Result};
use clap::{Args, Parser, Subcommand, ValueEnum};
use gedent::get_config;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    mode: Mode,
    /// Verbosity options.
    #[clap(flatten)]
    verbosity: clap_verbosity_flag::Verbosity,
}

#[derive(Debug, Subcommand)]
enum Mode {
    ///Generate a new input based on a template and a xyz file
    Gen {
        /// The template to look for in ~/.config/gedent/templates
        template: String,
        /// xyzfile to be used for structure
        xyz_file: std::path::PathBuf,
    },
    /// prints the current configurations as well as the location of the config file
    Config {},
    /// generates a new template for a jobfile specified for a certain software
    /// args are the software (plans are to support orca, gaussian and ADF initially)
    New {},
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.mode {
        Mode::Init { config_file } => {
            init();
        }
        Mode::Gen { template, xyz_file } => {
            gen_template();
        }
        Mode::New {} => {
            new_template();
        }
        Mode::Config {} => {
            // TODO: find out how to read from a specific directory (should this be so hard? lol
            let path = std::path::Path::new("gedent.toml");
            get_config(path)?
        }
    };

    Ok(())
}

fn init() {
    println!("init")
}

fn gen_template() {
    println!("generating input")
}

fn new_template() {
    println!("generating new template")
}
