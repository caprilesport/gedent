use crate::get_gedent_home;
use crate::Config;
use crate::Molecule;
use anyhow::{anyhow, Context, Error, Result};
use serde::Deserialize;
use serde_json::value::{from_value, to_value, Value};
use std::collections::HashMap;
use std::fs::{copy, read_dir, read_to_string};
use std::path::PathBuf;
use tera::Tera;

const PRESETS_DIR: &str = "presets";
const TEMPLATES_DIR: &str = "templates";

#[derive(Clone, Debug)]
pub struct Template {
    pub name: String,
    template: String,
    pub options: TemplateOptions,
}

// this can be expanded in the future, i dont know if there will be more useful stuff
// that could be in a metada section for the input. i though requiring different molecules
// could be nice, but thats quite a boring implementation for now, in the future i might come back
#[derive(Clone, Debug, Default, Deserialize)]
pub struct TemplateOptions {
    // required_files: Option<i64>,
    pub extension: Option<String>,
}

impl Template {
    // from a parsed template to a result, this does the heavy work
    pub fn render(&self, context: &tera::Context) -> Result<String, Error> {
        let mut tera = Tera::default();
        tera.register_function("print_molecule", print_molecule);
        tera.register_function("split_molecule", split_molecule);
        tera.add_raw_template(&self.name, &self.template)?;
        Ok(tera.render(&self.name, context)?)
    }

    pub fn get(template_name: String) -> Result<Template, Error> {
        let (parsed, opts) =
            Template::parse(&read_to_string(find_template_path(&template_name)?)?)?;
        let template = Template {
            name: template_name,
            template: parsed,
            options: opts,
        };
        Ok(template)
    }

