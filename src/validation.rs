use crate::molecule::Molecule;
use std::fmt;

#[derive(Debug, PartialEq, Eq)]
pub enum Severity {
    Error,
    #[allow(dead_code)] // warnings not yet generated but the variant is part of the public API
    Warning,
}

#[derive(Debug)]
pub struct Diagnostic {
    pub severity: Severity,
    pub message: String,
}

impl Diagnostic {
    pub fn error(message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Error,
            message: message.into(),
        }
    }

    #[allow(dead_code)] // not yet generated but part of the public API
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

/// Run all validation checks. Molecule-specific checks are skipped when
/// `molecule` is `None` (i.e. the no-molecule generation path).
pub fn validate(
    molecule: Option<&Molecule>,
    context: &tera::Context,
    requires: &[String],
) -> Vec<Diagnostic> {
    let mut diags = vec![];
    if let Some(mol) = molecule {
        diags.extend(check_superposed_atoms(mol));
        diags.extend(check_charge_mult(mol, context));
    }
    diags.extend(check_missing_vars(context, requires));
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
    const THRESHOLD: f64 = 0.5; // Å
    let mut diags = vec![];
    let atoms = &molecule.atoms;
    for i in 0..atoms.len() {
        for j in (i + 1)..atoms.len() {
            let dx = atoms[i].x - atoms[j].x;
            let dy = atoms[i].y - atoms[j].y;
            let dz = atoms[i].z - atoms[j].z;
            let dist = dz.mul_add(dz, dx.mul_add(dx, dy * dy)).sqrt();
            if dist < THRESHOLD {
                diags.push(Diagnostic::error(format!(
                    "atoms {} ({}) and {} ({}) are superposed: distance {dist:.3} Å",
                    i + 1,
                    atoms[i].element,
                    j + 1,
                    atoms[j].element,
                )));
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
        // CH: 6+1 = 7 electrons, charge=0, mult=2 → (7-1)=6, 6%2==0 actually ok
        // Use H (1 electron), mult=2 would be ok. Use mult=1 for mismatch: (1-0)%2 != 0
        let mol = make_molecule(vec![(Element::H, 0.0, 0.0, 0.0)]);
        // 1 electron, mult=2 → unpaired=1, (1-1)%2==0 → ok
        // 1 electron, mult=1 → unpaired=0, (1-0)%2!=0 → error
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

    // ── validate (integration) ─────────────────────────────────────────────────

    #[test]
    fn validate_collects_all_errors() {
        // Superposed atoms + bad charge/mult + missing var → 3 separate errors
        let mol = make_molecule(vec![
            (Element::H, 0.0, 0.0, 0.0),
            (Element::H, 0.0, 0.0, 0.0), // superposed
        ]);
        // 2 electrons, charge=0, mult=1 → (2-0)%2==0 → actually valid
        // Use mult=2 → (2-1)=1 unpaired, (2-1)%2 != 0 → parity error
        let ctx = ctx_with_ints(&[("charge", 0), ("mult", 2)]);
        let requires = vec!["basis_set".to_string()];
        let diags = validate(Some(&mol), &ctx, &requires);
        // superposed(1) + charge/mult parity(1) + missing basis_set(1) = 3
        assert_eq!(diags.len(), 3);
        assert!(diags.iter().all(|d| d.severity == Severity::Error));
    }

    #[test]
    fn validate_no_molecule_skips_geometry_checks() {
        let requires = vec!["method".to_string()];
        let diags = validate(None, &tera::Context::new(), &requires);
        assert_eq!(diags.len(), 1);
        assert!(diags[0].message.contains("method"));
    }
}
