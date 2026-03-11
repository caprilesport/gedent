#![allow(clippy::multiple_crate_versions)]

use crate::config::{Config, ModelConfig, ResourcesConfig};
use crate::molecule::Molecule;
use crate::template::Template;
use clap::{Command, CommandFactory, Parser, Subcommand};
use clap_complete::{generate, Shell};
use clap_verbosity_flag::{Verbosity, WarnLevel};
use color_eyre::eyre::{bail, eyre, Report as Error, Result, WrapErr};
use include_dir::{include_dir, Dir};
use log::{debug, error, info, warn};
use std::fs::{read_dir, write};
use std::path::PathBuf;

mod config;
mod elements;
mod molecule;
mod template;
mod validation;

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
    mult: Option<i64>,
    nprocs: Option<i64>,
    mem: Option<i64>,
    /// Raw `KEY=VALUE` strings from `--var`; parsed and inserted into context
    /// after `[parameters]`, so they win over config file values.
    vars: Vec<String>,
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
        /// Set mult
        #[arg(short, long, default_value = None)]
        mult: Option<i64>,
        /// Set nprocs
        #[arg(long, default_value = None)]
        nprocs: Option<i64>,
        /// Set mem
        #[arg(long, default_value = None)]
        mem: Option<i64>,
        /// Set an arbitrary template variable (KEY=VALUE, value parsed as TOML)
        #[arg(long = "var", value_name = "KEY=VALUE")]
        vars: Vec<String>,
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
    /// `gedent _complete templates` prints one completable name per line.
    #[command(hide = true, name = "_complete")]
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
    List {},
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

    if Config::gedent_home().is_err() {
        setup_gedent()?;
    }

    if cli.health {
        check_gedent_health()?;
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
                mult,
                nprocs,
                mem,
                vars,
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
                    mult,
                    nprocs,
                    mem,
                    vars,
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

fn print_completions(gen: Shell, cmd: &mut Command) {
    let mut buf: Vec<u8> = Vec::new();
    generate(gen, cmd, cmd.get_name().to_string(), &mut buf);
    let script = String::from_utf8(buf).unwrap_or_default();
    print!("{}", patch_completions(gen, script));
}

fn patch_completions(shell: Shell, script: String) -> String {
    match shell {
        Shell::Zsh => patch_zsh(script),
        Shell::Bash => patch_bash(script),
        Shell::Fish => patch_fish(script),
        _ => script,
    }
}

fn patch_zsh(mut script: String) -> String {
    // Wire template_name completion for `gen` and `template print/edit`
    script = script.replace(
        "':template_name -- The template to look for in ~/.config/gedent/templates:_default'",
        "':template_name -- The template to look for in ~/.config/gedent/templates:_gedent_template_names'",
    );
    script = script.replace("':template:_default'", "':template:_gedent_template_names'");
    script.push_str(
        r#"
_gedent_template_names() {
    local -a templates
    templates=("${(@f)$(gedent _complete templates 2>/dev/null)}")
    _describe 'template' templates
}
"#,
    );
    script
}

fn patch_fish(mut script: String) -> String {
    script.push_str(
        r#"
# Dynamic template name completions
complete -c gedent -n "__fish_gedent_using_subcommand gen; and test (count (commandline -opc)) -le 2" -f -a "(gedent _complete templates 2>/dev/null)"
complete -c gedent -n "__fish_gedent_using_subcommand template; and __fish_seen_subcommand_from print" -f -a "(gedent _complete templates 2>/dev/null)"
complete -c gedent -n "__fish_gedent_using_subcommand template; and __fish_seen_subcommand_from edit" -f -a "(gedent _complete templates 2>/dev/null)"
"#,
    );
    script
}

fn patch_bash(mut script: String) -> String {
    // `gen`: template_name is the first positional (COMP_CWORD 2)
    script = script.replace(
        r#"            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --ext)"#,
        r#"            if [[ ${cur} == -* ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            if [[ ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "$(gedent _complete templates 2>/dev/null)" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --ext)"#,
    );
    // `template print` and `template edit`: template is the first positional (COMP_CWORD 3)
    script = script.replace(
        r#"            opts="-v -q -h -V --verbose --quiet --help --version <TEMPLATE>"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()"#,
        r#"            opts="-v -q -h -V --verbose --quiet --help --version <TEMPLATE>"
            if [[ ${cur} == -* ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            if [[ ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "$(gedent _complete templates 2>/dev/null)" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()"#,
    );
    script
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

/// Parse a `KEY=VALUE` string into a key and a TOML value.
/// The value is first tried as a TOML literal (so integers, booleans, and
/// arrays work without quoting); bare strings that don't parse as TOML fall
/// back to `Value::String`.
fn parse_var(s: &str) -> Result<(String, toml::Value), Error> {
    let (key, val_str) = s
        .split_once('=')
        .ok_or_else(|| eyre!("--var must be KEY=VALUE, got {s:?}"))?;
    if key.is_empty() {
        bail!("--var key cannot be empty in {s:?}");
    }
    // Wrap in a dummy key to let the TOML parser infer the type, then extract.
    // Falls back to a plain string for values that aren't valid TOML literals.
    let value = toml::from_str::<toml::Table>(&format!("v = {val_str}"))
        .ok()
        .and_then(|mut t| t.remove("v"))
        .unwrap_or_else(|| toml::Value::String(val_str.to_string()));
    Ok((key.to_string(), value))
}

fn render_inputs(
    template: &Template,
    molecules: Vec<(PathBuf, Molecule)>,
    context: &tera::Context,
    extension: &str,
) -> Result<Vec<Input>, Error> {
    let mut results: Vec<Input> = vec![];

    if molecules.is_empty() {
        let filename = PathBuf::from(&template.name).with_extension(extension);
        let filename = filename
            .file_name()
            .ok_or_else(|| eyre!("Can't retrieve template name, exiting.."))?;

        results.push(Input {
            filename: PathBuf::from(filename),
            content: template.render(context)?,
        });
    }

    for (path, molecule) in molecules {
        let stem = path
            .file_stem()
            .ok_or_else(|| eyre!("Can't retrieve stem from path {}", path.display()))?
            .to_string_lossy();
        results.push(Input {
            filename: PathBuf::from(stem.as_ref()).with_extension(extension),
            content: template.render_with_molecule(context, &molecule, &stem)?,
        });
    }

    Ok(results)
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
    for s in &opts.vars {
        let (key, value) = parse_var(s)?;
        context.insert(key, &value);
    }

    let extension = opts
        .ext
        .as_ref()
        .unwrap_or(&config.gedent.default_extension);

    // Run validation on all inputs before rendering anything, so the user
    // sees every problem at once rather than one per run.
    let mut has_errors = false;
    if molecules.is_empty() {
        for d in validation::validate(None, &context, &template.meta.requires) {
            emit_diagnostic(&template.name, &d);
            if d.severity == validation::Severity::Error {
                has_errors = true;
            }
        }
    } else {
        for (path, molecule) in &molecules {
            let name = path
                .file_stem()
                .map_or_else(|| path.to_string_lossy(), |s| s.to_string_lossy());
            for d in validation::validate(Some(molecule), &context, &template.meta.requires) {
                emit_diagnostic(&name, &d);
                if d.severity == validation::Severity::Error {
                    has_errors = true;
                }
            }
        }
    }
    if has_errors {
        bail!("Validation failed — fix the errors above before generating.");
    }

    render_inputs(&template, molecules, &context, extension)
}

fn emit_diagnostic(name: &str, d: &validation::Diagnostic) {
    match d.severity {
        validation::Severity::Error => error!("{name}: {}", d.message),
        validation::Severity::Warning => warn!("{name}: {}", d.message),
    }
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

    #[test]
    fn render_inputs_no_molecules_uses_template_stem() {
        let template = Template::with_body("orca/sp", "hello");
        let inputs = render_inputs(&template, vec![], &tera::Context::new(), "inp").unwrap();
        assert_eq!(inputs.len(), 1);
        assert_eq!(inputs[0].filename, PathBuf::from("sp.inp"));
        assert_eq!(inputs[0].content, "hello");
    }

    #[test]
    fn render_inputs_single_molecule_uses_stem() {
        use crate::elements::Element;
        use crate::molecule::{Atom, Molecule};

        let template = Template::with_body("sp", "{{ name }}");
        let mol = Molecule {
            description: None,
            atoms: vec![Atom {
                element: Element::H,
                x: 0.0,
                y: 0.0,
                z: 0.0,
            }],
        };
        let inputs = render_inputs(
            &template,
            vec![(PathBuf::from("water.xyz"), mol)],
            &tera::Context::new(),
            "inp",
        )
        .unwrap();
        assert_eq!(inputs.len(), 1);
        assert_eq!(inputs[0].filename, PathBuf::from("water.inp"));
        assert_eq!(inputs[0].content, "water");
    }

    #[test]
    fn render_inputs_multiple_molecules_produce_separate_files() {
        use crate::elements::Element;
        use crate::molecule::{Atom, Molecule};

        let template = Template::with_body("sp", "{{ name }}");
        let mol = || Molecule {
            description: None,
            atoms: vec![Atom {
                element: Element::H,
                x: 0.0,
                y: 0.0,
                z: 0.0,
            }],
        };
        let inputs = render_inputs(
            &template,
            vec![
                (PathBuf::from("mol1.xyz"), mol()),
                (PathBuf::from("mol2.xyz"), mol()),
            ],
            &tera::Context::new(),
            "com",
        )
        .unwrap();
        assert_eq!(inputs.len(), 2);
        assert_eq!(inputs[0].filename, PathBuf::from("mol1.com"));
        assert_eq!(inputs[1].filename, PathBuf::from("mol2.com"));
    }

    // ── parse_var ─────────────────────────────────────────────────────────────

    #[test]
    fn parse_var_integer() {
        let (k, v) = parse_var("nsteps=20").unwrap();
        assert_eq!(k, "nsteps");
        assert_eq!(v, toml::Value::Integer(20));
    }

    #[test]
    fn parse_var_boolean() {
        let (k, v) = parse_var("flag=true").unwrap();
        assert_eq!(k, "flag");
        assert_eq!(v, toml::Value::Boolean(true));
    }

    #[test]
    fn parse_var_array() {
        let (k, v) = parse_var("atoms=[20, 28]").unwrap();
        assert_eq!(k, "atoms");
        assert_eq!(
            v,
            toml::Value::Array(vec![toml::Value::Integer(20), toml::Value::Integer(28)])
        );
    }

    #[test]
    fn parse_var_bare_string_fallback() {
        let (k, v) = parse_var("solvent=water").unwrap();
        assert_eq!(k, "solvent");
        assert_eq!(v, toml::Value::String("water".to_string()));
    }

    #[test]
    fn parse_var_quoted_string() {
        let (k, v) = parse_var("solvent=\"dichloromethane\"").unwrap();
        assert_eq!(k, "solvent");
        assert_eq!(v, toml::Value::String("dichloromethane".to_string()));
    }

    #[test]
    fn parse_var_value_with_equals_sign() {
        // Only the first '=' is the separator; the rest is part of the value.
        let (k, v) = parse_var("label=a=b").unwrap();
        assert_eq!(k, "label");
        assert_eq!(v, toml::Value::String("a=b".to_string()));
    }

    #[test]
    fn parse_var_missing_equals_errors() {
        assert!(parse_var("noequals").is_err());
    }

    #[test]
    fn parse_var_empty_key_errors() {
        assert!(parse_var("=value").is_err());
    }
}
