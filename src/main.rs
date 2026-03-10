#![allow(clippy::multiple_crate_versions)]

use crate::config::{Config, ModelConfig, ResourcesConfig};
use crate::molecule::Molecule;
use crate::template::Template;
use clap::{Command, CommandFactory, Parser, Subcommand};
use clap_complete::{generate, Generator, Shell};
use clap_verbosity_flag::{Verbosity, WarnLevel};
use color_eyre::eyre::{bail, eyre, Report as Error, Result, WrapErr};
use include_dir::{include_dir, Dir};
use log::{debug, info};
use std::fs::{read_dir, write};
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
    software: Option<String>,
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
        info!("Writing {}", self.filename.display());
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
    #[arg(long, default_value = None)]
    set_up: bool,
    /// If provided, outputs the completion file for given shell.
    #[arg(long = "generate", value_enum)]
    generator: Option<Shell>,
    #[command(flatten)]
    verbose: Verbosity<WarnLevel>,
}

#[derive(Debug, Subcommand)]
enum CompleteSubcommand {
    /// List completable template names (one per line).
    Templates,
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
        /// Override software (used for template disambiguation)
        #[arg(long, default_value = None)]
        software: Option<String>,
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
    /// Shell completion endpoint — hidden from normal help output.
    /// `gedent __complete templates` prints one completable name per line.
    #[command(hide = true)]
    Complete {
        #[command(subcommand)]
        complete_subcommand: CompleteSubcommand,
    },
    // Subcommand for init gedent "repo"
    /// Initiate a gedent project in the current directory.
    Init {
        /// Set software (used for template disambiguation)
        #[arg(long, default_value = None)]
        software: Option<String>,
        /// Set method
        #[arg(long, default_value = None)]
        method: Option<String>,
        /// Set `basis_set`
        #[arg(long, default_value = None)]
        basis_set: Option<String>,
        /// Set dispersion
        #[arg(long, default_value = None)]
        dispersion: Option<String>,
        /// Set solvent
        #[arg(short, long, default_value = None)]
        solvent: Option<String>,
        /// Set `solvation_model`
        #[arg(long, default_value = None)]
        solvation_model: Option<String>,
        /// Set charge
        #[arg(short, long, default_value = None)]
        charge: Option<i64>,
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
    /// Opens a config file in $EDITOR.
    Edit {
        /// Edit the global ~/.config/gedent/gedent.toml instead of the nearest local one.
        #[arg(short, long, default_value_t = false)]
        global: bool,
    },
}

#[allow(clippy::too_many_lines)]
fn main() -> Result<()> {
    color_eyre::install()?;
    let cli = Cli::parse();

    env_logger::Builder::new()
        .filter_level(cli.verbose.log_level_filter())
        .format_timestamp(None)
        .format_target(false)
        .init();

    if let Some(generator) = cli.generator {
        let mut cmd = Cli::command();
        info!("Generating completion file for {generator:?}...");
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
                software,
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
                let opts = GenOptions {
                    software,
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
                let results = generate_input(template_name, molecules, &opts)?;
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
                ConfigSubcommand::Edit { global } => Config::edit(global)?,
            },

            Mode::Template {
                template_subcommand,
            } => match template_subcommand {
                TemplateSubcommand::Print { template } => {
                    let software = Config::get().ok().and_then(|c| c.gedent.software);
                    Template::print_template(&template, software.as_deref())?;
                }
                TemplateSubcommand::New {
                    software,
                    template_name,
                } => {
                    Template::from_preset(software, &template_name)?;
                }
                TemplateSubcommand::List {} => Template::list_templates()?,
                TemplateSubcommand::Edit { template } => {
                    let software = Config::get().ok().and_then(|c| c.gedent.software);
                    Template::edit_template(&template, software.as_deref())?;
                }
            },

            Mode::Init {
                software,
                method,
                basis_set,
                dispersion,
                solvent,
                solvation_model,
                charge,
                mult,
                nprocs,
                mem,
            } => gedent_init(
                software,
                ModelConfig {
                    method,
                    basis_set,
                    charge,
                    mult,
                    dispersion,
                    solvent,
                    solvation_model,
                },
                ResourcesConfig { nprocs, mem },
            )?,

            Mode::Complete {
                complete_subcommand,
            } => match complete_subcommand {
                CompleteSubcommand::Templates => {
                    let software = Config::get().ok().and_then(|c| c.gedent.software);
                    for name in Template::list_names(software.as_deref())? {
                        println!("{name}");
                    }
                }
            },
        }
    }

