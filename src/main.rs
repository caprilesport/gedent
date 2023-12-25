#![allow(unused_variables, unused_imports)]
use anyhow::{Context, Result};
use clap::{Args, Parser, Subcommand, ValueEnum};
use gedent::{edit_template, generate_template, list_templates, new_template, print_template};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    mode: Mode,
    // #[clap(flatten)]
    // verbosity: clap_verbosity_flag::Verbosity,
}

#[derive(Debug, Subcommand)]
enum Mode {
    // Generate a new input based on a template and a xyz file
    Gen {
        // The template to look for in ~/.config/gedent/templates
        template: String,
        // TODO: Add some common parameters as flags:
        // Solvation, charge, mult, theory level, basis set (what else?)
        // Last arguments are the required xyz files
        // TODO: Make this a flag
        #[arg(last = true)]
        xyz_files: Vec<String>,
    },
    // Subcommand to deal with configurations
    // set, where, add, remove, get inspiration in gh
    // Config {},
    // Subcommand to deal with templates:
    Template {
        #[command(subcommand)]
        template_subcommand: TemplateSubcommand,
    },
    // Subcommand for init gedent "repo"
    Init {},
}

#[derive(Debug, Subcommand)]
enum TemplateSubcommand {
    // Prints the unformatted template to stdout
    Print {
        // name of template to search for
        template: String,
    },
    New {
        // Here there will ne an enum which will hold all basic boilerplate
        // templates for a simple singlepoint in the following softwares:
        // ADF, GAMESSUS, GAMESSUK, Gaussian, MOLPRO, NWChem, ORCA
        // also, template will be added in .gedent folder
        software: String,
    },
    List {
        // Lists all available templates
        // TODO: decide how to deal with organization in the folder
        // Prints primarely in .gedent available, otherwise falls back to
        // $XDG_CONFIG
    },
    Edit {
        // opens a given template in $EDITOR
        template: String,
    },
}

// main logic goes here
fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.mode {
        Mode::Gen {
            template,
            xyz_files,
        } => {
            // for now just call fn to generate template
            generate_template(template, xyz_files)?
        }
        // Mode::Config {} => {
        //     println!("Config placeholder, subcommand to be added");
        // }
        Mode::Template {
            template_subcommand,
        } => match template_subcommand {
            TemplateSubcommand::Print { template } => print_template(template)?,
            TemplateSubcommand::New {
                software,
                template_name,
            } => new_template(software, template_name)?,
            TemplateSubcommand::List {} => list_templates()?,
            TemplateSubcommand::Edit { template } => edit_template(template)?,
        },
        Mode::Init {} => {
            println!("Init placeholder, function to be added");
        }
    };

    Ok(())
}
