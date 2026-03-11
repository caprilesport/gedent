use crate::config::Config;
use color_eyre::eyre::{Result, WrapErr};
use serde::Deserialize;
use std::collections::HashMap;

/// Metadata for a known software package, from `[software.<name>]` in
/// `~/.config/gedent/software.toml`.
#[derive(Debug, Deserialize, Default)]
pub struct SoftwareEntry {
    /// Default output file extension for this software (e.g. `"inp"`, `"gjf"`).
    #[allow(dead_code)]
    // reserved for future checks (e.g. validate extension matches software)
    pub extension: Option<String>,
    /// Solvation models supported by this software (e.g. `["cpcm", "smd", "alpb"]`).
    #[allow(dead_code)] // reserved for solvation model validation
    pub solvation_models: Vec<String>,
}

/// Metadata for a known method, from `[methods.<name>]` in
/// `~/.config/gedent/software.toml`.
#[derive(Debug, Deserialize, Default)]
pub struct MethodEntry {
    /// `true` for composite methods that carry their own basis set
    /// (e.g. `pbeh-3c`, `r2scan-3c`, `xtb`). If `basis_set` is present in
    /// context, a warning is emitted rather than injecting a potentially wrong
    /// basis into the input file.
    #[serde(default)]
    pub has_own_basis: bool,
    /// `true` for methods with a built-in dispersion correction
    /// (e.g. `pbeh-3c`, `xtb`). If `dispersion` is in context, a warning is
    /// emitted.
    #[serde(default)]
    pub has_own_dispersion: bool,
}

/// A compatibility rule from `[[compat]]` in `~/.config/gedent/software.toml`.
///
/// Rules match on `method` and/or `software` (both optional; omitting one makes
/// the rule match any value of that field). When a rule matches and its
/// constraint is violated, a diagnostic is emitted.
#[derive(Debug, Deserialize)]
pub struct CompatRule {
    /// Method name to match (case-insensitive). `None` matches any method.
    pub method: Option<String>,
    /// Software name to match (case-insensitive). `None` matches any software.
    pub software: Option<String>,
    /// When set and solvation is active, `solvation_model` in context must equal
    /// this value (case-insensitive). Violation is an error.
    pub require_solvation_model: Option<String>,
    /// Error message shown when the constraint is violated. Falls back to a
    /// generic message if absent.
    pub message: Option<String>,
}

/// In-memory representation of `~/.config/gedent/software.toml`.
///
/// The file is extracted from the embedded default on first run and can be
/// freely edited by the user to add new methods, software entries, or compat
/// rules. If the file is missing, [`SoftwareDb::load`] returns an empty
/// database (non-fatal).
#[derive(Debug, Deserialize, Default)]
pub struct SoftwareDb {
    /// Known software packages, keyed by name.
    #[serde(default)]
    #[allow(dead_code)] // reserved for future checks
    pub software: HashMap<String, SoftwareEntry>,
    /// Known methods, keyed by name (lower-case in the file).
    #[serde(default)]
    pub methods: HashMap<String, MethodEntry>,
    /// Compatibility rules evaluated during validation.
    #[serde(default)]
    pub compat: Vec<CompatRule>,
}

impl SoftwareDb {
    /// Load the software database from `~/.config/gedent/software.toml`.
    ///
    /// Returns [`SoftwareDb::default`] (empty) if the file does not exist so
    /// that missing the file is non-fatal — all validation checks that depend
    /// on the database are simply skipped.
    pub fn load() -> Result<Self> {
        let path = Config::gedent_home()?.join("software.toml");
        match std::fs::read_to_string(&path) {
            Ok(content) => toml::from_str(&content)
                .wrap_err_with(|| format!("Failed to parse {}", path.display())),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(e) => Err(e).wrap_err_with(|| format!("Failed to read {}", path.display())),
        }
    }

    /// Look up a method entry by name (case-insensitive).
    pub fn get_method(&self, method: &str) -> Option<&MethodEntry> {
        let lower = method.to_lowercase();
        self.methods
            .iter()
            .find(|(k, _)| k.to_lowercase() == lower)
            .map(|(_, v)| v)
    }
}
