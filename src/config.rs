// #![allow(dead_code, unused_variables, unused_imports)]
use anyhow::{anyhow, Context, Error, Result};
use clap::{Parser, Subcommand, ValueEnum};
use serde::Deserialize;
use std::fs::{copy, read_dir, read_to_string, write};
use std::path::PathBuf;
use tera::Tera;
use toml::{map::Map, Table, Value};

const CONFIG_NAME: &str = "gedent.toml";

#[derive(Clone, Debug)]
pub struct Config {
    pub path: PathBuf,
    pub parameters: Map<String, Value>,
}

#[derive(Clone, Debug, Default, ValueEnum)]
pub enum ArgType {
    #[default]
    String,
    Float,
    Int,
    Bool,
}

impl Config {}

//Search for paths
fn get_gedent_home() -> Result<PathBuf, Error> {
    let home_dir = std::env::var_os("HOME").ok_or(anyhow!("Error fetching home directory"))?;
    // TODO: make this system agnostic in the future - only works in linux
    // I saw a dir crate that may help
    // https://docs.rs/dirs/latest/dirs/fn.config_dir.html
    let gedent_home: PathBuf = [home_dir, Into::into(".config/gedent")].iter().collect();
    Ok(gedent_home)
}

// git-like search, stop if .gedent folder is found or if dir.parent = none
fn find_config(dir: PathBuf) -> Result<PathBuf, Error> {
    let mut gedent = dir.clone();
    gedent.push(CONFIG_NAME);

    if std::path::Path::try_exists(&gedent)? {
        return Ok(gedent);
    } else {
        let parent_folder = dir.parent();
        match parent_folder {
            Some(parent) => return Ok(find_config(parent.to_path_buf())?),
            None => return Ok(get_gedent_home()?),
        };
    }
}

// Config functionality
// Can i test this somehow, or is it useless?
// Same applies to all functions that receive a pathbuf, should decouple whenever possible
fn load_config(config_path: &PathBuf) -> Result<Map<String, Value>, anyhow::Error> {
    let config_file =
        read_to_string(&config_path).context(format!("Cant open config {:?}", config_path))?;
    let config: Table = config_file.parse()?;
    Ok(config)
}

fn edit_config() -> Result<(), Error> {
    let config_path = get_config_path()?;
    edit::edit_file(config_path)?;
    Ok(())
}

// I can decouple thid fn from the Path, but maybe it's not worth it,
// as it does not have a return value
fn print_config(location: bool) -> Result<(), Error> {
    let config_path = get_config_path()?;
    let config = read_to_string(&config_path)?;
    if location {
        println!("Printing config from {:?}", config_path);
    }
    print!("{}", config);
    Ok(())
}

fn write_config(config_path: PathBuf, config: Map<String, Value>) -> Result<(), Error> {
    write(&config_path, config.to_string())?;
    println!("Config wrote to {:?}.", config_path);
    Ok(())
}

fn get_config_path() -> Result<PathBuf, Error> {
    let current_dir = std::env::current_dir()?;
    let config = PathBuf::from(CONFIG_NAME);
    Ok([find_config(current_dir)?, config].iter().collect())
}

fn delete_config(key: String, mut config: Map<String, Value>) -> Result<Map<String, Value>, Error> {
    config
        .remove(&key)
        .ok_or(anyhow!("Failed to remove key, not found."))?;
    println!("Removed key {}.", key);
    Ok(config)
}

fn add_config(
    key: String,
    value: String,
    toml_type: ArgType,
    mut config: Map<String, Value>,
) -> Result<Map<String, Value>, Error> {
    if config.contains_key(&key) {
        anyhow::bail!(format!("Config already contains {}, exiting.", key));
    }

    println!(
        "Setting config {} to {} with argtype {:?}",
        key, value, toml_type
    );

    match toml_type {
        ArgType::Int => {
            config.insert(key, Value::Integer(value.parse::<i64>()?));
        }
        ArgType::Bool => {
            config.insert(key, Value::Boolean(value.parse::<bool>()?));
        }
        ArgType::Float => {
            config.insert(key, Value::Float(value.parse::<f64>()?));
        }
        ArgType::String => {
            config.insert(key, Value::String(value));
        }
    }

    Ok(config)
}

fn set_config(
    key: String,
    value: String,
    mut config: Map<String, Value>,
) -> Result<Map<String, Value>, Error> {
    let current_value = config
        .get(&key)
        .ok_or(anyhow!("Cant find {} in config.", key))?;

    println!(
        "Changing config {}, from {} to {}.",
        key, current_value, value
    );

    match current_value {
        Value::String(_current_value) => {
            config[&key] = Value::String(value);
        }
        Value::Float(_current_value) => {
            config[&key] = Value::Float(value.parse::<f64>()?);
        }
        Value::Integer(_current_value) => {
            config[&key] = Value::Integer(value.parse::<i64>()?);
        }
        Value::Boolean(_current_value) => {
            config[&key] = Value::Boolean(value.parse::<bool>()?);
        }
        _ => anyhow::bail!("Unsupported type"),
    }

    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_add_key_works() {
        let mut final_config = Map::new();
        final_config.insert("testkey".to_string(), Value::Boolean(false));
        let config = Map::new();
        match add_config(
            "testkey".to_string(),
            "false".to_string(),
            ArgType::Bool,
            config,
        ) {
            Ok(val) => assert_eq!(val, final_config),
            Err(_) => core::panic!("Test failed to add key"),
        }
    }

    #[test]
    fn config_edit_key_works() {
        let mut final_config = Map::new();
        final_config.insert("testkey".to_string(), Value::Boolean(false));
        let mut config = Map::new();
        config.insert("testkey".to_string(), Value::Boolean(true));
        match set_config("testkey".to_string(), "false".to_string(), config) {
            Ok(conf) => assert_eq!(final_config, conf),
            Err(_) => core::panic!("Test failed to set key"),
        }
    }

    #[test]
    fn config_del_key_works() {
        let final_config = Map::new();
        let mut config = Map::new();
        config.insert("testkey".to_string(), Value::Boolean(true));
        match delete_config("testkey".to_string(), config) {
            Ok(conf) => assert_eq!(final_config, conf),
            Err(_) => core::panic!("Test failed to delete key"),
        }
    }
}
