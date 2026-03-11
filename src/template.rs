use crate::config::Config;
use crate::elements::Element;
use crate::molecule::Atom;
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

/// Metadata parsed from a template's frontmatter comment block.
///
/// Frontmatter is a Tera comment at the top of the template file:
/// ```text
/// {#
/// software = "orca"
/// jobtype  = "sp"
/// requires = ["method", "basis_set", "charge", "mult", "nprocs", "mem", "Molecule"]
/// description = "Single point energy"
/// #}
/// ```
#[derive(Clone, Debug, Default)]
pub struct TemplateMeta {
    /// Software the template targets (e.g. `"orca"`). Used for disambiguation
    /// when multiple templates share a short name.
    #[allow(dead_code)] // reserved for method/software compatibility checks (item 19)
    pub software: Option<String>,
    /// Job type (e.g. `"sp"`, `"opt"`). Reserved for the workflow layer.
    #[allow(dead_code)] // reserved for workflow layer (item 20)
    pub jobtype: Option<String>,
    /// Context variables that must be present before rendering. gedent reports
    /// a clear error listing any that are missing.
    pub requires: Vec<String>,
    /// Human-readable description shown in `gedent template list`.
    pub description: Option<String>,
}

/// A loaded template ready for rendering.
#[derive(Clone, Debug)]
pub struct Template {
    /// Template name as provided by the user (e.g. `"sp"` or `"orca/sp"`).
    pub name: String,
    /// Parsed frontmatter.
    pub meta: TemplateMeta,
    /// Raw template body (Tera source).
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
        tera.register_function("natoms", natoms);
        tera.register_function("count_element", count_element);
        tera.register_function("element_list", element_list);
        tera.register_function("atom_symbol", atom_symbol);
        tera.register_function("atom_coords", atom_coords);
        tera.register_function("measure", measure);
        tera.add_raw_template(&self.name, &self.body)?;
        Ok(tera.render(&self.name, context)?)
    }

    /// Render the template with a molecule injected into context.
    ///
    /// Injects `name` (the xyz file stem) and `Molecule` on top of `context`,
    /// then calls [`Template::render`].
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

    /// Resolve a template by name, load it from disk, and parse its frontmatter.
    ///
    /// `template_name` may be a short name (`"sp"`) or a qualified path
    /// (`"orca/sp"`). When multiple templates share a short name, `software`
    /// is used as a tiebreaker.
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

// ── Tera function helpers ─────────────────────────────────────────────────────

fn get_molecule(args: &HashMap<String, Value>) -> Result<Molecule, tera::Error> {
    args.get("molecule").map_or_else(
        || Err(tera::Error::msg("missing required `molecule` argument")),
        |val| {
            from_value(val.clone())
                .map_err(|_| tera::Error::msg("received an invalid `molecule` argument"))
        },
    )
}

/// Convert a 1-based `i` argument to a 0-based array index, checking bounds.
fn get_index(
    args: &HashMap<String, Value>,
    n_atoms: usize,
    fn_name: &str,
) -> Result<usize, tera::Error> {
    let i = args
        .get("i")
        .and_then(Value::as_i64)
        .ok_or_else(|| tera::Error::msg(format!("{fn_name}: requires an integer `i` (1-based)")))?;
    let idx = usize::try_from(i)
        .ok()
        .filter(|&idx| idx >= 1 && idx <= n_atoms)
        .ok_or_else(|| {
            tera::Error::msg(format!(
                "{fn_name}: index {i} out of range \
                 (molecule has {n_atoms} atoms, indices are 1-based)"
            ))
        })?;
    Ok(idx - 1)
}

// ── Tera functions ────────────────────────────────────────────────────────────

fn print_coords(args: &HashMap<String, Value>) -> Result<Value, tera::Error> {
    let molecule = get_molecule(args)?;
    let formatted = molecule
        .atoms
        .iter()
        .map(std::string::ToString::to_string)
        .collect::<Vec<_>>()
        .join("\n");
    Ok(to_value(formatted)?)
}

fn natoms(args: &HashMap<String, Value>) -> Result<Value, tera::Error> {
    let mol = get_molecule(args)?;
    Ok(to_value(mol.atoms.len())?)
}

fn count_element(args: &HashMap<String, Value>) -> Result<Value, tera::Error> {
    let mol = get_molecule(args)?;
    let symbol = args
        .get("symbol")
        .and_then(|v| v.as_str())
        .ok_or_else(|| tera::Error::msg("count_element: requires a `symbol` string argument"))?;
    let element = symbol.parse::<Element>().map_err(|_| {
        tera::Error::msg(format!("count_element: unknown element symbol {symbol:?}"))
    })?;
    let count = mol.atoms.iter().filter(|a| a.element == element).count();
    Ok(to_value(count)?)
}

