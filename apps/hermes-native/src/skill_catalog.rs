//! Public skill catalog used by the pinned Hermes Desktop Skills browser.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::{
    collections::{BTreeMap, HashMap},
    io::Read,
    sync::mpsc::{self, Receiver, SyncSender, TryRecvError},
    thread,
    time::{Duration, Instant},
};

const CATALOG_URL: &str = "https://nousresearch.github.io/hermes-agent/docs/api/skills.json";
const CATALOG_CACHE_TTL: Duration = Duration::from_secs(30 * 60);
const CATALOG_REQUEST_TIMEOUT: Duration = Duration::from_secs(60);
// The pinned Desktop handles a skills snapshot with roughly 100k rows. Allow
// 128 MiB for that public feed while bounding both network reads and parsing.
const MAX_CATALOG_BYTES: u64 = 128 * 1024 * 1024;
const MAX_CATALOG_ENTRIES: usize = 100_000;
const MAX_NAME_BYTES: usize = 512;
const MAX_IDENTIFIER_BYTES: usize = 2_048;
const MAX_DESCRIPTION_BYTES: usize = 8 * 1024;
const MAX_OVERVIEW_BYTES: usize = 16 * 1024;
const MAX_CATEGORY_BYTES: usize = 512;
const MAX_SOURCE_BYTES: usize = 256;
const MAX_AUTHOR_BYTES: usize = 512;
const MAX_REPO_BYTES: usize = 4 * 1024;
const MAX_TAGS: usize = 128;
const MAX_TAG_BYTES: usize = 256;

