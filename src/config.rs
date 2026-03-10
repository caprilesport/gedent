use color_eyre::eyre::{bail, eyre, Report as Error, Result, WrapErr};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use toml::{map::Map, Value};

const CONFIG_NAME: &str = "gedent.toml";

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GedentConfig {
    pub default_extension: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ChemistryConfig {
    pub method: Option<String>,
    pub basis_set: Option<String>,
    pub charge: Option<i64>,
    pub mult: Option<i64>,
    pub dispersion: Option<String>,
    pub solvent: Option<String>,
    pub solvation_model: Option<String>,
    pub nprocs: Option<i64>,
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

impl Config {
    pub fn get() -> Result<Self, Error> {
        let cfg_path = Self::get_path()?;
        let cfg: Self = toml::from_str(&std::fs::read_to_string(&cfg_path)?)
            .wrap_err(format!("Failed to read config file {}", cfg_path.display()))?;
        Ok(cfg)
    }

    pub fn print(self, location: bool) -> Result<(), Error> {
        if location {
            println!("Config printed from: {}", Self::get_path()?.display());
        }
        print!(
            "{}",
            toml::to_string(&self).wrap_err("Failed to serialize config")?
        );
        Ok(())
    }

    pub fn edit() -> Result<(), Error> {
        edit::edit_file(Self::get_path()?)?;
        Ok(())
    }

    pub fn get_path() -> Result<PathBuf, Error> {
        let current_dir = std::env::current_dir()?;
        Ok(Self::find(&current_dir)?.join(CONFIG_NAME))
    }

    fn find(dir: &Path) -> Result<PathBuf, Error> {
        let gedent_config = dir.join(CONFIG_NAME);
        if gedent_config.try_exists()? {
            Ok(dir.to_path_buf())
        } else {
            dir.parent().map_or_else(get_gedent_home, Self::find)
        }
    }
}

pub fn get_gedent_home() -> Result<PathBuf, Error> {
    let mut config_dir =
        dirs::config_dir().ok_or_else(|| eyre!("Cant retrieve system config directory."))?;
    config_dir.push("gedent");
    match config_dir.try_exists() {
        Ok(true) => (),
        Ok(false) => bail!(
            "Failed to retrieve gedent home, {} doesn't exist. \nCheck if you've finished the installation procces and created the config directory.",
            config_dir.display()
        ),
        Err(err) => bail!("Failed to retrieve gedent home, caused by {:?}", err),
    }
    Ok(config_dir)
}
