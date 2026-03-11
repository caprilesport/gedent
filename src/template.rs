use crate::config::Config;
use crate::Molecule;
use color_eyre::eyre::{bail, Report as Error, Result, WrapErr};
use comfy_table::{presets, Table};
use log::debug;
use serde_json::value::{from_value, to_value, Value};
use std::collections::HashMap;
use std::fs::{copy, read_dir, read_to_string};
use std::path::{Path, PathBuf};
use tera::Tera;
use walkdir::WalkDir;

const PRESETS_DIR: &str = "presets";
const TEMPLATES_DIR: &str = "templates";

#[derive(Clone, Debug, Default)]
pub struct TemplateMeta {
    #[allow(dead_code)] // reserved for method/software compatibility checks (item 19)
    pub software: Option<String>,
    #[allow(dead_code)] // reserved for workflow layer (item 20)
    pub jobtype: Option<String>,
    pub requires: Vec<String>,
    pub description: Option<String>,
}

#[derive(Clone, Debug)]
pub struct Template {
    pub name: String,
    pub meta: TemplateMeta,
    body: String,
}

impl Template {
    pub fn from_preset(software: String, template_name: &str) -> Result<(), Error> {
        let gedent_home = Config::gedent_home()?;
        let software_dir: PathBuf = [
            gedent_home.clone(),
            Into::into(TEMPLATES_DIR),
            Into::into(&software),
        ]
        .iter()
        .collect();
        std::fs::create_dir_all(&software_dir).wrap_err(format!(
            "Can't create template directory {}",
            software_dir.display()
        ))?;
        let template_path: PathBuf = software_dir.join(template_name);
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
        tera.register_function("print_coords", print_coords);
        tera.add_raw_template(&self.name, &self.body)?;
        Ok(tera.render(&self.name, context)?)
    }

