use crate::config::Config;
use crate::molecule::Molecule;
use crate::template::Template;
use include_dir::{include_dir, Dir};
use anyhow::{anyhow, Context, Error, Result};
use clap::{Command, CommandFactory, Parser, Subcommand};
use clap_complete::{Shell, generate, Generator};
use dialoguer::{theme::ColorfulTheme, FuzzySelect};
use std::fs::{copy, read_dir, write};
use std::path::PathBuf;
use std::io;

mod config;
mod molecule;
mod template;

const PRESETS_DIR: &str = "presets";
const TEMPLATES_DIR: &str = "templates";

static INCLUDE_PRESETS_DIR: Dir = include_dir!("presets");
static INCLUDE_TEMPLATES_DIR: Dir = include_dir!("templates");
static GEDENT_CONFIG: &str = include_str!("../gedent.toml");


#[derive(Debug)]
struct Input {
    filename: PathBuf,
    content: String,
}

impl Input {
    fn write(self) -> Result<(), Error> {
        println!("Writing {:?}", &self.filename);
        write(&self.filename, &self.content).context(anyhow!("Failed to save input."))
    }
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
#[command(arg_required_else_help = true)]
struct Cli {
    #[command(subcommand)]
    mode: Option<Mode>,
    /// Check if gedent is set up correctly.
    #[arg(long, default_value = None)]
    health: bool,
    /// Set up gedent configuration directory.
    // Bare, presets and full can be passed to create a bare directory with just the config, Presets create the config and the 
    /// presets, full creates the directory with templates 
    #[arg(long, default_value = None)]
    set_up: bool,
    // If provided, outputs the completion file for given shell
    #[arg(long = "generate", value_enum)]
    generator: Option<Shell>,
}

#[derive(Debug, Subcommand)]
enum Mode {
    /// Generate a new input based on a template
    Gen {
        /// The template to look for in ~/.config/gedent/templates
        template_name: String,
        /// xyz files
        #[arg(value_name = "XYZ files")]
        xyz_files: Option<Vec<PathBuf>>,
        /// Print to screen and don't save file
        #[arg(short, long, default_value_t = false)]
        print: bool,
        /// Set method
        #[arg(long, default_value = None)]
        method: Option<String>,
        /// Set basis_set
        #[arg(long, default_value = None)]
        basis_set: Option<String>,
        /// Set dispersion
        #[arg(long, default_value = None)]
        dispersion: Option<String>,
        /// Set solvent to value and solvation to true
        #[arg(short, long, default_value = None)]
        solvent: Option<Option<String>>,
        /// Set solvation_model
        #[arg(long, default_value = None)]
        solvation_model: Option<String>,
        /// Set charge
        #[arg(short, long, default_value = None)]
        charge: Option<usize>,
        /// Set hessian
        #[arg(long, default_value_t = false)]
        hessian: bool,
        #[arg(short, long, default_value = None)]
        /// Set mult
        #[arg(short, long, default_value = None)]
        mult: Option<usize>,
        /// Set nprocs
        #[arg(long, default_value = None)]
        nprocs: Option<usize>,
        /// Set mem
        #[arg(long, default_value = None)]
        mem: Option<usize>,
        /// Set split_index
        #[arg(long, default_value = None)]
        split_index: Option<usize>,
    },
    // Subcommand to deal with configurations
    /// Access gedent configuration
    Config {
        #[command(subcommand)]
        config_subcommand: ConfigSubcommand,
    },
    // Subcommand to deal with templates:
    /// Access template functionality
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
    Print {
        // name of template to search for
        template: Option<String>,
    },
    /// Create a new template from a preset located in ~/.config/gedent/presets
    New {
        // Here there will ne an enum which will hold all basic boilerplate
        // templates for a simple singlepoint in the following softwares:
        // ADF, GAMESSUS, GAMESSUK, Gaussian, MOLPRO, NWChem, ORCA
        // also, template will be added in .gedent folder
        template_name: String,
        software: Option<String>,
    },
    /// List available templates
    List {
        // Lists all available templates
        // TODO: decide how to deal with organization in the folder
        // Prints primarely in .gedent available, otherwise falls back to
        // $XDG_CONFIG
    },
    /// Edit a given template
    Edit {
        // opens a given template in $EDITOR
        template: Option<String>,
    },
}

#[derive(Debug, Subcommand)]
enum ConfigSubcommand {
    /// Prints the location and the currently used configuration
    Print {
        /// Print the path of the printed config.
        #[arg(short, long, default_value_t = false)]
        location: bool,
    },
    /// Sets key to value in the config file, keeps the same type as was setted.
    Set {
        /// Key to be added
        key: Option<String>,
        /// Value associated with key
        value: Option<String>,
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
        key: Option<String>,
    },
    /// Opens the currently used config file in your default editor.
    Edit {},
}

    

fn main() -> Result<()> {
    let cli = Cli::parse();

    if let Some(generator) = cli.generator {
        let mut cmd = Cli::command();
        eprintln!("Generating completion file for {generator:?}...");
        print_completions(generator, &mut cmd);
    } 

    if cli.health {
        check_gedent_health()?;
    }

    if cli.set_up {
        setup_gedent()?;
    }

    if let Some(mode) = cli.mode {
        match mode {
            Mode::Gen {
                template_name,
                xyz_files,
                print,
                method,
                basis_set,
                dispersion,
                solvent,
                solvation_model,
                charge,
                hessian,
                mult,
                nprocs,
                mem,
                split_index,
            } => {
                let mut molecules: Vec<Molecule> = vec![];
                if let Some(files) = xyz_files {
                    for file in files {
                        molecules = [molecules, Molecule::from_xyz(file)?].concat();
                    }
                };
                let template = Template::get(template_name)?;
                let results = generate_input(
                    template,
                    molecules,
                    solvent,
                    mult,
                    charge,
                    method,
                    basis_set,
                    dispersion,
                    solvation_model,
                    hessian,
                    nprocs,
                    mem,
                    split_index,
                )?;
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
                    let key = match key {
                        Some(key) => key,
                        None => select_key(&config)?,
                    };
                    let value = match value {
                        Some(val) => val,
                        None => dialoguer::Input::with_theme(&ColorfulTheme::default())
                            .with_prompt(format!("Set {} to:", key))
                            .interact_text()
                            .unwrap(),
                    };
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
                    let key = match key {
                        Some(key) => key,
                        None => select_key(&config)?,
                    };
                    config.delete(key)?;
                    config.write()?;
                }
                ConfigSubcommand::Edit {} => Config::edit()?,
            },

            Mode::Template {
                template_subcommand,
            } => match template_subcommand {
                TemplateSubcommand::Print { template } => {
                    let template = match template {
                        Some(templ) => templ,
                        None => select_template()?,
                    };
                    Template::print_template(template)?
                }
                TemplateSubcommand::New {
                    software,
                    template_name,
                } => {
                    let software = match software {
                        Some(software) => software,
                        None => select_software()?,
                    };
                    Template::from_preset(software, template_name)?
                }
                TemplateSubcommand::List {} => Template::list_templates()?,
                TemplateSubcommand::Edit { template } => {
                    let template = match template {
                        Some(template) => template,
                        None => select_template()?,
                    };
                    Template::edit_template(template)?
                }
            },

            Mode::Init { config } => gedent_init(config)?,
        }
    };