fn element_list(args: &HashMap<String, Value>) -> Result<Value, tera::Error> {
    let mol = get_molecule(args)?;
    let mut seen = std::collections::HashSet::new();
    let mut elements: Vec<Element> = mol
        .atoms
        .iter()
        .map(|a| a.element)
        .filter(|e| seen.insert(*e))
        .collect();
    elements.sort();
    Ok(to_value(
        elements
            .iter()
            .map(std::string::ToString::to_string)
            .collect::<Vec<_>>(),
    )?)
}

fn atom_symbol(args: &HashMap<String, Value>) -> Result<Value, tera::Error> {
    let mol = get_molecule(args)?;
    let idx = get_index(args, mol.atoms.len(), "atom_symbol")?;
    Ok(to_value(mol.atoms[idx].element.to_string())?)
}

fn atom_coords(args: &HashMap<String, Value>) -> Result<Value, tera::Error> {
    let mol = get_molecule(args)?;
    let idx = get_index(args, mol.atoms.len(), "atom_coords")?;
    let a = &mol.atoms[idx];
    Ok(to_value(vec![a.x, a.y, a.z])?)
}

fn measure(args: &HashMap<String, Value>) -> Result<Value, tera::Error> {
    let mol = get_molecule(args)?;
    let raw = args
        .get("atoms")
        .and_then(|v| v.as_array())
        .ok_or_else(|| {
            tera::Error::msg(
                "measure: requires an `atoms` array of 1-based integer indices (2, 3, or 4)",
            )
        })?;

    let n_atoms = mol.atoms.len();
    let indices: Vec<usize> = raw
        .iter()
        .map(|v| {
            let i = v
                .as_i64()
                .ok_or_else(|| tera::Error::msg("measure: atom indices must be integers"))?;
            usize::try_from(i)
                .ok()
                .filter(|&idx| idx >= 1 && idx <= n_atoms)
                .map(|idx| idx - 1)
                .ok_or_else(|| {
                    tera::Error::msg(format!(
                        "measure: index {i} out of range \
                         (molecule has {n_atoms} atoms, indices are 1-based)"
                    ))
                })
        })
        .collect::<Result<_, _>>()?;

    match indices.len() {
        2 => Ok(to_value(calc_distance(
            &mol.atoms[indices[0]],
            &mol.atoms[indices[1]],
        ))?),
        3 => Ok(to_value(calc_angle(
            &mol.atoms[indices[0]],
            &mol.atoms[indices[1]],
            &mol.atoms[indices[2]],
        )?)?),
        4 => Ok(to_value(calc_dihedral(
            &mol.atoms[indices[0]],
            &mol.atoms[indices[1]],
            &mol.atoms[indices[2]],
            &mol.atoms[indices[3]],
        )?)?),
        n => Err(tera::Error::msg(format!(
            "measure: expected 2, 3, or 4 atom indices, got {n}"
        ))),
    }
}

// ── Geometry primitives ───────────────────────────────────────────────────────

fn vec3(a: &Atom, b: &Atom) -> [f64; 3] {
    [b.x - a.x, b.y - a.y, b.z - a.z]
}

fn dot(a: &[f64; 3], b: &[f64; 3]) -> f64 {
    a[2].mul_add(b[2], a[1].mul_add(b[1], a[0] * b[0]))
}

fn cross(a: &[f64; 3], b: &[f64; 3]) -> [f64; 3] {
    [
        a[1].mul_add(b[2], -(a[2] * b[1])),
        a[2].mul_add(b[0], -(a[0] * b[2])),
        a[0].mul_add(b[1], -(a[1] * b[0])),
    ]
}

fn norm(v: &[f64; 3]) -> f64 {
    v[2].mul_add(v[2], v[0].mul_add(v[0], v[1] * v[1])).sqrt()
}

fn calc_distance(a: &Atom, b: &Atom) -> f64 {
    norm(&vec3(a, b))
}

fn calc_angle(a: &Atom, b: &Atom, c: &Atom) -> Result<f64, tera::Error> {
    let v1 = vec3(b, a); // vectors away from central atom b
    let v2 = vec3(b, c);
    let n1 = norm(&v1);
    let n2 = norm(&v2);
    if n1 < 1e-10 || n2 < 1e-10 {
        return Err(tera::Error::msg(
            "measure: coincident atoms — angle is undefined",
        ));
    }
    let cos_theta = (dot(&v1, &v2) / (n1 * n2)).clamp(-1.0, 1.0);
    Ok(cos_theta.acos().to_degrees())
}

