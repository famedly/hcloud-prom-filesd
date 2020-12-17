mod cli;
mod config;
mod logging;
mod prometheus;

use crate::config::{read_conf, Filter};

use anyhow::Context;

use hcloud::{
    apis::{configuration::Configuration as HcloudConfig, list_servers, ServersApiListServers},
    models::Server,
};

use regex::Regex;

use std::{
    collections::{hash_map::DefaultHasher, HashMap},
    hash::{Hash, Hasher},
};

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

    let config = read_conf(cli_matches.value_of("config").unwrap_or("./config.toml"))?;
    log::debug!("read and parsed config file");

    let mut tera = tera::Tera::default();
    tera.add_raw_template("target", &config.target)
        .context("Couldn't load target template")?;

    std::fs::create_dir_all(&config.output_folder)?;

    let mut hash = 0u64;

    let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
    log::debug!("initialized loop interval");

    loop {
        interval.tick().await;
        match async {
            let mut entries = Vec::new();

            for project in &config.projects {
                let servers = load_server_list(&project.api_token, &project.name).await?;
                for server in servers {
                    entries.push(prometheus::FileSdEntry {
                        targets: vec![tera
                            .render("target", &build_server_template_context(&server))
                            .with_context(|| {
                                format!("Couldn't render target string for host {}", server.name)
                            })?],
                        labels: {
                            let mut labels = project.labels.clone();
                            labels.extend(server.labels);
                            filter_labels(labels, &server.name)
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
                std::fs::write(
                    &format!("{}/all.json", &config.output_folder),
                    sd_content.as_bytes(),
                )?;
                for filter_list in &config.filters {
                    let additional_outputs =
                        fan_out_entries(entries.to_vec(), &filter_list, &config.output_folder);
                    for (path, entries) in additional_outputs {
                        std::fs::write(path, serde_json::to_string(&entries)?.as_bytes())?;
                    }
                }
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

fn fan_out_entries(
    entries: Vec<prometheus::FileSdEntry>,
    filters: &[Filter],
    path: &str,
) -> HashMap<String, Vec<prometheus::FileSdEntry>> {
    match filters.len() {
        0 => {
            let mut map = HashMap::new();
            map.insert(format!("{}.json", path), entries);
            map
        }
        _ => match &filters[0] {
            Filter::Label { name } => {
                let mut value_entries: HashMap<String, Vec<prometheus::FileSdEntry>> =
                    HashMap::new();
                for entry in entries {
                    if let Some(value) = entry.labels.get(name) {
                        if let Some(entries) = value_entries.get_mut(value) {
                            entries.push(entry);
                        } else {
                            value_entries.insert(value.to_string(), vec![entry]);
                        }
                    }
                }
                let mut ret_val = HashMap::new();
                for (value, entries) in value_entries {
                    let new_path = format!("{}/{}-{}", path, name, value);
                    ret_val.extend(fan_out_entries(entries, &filters[1..], &new_path));
                }
                return ret_val;
            }
            Filter::LabelValue { name, value } => {
                let new_path = format!("{}/{}-{}", path, name, value);
                let mut new_entries = Vec::new();
                for entry in entries {
                    if let Some(actual_value) = entry.labels.get(name) {
                        if actual_value == value {
                            new_entries.push(entry);
                        }
                    }
                }
                return fan_out_entries(new_entries, &filters[1..], &new_path);
            }
        },
    }
}

async fn load_server_list(token: &str, name: &str) -> anyhow::Result<Vec<Server>> {
    let mut hcloud_config = HcloudConfig::new();
    hcloud_config.bearer_access_token = Some(token.to_string());

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
    .with_context(|| format!("Fetching servers failed for project {}", name))?;
    Ok(servers_resp.servers)
}

fn build_server_template_context(server: &Server) -> tera::Context {
    let mut context = tera::Context::new();
    context.insert("ipv4", &server.public_net.ipv4.ip.to_string());
    context.insert(
        "ipv6",
        &server
            .public_net
            .ipv6
            .ip
            .hosts()
            .nth(1)
            .unwrap()
            .to_string(),
    );
    context.insert("hostname", &server.name);
    context.insert("labels", &server.labels);
    context
}

fn filter_labels(
    mut labels: HashMap<String, String>,
    server_name: &str,
) -> HashMap<String, String> {
    labels.retain(|k, _| {
        lazy_static::lazy_static! {
            static ref RE: Regex = Regex::new("^[a-zA-Z_][a-zA-Z0-9_]*$").unwrap();
        }
        let is_match = RE.is_match(&k);
        if !is_match {
            log::warn!(
                "Label key {} on host {} is not a valid label key \
                       and will therefore not be included in the labels \
                       in the file_sd file",
                k,
                server_name
            );
        } else {
            log::debug!(
                "Label key {} on host {} is a valid label key",
                k,
                server_name
            );
        }
        is_match
    });
    labels
}
