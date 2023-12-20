#![allow(unused_variables, unused_imports)]
use std::collections::HashMap;
use std::path::Path;
use toml::Table;
use std::fs;
use anyhow::{Context, Result};


// Get config file
pub fn get_config(config_file: &Path) -> Result<()> {
    let config_file = fs::read_to_string(config_file)?;
    let cfg: Table = config_file.parse().unwrap();
    println!("Config in table format\n");
    dbg!(&cfg);
    Ok(())
}


// Check for config file in projct folder
