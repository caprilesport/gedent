use clap::ValueEnum;
use color_eyre::eyre::{bail, eyre, Report as Error, Result, WrapErr};
use dialoguer::{theme::ColorfulTheme, FuzzySelect};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use toml::{map::Map, Value};

const CONFIG_NAME: &str = "gedent.toml";

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GedentConfig {
    pub default_extension: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Config {
    pub gedent: GedentConfig,
    pub parameters: Map<String, Value>,
}

#[derive(Clone, Copy, Debug, Default, ValueEnum)]
pub enum ArgType {
    #[default]
    String,
    Float,
    Bool,
    Int,
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
        for (k, v) in self.parameters {
            println!("{k}: {v}");
        }
        Ok(())
    }

    pub fn edit() -> Result<(), Error> {
        edit::edit_file(Self::get_path()?)?;
        Ok(())
    }

    pub fn write(&self) -> Result<(), Error> {
        let cfg_path = Self::get_path()?;
        std::fs::write(&cfg_path, toml::to_string(self)?)?;
        println!("Config wrote to {}.", cfg_path.display());
        Ok(())
    }

    pub fn set(&mut self, key: &str, value: String) -> Result<(), Error> {
        let current_value = self
            .parameters
            .get(key)
            .ok_or_else(|| eyre!("Cant find {} in config.", key))?
            .clone();

        println!("Changing config {key}, from {current_value} to {value}.");

        let new_value = match current_value {
            Value::String(_) => Value::String(value),
            Value::Float(_) => Value::Float(value.parse::<f64>()?),
            Value::Integer(_) => Value::Integer(value.parse::<i64>()?),
            Value::Boolean(_) => Value::Boolean(value.parse::<bool>()?),
            _ => bail!("Unsupported type"),
        };
        self.parameters.insert(key.to_owned(), new_value);

        Ok(())
    }

    pub fn delete(&mut self, key: &str) -> Result<(), Error> {
        self.parameters
            .remove(key)
            .ok_or_else(|| eyre!("Failed to remove key, not found."))?;
        println!("Removed key {key}.");
        Ok(())
    }

    pub fn add(&mut self, key: String, value: String, toml_type: ArgType) -> Result<(), Error> {
        if self.parameters.contains_key(&key) {
            bail!(format!("Config already contains {}, exiting.", key));
        }

        println!("Setting config {key} to {value} with argtype {toml_type:?}");

        // TODO: add array and table as well
        match toml_type {
            ArgType::Int => {
                self.parameters
                    .insert(key, Value::Integer(value.parse::<i64>()?));
            }
            ArgType::Bool => {
                self.parameters
                    .insert(key, Value::Boolean(value.parse::<bool>()?));
            }
            ArgType::Float => {
                self.parameters
                    .insert(key, Value::Float(value.parse::<f64>()?));
            }
            ArgType::String => {
                self.parameters.insert(key, Value::String(value));
            }
        }

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

    #[cfg(test)]
    fn new() -> Self {
        Self {
            gedent: GedentConfig {
                default_extension: String::new(),
            },
            parameters: Map::new(),
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

pub fn select_key(config: &Config) -> Result<String, Error> {
    let keys: Vec<&String> = config.parameters.keys().collect();
    let mut select = vec![];
    for (k, v) in &config.parameters {
        select.push(format!("{k} (current value: {v})"));
    }
    let selection = FuzzySelect::with_theme(&ColorfulTheme::default())
        .default(0)
        .items(&select[..])
        .interact()?;
    Ok(keys[selection].clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_add_works() {
        let mut final_config = Config::new();
        final_config
            .parameters
            .insert("testkey".to_string(), Value::Boolean(false));
        let mut config = Config::new();
        match config.add("testkey".to_string(), "false".to_string(), ArgType::Bool) {
            Ok(_) => assert_eq!(config.parameters, final_config.parameters),
            Err(_) => core::panic!("Test failed to add key"),
        }
    }

    #[test]
    fn config_set_works() {
        let mut final_config = Config::new();
        final_config
            .parameters
            .insert("testkey".to_string(), Value::Boolean(false));
        let mut config = Config::new();
        config
            .parameters
            .insert("testkey".to_string(), Value::Boolean(true));
        match config.set("testkey", "false".to_string()) {
            Ok(_) => assert_eq!(config.parameters, final_config.parameters),
            Err(_) => core::panic!("Test failed to set key"),
        }
    }

    #[test]
    fn config_del_works() {
        let mut final_config = Config::new();
        final_config
            .parameters
            .insert("testkey".to_string(), Value::Boolean(false));
        let config = Config::new();
        match final_config.delete("testkey") {
            Ok(_) => assert_eq!(final_config.parameters, config.parameters),
            Err(_) => core::panic!("Test failed to delete key"),
        }
    }
}
