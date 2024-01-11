#![allow(dead_code, unused_variables, unused_imports)]
use crate::config::Config;
use crate::molecule::Molecule;
use crate::template::{edit_template, list_templates, new_template, print_template};
use anyhow::{anyhow, Context, Error, Result};
use clap::{Parser, Subcommand};
use serde::Deserialize;
use std::fs::{copy, read_dir, read_to_string, write, File};
use std::path::{Path, PathBuf};
use template::Template;
use tera::Tera;

mod config;
mod molecule;
mod template;

#[derive(Debug)]
struct Input {
    filename: PathBuf,
    content: String,
}

impl Input {
    fn write(self) -> Result<(), Error> {
        write(&self.filename, &self.content).context(anyhow!("Failed to save input."))
    }
}

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
        template_name: String,
        // TODO: Add some common parameters as flags:
        // Solvation, charge, mult, theory level, basis set (what else?)
        // Last arguments are the required xyz files
        // TODO: Make this a flag
        /// xyz files
        #[arg(value_name = "XYZ files")]
        xyz_files: Option<Vec<PathBuf>>,
        #[arg(short, long, default_value_t = false)]
        print: bool,
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
        config: Option<PathBuf>,
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
            template_name,
            xyz_files,
            print,
        } => {
            let mut molecules: Vec<Molecule> = vec![];
            match xyz_files {
                Some(files) => {
                    for file in files {
                        molecules = [molecules, Molecule::from_xyz(file)?].concat();
                    }
                }
                None => (),
            };
            let template = Template::get(template_name)?;
            let results = generate_input(template, molecules)?;
            for input in results {
                if print {
                    println!("{}", input.content);
                } else {
                    input.write()?;
                }
            }
        }

        Mode::Config { config_subcommand } => match config_subcommand {
            ConfigSubcommand::Print { location } => {
                let config = Config::get()?;
                config.print(location)?
            }
            ConfigSubcommand::Set { key, value } => {
                let mut config = Config::get()?;
                config.set(key, value)?;
                config.write()?;
            }
            ConfigSubcommand::Add {
                key,
                value,
                toml_type,
            } => {
                let mut config = Config::get()?;
                config.add(key, value, toml_type)?;
                config.write()?;
            }
            ConfigSubcommand::Del { key } => {
                let mut config = Config::get()?;
                config.delete(key)?;
                config.write()?;
            }
            ConfigSubcommand::Edit {} => Config::edit()?,
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

//Search for paths
fn get_gedent_home() -> Result<PathBuf, Error> {
    let home_dir = std::env::var_os("HOME").ok_or(anyhow!("Error fetching home directory"))?;
    // TODO: make this system agnostic in the future - only works in linux
    // I saw a dir crate that may help
    // https://docs.rs/dirs/latest/dirs/fn.config_dir.html
    let gedent_home: PathBuf = [home_dir, Into::into(".config/gedent")].iter().collect();
    Ok(gedent_home)
}

// copy the specified or currently used config to cwd
fn gedent_init(config: Option<PathBuf>) -> Result<(), Error> {
    let config_path = match config {
        Some(file) => file,
        None => Config::get_path()?,
    };

    if std::path::Path::try_exists(&PathBuf::from("./gedent.toml"))? {
        anyhow::bail!("gedent.toml already exists, exiting...");
    }

    copy(config_path, "./gedent.toml")?;
    Ok(())
}

fn generate_input(template: Template, molecules: Vec<Molecule>) -> Result<Vec<Input>, Error> {
    let mut context = tera::Context::new();
    let config = Config::get()?;
    for (key, value) in config.parameters {
        context.insert(key, &value);
    }

    let extension = match &template.options.extension {
        Some(ext) => ext,
        None => &config.gedent.default_extension,
    };

    let mut results: Vec<Input> = vec![];

    if molecules.is_empty() {
        results.push(Input {
            filename: PathBuf::from(&template.name).with_extension(&extension),
            content: template.render(&context)?,
        });
    }

    for molecule in molecules {
        let mut mol_context = context.clone();
        mol_context.insert("molecule", &molecule);
        results.push(Input {
            filename: PathBuf::from(molecule.filename).with_extension(&extension),
            content: template.render(&mol_context)?,
        });
    }

    Ok(results)
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