    Ok(())
}

fn print_completions<G: Generator>(gen: G, cmd: &mut Command) {
    generate(gen, cmd, cmd.get_name().to_string(), &mut io::stdout());
}

//Search for paths
fn get_gedent_home() -> Result<PathBuf, Error> {
    let mut config_dir =
        dirs::config_dir().ok_or(anyhow!("Cant retrieve system config directory."))?;
    config_dir.push("gedent");
    match config_dir.try_exists() {
        Ok(exists) => {
            match exists {
                true => (),
                false => anyhow::bail!(format!("Failed to retrieve gedent home, {:?} doesn't exist. \nCheck if you've finished the installation procces and created the config directory.", config_dir)), 
            }
        },
        Err(err) => anyhow::bail!(format!("Failed to retrieve gedent home, caused by {:?}", err)), 
    }
    Ok(config_dir)
}

fn select_key(config: &Config) -> Result<String, Error> {
    let keys: Vec<&String> = config.parameters.keys().collect();
    let mut select = vec![];
    for (k, v) in &config.parameters {
        select.push(format!("{} (current value: {})", &k, v));
    }
    let selection = FuzzySelect::with_theme(&ColorfulTheme::default())
        .default(0)
        .items(&select[..])
        .interact()?;
    Ok(keys[selection].to_string())
}

fn select_template() -> Result<String, Error> {
    let gedent_home: PathBuf = [get_gedent_home()?, Into::into(TEMPLATES_DIR)]
        .iter()
        .collect();
    let gedent_home_len = gedent_home.to_string_lossy().len();
    let templates = Template::get_templates(gedent_home, gedent_home_len, vec![])?;
    let selection = FuzzySelect::with_theme(&ColorfulTheme::default())
        .default(0)
        .items(&templates[..])
        .interact()
        .unwrap();
    Ok(templates[selection].to_string())
}

fn select_software() -> Result<String, Error> {
    let softwares: Vec<String> = read_dir(
        [get_gedent_home()?, Into::into(PRESETS_DIR)]
            .iter()
            .collect::<PathBuf>(),
    )?
    .filter_map(|e| e.ok())
    .map(|e| e.path().file_name().unwrap().to_string_lossy().into_owned())
    .collect();
    let selection = FuzzySelect::with_theme(&ColorfulTheme::default())
        .default(0)
        .items(&softwares[..])
        .interact()
        .unwrap();
    Ok(softwares[selection].to_string())
}