    pub fn render_with_molecule(
        &self,
        context: &tera::Context,
        molecule: &Molecule,
        stem: &str,
    ) -> Result<String, Error> {
        let mut ctx = context.clone();
        ctx.insert("name", stem);
        ctx.insert("Molecule", molecule);
        self.render(&ctx)
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

    pub fn get(template_name: String, software: Option<&str>) -> Result<Self, Error> {
        let path = Self::find_path(&template_name, software)?;
        let body =
            read_to_string(&path).wrap_err(format!("Can't read template {template_name}"))?;
        let meta = parse_frontmatter(&body);
        Ok(Self {
            name: template_name,
            meta,
            body,
        })
    }

    pub fn print_template(template: &str, software: Option<&str>) -> Result<(), Error> {
        let template_path = Self::find_path(template, software)?;
        let body = read_to_string(&template_path)
            .wrap_err(format!("Cant find template {}", template_path.display()))?;
        println!("{body}");
        Ok(())
    }

    pub fn edit_template(template: &str, software: Option<&str>) -> Result<(), Error> {
        let template_path = Self::find_path(template, software)?;
        edit::edit_file(template_path)?;
        Ok(())
    }

    pub fn list_templates() -> Result<(), Error> {
        let templates_home: PathBuf = [Config::gedent_home()?, Into::into(TEMPLATES_DIR)]
            .iter()
            .collect();
        let mut templates = Self::get_templates(&templates_home);
        templates.sort();

        // Build (software, [(name, description)]) groups in sorted order.
        let mut groups: Vec<(String, Vec<(String, String)>)> = Vec::new();
        for t in &templates {
            let (sw, name) = t.split_once('/').map_or_else(
                || (String::new(), t.clone()),
                |(s, n)| (s.to_string(), n.to_string()),
            );
            let desc = read_to_string(templates_home.join(t))
                .ok()
                .and_then(|body| parse_frontmatter(&body).description)
                .unwrap_or_default();
            match groups.last_mut() {
                Some((g_sw, entries)) if *g_sw == sw => entries.push((name, desc)),
                _ => groups.push((sw, vec![(name, desc)])),
            }
        }

        for (sw, entries) in &groups {
            if !sw.is_empty() {
                println!("{sw}:");
            }
            let mut table = Table::new();
            table.load_preset(presets::NOTHING);
            for (name, desc) in entries {
                table.add_row(vec![format!("  {name}"), desc.clone()]);
            }
            println!("{table}");
        }
        Ok(())
    }

    /// Returns template names suitable for shell completion.
    ///
    /// Unambiguous short names (jobtype only, e.g. `neb`) are returned as-is.
    /// When a short name collides across software directories, the software-
    /// qualified full name is used (e.g. `orca/opt`, `xtb/opt`) — except that
    /// the short name is also included if `software` matches one of the
    /// candidates (since it would resolve unambiguously via the config).
    pub fn list_names(software: Option<&str>) -> Result<Vec<String>, Error> {
        let templates_home: PathBuf = [Config::gedent_home()?, Into::into(TEMPLATES_DIR)]
            .iter()
            .collect();
        let templates = Self::get_templates(&templates_home);

        // Count how many software dirs each short name appears in.
        let mut counts: HashMap<String, usize> = HashMap::new();
        for t in &templates {
            let short = t.split('/').next_back().unwrap_or(t).to_string();
            *counts.entry(short).or_insert(0) += 1;
        }

        let mut names: Vec<String> = templates
            .iter()
            .flat_map(|t| {
                let short = t.split('/').next_back().unwrap_or(t).to_string();
                if counts[&short] == 1 {
                    vec![short]
                } else if software.is_some_and(|sw| t == &format!("{sw}/{short}")) {
                    // Resolves unambiguously via configured software — offer
                    // both the short name and the full names of the others.
                    vec![short]
                } else {
                    vec![t.clone()]
                }
            })
            .collect();
        names.sort();
        names.dedup();
        Ok(names)
    }

    fn find_path(template: &str, software: Option<&str>) -> Result<PathBuf, Error> {
        let templates_home: PathBuf = [Config::gedent_home()?, Into::into(TEMPLATES_DIR)]
            .iter()
            .collect();

        // Full name (contains '/'): direct lookup, no disambiguation needed.
        if template.contains('/') {
            let path = templates_home.join(template);
            return if path.try_exists()? {
                Ok(path)
            } else {
                bail!("Can't find template {}.", path.display())
            };
        }

        // Short name: scan templates/*/name for matches.
        let mut matches: Vec<PathBuf> = read_dir(&templates_home)
            .wrap_err(format!(
                "Can't read templates directory {}",
                templates_home.display()
            ))?
            .filter_map(std::result::Result::ok)
            .filter(|e| e.file_type().is_ok_and(|t| t.is_dir()))
            .map(|e| e.path().join(template))
            .filter(|p| p.exists())
            .collect();

        match matches.len() {
            0 => bail!(
                "No template named \"{}\" found.\nHint: run `gedent template list` to see available templates.",
                template
            ),
            1 => {
                debug!("Template {:?} resolved to {}", template, matches[0].display());
                Ok(matches.remove(0))
            }
            _ => {
                // Use software config as tiebreaker.
                if let Some(sw) = software {
                    let tiebreak = templates_home.join(sw).join(template);
                    if tiebreak.try_exists()? {
                        debug!("Template {:?} resolved to {} via software tiebreaker {:?}", template, tiebreak.display(), sw);
                        return Ok(tiebreak);
                    }
                }
                let names: Vec<String> = matches
                    .iter()
                    .filter_map(|p| {
                        p.strip_prefix(&templates_home)
                            .ok()
                            .map(|rel| rel.to_string_lossy().into_owned())
                    })
                    .collect();
                bail!(
                    "Template \"{}\" is ambiguous: {}.\nHint: use the full name (e.g. `gedent gen {}`) or set `software` in gedent.toml.",
                    template,
                    names.join(", "),
                    names[0],
                )
            }
        }
    }

    #[cfg(test)]
    fn new() -> Self {
        Self {
            name: String::new(),
            meta: TemplateMeta::default(),
            body: String::new(),
        }
    }

    #[cfg(test)]
    pub fn with_body(name: &str, body: &str) -> Self {
        Self {
            name: name.to_string(),
            meta: TemplateMeta::default(),
            body: body.to_string(),
        }
    }
}

fn parse_frontmatter(body: &str) -> TemplateMeta {
    let Some(start) = body.find("{#") else {
        return TemplateMeta::default();
    };
    let inner_start = start + 2;
    let Some(end_offset) = body[inner_start..].find("#}") else {
        return TemplateMeta::default();
    };
    let end = inner_start + end_offset;
    let raw = body[inner_start..end].trim();
    let table: toml::Table = match toml::from_str(raw) {
        Ok(t) => t,
        Err(_) => return TemplateMeta::default(),
    };
    TemplateMeta {
        software: table
            .get("software")
            .and_then(|v| v.as_str())
            .map(str::to_string),
        jobtype: table
            .get("jobtype")
            .and_then(|v| v.as_str())
            .map(str::to_string),
        requires: table
            .get("requires")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str())
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default(),
        description: table
            .get("description")
            .and_then(|v| v.as_str())
            .map(str::to_string),
    }
}