/// Dihedral angle a–b–c–d using the atan2 formula (range −180°..180°).
#[allow(clippy::many_single_char_names)]
fn calc_dihedral(a: &Atom, b: &Atom, c: &Atom, d: &Atom) -> Result<f64, tera::Error> {
    let b1 = vec3(a, b);
    let b2 = vec3(b, c);
    let b3 = vec3(c, d);
    let n = norm(&b2);
    if n < 1e-10 {
        return Err(tera::Error::msg(
            "measure: coincident central atoms — dihedral is undefined",
        ));
    }
    let n1 = cross(&b1, &b2);
    let n2 = cross(&b2, &b3);
    let m = cross(&n1, &b2);
    let x = dot(&n1, &n2);
    let y = dot(&m, &n2) / n;
    Ok(y.atan2(x).to_degrees())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::elements::Element;
    use crate::molecule::{Atom, Molecule};
    use toml::Value;

    /// A four-atom molecule with exact geometry:
    ///
    /// - atom 1 (H): (1, 0, 0)
    /// - atom 2 (C): (0, 0, 0)
    /// - atom 3 (N): (0, 1, 0)
    /// - atom 4 (O): (0, 1, 1)
    ///
    /// Known values (1-based indices):
    ///   distance(1,2) = 1.0 Å
    ///   angle(1,2,3)  = 90.0°
    ///   dihedral(1,2,3,4) = 90.0°
    fn geo_mol() -> Molecule {
        Molecule {
            description: None,
            atoms: vec![
                Atom {
                    element: Element::H,
                    x: 1.0,
                    y: 0.0,
                    z: 0.0,
                },
                Atom {
                    element: Element::C,
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                },
                Atom {
                    element: Element::N,
                    x: 0.0,
                    y: 1.0,
                    z: 0.0,
                },
                Atom {
                    element: Element::O,
                    x: 0.0,
                    y: 1.0,
                    z: 1.0,
                },
            ],
        }
    }

    fn render(body: &str, mol: &Molecule) -> String {
        Template::with_body("t", body)
            .render_with_molecule(&tera::Context::new(), mol, "t")
            .unwrap()
    }

    fn render_err(body: &str, mol: &Molecule) -> String {
        // Use debug format to include the full cause chain — Tera nests the
        // actual function error message one level deep.
        format!(
            "{:?}",
            Template::with_body("t", body)
                .render_with_molecule(&tera::Context::new(), mol, "t")
                .unwrap_err()
        )
    }

    fn parse_f64(s: &str) -> f64 {
        s.trim().parse::<f64>().expect("expected a float")
    }

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

    // ── natoms ────────────────────────────────────────────────────────────────

    #[test]
    fn natoms_returns_count() {
        assert_eq!(render("{{ natoms(molecule=Molecule) }}", &geo_mol()), "4");
    }

    // ── count_element ─────────────────────────────────────────────────────────

    #[test]
    fn count_element_known_element() {
        assert_eq!(
            render(
                "{{ count_element(molecule=Molecule, symbol='H') }}",
                &geo_mol()
            ),
            "1"
        );
    }

    #[test]
    fn count_element_absent_element() {
        assert_eq!(
            render(
                "{{ count_element(molecule=Molecule, symbol='Fe') }}",
                &geo_mol()
            ),
            "0"
        );
    }

    // ── element_list ──────────────────────────────────────────────────────────

    #[test]
    fn element_list_sorted_unique() {
        // H(1) < C(6) < N(7) < O(8) — sorted by atomic number via Ord on Element
        // Tera renders string arrays without surrounding quotes on each element.
        assert_eq!(
            render("{{ element_list(molecule=Molecule) }}", &geo_mol()),
            "[H, C, N, O]"
        );
    }

    #[test]
    fn element_list_deduplicates() {
        let mol = Molecule {
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
                Atom {
                    element: Element::C,
                    x: -1.0,
                    y: 0.0,
                    z: 0.0,
                },
            ],
        };
        assert_eq!(
            render("{{ element_list(molecule=Molecule) }}", &mol),
            "[H, C]"
        );
    }

    // ── atom_symbol / atom_coords ─────────────────────────────────────────────

    #[test]
    fn atom_symbol_first_atom() {
        assert_eq!(
            render("{{ atom_symbol(molecule=Molecule, i=1) }}", &geo_mol()),
            "H"
        );
    }

    #[test]
    fn atom_symbol_out_of_bounds_errors() {
        let err = render_err("{{ atom_symbol(molecule=Molecule, i=99) }}", &geo_mol());
        assert!(err.contains("out of range"));
    }

    #[test]
    fn atom_coords_first_atom() {
        // Tera renders whole-number floats without a decimal point.
        assert_eq!(
            render("{{ atom_coords(molecule=Molecule, i=1) }}", &geo_mol()),
            "[1, 0, 0]"
        );
    }

    // ── measure: distance ─────────────────────────────────────────────────────

    #[test]
    fn measure_distance_exact() {
        let v = parse_f64(&render(
            "{{ measure(molecule=Molecule, atoms=[1,2]) }}",
            &geo_mol(),
        ));
        approx::assert_relative_eq!(v, 1.0);
    }

    #[test]
    fn measure_distance_pythagorean() {
        // (0,0,0) to (3,4,0): distance = 5
        let mol = Molecule {
            description: None,
            atoms: vec![
                Atom {
                    element: Element::H,
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                },
                Atom {
                    element: Element::H,
                    x: 3.0,
                    y: 4.0,
                    z: 0.0,
                },
            ],
        };
        let v = parse_f64(&render(
            "{{ measure(molecule=Molecule, atoms=[1,2]) }}",
            &mol,
        ));
        approx::assert_relative_eq!(v, 5.0);
    }

    // ── measure: angle ────────────────────────────────────────────────────────

    #[test]
    fn measure_angle_right_angle() {
        let v = parse_f64(&render(
            "{{ measure(molecule=Molecule, atoms=[1,2,3]) }}",
            &geo_mol(),
        ));
        approx::assert_relative_eq!(v, 90.0);
    }

    #[test]
    fn measure_angle_180_degrees() {
        let mol = Molecule {
            description: None,
            atoms: vec![
                Atom {
                    element: Element::H,
                    x: -1.0,
                    y: 0.0,
                    z: 0.0,
                },
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
        let v = parse_f64(&render(
            "{{ measure(molecule=Molecule, atoms=[1,2,3]) }}",
            &mol,
        ));
        approx::assert_relative_eq!(v, 180.0);
    }

    // ── measure: dihedral ─────────────────────────────────────────────────────

    #[test]
    fn measure_dihedral_90_degrees() {
        let v = parse_f64(&render(
            "{{ measure(molecule=Molecule, atoms=[1,2,3,4]) }}",
            &geo_mol(),
        ));
        approx::assert_relative_eq!(v, 90.0);
    }

    #[test]
    fn measure_dihedral_180_degrees() {
        // a=(0,1,0) b=(0,0,0) c=(0,0,1) d=(0,-1,1) → trans, 180°
        let mol = Molecule {
            description: None,
            atoms: vec![
                Atom {
                    element: Element::H,
                    x: 0.0,
                    y: 1.0,
                    z: 0.0,
                },
                Atom {
                    element: Element::C,
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                },
                Atom {
                    element: Element::C,
                    x: 0.0,
                    y: 0.0,
                    z: 1.0,
                },
                Atom {
                    element: Element::H,
                    x: 0.0,
                    y: -1.0,
                    z: 1.0,
                },
            ],
        };
        let v = parse_f64(&render(
            "{{ measure(molecule=Molecule, atoms=[1,2,3,4]) }}",
            &mol,
        ));
        approx::assert_relative_eq!(v, 180.0, epsilon = 1e-10);
    }

    // ── measure: error cases ──────────────────────────────────────────────────

    #[test]
    fn measure_wrong_atom_count_errors() {
        let err = render_err("{{ measure(molecule=Molecule, atoms=[1]) }}", &geo_mol());
        assert!(err.contains("expected 2, 3, or 4"));
    }

    #[test]
    fn measure_out_of_bounds_errors() {
        let err = render_err(
            "{{ measure(molecule=Molecule, atoms=[1, 99]) }}",
            &geo_mol(),
        );
        assert!(err.contains("out of range"));
    }

    #[test]
    fn measure_zero_index_errors() {
        let err = render_err("{{ measure(molecule=Molecule, atoms=[0, 1]) }}", &geo_mol());
        assert!(err.contains("out of range"));
    }

    // ── print_coords ──────────────────────────────────────────────────────────

    #[test]
    fn print_coords_formats_atoms_correctly() {
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
