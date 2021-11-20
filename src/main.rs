mod cli;
mod config;
mod logging;
mod prometheus;

use crate::config::read_conf;

use anyhow::Context;

use hcloud::{
    apis::{
        configuration::Configuration as HcloudConfig,
        servers_api::{list_servers, ListServersParams},
    },
    models::Server,
};

use regex::Regex;

use std::{
    collections::{hash_map::DefaultHasher, HashMap},
    hash::{Hash, Hasher},
};

use log::trace;

use ipnet::Ipv6Net;

use itertools::Itertools;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli_matches = cli::setup_cli();
    let config = read_conf(cli_matches.value_of("config").unwrap_or("./config.yaml"))
        .context("couldn't read config file")?;
    logging::setup_logging(config.log_level.unwrap_or(log::LevelFilter::Warn));

    log::debug!("read and parsed config file, configured logging");

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

            let sd_content = serde_json::to_string_pretty(&entries)?;
            let mut hasher = DefaultHasher::new();
            sd_content.hash(&mut hasher);
            let new_hash = hasher.finish();

            if hash != new_hash {
                log::info!("services changed, attemting to write new sd file");
                let path = &format!("{}/all.json", &config.output_folder);
                let path = std::path::Path::new(path);
                std::fs::create_dir_all(
                    path.parent()
                        .context("something went wrong generating the path, parent not found")?,
                )?;
                std::fs::write(path, sd_content.as_bytes())?;
                for filter_list in &config.filters {
                    let additional_outputs =
                        fan_out_entries(entries.to_vec(), filter_list, &config.output_folder);
                    for (path, entries) in additional_outputs {
                        let path = std::path::Path::new(&path);
                        std::fs::create_dir_all(path.parent().context(
                            "something went wrong generating the path, parent not found",
                        )?)?;
                        std::fs::write(
                            path,
                            serde_json::to_string_pretty(&entries)
                                .context("failed to serialize service discovery file")?
                                .as_bytes(),
                        )
                        .context("failed to write service discovery file")?;
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
    filters: &[String],
    path: &str,
) -> HashMap<String, Vec<prometheus::FileSdEntry>> {
    match filters.len() {
        0 => {
            let mut map = HashMap::new();
            map.insert(format!("{}.json", path), entries);
            map
        }
        _ => {
            let name = &filters[0];
            let values: Vec<String> = entries
                .iter()
                .map(|o| o.labels.get(name).map(|o| o.to_string()))
                .flatten()
                .dedup()
                .collect();
            let mut value_entries: HashMap<LabelGroup, Vec<prometheus::FileSdEntry>> =
                HashMap::new();
            for entry in entries {
                let value = entry.labels.get(name).map(|o| o.to_string());
                for group in LabelGroup::groups(value, &values) {
                    if let Some(entries) = value_entries.get_mut(&group) {
                        entries.push(entry.clone());
                    } else {
                        value_entries.insert(group, vec![entry.clone()]);
                    }
                }
            }
            let mut ret_val = HashMap::new();
            for (group, entries) in value_entries {
                let new_path = match group {
                    LabelGroup::Value(value) => format!("{}/{}-is-{}", path, name, value),
                    LabelGroup::NotValue(value) => format!("{}/{}-is-not-{}", path, name, value),
                    LabelGroup::Set => format!("{}/{}-is-set", path, name),
                    LabelGroup::Empty => format!("{}/{}-is-empty", path, name),
                    LabelGroup::Unset => format!("{}/{}-is-not-set", path, name),
                };
                ret_val.extend(fan_out_entries(entries, &filters[1..], &new_path));
            }
            ret_val
        }
    }
}

#[derive(Hash, Eq, PartialEq, Debug)]
enum LabelGroup {
    Value(String),
    NotValue(String),
    Set,
    Empty,
    Unset,
}

impl LabelGroup {
    fn groups(value: Option<String>, values: &[String]) -> Vec<LabelGroup> {
        let mut groups = vec![];
        match value {
            None => {
                groups.push(LabelGroup::Unset);
            }
            Some(ref value) => {
                groups.push(LabelGroup::Set);
                if value.is_empty() {
                    groups.push(LabelGroup::Empty);
                } else {
                    groups.push(LabelGroup::Value(value.to_string()));
                    groups.append(
                        &mut values
                            .iter()
                            .filter(|&o| o != value)
                            .map(|o| LabelGroup::NotValue(o.to_string()))
                            .collect(),
                    );
                }
            }
        }
        trace!(
            "value groups for {:?} with values {:?} are {:?}",
            &value,
            values,
            groups
        );
        groups
    }
}

async fn load_server_list(token: &str, name: &str) -> anyhow::Result<Vec<Server>> {
    let mut hcloud_config = HcloudConfig::new();
    hcloud_config.bearer_access_token = Some(token.to_string());
    let servers_resp = list_servers(
        &hcloud_config,
        ListServersParams {
            status: None,
            sort: None,
            name: None,
            label_selector: None,
            page: None,
            per_page: None,
        },
    )
    .await
    .with_context(|| format!("Fetching servers failed for project {}", name))?;
    trace!("{:?}", servers_resp.meta.clone());
    let mut servers = servers_resp.servers;
    let last_page = servers_resp
        .meta
        .map(|meta| meta.pagination)
        .map(|pagination| pagination.last_page)
        .flatten();
    if let Some(last_page) = last_page {
        for i in 2..=last_page {
            let mut servers_resp = list_servers(
                &hcloud_config,
                ListServersParams {
                    status: None,
                    sort: None,
                    name: None,
                    label_selector: None,
                    page: Some(i),
                    per_page: None,
                },
            )
            .await
            .with_context(|| format!("Fetching servers (page)failed for project {}", name))?;
            trace!("{:?}", servers_resp.meta.clone());
            servers.append(&mut servers_resp.servers);
        }
    }
    Ok(servers)
}

fn build_server_template_context(server: &Server) -> tera::Context {
    let mut context = tera::Context::new();
    if let Some(ipv4) = &server.public_net.ipv4 {
        context.insert("ipv4", &ipv4.ip.to_string());
    }
    if let Some(ipv6) = &server.public_net.ipv6 {
        context.insert(
            "ipv6",
            &ipv6
                .ip
                .parse::<Ipv6Net>()
                .unwrap()
                .hosts()
                .nth(1)
                .unwrap()
                .to_string(),
        );
    }
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
        let is_match = RE.is_match(k);
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