    Ok(())
}

fn print_completions<G: Generator>(gen: G, cmd: &mut Command) {
    generate(gen, cmd, cmd.get_name().to_string(), &mut io::stdout());
}

fn check_gedent_health() -> Result<(), Error> {
    match Config::gedent_home() {
        Ok(dir) => {
            info!("Found config dir for gedent in {}.", dir.display());
        }
        Err(err) => {
            bail!("{:?}", err);
        }
    }

    let softwares: Vec<String> = read_dir(
        [Config::gedent_home()?, Into::into(PRESETS_DIR)]
            .iter()
            .collect::<PathBuf>(),
    )?
    .filter_map(std::result::Result::ok)
    .map(|e| e.path().file_name().unwrap().to_string_lossy().into_owned())
    .collect();
    info!("Found {} presets.", softwares.len());

    let templates_home: PathBuf = [Config::gedent_home()?, Into::into(TEMPLATES_DIR)]
        .iter()
        .collect();
    let templates = Template::get_templates(&templates_home);
    info!("Found {} templates.", templates.len());

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
            info!("Creating config dir in {}.", config_dir.display());
            std::fs::create_dir(&config_dir).wrap_err("Failed to create config dir.")?;
            info!("Creating gedent.toml.");
            let config_path: PathBuf = [config_dir.clone(), Into::into("gedent.toml")]
                .iter()
                .collect();
            std::fs::write(&config_path, GEDENT_CONFIG)
                .wrap_err("Failed to create gedent config.")?;

            info!("Generating presets.");
            let presets: PathBuf = [config_dir.clone(), Into::into(PRESETS_DIR)]
                .iter()
                .collect();
            std::fs::create_dir(&presets).wrap_err("Failed to create presets dir.")?;
            INCLUDE_PRESETS_DIR
                .extract(presets)
                .wrap_err("Failed to extract presets.")?;

            info!("Generating default templates.");
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

fn gedent_init(
    software: Option<String>,
    model: ModelConfig,
    resources: ResourcesConfig,
) -> Result<(), Error> {
    if std::path::Path::try_exists(&PathBuf::from("./gedent.toml"))? {
        bail!("gedent.toml already exists, exiting...");
    }

    let no_flags_set = software.is_none()
        && model.method.is_none()
        && model.basis_set.is_none()
        && model.dispersion.is_none()
        && model.solvent.is_none()
        && model.solvation_model.is_none()
        && model.charge.is_none()
        && model.mult.is_none()
        && resources.nprocs.is_none()
        && resources.mem.is_none();

    let content = if no_flags_set {
        "[gedent]\n\n[model]\n\n[resources]\n\n[parameters]\n".to_string()
    } else {
        #[derive(serde::Serialize)]
        struct InitGedentConfig {
            #[serde(skip_serializing_if = "Option::is_none")]
            software: Option<String>,
        }
        #[derive(serde::Serialize)]
        struct InitConfig {
            #[serde(skip_serializing_if = "Option::is_none")]
            gedent: Option<InitGedentConfig>,
            model: ModelConfig,
            resources: ResourcesConfig,
        }
        let gedent = software.map(|sw| InitGedentConfig { software: Some(sw) });
        toml::to_string(&InitConfig {
            gedent,
            model,
            resources,
        })
        .wrap_err("Failed to serialize init config")?
    };

    write("./gedent.toml", content).wrap_err("Failed to write gedent.toml")?;
    info!("Created gedent.toml.");
    Ok(())
}

fn build_context(
    model: &ModelConfig,
    resources: &ResourcesConfig,
    opts: &GenOptions,
) -> tera::Context {
    let mut context = tera::Context::new();

    // Layer 1: model config (what the calculation is)
    if let Some(ref v) = model.solvent {
        context.insert("solvation", &true);
        context.insert("solvent", v);
    }
    for (k, v) in [
        ("method", model.method.as_deref()),
        ("basis_set", model.basis_set.as_deref()),
        ("dispersion", model.dispersion.as_deref()),
        ("solvation_model", model.solvation_model.as_deref()),
    ] {
        if let Some(v) = v {
            context.insert(k, v);
        }
    }
    for (k, v) in [("charge", model.charge), ("mult", model.mult)] {
        if let Some(v) = v {
            context.insert(k, &v);
        }
    }

    // Layer 2: resources config (what machine it runs on)
    for (k, v) in [("nprocs", resources.nprocs), ("mem", resources.mem)] {
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
    template_name: String,
    molecules: Vec<(PathBuf, Molecule)>,
    opts: &GenOptions,
) -> Result<Vec<Input>, Error> {
    let config = Config::get()?;

    let software = opts
        .software
        .as_deref()
        .or(config.gedent.software.as_deref());
    debug!("Resolving template {template_name:?} with software hint {software:?}");
    let template = Template::get(template_name, software)?;

    let mut context = build_context(&config.model, &config.resources, opts);
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
        results.push(Input {
            filename: PathBuf::from(stem.as_ref()).with_extension(extension),
            content: template.render_with_molecule(&context, &molecule, &stem)?,
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

    #[test]
    fn build_context_config_values_inserted() {
        let model = ModelConfig {
            method: Some("pbe0".into()),
            basis_set: Some("def2-tzvp".into()),
            charge: Some(-1),
            ..ModelConfig::default()
        };
        let resources = ResourcesConfig {
            nprocs: Some(8),
            ..ResourcesConfig::default()
        };
        let ctx = build_context(&model, &resources, &GenOptions::default()).into_json();
        assert_eq!(ctx["method"], "pbe0");
        assert_eq!(ctx["basis_set"], "def2-tzvp");
        assert_eq!(ctx["charge"], -1);
        assert_eq!(ctx["nprocs"], 8);
    }

    #[test]
    fn build_context_cli_overrides_config() {
        let model = ModelConfig {
            method: Some("pbe0".into()),
            charge: Some(0),
            ..ModelConfig::default()
        };
        let opts = GenOptions {
            method: Some("b3lyp".into()),
            charge: Some(2),
            ..GenOptions::default()
        };
        let ctx = build_context(&model, &ResourcesConfig::default(), &opts).into_json();
        assert_eq!(ctx["method"], "b3lyp");
        assert_eq!(ctx["charge"], 2);
    }

    #[test]
    fn build_context_config_falls_through_when_no_cli_override() {
        let model = ModelConfig {
            method: Some("pbe0".into()),
            ..ModelConfig::default()
        };
        let opts = GenOptions {
            basis_set: Some("def2-tzvp".into()),
            ..GenOptions::default()
        };
        let ctx = build_context(&model, &ResourcesConfig::default(), &opts).into_json();
        assert_eq!(ctx["method"], "pbe0");
        assert_eq!(ctx["basis_set"], "def2-tzvp");
    }

    #[test]
    fn build_context_solvent_sets_solvation_flag() {
        let model = ModelConfig {
            solvent: Some("water".into()),
            ..ModelConfig::default()
        };
        let ctx =
            build_context(&model, &ResourcesConfig::default(), &GenOptions::default()).into_json();
        assert_eq!(ctx["solvation"], true);
        assert_eq!(ctx["solvent"], "water");
    }

    #[test]
    fn build_context_cli_solvent_overrides_config() {
        let opts = GenOptions {
            solvent: Some(Some("thf".into())),
            ..GenOptions::default()
        };
        let ctx =
            build_context(&ModelConfig::default(), &ResourcesConfig::default(), &opts).into_json();
        assert_eq!(ctx["solvation"], true);
        assert_eq!(ctx["solvent"], "thf");
    }

    #[test]
    fn build_context_absent_fields_not_inserted() {
        let ctx = build_context(
            &ModelConfig::default(),
            &ResourcesConfig::default(),
            &GenOptions::default(),
        )
        .into_json();
        assert!(ctx.get("method").is_none());
        assert!(ctx.get("charge").is_none());
        assert!(ctx.get("solvation").is_none());
    }
}
