#![allow(dead_code, unused_variables, unused_imports)]
use crate::molecule::Molecule;
use anyhow::{anyhow, Context, Error, Result};
use clap::{Parser, Subcommand, ValueEnum};
use serde::Deserialize;
use std::fs::{copy, read_dir, read_to_string, write};
use std::path::PathBuf;
use tera::Tera;
use toml::{map::Map, Table, Value};

mod config;
mod molecule;
mod template;

const CONFIG_NAME: &str = "gedent.toml";
const PRESETS_DIR: &str = "presets";
const TEMPLATES_DIR: &str = "templates";

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    mode: Mode,
}

#[derive(Debug, Subcommand)]
enum Mode {
    /// Generate a new input based on a template and a xyz file
    #[command(alias = "g")]
    Gen {
        /// The template to look for in ~/.config/gedent/templates
        template: String,
        // TODO: Add some common parameters as flags:
        // Solvation, charge, mult, theory level, basis set (what else?)
        // Last arguments are the required xyz files
        // TODO: Make this a flag
        /// xyz files
        #[arg(value_name = "XYZ files")]
        xyz_files: Option<Vec<PathBuf>>,
        /// Sets a custom config file
        #[arg(short, long, value_name = "File")]
        config: Option<PathBuf>,
    },
    // Subcommand to deal with configurations
    /// Access gedent configuration
    #[command(alias = "c")]
    Config {
        #[command(subcommand)]
        config_subcommand: ConfigSubcommand,
    },
    // Subcommand to deal with templates:
    /// Access template functionality
    #[command(alias = "t")]
    Template {
        #[command(subcommand)]
        template_subcommand: TemplateSubcommand,
    },
    // Subcommand for init gedent "repo"
    /// Initiate a gedent project in the current directory.
    Init {
        // optional config to create when initiating the gedent repo
        config: Option<String>,
    },
}

#[derive(Debug, Subcommand)]
enum TemplateSubcommand {
    /// Prints the unformatted template to stdout
    #[command(alias = "p")]
    Print {
        // name of template to search for
        template: String,
    },
    /// Create a new template from a preset located in ~/.config/gedent/presets
    New {
        // Here there will ne an enum which will hold all basic boilerplate
        // templates for a simple singlepoint in the following softwares:
        // ADF, GAMESSUS, GAMESSUK, Gaussian, MOLPRO, NWChem, ORCA
        // also, template will be added in .gedent folder
        software: String,
        template_name: String,
    },
    /// List available templates
    #[command(alias = "l")]
    List {
        // Lists all available templates
        // TODO: decide how to deal with organization in the folder
        // Prints primarely in .gedent available, otherwise falls back to
        // $XDG_CONFIG
    },
    /// Edit a given template
    Edit {
        // opens a given template in $EDITOR
        template: String,
    },
}

#[derive(Debug, Subcommand)]
enum ConfigSubcommand {
    /// Prints the location and the currently used configuration
    #[command(alias = "p")]
    Print {
        /// Print the path of the printed config.
        #[arg(short, long, default_value_t = false)]
        location: bool,
    },
    /// Sets key to value in the config file, keeps the same type as was setted.
    Set {
        /// Key to be added
        key: String,
        /// Value associated with key
        value: String,
    },
    /// Adds a key, value to the config file, for typed values use an option
    Add {
        /// Key to be added
        key: String,
        /// Value associated with key, can be a string, int, float or bool. Default is string.
        value: String,
        /// Sets the type of the value in the config file
        #[arg(short, long, default_value = "string")]
        toml_type: crate::config::ArgType,
    },
    /// Deletes a certain key in the configuration
    Del {
        /// Key to be deleted.
        key: String,
    },
    /// Opens the currently used config file in your default editor.
    #[command(alias = "e")]
    Edit {},
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.mode {
        Mode::Gen {
            template,
            xyz_files,
            config,
        } => {
            // for now just call fn to generate template
            generate_template(template, xyz_files, config)?
        }

        Mode::Config { config_subcommand } => match config_subcommand {
            ConfigSubcommand::Print { location } => print_config(location)?,
            ConfigSubcommand::Set { key, value } => {
                let config_path = get_config_path()?;
                let config = load_config(&config_path)?;
                let config = set_config(key, value, config)?;
                write_config(config_path, config)?
            }
            ConfigSubcommand::Add {
                key,
                value,
                toml_type,
            } => {
                let config_path = get_config_path()?;
                let config = load_config(&config_path)?;
                let config = add_config(key, value, toml_type, config)?;
                write_config(config_path, config)?
            }
            ConfigSubcommand::Del { key } => {
                let config_path = get_config_path()?;
                let config = load_config(&config_path)?;
                let config = delete_config(key, config)?;
                write_config(config_path, config)?
            }
            ConfigSubcommand::Edit {} => edit_config()?,
        },

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

        Mode::Init { config } => gedent_init(config)?,
    };

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verify_cli() {
        use clap::CommandFactory;

        Cli::command().debug_assert();
    }
}
