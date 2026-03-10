use crate::config::get_gedent_home;
use crate::Molecule;
use color_eyre::eyre::{bail, Report as Error, Result, WrapErr};
use serde_json::value::{from_value, to_value, Value};
use std::collections::HashMap;
use std::fs::{copy, read_to_string};
use std::path::{Path, PathBuf};
use tera::Tera;
use walkdir::WalkDir;

const PRESETS_DIR: &str = "presets";
const TEMPLATES_DIR: &str = "templates";

#[derive(Clone, Debug)]
pub struct Template {
    pub name: String,
    body: String,
}

impl Template {
    pub fn from_preset(software: String, template_name: String) -> Result<(), Error> {
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
        copy(&boilerplate, &template_path).wrap_err(format!(
            "Cant copy base {} template to {}",
            boilerplate.display(),
            template_path.display()
        ))?;
        edit::edit_file(&template_path)
            .wrap_err(format!("Cant open {} in editor.", template_path.display()))?;
        Ok(())
    }

    pub fn render(&self, context: &tera::Context) -> Result<String, Error> {
        let mut tera = Tera::default();
        tera.register_function("print_molecule", print_molecule);
        tera.add_raw_template(&self.name, &self.body)?;
        Ok(tera.render(&self.name, context)?)
    }

    pub fn get_templates(templates_home: &Path) -> Vec<String> {
        let home_len = templates_home.to_string_lossy().len();
        WalkDir::new(templates_home)
            .into_iter()
            .filter_map(std::result::Result::ok)
            .filter(|e| e.file_type().is_file())
            .map(|e| e.path().to_string_lossy()[home_len + 1..].to_string())
            .collect()
    }

    pub fn get(template_name: String) -> Result<Self, Error> {
        let body = read_to_string(Self::find_path(&template_name)?)
            .wrap_err(format!("Can't read template {template_name}"))?;
        Ok(Self {
            name: template_name,
            body,
        })
    }

    pub fn print_template(template: &str) -> Result<(), Error> {
        let template_path = Self::find_path(template)?;
        let template = read_to_string(&template_path)
            .wrap_err(format!("Cant find template {}", template_path.display()))?;
        println!("{template}");
        Ok(())
    }

    pub fn edit_template(template: &str) -> Result<(), Error> {
        let template_path = Self::find_path(template)?;
        // The edit crate makes this work in all platforms.
        edit::edit_file(template_path)?;
        Ok(())
    }

    pub fn list_templates() -> Result<(), Error> {
        let templates_home: PathBuf = [get_gedent_home()?, Into::into(TEMPLATES_DIR)]
            .iter()
            .collect();
        let templates = Self::get_templates(&templates_home);
        for i in templates {
            println!("{i}");
        }
        Ok(())
    }

    fn find_path(template: &str) -> Result<PathBuf, Error> {
        let template_path: PathBuf = [
            get_gedent_home()?,
            Into::into(TEMPLATES_DIR),
            Into::into(template),
        ]
        .iter()
        .collect();
        if template_path.try_exists()? {
            Ok(template_path)
        } else {
            bail!("Cant find template {}.", template_path.display())
        }
    }

    #[cfg(test)]
    fn new() -> Self {
        Self {
            name: String::new(),
            body: String::new(),
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
                    "Function `print_molecule` received an object of type {val}, not `Molecule`"
                )));
            }
        },
        None => {
            return Err(tera::Error::msg(
                "Function `print_molecule` didn't receive a `molecule` argument",
            ))
        }
    };

    let formatted = molecule
        .atoms
        .iter()
        .map(std::string::ToString::to_string)
        .collect::<Vec<_>>()
        .join("\n");
    Ok(to_value(formatted)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use toml::Value;

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
            body: parsed_template,
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
}
