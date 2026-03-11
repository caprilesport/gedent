use color_eyre::eyre::{bail, eyre, Report as Error, Result, WrapErr};
use log::debug;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use toml::{map::Map, Value};

const CONFIG_NAME: &str = "gedent.toml";

// ── Public types ──────────────────────────────────────────────────────────────

/// Tool-level settings from the `[gedent]` config section.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GedentConfig {
    /// Default output file extension (e.g. `"inp"`, `"gjf"`).
    pub default_extension: String,
    /// Default software name used for template disambiguation when a short
    /// template name matches multiple `software/name` paths.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub software: Option<String>,
}

/// Chemistry parameters from the `[model]` config section.
///
/// All fields are optional; unset fields are not injected into the Tera context.
/// Unknown fields in the config file are rejected with an error.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelConfig {
    /// DFT functional or method keyword (e.g. `"pbe0"`, `"pbeh-3c"`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub method: Option<String>,
    /// Basis set keyword (e.g. `"def2-tzvp"`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub basis_set: Option<String>,
    /// Total molecular charge.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub charge: Option<i64>,
    /// Spin multiplicity (2S+1).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mult: Option<i64>,
    /// Dispersion correction keyword (e.g. `"d3bj"`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dispersion: Option<String>,
    /// Solvent name. Setting this also injects `solvation = true` into context.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub solvent: Option<String>,
    /// Solvation model keyword (e.g. `"smd"`, `"cpcm"`, `"alpb"`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub solvation_model: Option<String>,
}

/// Compute resource settings from the `[resources]` config section.
///
/// Unknown fields in the config file are rejected with an error.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResourcesConfig {
    /// Number of parallel processes / CPU cores.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nprocs: Option<i64>,
    /// Memory per core in MB.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mem: Option<i64>,
}

/// Fully resolved configuration, produced by merging the cascade chain.
///
/// The cascade walks up from cwd to `~/.config/gedent/`, merging
/// `gedent.toml` files key-by-key. Deeper (closer to cwd) files win.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Config {
    /// Tool settings (`[gedent]` section).
    pub gedent: GedentConfig,
    /// Chemistry parameters (`[model]` section).
    #[serde(default)]
    pub model: ModelConfig,
    /// Compute resources (`[resources]` section).
    #[serde(default)]
    pub resources: ResourcesConfig,
    /// Arbitrary Tera context variables (`[parameters]` section).
    #[serde(default)]
    pub parameters: Map<String, Value>,
}

// ── Private parse/merge types ─────────────────────────────────────────────────

/// All-optional version of `GedentConfig` used during cascade parsing.
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawGedentConfig {
    default_extension: Option<String>,
    software: Option<String>,
}

/// All-optional config used to parse individual files in the cascade chain.
/// Each file in the chain may omit any section.
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawConfig {
    #[serde(default)]
    gedent: RawGedentConfig,
    #[serde(default)]
    model: ModelConfig,
    #[serde(default)]
    resources: ResourcesConfig,
    #[serde(default)]
    parameters: Map<String, Value>,
}

