#![allow(clippy::multiple_crate_versions)]

use crate::config::{get_gedent_home, ChemistryConfig, Config};
use crate::molecule::Molecule;
use crate::template::Template;
use clap::{Command, CommandFactory, Parser, Subcommand};
use clap_complete::{generate, Generator, Shell};
use color_eyre::eyre::{bail, eyre, Report as Error, Result, WrapErr};
use include_dir::{include_dir, Dir};
use std::fs::{copy, read_dir, write};
use std::io;
use std::path::PathBuf;

mod config;
mod molecule;
mod template;

const PRESETS_DIR: &str = "presets";
const TEMPLATES_DIR: &str = "templates";

static INCLUDE_PRESETS_DIR: Dir = include_dir!("presets");
static INCLUDE_TEMPLATES_DIR: Dir = include_dir!("templates");
static GEDENT_CONFIG: &str = include_str!("../gedent.toml");

#[derive(Debug, Default)]
struct GenOptions {
    ext: Option<String>,
    method: Option<String>,
    basis_set: Option<String>,
    dispersion: Option<String>,
    #[allow(clippy::option_option)]
    solvent: Option<Option<String>>,
    solvation_model: Option<String>,
    charge: Option<i64>,
    hessian: bool,
    mult: Option<i64>,
    nprocs: Option<i64>,
    mem: Option<i64>,
}

#[derive(Debug)]
struct Input {
    filename: PathBuf,
    content: String,
}

impl Input {
    fn write(self) -> Result<(), Error> {
        println!("Writing {}", self.filename.display());
        write(&self.filename, &self.content).wrap_err("Failed to save input.")
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
#[allow(clippy::large_enum_variant)]
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
        /// Override output file extension
        #[arg(long, default_value = None)]
        ext: Option<String>,
        /// Set method
        #[arg(long, default_value = None)]
        method: Option<String>,
        /// Set `basis_set`
        #[arg(long, default_value = None)]
        basis_set: Option<String>,
        /// Set dispersion
        #[arg(long, default_value = None)]
        dispersion: Option<String>,
        /// Set solvent to value and solvation to true
        #[arg(short, long, default_value = None)]
        #[allow(clippy::option_option)]
        solvent: Option<Option<String>>,
        /// Set `solvation_model`
        #[arg(long, default_value = None)]
        solvation_model: Option<String>,
        /// Set charge
        #[arg(short, long, default_value = None)]
        charge: Option<i64>,
        /// Set hessian
        #[arg(long, default_value_t = false)]
        hessian: bool,
        /// Set mult
        #[arg(short, long, default_value = None)]
        mult: Option<i64>,
        /// Set nprocs
        #[arg(long, default_value = None)]
        nprocs: Option<i64>,
        /// Set mem
        #[arg(long, default_value = None)]
        mem: Option<i64>,
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
    Print { template: String },
    /// Create a new template from a preset located in ~/.config/gedent/presets
    New {
        template_name: String,
        software: String,
    },
    /// List available templates
    List {
        // Lists all available templates
        // TODO: decide how to deal with organization in the folder
        // Prints primarely in .gedent available, otherwise falls back to
        // $XDG_CONFIG
    },
    /// Edit a given template
    Edit { template: String },
}

#[derive(Debug, Subcommand)]
enum ConfigSubcommand {
    /// Prints the location and the currently used configuration
    Print {
        /// Print the path of the printed config.
        #[arg(short, long, default_value_t = false)]
        location: bool,
    },
    /// Opens the currently used config file in your default editor.
    Edit {},
}

#[allow(clippy::too_many_lines)]
fn main() -> Result<()> {
    color_eyre::install()?;
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
                ext,
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
            } => {
                let mut molecules: Vec<(PathBuf, Molecule)> = vec![];
                if let Some(files) = xyz_files {
                    for file in files {
                        molecules.push((file.clone(), Molecule::from_xyz(&file)?));
                    }
                }
                let template = Template::get(template_name)?;
                let opts = GenOptions {
                    ext,
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
                };
                let results = generate_input(&template, molecules, &opts)?;
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
                    config.print(location)?;
                }
                ConfigSubcommand::Edit {} => Config::edit()?,
            },

            Mode::Template {
                template_subcommand,
            } => match template_subcommand {
                TemplateSubcommand::Print { template } => {
                    Template::print_template(&template)?;
                }
                TemplateSubcommand::New {
                    software,
                    template_name,
                } => {
                    Template::from_preset(software, template_name)?;
                }
                TemplateSubcommand::List {} => Template::list_templates()?,
                TemplateSubcommand::Edit { template } => {
                    Template::edit_template(&template)?;
                }
            },

            Mode::Init { config } => gedent_init(config)?,
        }
    }

    Ok(())
}

fn print_completions<G: Generator>(gen: G, cmd: &mut Command) {
    generate(gen, cmd, cmd.get_name().to_string(), &mut io::stdout());
}

fn check_gedent_health() -> Result<(), Error> {
    match get_gedent_home() {
        Ok(dir) => {
            println!("Found config dir for gedent in {}.", dir.display());
        }
        Err(err) => {
            bail!("{:?}", err);
        }
    }

    let softwares: Vec<String> = read_dir(
        [get_gedent_home()?, Into::into(PRESETS_DIR)]
            .iter()
            .collect::<PathBuf>(),
    )?
    .filter_map(std::result::Result::ok)
    .map(|e| e.path().file_name().unwrap().to_string_lossy().into_owned())
    .collect();
    println!("Found {} presets.", softwares.len());

    let templates_home: PathBuf = [get_gedent_home()?, Into::into(TEMPLATES_DIR)]
        .iter()
        .collect();
    let templates = Template::get_templates(&templates_home);
    println!("Found {} templates.", templates.len());

    Ok(())
}

