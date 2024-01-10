// #![allow(dead_code, unused_variables, unused_imports)]
use crate::config::Config;
use crate::molecule::Molecule;
use anyhow::{anyhow, Context, Error, Result};
use clap::{Parser, Subcommand};
use serde::Deserialize;
use std::fs::{copy, read_dir, read_to_string};
use std::path::PathBuf;
use tera::Tera;

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

// #[derive(Clone, Debug)]
// struct Template {
//     name: String,
//     path: PathBuf,
//     raw_template: String,
//     parsed_template: String,
// }

// this can be expanded in the future, i dont know if there will be more useful stuff
// that could be in a metada section for the input. i though requiring different molecules
// could be nice, but thats quite a boring implementation for now, in the future i might come back
#[derive(Clone, Debug, Default, Deserialize)]
struct TemplateOptions {
    extension: Option<String>,
}

// impl Template {
//     fn new() -> Template {
//         return Template {
//             name: "".to_string(),
//             path: PathBuf::from(""),
//             raw_template: "".to_string(),
//             parsed_template: "".to_string(),
//         };
//     }

//     fn gen(
//         &self,
//         xyz_files: Option<Vec<PathBuf>>,
//         config: Option<Config>,
//     ) -> Result<Template, Error> {
//         Ok(Template::new())
//     }
// }

// Template functionality
fn generate_template(
    template: String,
    xyz_files: Option<Vec<PathBuf>>,
    _config: Option<PathBuf>,
) -> Result<(), Error> {
    // let config_path = match config {
    //     Some(config_path) => config_path,
    //     None => get_config_path()?,
    // };

    let context = tera::Context::new();
    // let config = load_config(&config_path)?;
    // for (key, value) in config {
    //     context.insert(key, &value);
    // }

    let results = render_template(template, context, xyz_files)?;

    for i in results {
        println!("{} \n{}", i.1, i.0);
    }

    Ok(())
}

// TODO: refactor this little guy
fn render_template(
    template_name: String,
    mut context: tera::Context,
    xyz_files: Option<Vec<PathBuf>>,
) -> Result<Vec<(String, String)>, Error> {
    // this doesnt belong here
    let template_path = get_template_path(&template_name)?;
    let raw_template = read_to_string(&template_path)
        .context(format!("Cant find template {:?}", template_path))?;

    let (parsed_template, opts) = parse_template(&raw_template)?;
    let extension = match opts.extension {
        Some(ext) => ext,
        None => "inp".to_string(),
    };

    let mut tera = Tera::default();
    // tera.register_function(, ); split returns the two splitted molecules
    // tera.register_function(, ); print returns a string with the xyz structure of the molecule
    tera.add_raw_template("template", &parsed_template)?;

    let mut result = vec![];

    let (n, mut xyzfiles) = match xyz_files {
        Some(files) => (files.len(), files),
        None => (0, vec![]),
    };

    if n == 0 {
        result.push((
            tera.render("template", &context)?,
            [template_name, extension.clone()].join("."),
        ));
    } else {
        // this loop is neccessary because there might be xyz files with
        // multiple structures
        for _index in 0..n {
            let mut molecules = Molecule::from_xyz(xyzfiles.pop().unwrap())?;
            if molecules.len() != 1 {}
            loop {
                let molecule = molecules.pop();
                match molecule {
                    Some(mol) => {
                        context.insert("molecule", &mol);
                        result.push((
                            tera.render("template", &context)?,
                            [mol.filename, extension.clone()].join("."),
                        ));
                    }
                    None => break,
                }
            }
        }
    }
    Ok(result)
}

fn parse_template(raw_template: &String) -> Result<(String, TemplateOptions), Error> {
    let mut lines = raw_template.lines().peekable();
    let mut header = "".to_string();
    let mut template = "".to_string();

    loop {
        let next = lines.next();
        if next.is_none() {
            break;
        } else if next.unwrap().contains("---") {
            loop {
                if lines.peek().unwrap().contains("---") {
                    let _ = lines.next();
                    break;
                }
                header = [header, lines.next().unwrap().to_string()].join("\n");
            }
        } else {
            template = [template, next.unwrap().to_string()].join("\n");
        }
    }
    template = template.replacen("\n", "", 1); //remove first empty line
                                               // template = template[1..template.len() - 2].; //remove first empty line

    let template_opts: TemplateOptions =
        toml::from_str(&header).context("Failed to parse extension in template header")?;
    Ok((template, template_opts))
}

fn edit_template(template: String) -> Result<(), Error> {
    let template_path = get_template_path(&template)?;
    // The edit crate makes this work in all platforms.
    edit::edit_file(template_path)?;
    Ok(())
}

fn print_template(template: String) -> Result<(), Error> {
    let template_path = get_template_path(&template)?;
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

fn get_template_path(template: &String) -> Result<PathBuf, Error> {
    let template_path: PathBuf = [
        get_gedent_home()?,
        Into::into(TEMPLATES_DIR),
        Into::into(template),
    ]
    .iter()
    .collect();
    Ok(template_path)
}

// There may be a better way to write this?
// Decide if there will be templates here - inclined to no for now.
fn gedent_init(config: Option<String>) -> Result<(), Error> {
    let mut config_path = PathBuf::new();
    match config {
        Some(file) => config_path.push(file),
        None => {
            config_path.push(get_gedent_home()?);
            config_path.push(CONFIG_NAME);
        }
    };

    let gedent = PathBuf::from(CONFIG_NAME.to_string());

    if std::path::Path::try_exists(&gedent)? {
        anyhow::bail!("gedent.toml already exists, exiting...");
    }

    copy(config_path, gedent)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_template_works() {
        let raw_template = "
---
extension = \"inp\"
---
! {{ dft_level }} {{ dft_basis_set }} 
! Opt freq D3BJ

%pal
 nprocs {{ nprocs }}
end

%maxcore {{ memory }} 

{% if solvation -%}
%cpcm
 smd true
 smdsolvent \"{{ solvent }}\"
end

{% endif -%}"
            .to_string();

        let test_parsed_template = "
! {{ dft_level }} {{ dft_basis_set }} 
! Opt freq D3BJ

%pal
 nprocs {{ nprocs }}
end

%maxcore {{ memory }} 

{% if solvation -%}
%cpcm
 smd true
 smdsolvent \"{{ solvent }}\"
end

{% endif -%}"
            .to_string();

        match parse_template(&raw_template) {
            Ok((template, opts)) => {
                assert_eq!(template, test_parsed_template);
                assert_eq!(opts.extension, Some("inp".to_string()))
            }
            Err(_) => core::panic!("Error parsing template!"),
        }

        // when there is no header opts.extension shoud be none
        match parse_template(&test_parsed_template) {
            Ok((template, opts)) => {
                assert_eq!(template, test_parsed_template);
                assert_eq!(opts.extension, None)
            }
            Err(_) => core::panic!("Error parsing template!"),
        }
    }

    #[test]
    fn verify_cli() {
        use clap::CommandFactory;

        Cli::command().debug_assert();
    }
}