fn check_gedent_health() -> Result<(), Error> {
    match get_gedent_home() {
        Ok(dir) => {
            println!("Found config dir for gedent in {:?}.", dir)
        }
        Err(err) => {
            anyhow::bail!("{:?}", err);
        }
    }

    let softwares: Vec<String> = read_dir(
        [get_gedent_home()?, Into::into(PRESETS_DIR)]
            .iter()
            .collect::<PathBuf>(),
    )?
    .filter_map(|e| e.ok())
    .map(|e| e.path().file_name().unwrap().to_string_lossy().into_owned())
    .collect();
    println!("Found {} presets.", softwares.len());

    let templates_home: PathBuf = [get_gedent_home()?, Into::into(TEMPLATES_DIR)]
        .iter()
        .collect();
    let templates_home_len = templates_home.to_string_lossy().len();
    let templates = Template::get_templates(templates_home, templates_home_len, vec![])?;
    println!("Found {} templates.", templates.len());
    
    
    Ok(())
}

fn setup_gedent() -> Result<(), Error> {
    let mut config_dir = dirs::config_dir().ok_or(anyhow!("Cant retrieve system config directory."))?;
    config_dir.push("gedent");

    match config_dir.try_exists() {
        Ok(exists) => {
            match exists {
                true => anyhow::bail!(format!("Gedent home already exists, if you want to set it up again delete the config dir {:?}.", config_dir)),
                false => {
                    println!("Creating config dir in {:?}.", config_dir);
                    std::fs::create_dir(&config_dir).context("Failed to create config dir.")?;
                    println!("Creating gedent.toml.");
                    let config_path: PathBuf = [config_dir.clone(), Into::into("gedent.toml")].iter().collect();
                    std::fs::write(&config_path, GEDENT_CONFIG).context("Failed to create gedent config.")?;

                    println!("Generating presets.");
                    let presets: PathBuf = [config_dir.clone(), Into::into(PRESETS_DIR)].iter().collect();
                    std::fs::create_dir(&presets).context("Failed to create presets dir.")?;
                    INCLUDE_PRESETS_DIR.extract(presets).context("Failed to extract presets.")?;

                    println!("Generating default templates.");
                    let templates: PathBuf = [config_dir.clone(), Into::into(TEMPLATES_DIR)].iter().collect();
                    std::fs::create_dir(&templates).context("Failed to create templates dir.")?;
                    INCLUDE_TEMPLATES_DIR.extract(templates).context("Failed to extract templates.")?;
                }, 
            }
        },
        Err(err) => anyhow::bail!(format!("Failed to check if gedent home exists, caused by {:?}", err)), 
    }

    Ok(())
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

fn generate_input(
    template: Template,
    molecules: Vec<Molecule>,
    solvation: Option<Option<String>>,
    mult: Option<usize>,
    charge: Option<usize>,
    method: Option<String>,
    basis_set: Option<String>,
    dispersion: Option<String>,
    solvation_model: Option<String>,
    hessian: bool,
    nprocs: Option<usize>,
    mem: Option<usize>,
    split_index: Option<usize>,
) -> Result<Vec<Input>, Error> {
    let mut context = tera::Context::new();
    let config = Config::get()?;
    for (key, value) in config.parameters {
        context.insert(key, &value);
    }

    if let Some(solvation) = solvation {
        context.insert("solvation", &true);
        match solvation {
            Some(solvent) => context.insert("solvent", &solvent),
            None => (),
        }
    }

    if hessian {
        context.insert("hessian", &hessian);
    }

    for (k, v) in [
        ("charge", charge),
        ("mult", mult),
        ("nprocs", nprocs),
        ("mem", mem),
        ("split_index", split_index),
    ] {
        if let Some(v) = v {
            context.insert(k, &v);
        }
    }

    for (k, v) in [
        ("method", method),
        ("basis_set", basis_set),
        ("dispersion", dispersion),
        ("solvation_model", solvation_model),
    ] {
        if let Some(v) = v {
            context.insert(k, &v);
        }
    }

    let extension = match &template.options.extension {
        Some(ext) => ext,
        None => &config.gedent.default_extension,
    };

    let mut results: Vec<Input> = vec![];

    if molecules.is_empty() {
        let filename = PathBuf::from(&template.name).with_extension(extension);
        let filename = filename
            .file_name()
            .ok_or(anyhow!("Can't retrieve template name, exiting.."))?;

        results.push(Input {
            filename: PathBuf::from(filename),
            content: template.render(&context)?,
        });
    }

    for molecule in molecules {
        let mut mol_context = context.clone();
        mol_context.insert("Molecule", &molecule);
        results.push(Input {
            filename: PathBuf::from(molecule.filename).with_extension(extension),
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
