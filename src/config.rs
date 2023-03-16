use anyhow::{Context, Result};
use log::LevelFilter;
use serde::Deserialize;

use std::collections::HashMap;
use std::path::Path;

#[derive(Deserialize)]
pub struct Config {
    pub log_level: Option<LevelFilter>,
    pub output_folder: String,
    pub target: String,
    pub projects: Vec<Project>,
    pub filters: Vec<Vec<String>>,
}

#[derive(Deserialize)]
pub struct Project {
    pub name: String,
    pub api_token: String,
    pub labels: HashMap<String, String>,
}

pub(crate) fn read_conf<P: AsRef<Path>>(path: P) -> Result<Config> {
    let conf_file = std::fs::read_to_string(path).context("Couldn't read config file")?;
    serde_yaml::from_str(&conf_file).context("Couldn't parse config file")
}
