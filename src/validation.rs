use crate::molecule::Molecule;
use crate::software::SoftwareDb;
use std::fmt;

/// Severity of a validation [`Diagnostic`].
#[derive(Debug, PartialEq, Eq)]
pub enum Severity {
    /// Generation is aborted; all errors across all inputs are reported first.
    Error,
    /// Generation proceeds; the message is printed as a warning.
    Warning,
}

/// A single validation finding with a severity and a human-readable message.
#[derive(Debug)]
pub struct Diagnostic {
    /// Whether this finding aborts generation or just warns.
    pub severity: Severity,
    /// Human-readable description of the problem.
    pub message: String,
}

impl Diagnostic {
    /// Create an error-severity diagnostic.
    pub fn error(message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Error,
            message: message.into(),
        }
    }

    /// Create a warning-severity diagnostic.
    pub fn warning(message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Warning,
            message: message.into(),
        }
    }
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let prefix = match self.severity {
            Severity::Error => "error",
            Severity::Warning => "warning",
        };
        write!(f, "{prefix}: {}", self.message)
    }
}

/// Run all validation checks and return every finding as a [`Vec<Diagnostic>`].
///
/// Molecule-specific checks (charge/mult parity, superposed atoms) are skipped
/// when `molecule` is `None`. `software` is the resolved software name from
/// config/CLI options and is used for compatibility rule matching.
pub fn validate(
    molecule: Option<&Molecule>,
    context: &tera::Context,
    requires: &[String],
    db: &SoftwareDb,
    software: Option<&str>,
) -> Vec<Diagnostic> {
    let mut diags = vec![];
    if let Some(mol) = molecule {
        diags.extend(check_superposed_atoms(mol));
        diags.extend(check_charge_mult(mol, context));
    }
    // "Molecule" is injected per-render in render_with_molecule(), not into the
    // base context. Skip it from the missing-vars check when a molecule is provided.
    let filtered_requires: Vec<String>;
    let effective_requires: &[String] = if molecule.is_some() {
        filtered_requires = requires
            .iter()
            .filter(|k| k.as_str() != "Molecule")
            .cloned()
            .collect();
        &filtered_requires
    } else {
        requires
    };
    diags.extend(check_missing_vars(context, effective_requires));
    diags.extend(check_compat(context, db, software));
    diags.extend(check_method_vars(context, db));
    diags
}

fn check_charge_mult(molecule: &Molecule, context: &tera::Context) -> Vec<Diagnostic> {
    let json = context.clone().into_json();
    let charge = json.get("charge").and_then(serde_json::Value::as_i64);
    let mult = json.get("mult").and_then(serde_json::Value::as_i64);

    // Skip silently if charge/mult are not in context — they may not be needed.
    let (Some(charge), Some(mult)) = (charge, mult) else {
        return vec![];
    };

    let total_z: i64 = molecule
        .atoms
        .iter()
        .map(|a| i64::from(a.element as u8))
        .sum();
    let electrons = total_z - charge;

    let mut diags = vec![];

    if electrons < 0 {
        diags.push(Diagnostic::error(format!(
            "charge {charge} gives a negative electron count ({electrons})"
        )));
        return diags;
    }

    if mult < 1 {
        diags.push(Diagnostic::error(format!(
            "multiplicity must be >= 1, got {mult}"
        )));
        return diags;
    }

    let unpaired = mult - 1;
    if electrons < unpaired {
        diags.push(Diagnostic::error(format!(
            "multiplicity {mult} requires {unpaired} unpaired electrons \
             but molecule only has {electrons} electrons"
        )));
    } else if (electrons - unpaired) % 2 != 0 {
        diags.push(Diagnostic::error(format!(
            "charge {charge} and multiplicity {mult} are inconsistent: \
             {electrons} electrons cannot accommodate {unpaired} unpaired electrons"
        )));
    }

    diags
}

