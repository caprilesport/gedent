#![allow(unused_variables)]
use anyhow::{anyhow, Context, Error, Result};
use std::{fs, path::PathBuf};
use tera::Tera;
use toml::{Table, Value};

// Config functionality
fn get_config(config_file: String) -> Result<toml::map::Map<String, Value>, anyhow::Error> {
    let mut config_dir = get_config_dir()?;
    config_dir.push(config_file);
    let config_file = std::fs::read_to_string(&config_dir)
        .context(format!("Cant open config {:?}", config_dir))?;
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
    gedent_home.push(String::from(".config/gedent"));
    Ok(gedent_home)
}

// Template functionality
pub fn generate_template(template: String, options: Vec<String>) -> Result<(), Error> {
    let config_file = String::from("gedent.toml");
    let cfg = get_config(config_file)?;
    let mut context = tera::Context::new();

    // Surprisingly, for me at least, passing toml::Value already works
    // when using the typed values in TERA templates.
    for (key, value) in cfg {
        context.insert(key, &value);
    }

    // TODO: parse template to see if xyz file is needed

    let result = render_template(template, context)?;
    println!("{}", &result);
    Ok(())
}

pub fn edit_template(template: String) -> Result<(), Error> {
    let template_path = get_template_path(template)?;
    // The edit crate makes this work in all platforms.
    edit::edit_file(template_path)?;
    Ok(())
}

pub fn print_template(template: String) -> Result<(), Error> {
    let template_path = get_template_path(template)?;
    let template = std::fs::read_to_string(&template_path)
        .context(format!("Cant find template {:?}", template_path))?;
    println!("{}", &template);
    Ok(())
}

// Basic logic is correct, would be nice if the user could set where these
// directories are.
pub fn new_template(software: String, template_name: String) -> Result<(), Error> {
    let mut boilerplate = get_gedent_home()?;
    let mut template_path = boilerplate.clone();
    template_path.push(String::from("templates"));
    template_path.push(template_name);
    boilerplate.push(String::from("presets"));
    boilerplate.push(software);
    fs::copy(&boilerplate, &template_path)
        .context(format!("Cant open base {:?} template.", &boilerplate))?;
    edit::edit_file(template_path).context("Cant open editor.")?;
    Ok(())
}

pub fn list_templates() -> Result<(), Error> {
    let mut gedent_home = get_gedent_home()?;
    gedent_home.push(String::from("templates"));
    // +1 is here to remove the first slash
    let gedent_home_len = gedent_home
        .to_str()
        .ok_or(anyhow!("Cant retrieve gedent home len"))?
        .len()
        + 1;
    for entry in fs::read_dir(gedent_home)? {
        print_descent_dir(entry.as_ref().unwrap().path(), gedent_home_len)?;
    }
    Ok(())
}

fn print_descent_dir(entry: PathBuf, gedent_home_len: usize) -> Result<(), Error> {
    if entry.is_dir() {
        let new_dir = fs::read_dir(entry)?;
        for new_entry in new_dir {
            let _ = print_descent_dir(new_entry.as_ref().unwrap().path(), gedent_home_len)?;
        }
        Ok(())
    } else {
        println!("{}", &entry.to_str().unwrap()[gedent_home_len..]);
        Ok(())
    }
}

fn get_template_path(template: String) -> Result<PathBuf, Error> {
    let mut tpl_path = get_gedent_home()?;
    tpl_path.push(String::from("templates/") + &template);
    Ok(tpl_path)
}

fn render_template(template_name: String, context: tera::Context) -> Result<String, Error> {
    let template_path = get_template_path(template_name)?;
    let template =
        fs::read_to_string(&template_path).context(format!("Cant find template {:?}", template_path))?;
    let result = Tera::one_off(&template, &context, true)?;
    Ok(result)
}