impl RawConfig {
    /// Merge `overlay` on top of `self`. All `Some` values in `overlay` win;
    /// `None` values fall through from `self`.
    fn merge(self, overlay: Self) -> Self {
        let model = ModelConfig {
            method: overlay.model.method.or(self.model.method),
            basis_set: overlay.model.basis_set.or(self.model.basis_set),
            charge: overlay.model.charge.or(self.model.charge),
            mult: overlay.model.mult.or(self.model.mult),
            dispersion: overlay.model.dispersion.or(self.model.dispersion),
            solvent: overlay.model.solvent.or(self.model.solvent),
            solvation_model: overlay.model.solvation_model.or(self.model.solvation_model),
        };
        let resources = ResourcesConfig {
            nprocs: overlay.resources.nprocs.or(self.resources.nprocs),
            mem: overlay.resources.mem.or(self.resources.mem),
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
            model,
            resources,
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
            model: self.model,
            resources: self.resources,
            parameters: self.parameters,
        }
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Format the non-empty fields of a `RawConfig` as one line per section,
/// e.g. `[model]  method = "pbe0", charge = 0`.
fn raw_contributions(raw: &RawConfig) -> Vec<String> {
    let mut lines = vec![];

    let mut gedent_parts = vec![];
    if let Some(ref v) = raw.gedent.default_extension {
        gedent_parts.push(format!("default_extension = {v:?}"));
    }
    if let Some(ref v) = raw.gedent.software {
        gedent_parts.push(format!("software = {v:?}"));
    }
    if !gedent_parts.is_empty() {
        lines.push(format!("[gedent]     {}", gedent_parts.join(", ")));
    }

    if let Ok(s) = toml::to_string(&raw.model) {
        let parts: Vec<&str> = s.lines().filter(|l| !l.is_empty()).collect();
        if !parts.is_empty() {
            lines.push(format!("[model]      {}", parts.join(", ")));
        }
    }

    if let Ok(s) = toml::to_string(&raw.resources) {
        let parts: Vec<&str> = s.lines().filter(|l| !l.is_empty()).collect();
        if !parts.is_empty() {
            lines.push(format!("[resources]  {}", parts.join(", ")));
        }
    }

    if !raw.parameters.is_empty() {
        let parts: Vec<String> = raw
            .parameters
            .iter()
            .map(|(k, v)| format!("{k} = {v}"))
            .collect();
        lines.push(format!("[parameters] {}", parts.join(", ")));
    }

    lines
}

// ── Config impl ───────────────────────────────────────────────────────────────

impl Config {
    /// Load and merge the full config cascade into a resolved [`Config`].
    ///
    /// Reads every `gedent.toml` in the chain (global `~/.config/gedent/` through
    /// cwd) and merges them left-to-right: later (deeper) files win per key.
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

    /// Print the merged config. With `--location`, shows per-file contributions
    /// so the user can see exactly where each value came from.
    pub fn print(self, location: bool) -> Result<(), Error> {
        if location {
            println!("Config chain (global → local):\n");
            for (path, raw) in Self::collect_chain_raw()? {
                println!("{}:", path.display());
                let lines = raw_contributions(&raw);
                if lines.is_empty() {
                    println!("  (no values set)");
                } else {
                    for line in lines {
                        println!("  {line}");
                    }
                }
                println!();
            }
            println!("merged:");
        }
        print!(
            "{}",
            toml::to_string(&self).wrap_err("Failed to serialize config")?
        );
        Ok(())
    }

    /// Return each config file in the cascade paired with its raw (un-merged) content.
    fn collect_chain_raw() -> Result<Vec<(PathBuf, RawConfig)>, Error> {
        Self::collect_chain()?
            .into_iter()
            .map(|path| {
                let content = std::fs::read_to_string(&path)
                    .wrap_err(format!("Failed to read {}", path.display()))?;
                let raw: RawConfig = toml::from_str(&content)
                    .wrap_err(format!("Failed to parse {}", path.display()))?;
                Ok((path, raw))
            })
            .collect()
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
        for path in &chain {
            debug!("Config chain: {}", path.display());
        }
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
            model: ModelConfig {
                method: method.map(str::to_string),
                charge,
                ..ModelConfig::default()
            },
            resources: ResourcesConfig::default(),
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
        assert_eq!(merged.model.method, Some("pbe0".to_string())); // falls through
        assert_eq!(merged.model.charge, Some(1)); // local wins
    }

    #[test]
    fn cascade_local_chemistry_partial_override() {
        let global = raw(Some("inp"), Some("pbe0"), Some(0), &[]);
        let local = raw(None, Some("b3lyp"), None, &[]);
        let merged = global.merge(local);
        assert_eq!(merged.model.method, Some("b3lyp".to_string())); // local wins
        assert_eq!(merged.model.charge, Some(0)); // falls through
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
        assert_eq!(config.model.method, Some("pbe0".to_string()));
        assert_eq!(config.model.charge, Some(-1));
        assert_eq!(config.parameters["key"], Value::Integer(42));
    }

    #[test]
    fn unknown_field_in_model_errors() {
        let toml = "[model]\nmethod = \"pbe0\"\ntypo_field = \"oops\"";
        assert!(
            toml::from_str::<RawConfig>(toml).is_err(),
            "Unknown key in [model] should be rejected"
        );
    }

    #[test]
    fn unknown_top_level_section_errors() {
        let toml = "[model]\nmethod = \"pbe0\"\n\n[completely_unknown]\nfoo = \"bar\"";
        assert!(
            toml::from_str::<RawConfig>(toml).is_err(),
            "Unknown top-level section should be rejected"
        );
    }

    #[test]
    fn cascade_three_level() {
        // global: method=pbe0, basis=def2-tzvp, charge=0
        let global = RawConfig {
            gedent: RawGedentConfig {
                default_extension: Some("inp".to_string()),
                software: None,
            },
            model: ModelConfig {
                method: Some("pbe0".to_string()),
                basis_set: Some("def2-tzvp".to_string()),
                charge: Some(0),
                ..ModelConfig::default()
            },
            resources: ResourcesConfig::default(),
            parameters: Map::new(),
        };
        // project: charge=1 (overrides global)
        let project = raw(None, None, Some(1), &[]);
        // cwd: method=b3lyp (overrides global, ignores project)
        let cwd = raw(None, Some("b3lyp"), None, &[]);

        let merged = global.merge(project).merge(cwd);

        assert_eq!(merged.model.method, Some("b3lyp".to_string())); // cwd wins
        assert_eq!(merged.model.basis_set, Some("def2-tzvp".to_string())); // global falls through
        assert_eq!(merged.model.charge, Some(1)); // project wins over global
    }
}