// functions for the templates
fn print_coords(args: &HashMap<String, Value>) -> Result<Value, tera::Error> {
    let molecule: Molecule = match args.get("molecule") {
        Some(val) => match from_value(val.clone()) {
            Ok(v) => v,
            Err(_) => {
                return Err(tera::Error::msg(format!(
                    "Function `print_coords` received an object of type {val}, not `Molecule`"
                )));
            }
        },
        None => {
            return Err(tera::Error::msg(
                "Function `print_coords` didn't receive a `molecule` argument",
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
    use crate::elements::Element;
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

    #[test]
    fn render_with_molecule_inserts_name_and_molecule() {
        use crate::molecule::{Atom, Molecule};

        let template = Template {
            body: "{{ name }} {{ Molecule.atoms | length }}".to_string(),
            ..Template::new()
        };
        let molecule = Molecule {
            description: None,
            atoms: vec![
                Atom {
                    element: Element::C,
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                },
                Atom {
                    element: Element::H,
                    x: 1.0,
                    y: 0.0,
                    z: 0.0,
                },
            ],
        };
        let result = template
            .render_with_molecule(&tera::Context::new(), &molecule, "mymol")
            .unwrap();
        assert_eq!(result, "mymol 2");
    }

    #[test]
    fn parse_frontmatter_works() {
        let body = "{#\nsoftware = \"orca\"\njobtype = \"sp\"\nrequires = [\"method\", \"basis_set\"]\ndescription = \"Single point\"\n#}\n! {{ method }}";
        let meta = parse_frontmatter(body);
        assert_eq!(meta.software.as_deref(), Some("orca"));
        assert_eq!(meta.jobtype.as_deref(), Some("sp"));
        assert_eq!(meta.requires, vec!["method", "basis_set"]);
        assert_eq!(meta.description.as_deref(), Some("Single point"));
    }

    #[test]
    fn parse_frontmatter_missing_returns_default() {
        let body = "! {{ method }} {{ basis_set }}";
        let meta = parse_frontmatter(body);
        assert!(meta.software.is_none());
        assert!(meta.requires.is_empty());
    }

    #[test]
    fn print_coords_formats_atoms_correctly() {
        use crate::molecule::{Atom, Molecule};

        let template = Template::with_body("t", "{{ print_coords(molecule=Molecule) }}");
        let molecule = Molecule {
            description: None,
            atoms: vec![
                Atom {
                    element: Element::C,
                    x: 1.5,
                    y: -2.0,
                    z: 0.5,
                },
                Atom {
                    element: Element::H,
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                },
            ],
        };
        let result = template
            .render_with_molecule(&tera::Context::new(), &molecule, "t")
            .unwrap();
        let expected = molecule
            .atoms
            .iter()
            .map(std::string::ToString::to_string)
            .collect::<Vec<_>>()
            .join("\n");
        assert_eq!(result, expected);
    }
}