fn check_superposed_atoms(molecule: &Molecule) -> Vec<Diagnostic> {
    const ERROR_THRESHOLD: f64 = 0.5; // Å — definitely same point
    const RADII_FACTOR: f64 = 0.5; // fraction of sum of covalent radii

    let mut diags = vec![];
    let atoms = &molecule.atoms;
    for i in 0..atoms.len() {
        for j in (i + 1)..atoms.len() {
            let dx = atoms[i].x - atoms[j].x;
            let dy = atoms[i].y - atoms[j].y;
            let dz = atoms[i].z - atoms[j].z;
            let dist = dz.mul_add(dz, dx.mul_add(dx, dy * dy)).sqrt();

            if dist < ERROR_THRESHOLD {
                diags.push(Diagnostic::error(format!(
                    "atoms {} ({}) and {} ({}) are superposed: distance {dist:.3} Å",
                    i + 1,
                    atoms[i].element,
                    j + 1,
                    atoms[j].element,
                )));
            } else {
                let ri = atoms[i].element.get_radius().map(f64::from);
                let rj = atoms[j].element.get_radius().map(f64::from);
                if let (Some(ri), Some(rj)) = (ri, rj) {
                    let threshold = RADII_FACTOR * (ri + rj);
                    if dist < threshold {
                        diags.push(Diagnostic::warning(format!(
                            "atoms {} ({}) and {} ({}) are unusually close: \
                             {dist:.3} Å (sum of covalent radii = {:.3} Å)",
                            i + 1,
                            atoms[i].element,
                            j + 1,
                            atoms[j].element,
                            ri + rj,
                        )));
                    }
                }
            }
        }
    }
    diags
}

fn check_missing_vars(context: &tera::Context, requires: &[String]) -> Vec<Diagnostic> {
    let json = context.clone().into_json();
    requires
        .iter()
        .filter(|k| json.get(k.as_str()).is_none())
        .map(|k| {
            Diagnostic::error(format!(
                "template requires `{k}` but it is not defined in context"
            ))
        })
        .collect()
}

fn check_compat(
    context: &tera::Context,
    db: &SoftwareDb,
    software: Option<&str>,
) -> Vec<Diagnostic> {
    let json = context.clone().into_json();
    let method = json
        .get("method")
        .and_then(|v| v.as_str())
        .map(str::to_lowercase);
    let solvation = json
        .get("solvation")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);
    let solvation_model = json
        .get("solvation_model")
        .and_then(|v| v.as_str())
        .map(str::to_lowercase);

    let mut diags = vec![];

    for rule in &db.compat {
        let method_matches = rule.method.as_deref().map_or(true, |rm| {
            method.as_deref() == Some(rm.to_lowercase().as_str())
        });
        let software_matches = rule.software.as_deref().map_or(true, |rs| {
            software.is_some_and(|s| s.to_lowercase() == rs.to_lowercase())
        });

        if !method_matches || !software_matches {
            continue;
        }

        if let Some(required_model) = &rule.require_solvation_model {
            if solvation {
                let model_ok = solvation_model
                    .as_deref()
                    .is_some_and(|m| m == required_model.to_lowercase().as_str());
                if !model_ok {
                    let msg = rule.message.as_deref().unwrap_or(
                        "incompatible solvation model for this method/software combination",
                    );
                    diags.push(Diagnostic::error(msg));
                }
            }
        }
    }

    diags
}