/// A normalized row from Hermes' public `skills.json` feed.
///
/// `id` follows the pinned Desktop catalog identity rule: `source:identifier`.
/// `identifier` is kept separate because it is the source-specific install key.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct SkillCatalogEntry {
    pub id: String,
    pub identifier: String,
    pub name: String,
    pub description: String,
    pub overview: String,
    pub category: String,
    pub category_label: String,
    pub source: String,
    pub install_identifier: Option<String>,
    pub author: String,
    pub tags: Vec<String>,
    pub repo: String,
    pub platforms: Vec<String>,
    pub requirements: Vec<String>,
    pub tools: Vec<String>,
    pub hooks: Vec<String>,
    pub middleware: Vec<String>,
    pub commands: Vec<String>,
    pub added_at: Option<String>,
    pub updated_at: Option<String>,
    pub stars: Option<f64>,
    pub featured: Option<f64>,
    pub search: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct SkillHubScanFinding {
    pub severity: String,
    pub category: String,
    pub file: String,
    #[serde(default)]
    pub line: Option<u32>,
    pub description: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct SkillHubScanResult {
    pub name: String,
    pub identifier: String,
    pub source: String,
    pub trust_level: String,
    pub verdict: String,
    pub summary: String,
    pub policy: String,
    #[serde(default)]
    pub policy_reason: Option<String>,
    #[serde(default)]
    pub findings: Vec<SkillHubScanFinding>,
    #[serde(default)]
    pub severity_counts: BTreeMap<String, u32>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SkillHubInstallBlock {
    pub findings: Option<usize>,
    pub unverified_source: bool,
}

/// Detect only the pinned Desktop's structured security-gate messages. Other
/// install failures keep the ordinary action error and do not trigger a scan.
pub fn parse_skill_hub_install_block(
    lines: &[String],
    error: Option<&str>,
) -> Option<SkillHubInstallBlock> {
    let text = lines
        .iter()
        .map(String::as_str)
        .chain(error)
        .collect::<Vec<_>>()
        .join("\n")
        .to_ascii_lowercase();
    if let Some(index) = text.find("not installed:") {
        let tail = &text[index + "not installed:".len()..];
        let normalized = tail.split_whitespace().collect::<Vec<_>>().join(" ");
        if normalized.starts_with("the security scan found") {
            let remainder = normalized["the security scan found".len()..].trim_start();
            let digits = remainder
                .chars()
                .take_while(char::is_ascii_digit)
                .collect::<String>();
            return Some(SkillHubInstallBlock {
                findings: (!digits.is_empty()).then(|| digits.parse().ok()).flatten(),
                unverified_source: text.contains("never installs unverified"),
            });
        }
    }
    if let Some(index) = text.find("installation blocked:") {
        let tail = &text[index..];
        if tail.contains(" source + ") && tail.contains(" verdict, ") && tail.contains(" finding") {
            let source = tail
                .split_once('(')
                .and_then(|(_, details)| details.split_once(" source + "))
                .map(|(source, _)| source.trim());
            let findings = tail
                .split(" verdict, ")
                .nth(1)
                .and_then(|value| value.split_whitespace().next())
                .and_then(|value| value.parse().ok());
            return Some(SkillHubInstallBlock {
                findings,
                unverified_source: source.is_some_and(|source| source != "official"),
            });
        }
    }
    None
}

pub fn validate_skill_hub_scan_result(
    scan: SkillHubScanResult,
    expected_identifier: &str,
) -> Result<SkillHubScanResult, String> {
    let bounded =
        |value: &str, max: usize| value.len() <= max && !value.chars().any(char::is_control);
    if scan.identifier != expected_identifier
        || !bounded(&scan.name, MAX_NAME_BYTES)
        || scan.name.trim().is_empty()
        || !bounded(&scan.identifier, MAX_IDENTIFIER_BYTES)
        || !bounded(&scan.source, MAX_SOURCE_BYTES)
        || !bounded(&scan.trust_level, 128)
        || !bounded(&scan.verdict, 128)
        || !bounded(&scan.summary, MAX_DESCRIPTION_BYTES)
        || !matches!(scan.policy.as_str(), "allow" | "ask" | "block")
        || scan
            .policy_reason
            .as_deref()
            .is_some_and(|reason| !bounded(reason, MAX_DESCRIPTION_BYTES))
        || scan.findings.len() > 1_000
        || scan.severity_counts.len() > 16
        || scan
            .severity_counts
            .iter()
            .any(|(severity, count)| !bounded(severity, 64) || *count > 1_000_000)
        || scan.findings.iter().any(|finding| {
            !bounded(&finding.severity, 64)
                || !bounded(&finding.category, 256)
                || !bounded(&finding.file, MAX_REPO_BYTES)
                || !bounded(&finding.description, MAX_DESCRIPTION_BYTES)
        })
    {
        return Err(
            "Hermes returned invalid, mismatched, or oversized Skill Hub scan details".into(),
        );
    }
    Ok(scan)
}

#[derive(Debug)]
pub struct SkillCatalogResult {
    pub request_id: String,
    pub result: Result<Vec<SkillCatalogEntry>, String>,
}

enum Command {
    Fetch(String),
}

/// Fetches the public catalog off the UI thread and caches successful results
/// for the same 30-minute window as the pinned Desktop query.
pub struct SkillCatalogWorker {
    requests: SyncSender<Command>,
    results: Receiver<SkillCatalogResult>,
}

impl SkillCatalogWorker {
    pub fn start() -> Self {
        let (request_tx, request_rx) = mpsc::sync_channel(1);
        let (result_tx, result_rx) = mpsc::channel();
        thread::Builder::new()
            .name("hermes-skill-catalog".into())
            .spawn(move || {
                let mut cache: Option<(Instant, Vec<SkillCatalogEntry>)> = None;
                while let Ok(Command::Fetch(request_id)) = request_rx.recv() {
                    let result = if let Some((cached_at, entries)) = cache.as_ref()
                        && cached_at.elapsed() < CATALOG_CACHE_TTL
                    {
                        Ok(entries.clone())
                    } else {
                        let loaded = load_public_skill_catalog();
                        if let Ok(entries) = &loaded {
                            cache = Some((Instant::now(), entries.clone()));
                        }
                        loaded
                    };
                    if result_tx
                        .send(SkillCatalogResult { request_id, result })
                        .is_err()
                    {
                        break;
                    }
                }
            })
            .expect("skill catalog worker thread must start");
        Self {
            requests: request_tx,
            results: result_rx,
        }
    }

    pub fn fetch(&self, request_id: String) -> Result<(), String> {
        self.requests
            .try_send(Command::Fetch(request_id))
            .map_err(|error| format!("Skill catalog worker is busy: {error}"))
    }

    pub fn poll(&self) -> Option<SkillCatalogResult> {
        let mut latest = None;
        loop {
            match self.results.try_recv() {
                Ok(result) => latest = Some(result),
                Err(TryRecvError::Empty | TryRecvError::Disconnected) => return latest,
            }
        }
    }
}

/// Load the fixed public skill catalog. This feed is not profile-specific and
/// is intentionally unauthenticated; profile-specific installed state remains
/// owned by the Hermes Gateway.
pub(crate) fn load_public_skill_catalog() -> Result<Vec<SkillCatalogEntry>, String> {
    fetch_skill_catalog_from_url(CATALOG_URL)
}

pub fn parse_skill_catalog(data: &[u8]) -> Result<Vec<SkillCatalogEntry>, String> {
    parse_skill_catalog_with_limit(data, MAX_CATALOG_BYTES)
}

fn parse_skill_catalog_with_limit(
    data: &[u8],
    response_limit: u64,
) -> Result<Vec<SkillCatalogEntry>, String> {
    if data.len() as u64 > response_limit {
        return Err(format!(
            "Skill catalog exceeded the {} MiB response limit",
            response_limit.div_ceil(1024 * 1024)
        ));
    }
    let rows: Value = serde_json::from_slice(data)
        .map_err(|_| "Hermes returned invalid skill catalog JSON".to_owned())?;
    let rows = rows
        .as_array()
        .ok_or_else(|| "Hermes skill catalog response was not an array".to_owned())?;
    if rows.len() > MAX_CATALOG_ENTRIES {
        return Err("Hermes returned too many skill catalog entries".into());
    }

    let mut entries = Vec::with_capacity(rows.len());
    let mut entry_indexes = HashMap::with_capacity(rows.len());
    for row in rows {
        let Some(row) = row.as_object() else {
            continue;
        };
        let Some(raw_name) = bounded_field(row, "name", MAX_NAME_BYTES) else {
            continue;
        };
        if raw_name.is_empty() {
            continue;
        }
        let Some(raw_identifier) = optional_bounded_field(row, "identifier", MAX_IDENTIFIER_BYTES)
        else {
            continue;
        };
        let Some(explicit_install_identifier) =
            optional_bounded_field(row, "installIdentifier", MAX_IDENTIFIER_BYTES)
        else {
            continue;
        };
        let identifier = raw_identifier.clone().unwrap_or_else(|| raw_name.clone());
        let name = if raw_name.chars().any(char::is_alphanumeric) {
            raw_name.clone()
        } else {
            identifier.clone()
        };
        let Some(description) = bounded_field(row, "description", MAX_DESCRIPTION_BYTES) else {
            continue;
        };
        let Some(overview) = bounded_field(row, "overview", MAX_OVERVIEW_BYTES) else {
            continue;
        };
        let Some(category) = optional_bounded_field(row, "category", MAX_CATEGORY_BYTES) else {
            continue;
        };
        let category = category
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| "uncategorized".into());
        let Some(category_label) = optional_bounded_field(row, "categoryLabel", MAX_CATEGORY_BYTES)
        else {
            continue;
        };
        let category_label = category_label
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| category.clone());
        let Some(source) = bounded_field(row, "source", MAX_SOURCE_BYTES) else {
            continue;
        };
        let install_identifier = skill_catalog_install_identifier(
            &raw_name,
            &source,
            raw_identifier.as_deref(),
            explicit_install_identifier.as_deref(),
        );
        let author_field = if row.get("maintainer").is_some_and(|value| !value.is_null()) {
            "maintainer"
        } else {
            "author"
        };
        let Some(author) = bounded_field(row, author_field, MAX_AUTHOR_BYTES) else {
            continue;
        };
        let Some(repo) = bounded_field(row, "repo", MAX_REPO_BYTES) else {
            continue;
        };
        let Some(tags) = bounded_string_list(row.get("tags"), MAX_TAGS, MAX_TAG_BYTES) else {
            continue;
        };
        let capabilities = row.get("capabilities").and_then(Value::as_object);
        let Some(platforms) = bounded_string_list(row.get("platforms"), MAX_TAGS, MAX_TAG_BYTES)
        else {
            continue;
        };
        let Some(requirements) = bounded_string_list(row.get("envVars"), MAX_TAGS, MAX_TAG_BYTES)
        else {
            continue;
        };
        let Some(tools) = bounded_string_list(
            capabilities.and_then(|caps| caps.get("providesTools")),
            MAX_TAGS,
            MAX_TAG_BYTES,
        ) else {
            continue;
        };
        let Some(hooks) = bounded_string_list(
            capabilities.and_then(|caps| caps.get("providesHooks")),
            MAX_TAGS,
            MAX_TAG_BYTES,
        ) else {
            continue;
        };
        let Some(middleware) = bounded_string_list(
            capabilities.and_then(|caps| caps.get("providesMiddleware")),
            MAX_TAGS,
            MAX_TAG_BYTES,
        ) else {
            continue;
        };
        let Some(commands) = bounded_string_list(row.get("commands"), MAX_TAGS, MAX_TAG_BYTES)
        else {
            continue;
        };
        let Some(added_at) = bounded_optional_metadata(row, "addedAt") else {
            continue;
        };
        let Some(updated_at) = bounded_optional_metadata(row, "updatedAt") else {
            continue;
        };
        let stars = finite_number(row.get("stars"));
        let featured = finite_number(row.get("featured"));

        let id = format!("{source}:{identifier}");
        if id.len() > MAX_IDENTIFIER_BYTES + MAX_SOURCE_BYTES + 1 {
            continue;
        }
        let mut searchable = vec![
            name.clone(),
            description.clone(),
            overview.clone(),
            author.clone(),
            category.clone(),
            category_label.clone(),
            source.clone(),
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

        let entry = SkillCatalogEntry {
            id: id.clone(),
            identifier,
            name,
            description,
            overview,
            category,
            category_label,
            source,
            install_identifier,
            author,
            tags,
            repo,
            platforms,
            requirements,
            tools,
            hooks,
            middleware,
            commands,
            added_at,
            updated_at,
            stars,
            featured,
            search,
        };
        if let Some(index) = entry_indexes.get(&id).copied() {
            // Pinned `Map.set` replaces a duplicate without changing its order.
            entries[index] = entry;
        } else {
            entry_indexes.insert(id, entries.len());
            entries.push(entry);
        }
    }
    Ok(entries)
}

fn bounded_field(row: &Map<String, Value>, name: &str, max_bytes: usize) -> Option<String> {
    let Some(value) = row.get(name) else {
        return Some(String::new());
    };
    let Some(value) = value.as_str() else {
        return Some(String::new());
    };
    (value.len() <= max_bytes).then(|| value.to_owned())
}

fn skill_catalog_install_identifier(
    name: &str,
    source: &str,
    identifier: Option<&str>,
    explicit_install_identifier: Option<&str>,
) -> Option<String> {
    if let Some(explicit) = explicit_install_identifier.filter(|value| !value.is_empty()) {
        return Some(explicit.to_owned());
    }
    let Some(identifier) = identifier.filter(|value| !value.is_empty()) else {
        return match source {
            "built-in" => None,
            "optional" => Some(format!("official/{name}")),
            _ => Some(name.to_owned()),
        };
    };
    Some(
        if source.eq_ignore_ascii_case("clawhub") && !identifier.starts_with("clawhub/") {
            format!("clawhub/{identifier}")
        } else {
            identifier.to_owned()
        },
    )
}

fn optional_bounded_field(
    row: &Map<String, Value>,
    name: &str,
    max_bytes: usize,
) -> Option<Option<String>> {
    let Some(value) = row.get(name) else {
        return Some(None);
    };
    if value.is_null() {
        return Some(None);
    }
    let Some(value) = value.as_str() else {
        // The pinned `text()` helper treats a non-string as empty.
        return Some(Some(String::new()));
    };
    (value.len() <= max_bytes).then(|| Some(value.to_owned()))
}

fn bounded_string_list(
    value: Option<&Value>,
    max_items: usize,
    max_item_bytes: usize,
) -> Option<Vec<String>> {
    let Some(value) = value.and_then(Value::as_array) else {
        return Some(Vec::new());
    };
    if value.len() > max_items {
        return None;
    }
    let mut result = Vec::with_capacity(value.len());
    for item in value {
        if let Some(item) = item.as_str() {
            if item.len() > max_item_bytes {
                return None;
            }
            result.push(item.to_owned());
        }
    }
    Some(result)
}

fn bounded_optional_metadata(row: &Map<String, Value>, name: &str) -> Option<Option<String>> {
    let Some(value) = row.get(name).and_then(Value::as_str) else {
        return Some(None);
    };
    if value.len() > 128 {
        return None;
    }
    // The source uses Date.parse and only retains strings it recognizes.
    // Catalog snapshots publish ISO dates/timestamps; accept RFC3339 and
    // date-only values, and omit malformed metadata without dropping a row.
    let valid = time::OffsetDateTime::parse(value, &time::format_description::well_known::Rfc3339)
        .is_ok()
        || time::Date::parse(
            value,
            &time::format_description::parse_borrowed::<3>("[year]-[month]-[day]")
                .expect("static date format is valid"),
        )
        .is_ok();
    Some(valid.then(|| value.to_owned()))
}

fn finite_number(value: Option<&Value>) -> Option<f64> {
    value
        .and_then(Value::as_f64)
        .filter(|value| value.is_finite())
}

fn fetch_skill_catalog_from_url(url: &str) -> Result<Vec<SkillCatalogEntry>, String> {
    fetch_skill_catalog_from_url_with_limit(url, MAX_CATALOG_BYTES)
}

fn fetch_skill_catalog_from_url_with_limit(
    url: &str,
    response_limit: u64,
) -> Result<Vec<SkillCatalogEntry>, String> {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(CATALOG_REQUEST_TIMEOUT))
        .max_redirects(0)
        .build()
        .into();
    let response = agent
        .get(url)
        .header("Accept", "application/json")
        .call()
        .map_err(|error| format!("Skill catalog request failed: {error}"))?;
    let mut body = Vec::new();
    response
        .into_body()
        .into_reader()
        .take(response_limit + 1)
        .read_to_end(&mut body)
        .map_err(|error| format!("Could not read skill catalog response: {error}"))?;
    if body.len() as u64 > response_limit {
        return Err(format!(
            "Skill catalog exceeded the {} MiB response limit",
            response_limit.div_ceil(1024 * 1024)
        ));
    }
    parse_skill_catalog_with_limit(&body, response_limit)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        io::Write,
        net::{TcpListener, TcpStream},
    };

    fn fixture_response(
        status: &str,
        headers: &str,
        body: &[u8],
    ) -> (String, thread::JoinHandle<String>) {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind catalog HTTP fixture");
        let address = listener.local_addr().expect("fixture address");
        let status = status.to_owned();
        let headers = headers.to_owned();
        let body = body.to_vec();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept catalog request");
            let request = read_request(&mut stream);
            let response = format!(
                "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n{headers}\r\n",
                body.len()
            );
            stream
                .write_all(response.as_bytes())
                .expect("write fixture headers");
            let _ = stream.write_all(&body);
            request
        });
        (format!("http://{address}/fixture"), server)
    }

    fn read_request(stream: &mut TcpStream) -> String {
        let mut bytes = Vec::new();
        let mut chunk = [0; 1024];
        while !bytes.windows(4).any(|window| window == b"\r\n\r\n") {
            let count = stream.read(&mut chunk).expect("read request");
            if count == 0 {
                break;
            }
            bytes.extend_from_slice(&chunk[..count]);
            assert!(bytes.len() < 16 * 1024, "HTTP fixture request is bounded");
        }
        String::from_utf8(bytes).expect("request headers are UTF-8")
    }

    #[test]
    fn parser_normalizes_rows_ignores_malformed_entries_and_preserves_source_identity() {
        let entries = parse_skill_catalog(
            br#"[
              {"name":"research","identifier":"official/research","source":"official","description":"Research tools","overview":"Find useful sources","category":"research","categoryLabel":"Research","maintainer":"Hermes","repo":"https://github.com/nous/research","tags":["web","sources"],"stars":12,"featured":2,"addedAt":"2026-01-03T00:00:00Z"},
              {"name":"research","identifier":"community/research","source":"community","description":"Community version","tags":["community"]},
              {"name":"research","identifier":"official/research","source":"official","description":"Updated official row","category":"research"},
              {"name":"---","identifier":"community/not-renderable","source":"community"},
              {"name":7,"identifier":"bad","source":"official"},
              null,
              "bad row"
            ]"#,
        )
        .expect("catalog fixture parses");
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].id, "official:official/research");
        assert_eq!(entries[0].identifier, "official/research");
        assert_eq!(entries[0].description, "Updated official row");
        assert_eq!(entries[0].name, "research");
        assert_eq!(entries[0].category_label, "research");
        // The last row for a source-qualified ID replaces the whole prior row.
        assert_eq!(entries[0].stars, None);
        assert_eq!(entries[0].featured, None);
        assert!(entries[0].search.contains("updated official row"));
        assert_eq!(entries[1].id, "community:community/research");
        assert_eq!(entries[2].name, "community/not-renderable");
    }

    #[test]
    fn parser_ignores_rows_exceeding_field_and_tag_limits() {
        let long_name = "x".repeat(MAX_NAME_BYTES + 1);
        let long_tag = "x".repeat(MAX_TAG_BYTES + 1);
        let rows = serde_json::json!([
            {"name":long_name,"identifier":"too-long","source":"official"},
            {"name":"too-many-tags","identifier":"many","source":"official","tags":[long_tag]},
            {"name":"valid","identifier":"valid","source":"official"}
        ]);
        let entries = parse_skill_catalog(rows.to_string().as_bytes()).expect("bounded fixture");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "valid");
    }

    #[test]
    fn parser_resolves_pinned_install_identifier_fallbacks() {
        let entries = parse_skill_catalog(
            br#"[
              {"name":"optional-skill","source":"optional"},
              {"name":"bundled-skill","source":"built-in"},
              {"name":"claw-skill","source":"ClawHub","identifier":"org/claw-skill"},
              {"name":"explicit-skill","source":"clawhub","identifier":"org/old","installIdentifier":"official/curated/new"},
              {"name":"plain-skill","source":"official"}
            ]"#,
        )
        .expect("install identifier fixture parses");
        assert_eq!(
            entries[0].install_identifier.as_deref(),
            Some("official/optional-skill")
        );
        assert_eq!(entries[1].install_identifier, None);
        assert_eq!(
            entries[2].install_identifier.as_deref(),
            Some("clawhub/org/claw-skill")
        );
        assert_eq!(
            entries[3].install_identifier.as_deref(),
            Some("official/curated/new")
        );
        assert_eq!(
            entries[4].install_identifier.as_deref(),
            Some("plain-skill")
        );
    }

    #[test]
    fn install_block_parser_matches_current_and_legacy_security_gate_messages() {
        let current = vec![
            "Not installed: the security scan found 2 high-risk pattern(s) in 'org/skill'."
                .to_owned(),
            "Hermes never installs unverified skills with high-risk findings, even with --force."
                .to_owned(),
        ];
        assert_eq!(
            parse_skill_hub_install_block(&current, None),
            Some(SkillHubInstallBlock {
                findings: Some(2),
                unverified_source: true,
            })
        );
        let legacy = vec![
            "Installation blocked: Blocked (community source + caution verdict, 4 findings)."
                .to_owned(),
        ];
        assert_eq!(
            parse_skill_hub_install_block(&legacy, None),
            Some(SkillHubInstallBlock {
                findings: Some(4),
                unverified_source: true,
            })
        );
        let official_legacy = vec![
            "Installation blocked: Blocked (official source + caution verdict, 1 finding)."
                .to_owned(),
        ];
        assert_eq!(
            parse_skill_hub_install_block(&official_legacy, None),
            Some(SkillHubInstallBlock {
                findings: Some(1),
                unverified_source: false,
            })
        );
        assert_eq!(
            parse_skill_hub_install_block(&[], Some("network request timed out")),
            None
        );
    }

    #[test]
    fn scan_result_requires_matching_identifier_bounded_fields_and_known_policy() {
        let valid: SkillHubScanResult = serde_json::from_value(serde_json::json!({
            "name":"research",
            "identifier":"official/research",
            "source":"official",
            "trust_level":"official",
            "verdict":"clean",
            "summary":"No high-risk findings",
            "policy":"allow",
            "policy_reason":null,
            "findings":[{"severity":"low","category":"metadata","file":"SKILL.md","line":1,"description":"Check metadata"}],
            "severity_counts":{"low":1}
        }))
        .expect("typed scan fixture");
        assert_eq!(
            validate_skill_hub_scan_result(valid.clone(), "official/research").expect("valid scan"),
            valid
        );
        assert!(validate_skill_hub_scan_result(valid.clone(), "other/skill").is_err());
        let mut invalid_policy = valid.clone();
        invalid_policy.policy = "override_allowed".into();
        assert!(validate_skill_hub_scan_result(invalid_policy, "official/research").is_err());
        let mut oversized = valid;
        oversized.findings[0].description = "x".repeat(MAX_DESCRIPTION_BYTES + 1);
        assert!(validate_skill_hub_scan_result(oversized, "official/research").is_err());
    }

    #[test]
    fn parser_rejects_invalid_root_oversized_body_and_excessive_row_count() {
        assert!(
            parse_skill_catalog(br#"{"error":"offline"}"#)
                .unwrap_err()
                .contains("not an array")
        );
        assert!(
            parse_skill_catalog_with_limit(b"12345", 4)
                .unwrap_err()
                .contains("1 MiB")
        );
        let too_many_rows = serde_json::to_vec(&vec![Value::Null; MAX_CATALOG_ENTRIES + 1])
            .expect("bounded row-count fixture serializes");
        assert!(
            parse_skill_catalog(&too_many_rows)
                .unwrap_err()
                .contains("too many skill catalog entries")
        );
    }

    #[test]
    fn http_reader_sends_only_public_json_request_and_parses_response() {
        let body = br#"[{"name":"weather","identifier":"official/weather","source":"official","description":"Forecasts"}]"#;
        let (url, server) = fixture_response("200 OK", "Content-Type: application/json\r\n", body);
        let entries = fetch_skill_catalog_from_url(&url).expect("public catalog request passes");
        let request = server.join().expect("fixture thread exits");
        let request_lower = request.to_ascii_lowercase();
        assert!(request_lower.starts_with("get /fixture http/1.1\r\n"));
        assert!(request_lower.contains("accept: application/json\r\n"));
        assert!(!request_lower.contains("authorization:"));
        assert!(!request_lower.contains("cookie:"));
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].id, "official:official/weather");
    }

    #[test]
    fn http_reader_fails_closed_on_redirect() {
        let (url, server) = fixture_response(
            "302 Found",
            "Location: https://example.com/redirected.json\r\n",
            b"",
        );
        assert!(fetch_skill_catalog_from_url(&url).is_err());
        let request = server.join().expect("fixture thread exits");
        assert!(
            request
                .to_ascii_lowercase()
                .starts_with("get /fixture http/1.1\r\n")
        );
    }

    #[test]
    fn http_reader_rejects_oversized_response() {
        let body = b"0123456789";
        let (url, server) = fixture_response("200 OK", "Content-Type: application/json\r\n", body);
        assert!(
            fetch_skill_catalog_from_url_with_limit(&url, 4)
                .unwrap_err()
                .contains("1 MiB")
        );
        server.join().expect("fixture thread exits");
    }
}
