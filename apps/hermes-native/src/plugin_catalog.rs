//! Public plugin catalog used by the pinned Hermes Desktop Plugins browser.

use serde_json::Value;
use std::{
    collections::HashMap,
    io::Read,
    sync::mpsc::{self, Receiver, SyncSender, TryRecvError},
    thread,
    time::Duration,
};

const CATALOG_URL: &str = "https://nousresearch.github.io/hermes-agent/docs/api/plugins.json";
const MAX_CATALOG_BYTES: u64 = 32 * 1024 * 1024;
const MAX_CATALOG_ENTRIES: usize = 20_000;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PluginCatalogEntry {
    pub name: String,
    pub description: String,
    pub overview: String,
    pub category: String,
    pub category_label: String,
    pub tier: String,
    pub author: String,
    pub identifier: String,
    pub repo: String,
    pub sha: String,
    pub subdir: String,
    pub version: String,
    pub requires_hermes: String,
    pub tags: Vec<String>,
    pub platforms: Vec<String>,
    pub requirements: Vec<String>,
    pub tools: Vec<String>,
    pub hooks: Vec<String>,
    pub middleware: Vec<String>,
    pub commands: Vec<String>,
    pub license: String,
    pub search: String,
}

#[derive(Debug)]
pub struct PluginCatalogResult {
    pub request_id: String,
    pub result: Result<Vec<PluginCatalogEntry>, String>,
}

enum Command {
    Fetch(String),
}

/// Serializes catalog fetches off the UI thread and returns request-tagged results.
pub struct PluginCatalogWorker {
    requests: SyncSender<Command>,
    results: Receiver<PluginCatalogResult>,
}

impl PluginCatalogWorker {
    pub fn start() -> Self {
        let (request_tx, request_rx) = mpsc::sync_channel(1);
        let (result_tx, result_rx) = mpsc::channel();
        thread::Builder::new()
            .name("hermes-plugin-catalog".into())
            .spawn(move || {
                while let Ok(Command::Fetch(request_id)) = request_rx.recv() {
                    let result = fetch_plugin_catalog();
                    if result_tx
                        .send(PluginCatalogResult { request_id, result })
                        .is_err()
                    {
                        break;
                    }
                }
            })
            .expect("plugin catalog worker thread must start");
        Self {
            requests: request_tx,
            results: result_rx,
        }
    }

    pub fn fetch(&self, request_id: String) -> Result<(), String> {
        self.requests
            .try_send(Command::Fetch(request_id))
            .map_err(|error| format!("Plugin catalog worker is busy: {error}"))
    }

    pub fn poll(&self) -> Option<PluginCatalogResult> {
        let mut latest = None;
        loop {
            match self.results.try_recv() {
                Ok(result) => latest = Some(result),
                Err(TryRecvError::Empty | TryRecvError::Disconnected) => return latest,
            }
        }
    }
}

pub fn parse_plugin_catalog(data: &[u8]) -> Result<Vec<PluginCatalogEntry>, String> {
    if data.len() as u64 > MAX_CATALOG_BYTES {
        return Err("Plugin catalog exceeded the 32 MiB response limit".into());
    }
    let rows: Value = serde_json::from_slice(data)
        .map_err(|_| "Hermes returned invalid plugin catalog JSON".to_owned())?;
    let rows = rows
        .as_array()
        .ok_or_else(|| "Hermes plugin catalog response was not an array".to_owned())?;
    if rows.len() > MAX_CATALOG_ENTRIES {
        return Err("Hermes returned too many plugin catalog entries".into());
    }

    let mut entries = Vec::with_capacity(rows.len());
    let mut entry_indexes = HashMap::with_capacity(rows.len());
    for row in rows {
        let Some(row) = row.as_object() else {
            continue;
        };
        let raw_name = field(row, "name");
        if raw_name.is_empty() {
            continue;
        }
        let identifier = nonempty_field(row, "identifier").unwrap_or_else(|| raw_name.clone());
        let name = if raw_name.chars().any(char::is_alphanumeric) {
            raw_name
        } else {
            identifier.clone()
        };
        let caps = row.get("capabilities").and_then(Value::as_object);
        let category = nonempty_field(row, "category").unwrap_or_else(|| "uncategorized".into());
        let category_label =
            nonempty_field(row, "categoryLabel").unwrap_or_else(|| category.clone());
        let description = field(row, "description");
        let overview = field(row, "overview");
        let tier = field(row, "tier");
        let author = row
            .get("maintainer")
            .filter(|value| !value.is_null())
            .map(|value| value.as_str().unwrap_or_default().to_owned())
            .unwrap_or_else(|| field(row, "author"));
        let tags = string_list(row.get("tags"));
        let tools = string_list(caps.and_then(|caps| caps.get("providesTools")));
        let hooks = string_list(caps.and_then(|caps| caps.get("providesHooks")));
        let middleware = string_list(caps.and_then(|caps| caps.get("providesMiddleware")));
        let requirements = string_list(caps.and_then(|caps| caps.get("requiresEnv")));
        let commands = string_list(row.get("commands"));
        let platforms = string_list(row.get("platforms"));
        let mut searchable = vec![
            name.clone(),
            description.clone(),
            overview.clone(),
            author.clone(),
            category.clone(),
            category_label.clone(),
            tier.clone(),
        ];
        searchable.extend(tags.iter().cloned());
        searchable.extend(tools.iter().cloned());
        searchable.extend(hooks.iter().cloned());
        searchable.extend(platforms.iter().cloned());
        searchable.extend(requirements.iter().cloned());
        searchable.extend(commands.iter().cloned());
        searchable.extend(middleware.iter().cloned());
        let search = searchable
            .into_iter()
            .filter(|part| !part.is_empty())
            .collect::<Vec<_>>()
            .join(" ")
            .to_lowercase();

        let entry = PluginCatalogEntry {
            name,
            description,
            overview,
            category,
            category_label,
            tier,
            author,
            identifier,
            repo: field(row, "repo"),
            sha: field(row, "sha"),
            subdir: field(row, "subdir"),
            version: field(row, "version"),
            requires_hermes: field(row, "requiresHermes"),
            tags,
            platforms,
            requirements,
            tools,
            hooks,
            middleware,
            commands,
            license: field(row, "license"),
            search,
        };
        let key = format!("{}:{}", entry.tier, entry.identifier);
        if let Some(index) = entry_indexes.get(&key).copied() {
            entries[index] = entry;
        } else {
            entry_indexes.insert(key, entries.len());
            entries.push(entry);
        }
    }
    Ok(entries)
}

