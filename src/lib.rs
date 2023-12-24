#![allow(unused_variables, unused_imports)]
use std::path::PathBuf;
use toml::Table;
use anyhow::{Result, Error, anyhow, Context};
use toml::Value;
use tera::Tera;
use edit;


// Config functionality
fn get_config(config_file: String) -> Result<toml::map::Map<String, Value>, anyhow::Error > {
    let mut config_dir = get_config_dir()?;
    config_dir.push(config_file);
    let config_file = std::fs::read_to_string(&config_dir).context(format!("Cant open config {:?}", config_dir))?;
    let cfg: Table = config_file.parse()?;
    Ok(cfg)
}

// TODO: implement git-like functionality 
fn get_config_dir() -> Result<PathBuf, Error> {
    let gedent_home = get_gedent_home()?;
    Ok(gedent_home)
}

fn get_gedent_home() -> Result<PathBuf, Error> {
    // TODO: make this system agnostic in the future - only works in linux
    // I saw a dir crate that may help
    // https://docs.rs/dirs/latest/dirs/fn.config_dir.html
    let mut gedent_home = std::path::PathBuf::new();
    let home_dir = std::env::var_os("HOME").ok_or(anyhow!("Error fetching home directory"))?;
    gedent_home.push(home_dir);
    gedent_home.push(String::from(".config/gedent/"));
    Ok(gedent_home)
}

// Template functionality
fn get_template_path(template: String) -> Result<PathBuf, Error> {
    let mut tpl_path = get_gedent_home()?;
    tpl_path.push(String::from("templates/") + &template);
    Ok(tpl_path)
}

pub fn generate_template(template: String, options: Vec<String>) -> Result<(), Error> {
    let config_file = String::from("gedent.toml");
    let config_dir = get_config_dir()?;
    let cfg = get_config(config_file)?;
    let mut context = tera::Context::new();

    // Surprisingly, for me at least, passing toml::Value already works 
    // when using the typed values in TERA templates.
    for (key, value) in cfg {
        context.insert(key, &value);
    }
    
    // TODO: parse template to see if xyz file is needed
   
    let tpl_path = get_template_path(template)?;
    let tpl = std::fs::read_to_string(&tpl_path).context(format!("Cant find template {:?}", tpl_path))?;
    let result = Tera::one_off(&tpl, &context, true)?;
    println!("{}", result);
    Ok(())
}

pub fn edit_template(template: String) -> Result<(), Error> {
    let editor = std::env::var_os("EDITOR").ok_or(anyhow!("Error fetching default editor"))?;
    let template_path = get_template_path(template)?;
    edit::edit_file(template_path)?;
    Ok(())
}
