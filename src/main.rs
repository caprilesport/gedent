#![allow(unused_variables, unused_imports)]
use anyhow::{anyhow, Context, Error, Result};
use clap::{Parser, Subcommand, ValueEnum};
use std::fs::{copy, create_dir, read_dir, read_to_string, write};
use std::path::PathBuf;
use tera::Tera;
use toml::{Table, Value};

const CONFIG_NAME: &str = "gedent.toml";
const DIR_NAME: &str = ".gedent";
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
        #[arg(short, long)]
        type_of_value: ArgType,
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
            ConfigSubcommand::Print { location } => print_config(location)?,
            ConfigSubcommand::Set { key, value } => set_config(key, value)?,
            ConfigSubcommand::Add {
                key,
                value,
                type_of_value,
            } => add_config(key, value, type_of_value)?,
            ConfigSubcommand::Del { key } => delete_config(key)?,
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

//Search for paths
fn get_gedent_home() -> Result<PathBuf, Error> {
    let home_dir = std::env::var_os("HOME").ok_or(anyhow!("Error fetching home directory"))?;
    // TODO: make this system agnostic in the future - only works in linux
    // I saw a dir crate that may help
    // https://docs.rs/dirs/latest/dirs/fn.config_dir.html
    let gedent_home: PathBuf = [home_dir, Into::into(".config/gedent")].iter().collect();
    Ok(gedent_home)
}

// git-like search, stop if .gedent folder is found or if
// parent_folder = current_folder
fn find_gedent_folder(dir: PathBuf) -> Result<PathBuf, Error> {
    let mut gedent = dir.clone();
    gedent.push(DIR_NAME);

    if std::path::Path::try_exists(&gedent)? {
        return Ok(gedent);
    } else {
        let parent_folder = dir.parent();
        match parent_folder {
            Some(parent) => return Ok(find_gedent_folder(parent.to_path_buf())?),
            None => return Ok(get_gedent_home()?),
        };
    }
}

// Config functionality
fn parse_config(config_path: &PathBuf) -> Result<toml::map::Map<String, Value>, anyhow::Error> {
    let config_file =
        read_to_string(&config_path).context(format!("Cant open config {:?}", config_path))?;
    let config: Table = config_file.parse()?;
    Ok(config)
}

fn write_config(config_path: PathBuf, config: toml::map::Map<String, Value>) -> Result<(), Error> {
    write(&config_path, config.to_string())?;
    println!("Config wrote to {:?}.", config_path);
    Ok(())
}

fn get_config_path() -> Result<PathBuf, Error> {
    let current_dir = std::env::current_dir()?;
    let config = PathBuf::from(CONFIG_NAME);
    Ok([find_gedent_folder(current_dir)?, config].iter().collect())
}

fn delete_config(key: String) -> Result<(), Error> {
    let config_path = get_config_path()?;
    let mut config = parse_config(&config_path)?;
    config.remove(&key);
    println!("Removed key {}.", key);
    write_config(config_path, config)?;
    Ok(())
}

fn add_config(key: String, value: String, type_of_value: ArgType) -> Result<(), Error> {
    let config_path = get_config_path()?;
    let mut config = parse_config(&config_path)?;

    if config.contains_key(&key) {
        anyhow::bail!(format!("Config already contains {}, exiting.", key));
    }

    println!(
        "Setting config {} to {} with argtype {:?}",
        key, value, type_of_value
    );

    match type_of_value {
        ArgType::Int => {
            config.insert(key, Value::Integer(value.parse::<i64>()?));
        }
        ArgType::Bool => {
            config.insert(key, Value::Boolean(value.parse::<bool>()?));
        }
        ArgType::Float => {
            config.insert(key, Value::Float(value.parse::<f64>()?));
        }
        ArgType::String => {
            config.insert(key, Value::String(value));
        }
    }

    write_config(config_path, config)?;
    Ok(())
}

