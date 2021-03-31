use anyhow::{Context, Result};
use log::LevelFilter;
use serde::Deserialize;

use std::collections::HashMap;

#[derive(Deserialize)]
pub struct Config {
    pub log_level: Option<LevelFilter>,
    pub output_folder: String,
    pub target: String,
    pub projects: Vec<Project>,
    pub filters: Vec<Vec<Filter>>,
}

#[derive(Deserialize)]
pub struct Project {
    pub name: String,
    pub api_token: String,
    pub labels: HashMap<String, String>,
}

#[derive(Deserialize)]
pub enum Filter {
    Label { name: String },
    LabelValue { name: String, value: String },
}

pub(crate) fn read_conf(path: &str) -> Result<Config> {
    let conf_file = std::fs::read_to_string(path).context("Couldn't read config file")?;
    Ok(serde_yaml::from_str(&conf_file).context("Couldn't parse config file")?)
}