fn check_method_vars(context: &tera::Context, db: &SoftwareDb) -> Vec<Diagnostic> {
    let json = context.clone().into_json();
    let method_str = match json.get("method").and_then(|v| v.as_str()) {
        Some(m) => m.to_owned(),
        None => return vec![],
    };

    let Some(entry) = db.get_method(&method_str) else {
        return vec![];
    };

    let mut diags = vec![];

    if entry.has_own_basis && json.get("basis_set").is_some() {
        diags.push(Diagnostic::warning(format!(
            "{method_str} has its own basis set; \
             configured basis_set will be ignored by the template"
        )));
    }
    if entry.has_own_dispersion && json.get("dispersion").is_some() {
        diags.push(Diagnostic::warning(format!(
            "{method_str} has its own dispersion correction; \
             configured dispersion will be ignored by the template"
        )));
    }

    diags
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::elements::Element;
    use crate::molecule::{Atom, Molecule};

    fn make_molecule(atoms: Vec<(Element, f64, f64, f64)>) -> Molecule {
        Molecule {
            description: None,
            atoms: atoms
                .into_iter()
                .map(|(element, x, y, z)| Atom { element, x, y, z })
                .collect(),
        }
    }

    fn ctx_with_ints(pairs: &[(&str, i64)]) -> tera::Context {
        let mut ctx = tera::Context::new();
        for (k, v) in pairs {
            ctx.insert(*k, v);
        }
        ctx
    }

    fn empty_db() -> SoftwareDb {
        SoftwareDb::default()
    }

    // ── charge/mult ────────────────────────────────────────────────────────────

    #[test]
    fn charge_mult_valid_singlet() {
        // C + 4H = 6 + 4 = 10 electrons, charge=0, mult=1 → (10-0)%2 == 0 ✓
        let mol = make_molecule(vec![
            (Element::C, 0.0, 0.0, 0.0),
            (Element::H, 1.0, 0.0, 0.0),
            (Element::H, -1.0, 0.0, 0.0),
            (Element::H, 0.0, 1.0, 0.0),
            (Element::H, 0.0, -1.0, 0.0),
        ]);
        assert!(check_charge_mult(&mol, &ctx_with_ints(&[("charge", 0), ("mult", 1)])).is_empty());
    }

    #[test]
    fn charge_mult_valid_doublet_radical() {
        // H radical: 1 electron, charge=0, mult=2 → (1-1)%2 == 0 ✓
        let mol = make_molecule(vec![(Element::H, 0.0, 0.0, 0.0)]);
        assert!(check_charge_mult(&mol, &ctx_with_ints(&[("charge", 0), ("mult", 2)])).is_empty());
    }

    #[test]
    fn charge_mult_valid_cation() {
        // CH4+: 9 electrons, charge=1, mult=2 → (9-1)%2 == 0 ✓
        let mol = make_molecule(vec![
            (Element::C, 0.0, 0.0, 0.0),
            (Element::H, 1.0, 0.0, 0.0),
            (Element::H, -1.0, 0.0, 0.0),
            (Element::H, 0.0, 1.0, 0.0),
            (Element::H, 0.0, -1.0, 0.0),
        ]);
        assert!(check_charge_mult(&mol, &ctx_with_ints(&[("charge", 1), ("mult", 2)])).is_empty());
    }

    #[test]
    fn charge_mult_wrong_parity() {
        // H (1 electron), mult=1 → unpaired=0, (1-0)%2!=0 → error
        let mol = make_molecule(vec![(Element::H, 0.0, 0.0, 0.0)]);
        let diags = check_charge_mult(&mol, &ctx_with_ints(&[("charge", 0), ("mult", 1)]));
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].severity, Severity::Error);
    }

    #[test]
    fn charge_mult_negative_electrons() {
        let mol = make_molecule(vec![(Element::H, 0.0, 0.0, 0.0)]);
        let diags = check_charge_mult(&mol, &ctx_with_ints(&[("charge", 10), ("mult", 1)]));
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].severity, Severity::Error);
    }

    #[test]
    fn charge_mult_skipped_when_not_in_context() {
        let mol = make_molecule(vec![(Element::H, 0.0, 0.0, 0.0)]);
        assert!(check_charge_mult(&mol, &tera::Context::new()).is_empty());
    }

    // ── superposed atoms ───────────────────────────────────────────────────────

    #[test]
    fn superposed_atoms_detects_overlap() {
        let mol = make_molecule(vec![
            (Element::C, 0.0, 0.0, 0.0),
            (Element::H, 0.1, 0.0, 0.0), // 0.1 Å — clearly superposed
        ]);
        let diags = check_superposed_atoms(&mol);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].severity, Severity::Error);
    }

    #[test]
    fn superposed_atoms_ok_for_normal_bond() {
        let mol = make_molecule(vec![
            (Element::C, 0.0, 0.0, 0.0),
            (Element::H, 1.089, 0.0, 0.0), // typical C-H bond length
        ]);
        assert!(check_superposed_atoms(&mol).is_empty());
    }

    #[test]
    fn superposed_atoms_reports_all_overlapping_pairs() {
        // Three atoms all at the origin — should give 3 pairs
        let mol = make_molecule(vec![
            (Element::C, 0.0, 0.0, 0.0),
            (Element::H, 0.0, 0.0, 0.0),
            (Element::H, 0.0, 0.0, 0.0),
        ]);
        assert_eq!(check_superposed_atoms(&mol).len(), 3);
    }

    #[test]
    fn superposed_atoms_warns_on_radii_clash() {
        // H covalent radius = 0.31 Å; threshold = 0.5 * (0.31 + 0.31) = 0.31 Å
        // Distance 0.25 Å: above 0.5 hard threshold but below 0.31 radii threshold
        // Wait, 0.25 < 0.5 so this would be an error, not a warning.
        // Use dist = 0.52 Å (above 0.5 error threshold, below radii threshold for C-C):
        // C radius = 0.77 Å; threshold = 0.5 * (0.77 + 0.77) = 0.77 Å
        // 0.52 > 0.5 → not an error; 0.52 < 0.77 → warning
        let mol = make_molecule(vec![
            (Element::C, 0.0, 0.0, 0.0),
            (Element::C, 0.52, 0.0, 0.0),
        ]);
        let diags = check_superposed_atoms(&mol);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].severity, Severity::Warning);
    }

    // ── missing vars ───────────────────────────────────────────────────────────

    #[test]
    fn missing_vars_detects_absent_key() {
        let mut ctx = tera::Context::new();
        ctx.insert("method", "pbe0");
        let requires = vec!["method".to_string(), "basis_set".to_string()];
        let diags = check_missing_vars(&ctx, &requires);
        assert_eq!(diags.len(), 1);
        assert!(diags[0].message.contains("basis_set"));
        assert_eq!(diags[0].severity, Severity::Error);
    }

    #[test]
    fn missing_vars_empty_when_all_present() {
        let mut ctx = tera::Context::new();
        ctx.insert("method", "pbe0");
        ctx.insert("basis_set", "def2-tzvp");
        assert!(
            check_missing_vars(&ctx, &["method".to_string(), "basis_set".to_string()]).is_empty()
        );
    }

    #[test]
    fn missing_vars_empty_requires() {
        assert!(check_missing_vars(&tera::Context::new(), &[]).is_empty());
    }

    // ── check_compat ───────────────────────────────────────────────────────────

    #[test]
    fn compat_xtb_orca_no_solvation_ok() {
        // Rule fires only when solvation is active
        let mut db = SoftwareDb::default();
        db.compat.push(crate::software::CompatRule {
            method: Some("xtb".into()),
            software: Some("orca".into()),
            require_solvation_model: Some("alpb".into()),
            message: None,
        });
        let mut ctx = tera::Context::new();
        ctx.insert("method", "xtb");
        // no solvation
        assert!(check_compat(&ctx, &db, Some("orca")).is_empty());
    }

    #[test]
    fn compat_xtb_orca_wrong_model_errors() {
        let mut db = SoftwareDb::default();
        db.compat.push(crate::software::CompatRule {
            method: Some("xtb".into()),
            software: Some("orca".into()),
            require_solvation_model: Some("alpb".into()),
            message: Some("XTB in ORCA requires ALPB".into()),
        });
        let mut ctx = tera::Context::new();
        ctx.insert("method", "xtb");
        ctx.insert("solvation", &true);
        ctx.insert("solvation_model", "cpcm");
        let diags = check_compat(&ctx, &db, Some("orca"));
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].severity, Severity::Error);
        assert!(diags[0].message.contains("ALPB"));
    }

    #[test]
    fn compat_xtb_orca_correct_model_ok() {
        let mut db = SoftwareDb::default();
        db.compat.push(crate::software::CompatRule {
            method: Some("xtb".into()),
            software: Some("orca".into()),
            require_solvation_model: Some("alpb".into()),
            message: None,
        });
        let mut ctx = tera::Context::new();
        ctx.insert("method", "xtb");
        ctx.insert("solvation", &true);
        ctx.insert("solvation_model", "alpb");
        assert!(check_compat(&ctx, &db, Some("orca")).is_empty());
    }

    // ── check_method_vars ──────────────────────────────────────────────────────

    #[test]
    fn method_vars_warns_on_own_basis() {
        let mut db = SoftwareDb::default();
        db.methods.insert(
            "pbeh-3c".into(),
            crate::software::MethodEntry {
                has_own_basis: true,
                has_own_dispersion: false,
            },
        );
        let mut ctx = tera::Context::new();
        ctx.insert("method", "pbeh-3c");
        ctx.insert("basis_set", "def2-tzvp");
        let diags = check_method_vars(&ctx, &db);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].severity, Severity::Warning);
        assert!(diags[0].message.contains("basis_set"));
    }

    #[test]
    fn method_vars_silent_when_unknown_method() {
        let mut ctx = tera::Context::new();
        ctx.insert("method", "pbe0");
        ctx.insert("basis_set", "def2-tzvp");
        assert!(check_method_vars(&ctx, &empty_db()).is_empty());
    }

    // ── validate (integration) ─────────────────────────────────────────────────

    #[test]
    fn validate_collects_all_errors() {
        // Superposed atoms + bad charge/mult + missing var → 3 separate errors
        let mol = make_molecule(vec![
            (Element::H, 0.0, 0.0, 0.0),
            (Element::H, 0.0, 0.0, 0.0), // superposed
        ]);
        // 2 electrons, charge=0, mult=2 → (2-1)=1 unpaired, (2-1)%2 != 0 → parity error
        let ctx = ctx_with_ints(&[("charge", 0), ("mult", 2)]);
        let requires = vec!["basis_set".to_string()];
        let diags = validate(Some(&mol), &ctx, &requires, &empty_db(), None);
        // superposed(1) + charge/mult parity(1) + missing basis_set(1) = 3
        assert_eq!(diags.len(), 3);
        assert!(diags.iter().all(|d| d.severity == Severity::Error));
    }

    #[test]
    fn validate_no_molecule_skips_geometry_checks() {
        let requires = vec!["method".to_string()];
        let diags = validate(None, &tera::Context::new(), &requires, &empty_db(), None);
        assert_eq!(diags.len(), 1);
        assert!(diags[0].message.contains("method"));
    }
}