    fn parse(raw_template: &str) -> Result<(String, TemplateOptions), Error> {
        let mut lines = raw_template.lines().peekable();
        let mut header = "".to_string();
        let mut template = "".to_string();

        loop {
            let next = lines.next();
            if next.is_none() {
                break;
            // is it safe to call unwrap here?
            // I know next is not none so should be..
            } else if next.unwrap().contains("--@") {
                loop {
                    if lines.peek().unwrap().contains("--@") {
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

        let template_opts: TemplateOptions =
            toml::from_str(&header).context("Failed to parse template header")?;
        Ok((template, template_opts))
    }

    // #[cfg(test)]
    fn new() -> Template {
        Template {
            name: "".to_string(),
            template: "".to_string(),
            options: TemplateOptions {
                extension: None,
                // required_files: None,
            },
        }
    }
}

// functions for the templates
pub fn print_molecule(args: &HashMap<String, Value>) -> Result<Value, tera::Error> {
    let molecule: Molecule = match args.get("molecule") {
        Some(val) => match from_value(val.clone()) {
            Ok(v) => v,
            Err(_) => {
                return Err(tera::Error::msg(format!(
                    "Function `print_molecule` received molecule={} but `molecule` can only be of type Molecule",
                    val
                )));
            }
        },
        None => {
            return Err(tera::Error::msg(
                "Function `print_molecule` didn't receive a `molecule` argument",
            ))
        }
    };

    let mut full_molecule = "".to_string();
    for atom in molecule.atoms {
        full_molecule = [full_molecule, atom].join("\n");
    }

    full_molecule = full_molecule.replacen("\n", "", 1); //remove first empty line
    Ok(to_value(full_molecule)?)
}

pub fn split_molecule(args: &HashMap<String, Value>) -> Result<Value, tera::Error> {
    let molecule: Molecule = match args.get("molecule") {
        Some(val) => match from_value(val.clone()) {
            Ok(v) => v,
            Err(_) => {
                return Err(tera::Error::msg(format!(
                    "Function `print_molecule` received molecule={} but `molecule` can only be of type Molecule",
                    val
                )));
            }
        },
        None => {
            return Err(tera::Error::msg(
                "Function `print_molecule` didn't receive a `molecule` argument",
            ))
        }
    };

    let index: usize = match args.get("index") {
        Some(val) => match from_value(val.clone()) {
            Ok(v) => v,
            Err(_) => {
                return Err(tera::Error::msg(format!(
                    "Function `slit_molecule` received index={} but `index` can only be of type integer.",
                    val
                )));
            }
        },
        None => {
            return Err(tera::Error::msg(
                "Function `slit_molecule` didn't receive a `index` argument",
            ))
        }
    };

    let (mol1, mol2) = match molecule.split(index) {
        Ok(molecules) => molecules,
        Err(err) => {
            return Err(tera::Error::msg(format!(
                "Failed to split molecules, caused by {}",
                err
            )))
        }
    };
    let molecules = vec![mol1, mol2];

    Ok(to_value(molecules)?)
}

pub fn print_template(template: String) -> Result<(), Error> {
    let template_path = find_template_path(&template)?;
    let template = read_to_string(&template_path)
        .context(format!("Cant find template {:?}", template_path))?;
    println!("{}", &template);
    Ok(())
}

pub fn edit_template(template: String) -> Result<(), Error> {
    let template_path = find_template_path(&template)?;
    // The edit crate makes this work in all platforms.
    edit::edit_file(template_path)?;
    Ok(())
}

pub fn new_template(software: String, template_name: String) -> Result<(), Error> {
    let gedent_home = get_gedent_home()?;
    let template_path: PathBuf = [
        gedent_home.clone(),
        Into::into(TEMPLATES_DIR),
        Into::into(template_name),
    ]
    .iter()
    .collect();
    let boilerplate: PathBuf = [gedent_home, Into::into(PRESETS_DIR), Into::into(software)]
        .iter()
        .collect();
    copy(&boilerplate, &template_path).context(format!(
        "Cant copy base {:?} template to {:?}",
        &boilerplate, &template_path
    ))?;
    edit::edit_file(&template_path).context(format!("Cant open {:?} in editor.", template_path))?;
    Ok(())
}

pub fn list_templates() -> Result<(), Error> {
    let gedent_home: PathBuf = [get_gedent_home()?, Into::into(TEMPLATES_DIR)]
        .iter()
        .collect();
    // +1 is here to remove the first slash
    let gedent_home_len = gedent_home
        .to_str()
        .ok_or(anyhow!("Cant retrieve gedent home len"))?
        .len();
    for entry in read_dir(gedent_home)? {
        print_descent_templates(entry.as_ref().unwrap().path(), gedent_home_len)?;
    }
    Ok(())
}

pub fn print_descent_templates(entry: PathBuf, gedent_home_len: usize) -> Result<(), Error> {
    if entry.is_dir() {
        let new_dir = read_dir(entry)?;
        for new_entry in new_dir {
            let _ = print_descent_templates(new_entry.as_ref().unwrap().path(), gedent_home_len)?;
        }
        Ok(())
    } else {
        println!("{}", &entry.to_str().unwrap()[gedent_home_len..]);
        Ok(())
    }
}

fn find_template_path(template: &String) -> Result<PathBuf, Error> {
    let template_path: PathBuf = [
        get_gedent_home()?,
        Into::into(TEMPLATES_DIR),
        Into::into(template),
    ]
    .iter()
    .collect();
    Ok(template_path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use toml::{map::Map, Value};

    #[test]
    fn render_template_works() {
        let parsed_template = "! {{ dft_level }} {{ dft_basis_set }}

nprocs {{ nprocs }}
end

{% if solvation -%}
end

{% endif -%}
"
        .to_string();

        let rendered_template = "! PBE0 def2

nprocs 2
end

"
        .to_string();
        let template = Template {
            template: parsed_template,
            ..Template::new()
        };

        let mut context = tera::Context::new();
        context.insert("solvation".to_string(), &Value::Boolean(false));
        context.insert("dft_level".to_string(), &Value::String("PBE0".to_string()));
        context.insert(
            "dft_basis_set".to_string(),
            &Value::String("def2".to_string()),
        );
        context.insert("nprocs".to_string(), &Value::Integer(2));
        match template.render(&context) {
            Ok(result) => assert_eq!(result, rendered_template),
            Err(err) => core::panic!("Failed to render template, caused by {}", err),
        }
    }

    #[test]
    fn parse_template_works() {
        let mut template = Template::new();
        template.template = "--@
extension = \"inp\"
--@
! {{ dft_level }} {{ dft_basis_set }}

nprocs {{ nprocs }}
end

{% if solvation -%}
end

{% endif -%}"
            .to_string();

        let test_parsed_template = "! {{ dft_level }} {{ dft_basis_set }}

nprocs {{ nprocs }}
end

{% if solvation -%}
end

{% endif -%}"
            .to_string();

        match Template::parse(&template.template) {
            Ok((template, opts)) => {
                assert_eq!(template, test_parsed_template);
                assert_eq!(opts.extension, Some("inp".to_string()))
            }
            Err(_) => core::panic!("Error parsing template!"),
        }

        // when there is no header opts.extension shoud be none
        match Template::parse(&test_parsed_template) {
            Ok((template, opts)) => {
                assert_eq!(template, test_parsed_template);
                assert_eq!(opts.extension, None)
            }
            Err(_) => core::panic!("Error parsing template!"),
        }
    }
}
