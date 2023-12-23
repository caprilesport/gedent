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
    Template {
        #[command(subcommand)]
        template_subcommand: TemplateSubcommand,
    },
    // list, print, edit
    // Subcommand for init gedent "repo"
    Init {},
}

#[derive(Debug, Subcommand)]
enum TemplateSubcommand {
    Print { template: String },
    New {},
    List {},
    Edit {},
}

// main logic goes here
fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.mode {
        Mode::Gen { template, opt_args } => generate_template(template, opt_args)?,
        Mode::Config {} => {
            println!("Config placeholder, subcommand to be added");
        }
        Mode::Template {
            template_subcommand,
        } => match template_subcommand {
            TemplateSubcommand::New {} => {
                println!("template new");
            }
            TemplateSubcommand::List {} => {
                println!("Template list")
            }
            TemplateSubcommand::Edit {} => {
                println!("Template edit")
            }
            TemplateSubcommand::Print { template } => {
                println!("Template print {}", template)
            }
        },
        Mode::Init {} => {
            println!("Init placeholder, subcommand to be added");
        }
    };

    Ok(())
}
