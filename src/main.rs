mod cli;
mod logging;
mod prometheus;

use anyhow::Context;
use thiserror::Error;

use hcloud::apis::{
    configuration::Configuration as HcloudConfig, list_servers, ServersApiListServers,
};

use serde::Deserialize;
use std::{
    collections::{hash_map::DefaultHasher, HashMap},
    hash::{Hash, Hasher},
};

#[derive(Deserialize)]
struct Config {
    destination: String,
    #[serde(default = "Target::ipv6")]
    target: Target,
    projects: Vec<Project>,
}

#[derive(Deserialize)]
enum Target {
    #[serde(rename = "IPv4")]
    Ipv4,
    #[serde(rename = "IPv6")]
    Ipv6,
    #[serde(rename = "host")]
    Host,
    #[serde(rename = "label")]
    Label(String),
}

#[allow(dead_code)]
impl Target {
    fn ipv4() -> Self {
        Self::Ipv4
    }

    fn ipv6() -> Self {
        Self::Ipv6
    }

    fn host() -> Self {
        Self::Host
    }

    fn label(label: String) -> Self {
        Self::Label(label)
    }
}

#[derive(Deserialize)]
struct Project {
    name: String,
    api_token: String,
    labels: HashMap<String, String>,
}

#[derive(Error, Debug)]
#[error("target label {label} not found for host {host} in project {project}")]
struct TargetLabelMissing {
    label: String,
    host: String,
    project: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli_matches = cli::setup_cli();
    logging::setup_logging(match cli_matches.occurrences_of("v") {
        0 => log::LevelFilter::Error,
        1 => log::LevelFilter::Warn,
        2 => log::LevelFilter::Info,
        3 => log::LevelFilter::Debug,
        _ => log::LevelFilter::Trace,
    });

    let conf_path = cli_matches.value_of("config").unwrap_or("./config.toml");
    let conf_file = std::fs::read_to_string(conf_path).context("Couldn't read config file")?;
    let config: Config = toml::from_str(&conf_file).context("Couldn't parse config file")?;

    log::debug!("read and parsed config file");

    let mut hash = 0u64;

    let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
    log::debug!("initialized loop interval");

    loop {
        interval.tick().await;
        match async {
            let mut entries = Vec::new();

            for project in &config.projects {
                let mut hcloud_config = HcloudConfig::new();
                hcloud_config.bearer_access_token = Some(project.api_token.clone());

                let servers_resp = list_servers(
                    &hcloud_config,
                    ServersApiListServers {
                        status: None,
                        sort: None,
                        name: None,
                        label_selector: None,
                    },
                )
                .await
                .with_context(|| format!("Fetching servers failed for project {}", project.name))?;

                for server in servers_resp.servers {
                    entries.push(prometheus::FileSdEntry {
                        targets: vec![match &config.target {
                            Target::Ipv4 => server.public_net.ipv4.ip.to_string(),
                            Target::Ipv6 => server
                                .public_net
                                .ipv6
                                .ip
                                .hosts()
                                .nth(1)
                                .unwrap()
                                .to_string(),
                            Target::Host => server.name,
                            Target::Label(name) => server
                                .labels
                                .get(name)
                                .ok_or_else(|| TargetLabelMissing {
                                    label: name.to_string(),
                                    host: server.name.clone(),
                                    project: project.name.clone(),
                                })?
                                .to_string(),
                        }],
                        labels: {
                            let mut labels = project.labels.clone();
                            labels.extend(server.labels);
                            labels
                        },
                    });
                }
            }

            let sd_content = serde_json::to_string(&entries)?;
            let mut hasher = DefaultHasher::new();
            sd_content.hash(&mut hasher);
            let new_hash = hasher.finish();

            if hash != new_hash {
                log::info!("services changed, attemting to write new sd file");
                std::fs::write(&config.destination, sd_content.as_bytes())?;
                hash = new_hash
            }
            Ok::<(), anyhow::Error>(())
        }
        .await
        {
            Ok(()) => {}
            Err(error) => log::error!("service discovery failed: {:?}", error),
        };
    }
}
