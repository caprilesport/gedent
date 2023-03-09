#![allow(unused_variables, unused_imports)]
use anyhow::{Context, Result};
use serde_derive::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct GedentConfig {
    method: String,
    basis_set: String,
    solvation_model: String,
    solvent: String,
}

impl ::std::default::Default for GedentConfig {
    fn default() -> Self {
        Self {
            method: String::from("PBE0"),
            basis_set: String::from("def2-TZVP"),
            solvation_model: String::from("CPCM"),
            solvent: String::from("water"),
        }
    }
}

pub fn get_config() -> Result<GedentConfig, confy::ConfyError> {
    let cfg: GedentConfig = confy::load("gedent", ".gedent")?;
    Ok(cfg)
}