fn field(row: &serde_json::Map<String, Value>, name: &str) -> String {
    row.get(name)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned()
}

fn nonempty_field(row: &serde_json::Map<String, Value>, name: &str) -> Option<String> {
    let value = field(row, name);
    (!value.is_empty()).then_some(value)
}

fn string_list(value: Option<&Value>) -> Vec<String> {
    value
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_owned)
        .collect()
}

fn fetch_plugin_catalog() -> Result<Vec<PluginCatalogEntry>, String> {
    let agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(60)))
        .max_redirects(4)
        .build();
    let agent: ureq::Agent = agent.into();
    let response = agent
        .get(CATALOG_URL)
        .header("Accept", "application/json")
        .call()
        .map_err(|error| format!("Plugin catalog request failed: {error}"))?;
    let mut body = Vec::new();
    response
        .into_body()
        .into_reader()
        .take(MAX_CATALOG_BYTES + 1)
        .read_to_end(&mut body)
        .map_err(|error| format!("Could not read plugin catalog response: {error}"))?;
    if body.len() as u64 > MAX_CATALOG_BYTES {
        return Err("Plugin catalog exceeded the 32 MiB response limit".into());
    }
    parse_plugin_catalog(&body)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_catalog_fields_and_builds_case_insensitive_search_text() {
        let catalog = parse_plugin_catalog(
            br#"[
              {"name":"Weather","identifier":"weather","description":"Forecast tools","overview":"Detailed forecast","category":"productivity","categoryLabel":"Productivity","tier":"official","maintainer":"Hermes","repo":"https://github.com/nous/weather","sha":"0123456789012345678901234567890123456789","subdir":"plugin","version":"1.2.0","requiresHermes":">=0.9","tags":["forecast"],"platforms":["windows"],"license":"MIT","commands":["/weather"],"capabilities":{"providesTools":["forecast"],"providesHooks":["before_tool"],"providesMiddleware":["retry"],"requiresEnv":["WEATHER_KEY"]}},
              {"name":"Weather","identifier":"weather","description":"Later duplicate wins","category":"productivity","categoryLabel":"Productivity","tier":"official","maintainer":"Hermes","tags":["forecast"],"capabilities":{"providesTools":["forecast"],"requiresEnv":["WEATHER_KEY"]}},
              {"name":"Weather","identifier":"weather-alt","tier":"community"},
              {"name":"---","identifier":"not-renderable"},
              null
            ]"#,
        )
        .expect("fixture catalog is valid");
        assert_eq!(catalog.len(), 3);
        let weather = &catalog[0];
        assert_eq!(weather.name, "Weather");
        assert_eq!(weather.description, "Later duplicate wins");
        assert_eq!(weather.category_label, "Productivity");
        assert_eq!(weather.author, "Hermes");
        assert_eq!(weather.requirements, ["WEATHER_KEY"]);
        assert!(weather.search.contains("weather_key"));
        assert!(weather.search.contains("forecast"));
        assert_eq!(catalog[2].name, "not-renderable");
    }

    #[test]
    fn rejects_non_array_and_oversized_catalogs() {
        assert!(
            parse_plugin_catalog(br#"{}"#)
                .unwrap_err()
                .contains("not an array")
        );
        assert!(
            parse_plugin_catalog(&vec![b' '; MAX_CATALOG_BYTES as usize + 1])
                .unwrap_err()
                .contains("32 MiB")
        );
    }
}