fn set_config(key: String, value: String) -> Result<(), Error> {
    let config_path = get_config_path()?;
    let mut config = parse_config(&config_path)?;
    let current_value = config
        .get(&key)
        .ok_or(anyhow!("Cant find {} in config.", key))?;

    println!(
        "Changing config {}, from {} to {}.",
        key, current_value, value
    );

    match current_value {
        Value::String(_current_value) => {
            config[&key] = Value::String(value);
        }
        Value::Float(_current_value) => {
            config[&key] = Value::Float(value.parse::<f64>()?);
        }
        Value::Integer(_current_value) => {
            config[&key] = Value::Integer(value.parse::<i64>()?);
        }
        Value::Boolean(_current_value) => {
            config[&key] = Value::Boolean(value.parse::<bool>()?);
        }
        _ => anyhow::bail!("Unsupported type"),
    }

    write_config(config_path, config)?;
    Ok(())
}

fn edit_config() -> Result<(), Error> {
    let config_path = get_config_path()?;
    edit::edit_file(config_path)?;
    Ok(())
}

fn print_config(location: bool) -> Result<(), Error> {
    let config_path = get_config_path()?;
    let config = read_to_string(&config_path)?;
    if location {
        println!("Printing config from {:?}", config_path);
    }
    print!("{}", config);
    Ok(())
}

// Template functionality
fn generate_template(
    template: String,
    options: Vec<String>,
    config: Option<PathBuf>,
) -> Result<(), Error> {
    let config_path = get_config_path()?;
    let config = parse_config(&config_path)?;
    let mut context = tera::Context::new();

    // Surprisingly, for me at least, passing toml::Value already works
    // out of the box when using the typed values in TERA templates.
    for (key, value) in config {
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
    let template = read_to_string(&template_path)
        .context(format!("Cant find template {:?}", template_path))?;
    println!("{}", &template);
    Ok(())
}

// Basic logic is correct, would be nice if the user could set where these
// directories are.
fn new_template(software: String, template_name: String) -> Result<(), Error> {
    let gedent_home = get_gedent_home()?;
    let template_path: PathBuf = [
        gedent_home.clone(),
        Into::into(TEMPLATES_DIR),
        Into::into(&template_name),
    ]
    .iter()
    .collect();
    let boilerplate: PathBuf = [gedent_home, Into::into(PRESETS_DIR), Into::into(software)]
        .iter()
        .collect();
    copy(&boilerplate, &template_path)
        .context(format!("Cant open base {:?} template.", &boilerplate))?;
    edit::edit_file(template_path).context("Cant open editor.")?;
    Ok(())
}

fn list_templates() -> Result<(), Error> {
    let gedent_home: PathBuf = [get_gedent_home()?, Into::into(TEMPLATES_DIR)]
        .iter()
        .collect();
    // +1 is here to remove the first slash
    let gedent_home_len = gedent_home
        .to_str()
        .ok_or(anyhow!("Cant retrieve gedent home len"))?
        .len()
        + 1;
    for entry in read_dir(gedent_home)? {
        print_descent_dir(entry.as_ref().unwrap().path(), gedent_home_len)?;
    }
    Ok(())
}

fn print_descent_dir(entry: PathBuf, gedent_home_len: usize) -> Result<(), Error> {
    if entry.is_dir() {
        let new_dir = read_dir(entry)?;
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
    let template_path: PathBuf = [
        get_gedent_home()?,
        Into::into(TEMPLATES_DIR),
        Into::into(template),
    ]
    .iter()
    .collect();
    Ok(template_path)
}

fn render_template(template_name: String, context: tera::Context) -> Result<String, Error> {
    let template_path = get_template_path(template_name)?;
    let template = read_to_string(&template_path)
        .context(format!("Cant find template {:?}", template_path))?;
    let result = Tera::one_off(&template, &context, true).context("Failed to render template.")?;
    Ok(result)
}

// There may be a better way to write this?
fn gedent_init(config: Option<String>) -> Result<(), Error> {
    let mut config_path = PathBuf::new();
    match config {
        Some(file) => config_path.push(file),
        None => {
            config_path.push(get_gedent_home()?);
            config_path.push(CONFIG_NAME);
        }
    };

    let mut gedent = PathBuf::from(DIR_NAME);

    if std::path::Path::try_exists(&gedent)? {
        anyhow::bail!(".gedent already exists, exiting...");
    }

    let mut templates = gedent.clone();
    templates.push(TEMPLATES_DIR);
    create_dir(&gedent)?;
    create_dir(&templates)?;
    gedent.push(CONFIG_NAME);
    copy(config_path, gedent)?;
    Ok(())
}
