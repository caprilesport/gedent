#![allow(unused_variables, unused_imports)]
use anyhow::{anyhow, Context, Error, Result};
use clap::{Parser, Subcommand, ValueEnum};
use std::fs;
use std::fs::create_dir;
use std::path::PathBuf;
use tera::Tera;
use toml::{Table, Value};

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
        #[arg(last = true)]
        xyz_files: Vec<String>,
        /// Sets a custom config file
        #[arg(short, long, value_name = "FILE")]
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
    /// Initiate a gedent repository with config cloned from ~/.config/gedent
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
    Print {},
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
        #[arg(short, long)]
        type_of_value: ArgType,
    },
    /// Deletes a certain key in the configuration
    Del {
        /// Key to be deleted.
        key: String,
    },
    /// Opens the config file in your default editor.
    Edit {},
}

#[derive(Clone, Debug, Default, ValueEnum)]
enum ArgType {
    #[default]
    String,
    Float,
    Int,
    Bool,
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
            ConfigSubcommand::Print {} => {}
            ConfigSubcommand::Set { key, value } => {}
            ConfigSubcommand::Add {
                key,
                value,
                type_of_value,
            } => {}
            ConfigSubcommand::Del { key } => {}
            ConfigSubcommand::Edit {} => {}
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

// Config functionality
fn parse_config(config_path: PathBuf) -> Result<toml::map::Map<String, Value>, anyhow::Error> {
    let config_file = std::fs::read_to_string(&config_path)
        .context(format!("Cant open config {:?}", config_path))?;
    let config: Table = config_file.parse()?;
    Ok(config)
}

// this function should search for .gedent, if it doesnt find look for gedent home
// then parse the config and return it
// so other functions that need the config just call this function
fn get_config() -> Result<toml::map::Map<String, Value>, anyhow::Error> {
    let config_file = String::from("gedent.toml");
    let mut config_dir = get_config_dir()?;
    config_dir.push(config_file);
    let config = parse_config(config_dir)?;
    Ok(config)
}

// TODO: implement git-like functionality
fn get_config_dir() -> Result<PathBuf, Error> {
    let gedent_home = get_gedent_home()?;
    Ok(gedent_home)
}

fn get_gedent_home() -> Result<PathBuf, Error> {
    // TODO: make this system agnostic in the future - only works in linux
    // I saw a dir crate that may help
    // https://docs.rs/dirs/latest/dirs/fn.config_dir.html
    let mut gedent_home = std::path::PathBuf::new();
    let home_dir = std::env::var_os("HOME").ok_or(anyhow!("Error fetching home directory"))?;
    gedent_home.push(home_dir);
    gedent_home.push(String::from(".config/gedent"));
    Ok(gedent_home)
}

// Template functionality
fn generate_template(
    template: String,
    options: Vec<String>,
    config: Option<PathBuf>,
) -> Result<(), Error> {
    let cfg = get_config()?;
    let mut context = tera::Context::new();

    // Surprisingly, for me at least, passing toml::Value already works
    // when using the typed values in TERA templates.
    for (key, value) in cfg {
        context.insert(key, &value);
    }

    // TODO: parse template to see if xyz file is needed

    let result = render_template(template, context)?;
    print!("{}", &result);
    Ok(())
}

fn edit_template(template: String) -> Result<(), Error> {
    let template_path = get_template_path(template)?;
    // The edit crate makes this work in all platforms.
    edit::edit_file(template_path)?;
    Ok(())
}

fn print_template(template: String) -> Result<(), Error> {
    let template_path = get_template_path(template)?;
    let template = std::fs::read_to_string(&template_path)
        .context(format!("Cant find template {:?}", template_path))?;
    println!("{}", &template);
    Ok(())
}

// Basic logic is correct, would be nice if the user could set where these
// directories are.
fn new_template(software: String, template_name: String) -> Result<(), Error> {
    let mut boilerplate = get_gedent_home()?;
    let mut template_path = boilerplate.clone();
    template_path.push(String::from("templates"));
    template_path.push(template_name);
    boilerplate.push(String::from("presets"));
    boilerplate.push(software);
    fs::copy(&boilerplate, &template_path)
        .context(format!("Cant open base {:?} template.", &boilerplate))?;
    edit::edit_file(template_path).context("Cant open editor.")?;
    Ok(())
}

fn list_templates() -> Result<(), Error> {
    let mut gedent_home = get_gedent_home()?;
    gedent_home.push(String::from("templates"));
    // +1 is here to remove the first slash
    let gedent_home_len = gedent_home
        .to_str()
        .ok_or(anyhow!("Cant retrieve gedent home len"))?
        .len()
        + 1;
    for entry in fs::read_dir(gedent_home)? {
        print_descent_dir(entry.as_ref().unwrap().path(), gedent_home_len)?;
    }
    Ok(())
}

fn print_descent_dir(entry: PathBuf, gedent_home_len: usize) -> Result<(), Error> {
    if entry.is_dir() {
        let new_dir = fs::read_dir(entry)?;
        for new_entry in new_dir {
            let _ = print_descent_dir(new_entry.as_ref().unwrap().path(), gedent_home_len)?;
        }
        Ok(())
    } else {
        println!("{}", &entry.to_str().unwrap()[gedent_home_len..]);
        Ok(())
    }
}

fn get_template_path(template: String) -> Result<PathBuf, Error> {
    let mut template_path = get_gedent_home()?;
    template_path.push(String::from("templates"));
    template_path.push(template);
    Ok(template_path)
}

fn render_template(template_name: String, context: tera::Context) -> Result<String, Error> {
    let template_path = get_template_path(template_name)?;
    let template = fs::read_to_string(&template_path)
        .context(format!("Cant find template {:?}", template_path))?;
    let result = Tera::one_off(&template, &context, true).context("Failed to render template.")?;
    Ok(result)
}

fn gedent_init(config: Option<String>) -> Result<(), Error> {
    let mut config_path = PathBuf::new();
    match config {
        Some(file) => config_path.push(file),
        None => {
            config_path.push(get_gedent_home()?);
            config_path.push(String::from("gedent.toml"));
        }
    };

    if std::path::Path::try_exists(PathBuf::from(&".gedent").as_path())? {
        anyhow::bail!(".gedent already exists, exiting...");
    }

    let mut gedent = PathBuf::from(&".gedent");
    let mut templates = gedent.clone();
    templates.push("templates");
    create_dir(&gedent)?;
    create_dir(&templates)?;
    gedent.push(String::from("gedent.toml"));
    std::fs::copy(config_path, gedent)?;

    Ok(())
}
