use anyhow::{Context, Result};
use serde::Deserialize;

use std::collections::HashMap;

#[derive(Deserialize)]
pub struct Config {
    pub destination: String,
    pub target: String,
    pub projects: Vec<Project>,
}

#[derive(Deserialize)]
pub struct Project {
    pub name: String,
    pub api_token: String,
    pub labels: HashMap<String, String>,
}

pub(crate) fn read_conf(path: &str) -> Result<Config> {
    let conf_file = std::fs::read_to_string(path).context("Couldn't read config file")?;
    Ok(toml::from_str(&conf_file).context("Couldn't parse config file")?)
}
