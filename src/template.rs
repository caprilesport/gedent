#![allow(dead_code, unused_variables, unused_imports)]
use crate::config::Config;
use crate::molecule::Molecule;
use anyhow::{anyhow, Context, Error, Result};
use clap::{Parser, Subcommand, ValueEnum};
use serde::Deserialize;
use std::fs::{copy, read_dir, read_to_string, write};
use std::path::PathBuf;
use tera::Tera;
use toml::{map::Map, Table, Value};

const CONFIG_NAME: &str = "gedent.toml";
const PRESETS_DIR: &str = "presets";
const TEMPLATES_DIR: &str = "templates";
