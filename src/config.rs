use anyhow::{anyhow, Context, Error, Result};
use clap::ValueEnum;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use toml::{map::Map, Value};

const CONFIG_NAME: &str = "gedent.toml";

#[derive(Clone, Debug, Serialize, Deserialize)]
struct GedentConfig {
    default_extension: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Config {
    gedent: GedentConfig,
    parameters: Map<String, Value>,
}

#[derive(Clone, Debug, Default, ValueEnum)]
pub enum ArgType {
    #[default]
    String,
    Float,
    Bool,
    Int,
}

impl Config {
    pub fn get() -> Result<Config, Error> {
        let cfg_path = Config::get_path()?;
        let cfg: Config = toml::from_str(&std::fs::read_to_string(&cfg_path)?)
            .context(format!("Failed to read config file {:?}", cfg_path))?;
        Ok(cfg)
    }

    pub fn print(self, location: bool) -> Result<(), Error> {
        if location {
            println!("Config printed from: {:?}", Config::get_path()?)
        }
        for (k, v) in self.parameters {
            println!("{}: {}", k, v);
        }
        Ok(())
    }

    pub fn edit() -> Result<(), Error> {
        edit::edit_file(Config::get_path()?)?;
        Ok(())
    }

    // check how to parse params
    pub fn write(&self) -> Result<(), Error> {
        let cfg_path = Config::get_path()?;
        std::fs::write(&cfg_path, toml::to_string(self)?)?;
        println!("Config wrote to {:?}.", cfg_path);
        Ok(())
    }

    pub fn set(&mut self, key: String, value: String) -> Result<(), Error> {
        let current_value = self
            .parameters
            .get(&key)
            .ok_or(anyhow!("Cant find {} in config.", key))?;

        println!(
            "Changing config {}, from {} to {}.",
            key, current_value, value
        );

        match current_value {
            Value::String(_current_value) => {
                self.parameters[&key] = Value::String(value);
            }
            Value::Float(_current_value) => {
                self.parameters[&key] = Value::Float(value.parse::<f64>()?);
            }
            Value::Integer(_current_value) => {
                self.parameters[&key] = Value::Integer(value.parse::<i64>()?);
            }
            Value::Boolean(_current_value) => {
                self.parameters[&key] = Value::Boolean(value.parse::<bool>()?);
            }
            _ => anyhow::bail!("Unsupported type"),
        }

        Ok(())
    }

    pub fn delete(&mut self, key: String) -> Result<(), Error> {
        self.parameters
            .remove(&key)
            .ok_or(anyhow!("Failed to remove key, not found."))?;
        println!("Removed key {}.", key);
        Ok(())
    }

    pub fn add(&mut self, key: String, value: String, toml_type: ArgType) -> Result<(), Error> {
        if self.parameters.contains_key(&key) {
            anyhow::bail!(format!("Config already contains {}, exiting.", key));
        }

        println!(
            "Setting config {} to {} with argtype {:?}",
            key, value, toml_type
        );

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

    fn get_path() -> Result<PathBuf, Error> {
        let current_dir = std::env::current_dir()?;
        let cfg_path: PathBuf = [find_config(current_dir)?, PathBuf::from(CONFIG_NAME)]
            .iter()
            .collect();
        Ok(cfg_path)
    }

    #[cfg(test)]
    fn new() -> Config {
        Config {
            gedent: GedentConfig {
                default_extension: "".to_string(),
            },
            parameters: Map::new(),
        }
    }
}

// git-like search, stop if .gedent folder is found or if dir.parent = none
fn find_config(dir: PathBuf) -> Result<PathBuf, Error> {
    let cwd = dir.clone();
    let gedent_config: PathBuf = [dir.clone(), PathBuf::from(CONFIG_NAME)].iter().collect();

    if std::path::Path::try_exists(&gedent_config)? {
        return Ok(cwd);
    } else {
        let parent_folder = dir.parent();
        match parent_folder {
            Some(parent) => return Ok(find_config(parent.to_path_buf())?),
            None => return Ok(crate::get_gedent_home()?),
        };
    }
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
        match config.set("testkey".to_string(), "false".to_string()) {
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
        match final_config.delete("testkey".to_string()) {
            Ok(_) => assert_eq!(final_config.parameters, config.parameters),
            Err(_) => core::panic!("Test failed to delete key"),
        }
    }
}