fn setup_gedent() -> Result<(), Error> {
    let mut config_dir =
        dirs::config_dir().ok_or_else(|| eyre!("Cant retrieve system config directory."))?;
    config_dir.push("gedent");

    match config_dir.try_exists() {
        Ok(true) => bail!(
            "Gedent home already exists, if you want to set it up again delete the config dir {}.",
            config_dir.display()
        ),
        Ok(false) => {
            println!("Creating config dir in {}.", config_dir.display());
            std::fs::create_dir(&config_dir).wrap_err("Failed to create config dir.")?;
            println!("Creating gedent.toml.");
            let config_path: PathBuf = [config_dir.clone(), Into::into("gedent.toml")]
                .iter()
                .collect();
            std::fs::write(&config_path, GEDENT_CONFIG)
                .wrap_err("Failed to create gedent config.")?;

            println!("Generating presets.");
            let presets: PathBuf = [config_dir.clone(), Into::into(PRESETS_DIR)]
                .iter()
                .collect();
            std::fs::create_dir(&presets).wrap_err("Failed to create presets dir.")?;
            INCLUDE_PRESETS_DIR
                .extract(presets)
                .wrap_err("Failed to extract presets.")?;

            println!("Generating default templates.");
            let templates: PathBuf = [config_dir.clone(), Into::into(TEMPLATES_DIR)]
                .iter()
                .collect();
            std::fs::create_dir(&templates).wrap_err("Failed to create templates dir.")?;
            INCLUDE_TEMPLATES_DIR
                .extract(templates)
                .wrap_err("Failed to extract templates.")?;
        }
        Err(err) => bail!("Failed to check if gedent home exists, caused by {:?}", err),
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
        bail!("gedent.toml already exists, exiting...");
    }

    copy(config_path, "./gedent.toml")?;
    Ok(())
}

fn build_context(chemistry: &ChemistryConfig, opts: &GenOptions) -> tera::Context {
    let mut context = tera::Context::new();

    // Layer 1: chemistry config (base)
    let chem = chemistry;
    if let Some(ref v) = chem.solvent {
        context.insert("solvation", &true);
        context.insert("solvent", v);
    }
    for (k, v) in [
        ("method", chem.method.as_deref()),
        ("basis_set", chem.basis_set.as_deref()),
        ("dispersion", chem.dispersion.as_deref()),
        ("solvation_model", chem.solvation_model.as_deref()),
    ] {
        if let Some(v) = v {
            context.insert(k, v);
        }
    }
    for (k, v) in [
        ("charge", chem.charge),
        ("mult", chem.mult),
        ("nprocs", chem.nprocs),
        ("mem", chem.mem),
    ] {
        if let Some(v) = v {
            context.insert(k, &v);
        }
    }

    // Layer 2: CLI overrides (win over config)
    if let Some(solvation) = opts.solvent.as_ref() {
        context.insert("solvation", &true);
        if let Some(solvent) = solvation {
            context.insert("solvent", solvent);
        }
    }
    if opts.hessian {
        context.insert("hessian", &true);
    }
    for (k, v) in [
        ("charge", opts.charge),
        ("mult", opts.mult),
        ("nprocs", opts.nprocs),
        ("mem", opts.mem),
    ] {
        if let Some(v) = v {
            context.insert(k, &v);
        }
    }
    for (k, v) in [
        ("method", opts.method.as_deref()),
        ("basis_set", opts.basis_set.as_deref()),
        ("dispersion", opts.dispersion.as_deref()),
        ("solvation_model", opts.solvation_model.as_deref()),
    ] {
        if let Some(v) = v {
            context.insert(k, v);
        }
    }

    context
}

fn generate_input(
    template: &Template,
    molecules: Vec<(PathBuf, Molecule)>,
    opts: &GenOptions,
) -> Result<Vec<Input>, Error> {
    let config = Config::get()?;

    let mut context = build_context(&config.chemistry, opts);
    for (key, value) in config.parameters {
        context.insert(key, &value);
    }

    let extension = opts
        .ext
        .as_ref()
        .unwrap_or(&config.gedent.default_extension);

    let mut results: Vec<Input> = vec![];

    if molecules.is_empty() {
        let filename = PathBuf::from(&template.name).with_extension(extension);
        let filename = filename
            .file_name()
            .ok_or_else(|| eyre!("Can't retrieve template name, exiting.."))?;

        results.push(Input {
            filename: PathBuf::from(filename),
            content: template.render(&context)?,
        });
    }

    for (path, molecule) in molecules {
        let stem = path
            .file_stem()
            .ok_or_else(|| eyre!("Can't retrieve stem from path {}", path.display()))?
            .to_string_lossy();
        let mut mol_context = context.clone();
        mol_context.insert("name", stem.as_ref());
        mol_context.insert("Molecule", &molecule);
        results.push(Input {
            filename: PathBuf::from(stem.as_ref()).with_extension(extension),
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
