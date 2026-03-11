use crate::config::Config;
use color_eyre::eyre::{Result, WrapErr};
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Deserialize, Default)]
pub struct SoftwareEntry {
    #[allow(dead_code)] // reserved for future checks (e.g. validate extension matches software)
    pub extension: Option<String>,
    #[allow(dead_code)] // reserved for solvation model validation
    pub solvation_models: Vec<String>,
}

#[derive(Debug, Deserialize, Default)]
pub struct MethodEntry {
    #[serde(default)]
    pub has_own_basis: bool,
    #[serde(default)]
    pub has_own_dispersion: bool,
}

#[derive(Debug, Deserialize)]
pub struct CompatRule {
    pub method: Option<String>,
    pub software: Option<String>,
    pub require_solvation_model: Option<String>,
    pub message: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
pub struct SoftwareDb {
    #[serde(default)]
    #[allow(dead_code)] // reserved for future checks
    pub software: HashMap<String, SoftwareEntry>,
    #[serde(default)]
    pub methods: HashMap<String, MethodEntry>,
    #[serde(default)]
    pub compat: Vec<CompatRule>,
}

impl SoftwareDb {
    /// Load the software database from `~/.config/gedent/software.toml`.
    /// Returns an empty database if the file does not exist (non-fatal).
    pub fn load() -> Result<Self> {
        let path = Config::gedent_home()?.join("software.toml");
        match std::fs::read_to_string(&path) {
            Ok(content) => toml::from_str(&content)
                .wrap_err_with(|| format!("Failed to parse {}", path.display())),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(e) => Err(e).wrap_err_with(|| format!("Failed to read {}", path.display())),
        }
    }

    /// Case-insensitive method lookup.
    pub fn get_method(&self, method: &str) -> Option<&MethodEntry> {
        let lower = method.to_lowercase();
        self.methods
            .iter()
            .find(|(k, _)| k.to_lowercase() == lower)
            .map(|(_, v)| v)
    }
}
