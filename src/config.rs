use color_eyre::eyre::{bail, eyre, Report as Error, Result, WrapErr};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use toml::{map::Map, Value};

const CONFIG_NAME: &str = "gedent.toml";

// ── Public types ──────────────────────────────────────────────────────────────

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GedentConfig {
    pub default_extension: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub software: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ChemistryConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub method: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub basis_set: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub charge: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mult: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dispersion: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub solvent: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub solvation_model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nprocs: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mem: Option<i64>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Config {
    pub gedent: GedentConfig,
    #[serde(default)]
    pub chemistry: ChemistryConfig,
    #[serde(default)]
    pub parameters: Map<String, Value>,
}

// ── Private parse/merge types ─────────────────────────────────────────────────

/// All-optional version of `GedentConfig` used during cascade parsing.
#[derive(Debug, Default, Deserialize)]
struct RawGedentConfig {
    default_extension: Option<String>,
    software: Option<String>,
}

/// All-optional config used to parse individual files in the cascade chain.
/// Each file in the chain may omit any section.
#[derive(Debug, Default, Deserialize)]
struct RawConfig {
    #[serde(default)]
    gedent: RawGedentConfig,
    #[serde(default)]
    chemistry: ChemistryConfig,
    #[serde(default)]
    parameters: Map<String, Value>,
}

impl RawConfig {
    /// Merge `overlay` on top of `self`. All `Some` values in `overlay` win;
    /// `None` values fall through from `self`.
    fn merge(self, overlay: Self) -> Self {
        let chem = ChemistryConfig {
            method: overlay.chemistry.method.or(self.chemistry.method),
            basis_set: overlay.chemistry.basis_set.or(self.chemistry.basis_set),
            charge: overlay.chemistry.charge.or(self.chemistry.charge),
            mult: overlay.chemistry.mult.or(self.chemistry.mult),
            dispersion: overlay.chemistry.dispersion.or(self.chemistry.dispersion),
            solvent: overlay.chemistry.solvent.or(self.chemistry.solvent),
            solvation_model: overlay
                .chemistry
                .solvation_model
                .or(self.chemistry.solvation_model),
            nprocs: overlay.chemistry.nprocs.or(self.chemistry.nprocs),
            mem: overlay.chemistry.mem.or(self.chemistry.mem),
        };
        let mut params = self.parameters;
        for (k, v) in overlay.parameters {
            params.insert(k, v);
        }
        Self {
            gedent: RawGedentConfig {
                default_extension: overlay
                    .gedent
                    .default_extension
                    .or(self.gedent.default_extension),
                software: overlay.gedent.software.or(self.gedent.software),
            },
            chemistry: chem,
            parameters: params,
        }
    }

    /// Resolve into a final `Config`, filling in defaults for any missing values.
    fn resolve(self) -> Config {
        Config {
            gedent: GedentConfig {
                default_extension: self
                    .gedent
                    .default_extension
                    .unwrap_or_else(|| "inp".to_string()),
                software: self.gedent.software,
            },
            chemistry: self.chemistry,
            parameters: self.parameters,
        }
    }
}

// ── Config impl ───────────────────────────────────────────────────────────────

impl Config {
    /// Load and merge all config files in the cascade chain (global → local).
    /// Local values override global values key-by-key.
    pub fn get() -> Result<Self, Error> {
        let chain = Self::collect_chain()?;
        let merged = chain
            .iter()
            .map(|path| {
                let content = std::fs::read_to_string(path)
                    .wrap_err(format!("Failed to read config file {}", path.display()))?;
                toml::from_str::<RawConfig>(&content)
                    .wrap_err(format!("Failed to parse config file {}", path.display()))
            })
            .try_fold(RawConfig::default(), |acc, raw| raw.map(|r| acc.merge(r)))?;
        Ok(merged.resolve())
    }

    /// Print the merged config. With `--location`, lists all files in the chain.
    pub fn print(self, location: bool) -> Result<(), Error> {
        if location {
            println!("Config chain (global → local):");
            for path in Self::collect_chain()? {
                println!("  {}", path.display());
            }
        }
        print!(
            "{}",
            toml::to_string(&self).wrap_err("Failed to serialize config")?
        );
        Ok(())
    }

    /// Open a config file in `$EDITOR`.
    /// With `global = true`, opens the global `~/.config/gedent/gedent.toml`.
    /// Otherwise opens the nearest local `gedent.toml`, erroring if none exists.
    pub fn edit(global: bool) -> Result<(), Error> {
        let path = if global {
            Self::gedent_home()?.join(CONFIG_NAME)
        } else {
            Self::find_local(&std::env::current_dir()?)?
                .ok_or_else(|| {
                    eyre!(
                        "No local gedent.toml found. \
                         Use `gedent init` to create one, \
                         or pass `--global` to edit the global config."
                    )
                })?
                .join(CONFIG_NAME)
        };
        edit::edit_file(path)?;
        Ok(())
    }

    /// Returns all config file paths that contribute to the cascade, ordered
    /// from most global to most local (closest to cwd).
    pub fn collect_chain() -> Result<Vec<PathBuf>, Error> {
        let home = Self::gedent_home()?;
        let global = home.join(CONFIG_NAME);
        if !global.try_exists()? {
            bail!(
                "Global config not found at {}. Run `gedent --set-up` to initialize.",
                global.display()
            );
        }

        let current_dir = std::env::current_dir()?;
        let mut locals: Vec<PathBuf> = Vec::new();
        let mut dir = current_dir.as_path();

        loop {
            if dir == home {
                break;
            }
            let candidate = dir.join(CONFIG_NAME);
            if candidate.try_exists()? {
                locals.push(candidate);
            }
            match dir.parent() {
                Some(parent) => dir = parent,
                None => break,
            }
        }

        locals.reverse(); // cwd-first → global-first order
        let mut chain = vec![global];
        chain.extend(locals);
        Ok(chain)
    }

    /// Returns the gedent home directory (`~/.config/gedent/`), erroring if it
    /// does not exist.
    pub fn gedent_home() -> Result<PathBuf, Error> {
        let mut config_dir =
            dirs::config_dir().ok_or_else(|| eyre!("Can't retrieve system config directory."))?;
        config_dir.push("gedent");
        match config_dir.try_exists() {
            Ok(true) => Ok(config_dir),
            Ok(false) => bail!(
                "gedent home not found at {}. Run `gedent --set-up` to initialize.",
                config_dir.display()
            ),
            Err(err) => bail!("Failed to check gedent home: {:?}", err),
        }
    }

    /// Walk up from `dir` looking for a local `gedent.toml`, stopping before
    /// the global gedent home. Returns `None` if no local config is found.
    fn find_local(dir: &Path) -> Result<Option<PathBuf>, Error> {
        let home = Self::gedent_home()?;
        if dir == home {
            return Ok(None);
        }
        let candidate = dir.join(CONFIG_NAME);
        if candidate.try_exists()? {
            return Ok(Some(dir.to_path_buf()));
        }
        dir.parent().map_or_else(|| Ok(None), Self::find_local)
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn raw(
        default_extension: Option<&str>,
        method: Option<&str>,
        charge: Option<i64>,
        params: &[(&str, Value)],
    ) -> RawConfig {
        RawConfig {
            gedent: RawGedentConfig {
                default_extension: default_extension.map(str::to_string),
                software: None,
            },
            chemistry: ChemistryConfig {
                method: method.map(str::to_string),
                charge,
                ..ChemistryConfig::default()
            },
            parameters: params
                .iter()
                .map(|(k, v)| ((*k).to_string(), v.clone()))
                .collect(),
        }
    }

    #[test]
    fn cascade_local_chemistry_wins() {
        let global = raw(Some("inp"), Some("pbe0"), Some(0), &[]);
        let local = raw(None, None, Some(1), &[]);
        let merged = global.merge(local);
        assert_eq!(merged.chemistry.method, Some("pbe0".to_string())); // falls through
        assert_eq!(merged.chemistry.charge, Some(1)); // local wins
    }

    #[test]
    fn cascade_local_chemistry_partial_override() {
        let global = raw(Some("inp"), Some("pbe0"), Some(0), &[]);
        let local = raw(None, Some("b3lyp"), None, &[]);
        let merged = global.merge(local);
        assert_eq!(merged.chemistry.method, Some("b3lyp".to_string())); // local wins
        assert_eq!(merged.chemistry.charge, Some(0)); // falls through
    }

    #[test]
    fn cascade_parameters_local_wins() {
        let global = raw(Some("inp"), None, None, &[("key", Value::Integer(1))]);
        let local = raw(None, None, None, &[("key", Value::Integer(2))]);
        let merged = global.merge(local);
        assert_eq!(merged.parameters["key"], Value::Integer(2));
    }

    #[test]
    fn cascade_parameters_additive() {
        let global = raw(Some("inp"), None, None, &[("a", Value::Integer(1))]);
        let local = raw(None, None, None, &[("b", Value::Integer(2))]);
        let merged = global.merge(local);
        assert_eq!(merged.parameters["a"], Value::Integer(1));
        assert_eq!(merged.parameters["b"], Value::Integer(2));
    }

    #[test]
    fn cascade_extension_local_wins() {
        let global = raw(Some("inp"), None, None, &[]);
        let local = raw(Some("com"), None, None, &[]);
        let merged = global.merge(local);
        assert_eq!(merged.gedent.default_extension, Some("com".to_string()));
    }

    #[test]
    fn resolve_defaults_extension_to_inp() {
        let r = raw(None, None, None, &[]);
        assert_eq!(r.resolve().gedent.default_extension, "inp");
    }

    #[test]
    fn resolve_passes_through_set_values() {
        let r = raw(
            Some("com"),
            Some("pbe0"),
            Some(-1),
            &[("key", Value::Integer(42))],
        );
        let config = r.resolve();
        assert_eq!(config.gedent.default_extension, "com");
        assert_eq!(config.chemistry.method, Some("pbe0".to_string()));
        assert_eq!(config.chemistry.charge, Some(-1));
        assert_eq!(config.parameters["key"], Value::Integer(42));
    }

    #[test]
    fn cascade_three_level() {
        // global: method=pbe0, basis=def2-tzvp, charge=0
        let global = RawConfig {
            gedent: RawGedentConfig {
                default_extension: Some("inp".to_string()),
                software: None,
            },
            chemistry: ChemistryConfig {
                method: Some("pbe0".to_string()),
                basis_set: Some("def2-tzvp".to_string()),
                charge: Some(0),
                ..ChemistryConfig::default()
            },
            parameters: Map::new(),
        };
        // project: charge=1 (overrides global)
        let project = raw(None, None, Some(1), &[]);
        // cwd: method=b3lyp (overrides global, ignores project)
        let cwd = raw(None, Some("b3lyp"), None, &[]);

        let merged = global.merge(project).merge(cwd);

        assert_eq!(merged.chemistry.method, Some("b3lyp".to_string())); // cwd wins
        assert_eq!(merged.chemistry.basis_set, Some("def2-tzvp".to_string())); // global falls through
        assert_eq!(merged.chemistry.charge, Some(1)); // project wins over global
    }
}
