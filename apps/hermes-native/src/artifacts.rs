use serde_json::Value;
use std::sync::OnceLock;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArtifactKind {
    Image,
    File,
    Link,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ArtifactRecord {
    pub id: String,
    pub kind: ArtifactKind,
    pub value: String,
    pub href: String,
    pub label: String,
    pub session_id: String,
    pub profile: Option<String>,
    pub session_title: String,
    pub timestamp_ms: i64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ArtifactSession {
    pub session_id: String,
    pub profile: Option<String>,
    pub title: String,
    pub preview: String,
    pub started_at: f64,
    pub last_active: Option<f64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArtifactFilter {
    All,
    Image,
    File,
    Link,
}

impl ArtifactFilter {
    pub const ALL: [Self; 4] = [Self::All, Self::Image, Self::File, Self::Link];

    pub fn parse(value: &str) -> Self {
        match value {
            "image" => Self::Image,
            "file" => Self::File,
            "link" => Self::Link,
            _ => Self::All,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::Image => "image",
            Self::File => "file",
            Self::Link => "link",
        }
    }

    pub fn matches(self, kind: ArtifactKind) -> bool {
        self == Self::All
            || matches!(
                (self, kind),
                (Self::Image, ArtifactKind::Image)
                    | (Self::File, ArtifactKind::File)
                    | (Self::Link, ArtifactKind::Link)
            )
    }
}

const IMAGE_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "gif", "webp", "svg", "bmp"];
const FILE_EXTENSIONS: &[&str] = &[
    "png", "jpg", "jpeg", "gif", "webp", "svg", "bmp", "pdf", "txt", "json", "md", "csv", "xls",
    "xlsx", "doc", "docx", "ppt", "pptx", "html", "zip", "tar", "gz", "avi", "flac", "m4a", "mkv",
    "mp3", "ogg", "opus", "wav", "webm", "mp4", "mov",
];

fn with_regex<T>(pattern: &'static str, f: impl FnOnce(&regex::Regex) -> T) -> T {
    static CACHE: OnceLock<
        std::sync::Mutex<std::collections::HashMap<&'static str, regex::Regex>>,
    > = OnceLock::new();
    let cache = CACHE.get_or_init(Default::default);
    let mut cache = cache.lock().expect("artifact regex cache poisoned");
    let compiled = cache
        .entry(pattern)
        .or_insert_with(|| regex::Regex::new(pattern).expect("valid artifact regex"));
    f(compiled)
}

fn is_windows_path(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() >= 3
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && matches!(bytes[2], b'\\' | b'/')
        || value.starts_with("\\\\")
}

fn is_file_path(value: &str) -> bool {
    value.starts_with('/')
        || value.starts_with("~/")
        || value.starts_with("~\\")
        || value.starts_with("./")
        || value.starts_with("../")
        || value.starts_with(".\\")
        || value.starts_with("..\\")
        || is_windows_path(value)
}

fn extension(value: &str) -> Option<String> {
    let path = value.split(['?', '#']).next().unwrap_or(value);
    let name = path.rsplit(['/', '\\']).next().unwrap_or(path);
    let (_, ext) = name.rsplit_once('.')?;
    (!ext.is_empty()).then(|| ext.to_ascii_lowercase())
}

fn is_python_package_noise(value: &str) -> bool {
    let normalized = value.replace('\\', "/").to_ascii_lowercase();
    normalized.contains("/.cache/pip/")
        || normalized.contains("/library/caches/pip/")
        || normalized.contains("/appdata/local/pip/cache/")
        || normalized.ends_with(".whl")
        || normalized.ends_with(".whl.metadata")
        || (normalized.starts_with("http://") || normalized.starts_with("https://"))
            && normalized.contains("/packages/")
            && [".tar.gz", ".tar.bz2", ".zip", ".egg"].iter().any(|ext| {
                normalized.ends_with(ext) || normalized.ends_with(&format!("{ext}.metadata"))
            })
}

fn artifact_kind(value: &str) -> ArtifactKind {
    if value.starts_with("data:image/")
        || extension(value).is_some_and(|ext| IMAGE_EXTENSIONS.contains(&ext.as_str()))
    {
        ArtifactKind::Image
    } else if is_file_path(value) {
        ArtifactKind::File
    } else {
        ArtifactKind::Link
    }
}

fn looks_like_artifact(value: &str, explicit: bool) -> bool {
    if !explicit && is_python_package_noise(value) {
        return false;
    }
    if value.starts_with("http://")
        || value.starts_with("https://")
        || value.starts_with("data:image/")
    {
        return true;
    }
    if explicit {
        return is_file_path(value) || value.starts_with("file://");
    }
    (is_file_path(value) || value.starts_with("file://"))
        && extension(value).is_some_and(|ext| FILE_EXTENSIONS.contains(&ext.as_str()))
}

fn normalize_value(value: &str) -> String {
    let mut trimmed = value.trim();
    for _ in 0..3 {
        let Some(first) = trimmed.chars().next() else {
            break;
        };
        if trimmed.ends_with(first) && matches!(first, '"' | '\'' | '`') {
            trimmed = trimmed[first.len_utf8()..trimmed.len() - first.len_utf8()].trim();
        } else {
            break;
        }
    }
    trimmed
        .trim_end_matches(['`', '*', ')', ',', '.', ';'])
        .to_owned()
}

fn decode_media_href(value: &str) -> String {
    let Some(encoded) = value.strip_prefix("#media:") else {
        return value.to_owned();
    };
    let bytes = encoded.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' && index + 2 < bytes.len() {
            if let Ok(byte) = u8::from_str_radix(&encoded[index + 1..index + 3], 16) {
                decoded.push(byte);
                index += 3;
                continue;
            }
        }
        decoded.push(bytes[index]);
        index += 1;
    }
    String::from_utf8(decoded).unwrap_or_else(|_| value.to_owned())
}

fn push_candidate(candidates: &mut Vec<(String, bool)>, raw: &str, explicit: bool) {
    let value = normalize_value(&decode_media_href(raw));
    if !value.is_empty()
        && looks_like_artifact(&value, explicit)
        && !candidates.iter().any(|(found, _)| found == &value)
    {
        candidates.push((value, explicit));
    }
}

const MEDIA_DELIVERY_EXTENSIONS: &[&str] = &[
    "png", "jpg", "jpeg", "gif", "webp", "bmp", "tiff", "svg", "mp4", "mov", "avi", "mkv", "webm",
    "3gp", "mp3", "m2a", "wav", "ogg", "opus", "m4a", "flac", "pdf", "docx", "doc", "odt", "rtf",
    "txt", "md", "epub", "xlsx", "xls", "ods", "csv", "tsv", "json", "xml", "yaml", "yml", "kmz",
    "kml", "geojson", "gpx", "pptx", "ppt", "odp", "key", "zip", "tar", "gz", "tgz", "bz2", "xz",
    "7z", "rar", "apk", "ipa", "html", "htm",
];

fn collect_media_values(text: &str, candidates: &mut Vec<(String, bool)>) {
    static MEDIA_REGEX: OnceLock<regex::Regex> = OnceLock::new();
    let regex = MEDIA_REGEX.get_or_init(|| {
        let extensions = MEDIA_DELIVERY_EXTENSIONS.join("|");
        let pattern = format!(
            r#"(?i)MEDIA:\s*(?:"([^"\r\n]+)"|'([^'\r\n]+)'|`([^`\r\n]+)`|([^\s`"]+\.(?:{extensions})))"#
        );
        regex::Regex::new(&pattern).expect("valid Hermes MEDIA marker regex")
    });
    for capture in regex.captures_iter(text) {
        let value = (1..=4)
            .find_map(|index| capture.get(index))
            .map_or("", |value| value.as_str())
            .trim();
        if !value.contains('/') && !value.contains('\\') && !value.contains('.') {
            continue;
        }
        push_candidate(candidates, value, true);
    }
}

fn scan_text(text: &str, candidates: &mut Vec<(String, bool)>) {
    collect_media_values(text, candidates);
    with_regex(r"!\[[^\]]*\]\(([^)\s]+)\)", |re| {
        for capture in re.captures_iter(text) {
            push_candidate(
                candidates,
                capture.get(1).map_or("", |value| value.as_str()),
                false,
            );
        }
    });
    with_regex(r"\[([^\]]+)\]\(([^)\s]+)\)", |re| {
        for capture in re.captures_iter(text) {
            if capture.get(0).is_some_and(|found| {
                found.start() > 0 && text.as_bytes()[found.start() - 1] == b'!'
            }) {
                continue;
            }
            let value = capture.get(2).map_or("", |value| value.as_str());
            if looks_like_artifact(&decode_media_href(value), false) {
                push_candidate(candidates, value, false);
            }
        }
    });
    with_regex(r#"https?://[^\s<>"')`]+"#, |re| {
        for found in re.find_iter(text) {
            push_candidate(candidates, found.as_str(), false);
        }
    });
    with_regex(r#"(?:/|~[\\/]|\.\.?[\\/]|\\\\)[^\s"'`<>]+"#, |re| {
        for found in re.find_iter(text) {
            let boundary = found.start() == 0
                || text.as_bytes().get(found.start() - 1).is_some_and(|byte| {
                    byte.is_ascii_whitespace() || matches!(*byte, 40 | 34 | 96)
                });
            if boundary {
                push_candidate(candidates, found.as_str(), false);
            }
        }
    });
    with_regex(r#"[A-Za-z]:[\\/][^\s"'`<>]+"#, |re| {
        for found in re.find_iter(text) {
            let boundary = found.start() == 0
                || text.as_bytes().get(found.start() - 1).is_some_and(|byte| {
                    byte.is_ascii_whitespace() || matches!(*byte, 40 | 34 | 96)
                });
            if boundary {
                push_candidate(candidates, found.as_str(), false);
            }
        }
    });
}

fn producer_tool(name: &str) -> bool {
    name.starts_with("bfl_flux3_")
        || with_regex(
            r"(?i)(?:^|_)(?:creat(?:e|ion)|download|export|generat(?:e|ion)|render|save|speech|tts|write)(?:_|$)",
            |re| re.is_match(name),
        )
}

fn strong_artifact_key(segment: &str) -> bool {
    with_regex(
        r"(?i)^(?:artifact_(?:file|image|path|url)|files?_(?:created|modified|written)|generated_(?:file|image|path|url)|media_tag|output_(?:file|path|url)|result_(?:file|path|url)|saved_to|screenshot_path)$",
        |re| re.is_match(segment),
    )
}

fn producer_artifact_key(segment: &str) -> bool {
    with_regex(
        r"(?i)^(?:artifacts?|artifact_(?:file|image|path|url)|attachments?|attachment_(?:file|image|path|url)|downloads?|download_(?:file|path|url)|(?:audio|image|video)(?:_(?:file|path|url))?|file_path|local_path|media(?:_(?:file|path|url))?|path)$",
        |re| re.is_match(segment),
    )
}

fn explicit_tool_key(key_path: &str, producer: bool) -> bool {
    key_path
        .split('.')
        .filter(|segment| !segment.is_empty() && !segment.bytes().all(|byte| byte.is_ascii_digit()))
        .any(|segment| strong_artifact_key(segment) || producer && producer_artifact_key(segment))
}

fn shell_output_key(key_path: &str) -> bool {
    key_path.split('.').any(|segment| {
        matches!(
            segment.to_ascii_lowercase().as_str(),
            "output" | "stdout" | "path"
        )
    })
}

fn collect_tool_strings(
    value: &Value,
    key_path: &str,
    producer: bool,
    terminal: bool,
    out: &mut Vec<(String, bool)>,
) {
    match value {
        Value::String(text) => {
            if terminal && shell_output_key(key_path) {
                scan_text(text, out);
            } else if explicit_tool_key(key_path, producer) {
                collect_media_values(text, out);
                let value = normalize_value(&decode_media_href(text));
                if !value.is_empty() && looks_like_artifact(&value, false) {
                    push_candidate(out, &value, false);
                }
            }
        }
        Value::Array(items) => {
            for (index, item) in items.iter().enumerate() {
                let next = if key_path.is_empty() {
                    index.to_string()
                } else {
                    format!("{key_path}.{index}")
                };
                collect_tool_strings(item, &next, producer, terminal, out);
            }
        }
        Value::Object(object) => {
            for (key, child) in object {
                let next = if key_path.is_empty() {
                    key.clone()
                } else {
                    format!("{key_path}.{key}")
                };
                collect_tool_strings(child, &next, producer, terminal, out);
            }
        }
        _ => {}
    }
}

fn untrusted_tool_payload(value: &str) -> Option<&str> {
    let trimmed = value.trim();
    if !trimmed.starts_with("<untrusted_tool_result") {
        return None;
    }
    let content_start = trimmed.find('>').map(|index| index + 1)?;
    let close = trimmed.rfind("</untrusted_tool_result>")?;
    if close <= content_start {
        return None;
    }
    let wrapped = trimmed[content_start..close].trim();
    Some(
        wrapped
            .split_once("\n\n")
            .map_or(wrapped, |(_, body)| body)
            .trim(),
    )
}

fn collect_artifacts_from_message(message: &Value, candidates: &mut Vec<(String, bool)>) {
    let role = message.get("role").and_then(Value::as_str).unwrap_or("");
    if role == "assistant" {
        if let Some(text) = message_text(message) {
            scan_text(text, candidates);
        }
        return;
    }
    if role != "tool" {
        return;
    }
    let name = message
        .get("tool_name")
        .or_else(|| message.get("name"))
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim()
        .to_lowercase();
    let producer = producer_tool(&name);
    let terminal = name == "terminal";
    let text = message_text(message).unwrap_or("");
    if !text.is_empty() && (producer || terminal) {
        scan_text(text, candidates);
    }
    if name == "browser_vision" {
        with_regex(r"(?i)Screenshot path:\s*([^\r\n<>]+)", |re| {
            for capture in re.captures_iter(text) {
                push_candidate(
                    candidates,
                    capture.get(1).map_or("", |value| value.as_str()),
                    false,
                );
            }
        });
    }
    for source in [Some(text), untrusted_tool_payload(text)]
        .into_iter()
        .flatten()
    {
        if let Ok(parsed) = serde_json::from_str::<Value>(source) {
            collect_tool_strings(&parsed, "tool_result", producer, terminal, candidates);
        }
    }
    if let Some(content) = message
        .get("content")
        .filter(|value| value.is_object() || value.is_array())
    {
        let structured = if content.get("_multimodal").and_then(Value::as_bool) == Some(true) {
            content.get("meta").unwrap_or(&Value::Null)
        } else {
            content
        };
        if !structured.is_null() {
            collect_tool_strings(structured, "tool_result", producer, terminal, candidates);
        }
    }
}

fn message_text(message: &Value) -> Option<&str> {
    ["content", "text", "context"].iter().find_map(|field| {
        message
            .get(field)
            .and_then(Value::as_str)
            .filter(|text| !text.trim().is_empty())
    })
}

fn timestamp_ms(message: &Value, session: &ArtifactSession) -> i64 {
    let timestamp = message
        .get("timestamp")
        .and_then(Value::as_f64)
        .or(session.last_active)
        .or(Some(session.started_at))
        .unwrap_or(0.0);
    if !timestamp.is_finite() || timestamp <= 0.0 {
        return 0;
    }
    if timestamp < 10_000_000_000.0 {
        (timestamp * 1000.0) as i64
    } else {
        timestamp as i64
    }
}

fn label(value: &str) -> String {
    if let Ok(url) = url::Url::parse(value) {
        if matches!(url.scheme(), "http" | "https") {
            return url
                .path_segments()
                .and_then(|segments| segments.filter(|part| !part.is_empty()).next_back())
                .unwrap_or(value)
                .to_owned();
        }
    }
    value
        .split(['/', '\\'])
        .filter(|part| !part.is_empty())
        .next_back()
        .unwrap_or(value)
        .to_owned()
}

pub fn collect_artifacts_for_session(
    session: &ArtifactSession,
    messages: &[Value],
) -> Vec<ArtifactRecord> {
    let session_title = if !session.title.trim().is_empty() {
        session.title.trim()
    } else if !session.preview.trim().is_empty() {
        session.preview.trim()
    } else {
        "Untitled session"
    };
    let mut found = std::collections::BTreeMap::<String, ArtifactRecord>::new();
    for message in messages {
        let mut candidates = Vec::new();
        collect_artifacts_from_message(message, &mut candidates);
        for (value, _) in candidates {
            let id = format!("{}:{value}", session.session_id);
            found.entry(id.clone()).or_insert_with(|| ArtifactRecord {
                id,
                kind: artifact_kind(&value),
                label: label(&value),
                href: value.clone(),
                value,
                session_id: session.session_id.clone(),
                profile: session.profile.clone(),
                session_title: session_title.to_owned(),
                timestamp_ms: timestamp_ms(message, session),
            });
        }
    }
    let mut items: Vec<_> = found.into_values().collect();
    items.sort_by(|left, right| right.timestamp_ms.cmp(&left.timestamp_ms));
    items
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArtifactMessagePagination {
    pub limit: usize,
    pub offset: usize,
    pub order: String,
    pub returned: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ArtifactMessagePage {
    pub messages: Vec<Value>,
    pub pagination: Option<ArtifactMessagePagination>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArtifactLoadFailure {
    pub session_id: String,
    pub session_title: String,
    pub error: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ArtifactIndexSnapshot {
    pub session_count: usize,
    pub artifacts: Vec<ArtifactRecord>,
    pub failures: Vec<ArtifactLoadFailure>,
}

pub const ARTIFACT_MESSAGE_PAGE_SIZE: usize = 100;
pub const MAX_ARTIFACT_MESSAGE_PAGE_JSON_CHARS: usize = 32_000_000;

pub fn load_artifacts_for_sessions(
    sessions: &[ArtifactSession],
    mut load_page: impl FnMut(&ArtifactSession, usize, usize) -> Result<ArtifactMessagePage, String>,
) -> ArtifactIndexSnapshot {
    let mut artifacts = std::collections::BTreeMap::<String, ArtifactRecord>::new();
    let mut failures = Vec::new();

    for session in sessions {
        let mut session_items = std::collections::BTreeMap::<String, ArtifactRecord>::new();
        let mut offset = 0usize;
        let result = (|| {
            loop {
                let page = load_page(session, ARTIFACT_MESSAGE_PAGE_SIZE, offset)?;
                let json = serde_json::to_string(&page.messages)
                    .map_err(|_| "Hermes transcript page was not valid JSON".to_owned())?;
                let json_chars = json.encode_utf16().count();
                if json_chars > MAX_ARTIFACT_MESSAGE_PAGE_JSON_CHARS {
                    return Err(
                        "Session transcript page exceeds the Hermes Desktop safe-load limit".into(),
                    );
                }
                if let Some(pagination) = &page.pagination {
                    if page.messages.len() > ARTIFACT_MESSAGE_PAGE_SIZE
                        || pagination.limit != ARTIFACT_MESSAGE_PAGE_SIZE
                        || pagination.offset != offset
                        || pagination.order != "oldest"
                        || pagination.returned != page.messages.len()
                    {
                        return Err(
                            "Hermes did not confirm the requested artifact transcript page".into(),
                        );
                    }
                }
                for item in collect_artifacts_for_session(session, &page.messages) {
                    session_items.entry(item.id.clone()).or_insert(item);
                }
                let Some(pagination) = page.pagination else {
                    break;
                };
                if page.messages.is_empty() || page.messages.len() < pagination.limit {
                    break;
                }
                offset = offset
                    .checked_add(page.messages.len())
                    .ok_or_else(|| "Hermes artifact transcript offset overflowed".to_owned())?;
                if offset > 10_000_000 {
                    return Err("Hermes artifact transcript exceeded the safe page count".into());
                }
            }
            Ok(())
        })();
        match result {
            Ok(()) => artifacts.extend(session_items),
            Err(error) => failures.push(ArtifactLoadFailure {
                session_id: session.session_id.clone(),
                session_title: if session.title.trim().is_empty() {
                    session.preview.trim()
                } else {
                    session.title.trim()
                }
                .to_owned(),
                error,
            }),
        }
    }
    let mut artifacts: Vec<_> = artifacts.into_values().collect();
    artifacts.sort_by(|left, right| right.timestamp_ms.cmp(&left.timestamp_ms));
    ArtifactIndexSnapshot {
        session_count: sessions.len(),
        artifacts,
        failures,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn session() -> ArtifactSession {
        ArtifactSession {
            session_id: "s1".into(),
            profile: Some("default".into()),
            title: "Render session".into(),
            preview: String::new(),
            started_at: 1_700_000_000.0,
            last_active: None,
        }
    }

    #[test]
    fn collects_images_links_and_explicit_media_tags_deduplicated() {
        let messages = vec![
            json!({"role":"assistant","timestamp":1700000001.0,"content":"Here ![preview](/tmp/chart.png) and [report](https://example.test/report.pdf)\nMEDIA:/tmp/chart.png output at /var/tmp/answer.csv"}),
        ];
        let items = collect_artifacts_for_session(&session(), &messages);
        assert_eq!(items.len(), 3);
        assert_eq!(
            items
                .iter()
                .filter(|item| item.kind == ArtifactKind::Image)
                .count(),
            1
        );
        assert_eq!(items[0].timestamp_ms, 1_700_000_001_000);
        assert!(
            items
                .iter()
                .any(|item| item.value == "https://example.test/report.pdf")
        );
    }

    #[test]
    fn ignores_python_install_noise_but_accepts_generated_files() {
        let messages = vec![
            json!({"role":"assistant","content":"Downloading https://pypi.org/packages/pkg.zip and /home/user/.cache/pip/pkg.whl"}),
            json!({"role":"tool","tool_name":"generate_image","content":{"generated_file":"/tmp/output.webp"}}),
        ];
        let items = collect_artifacts_for_session(&session(), &messages);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].value, "/tmp/output.webp");
    }

    #[test]
    fn supports_media_aliases_browser_screenshots_and_producer_payloads() {
        let messages = vec![
            json!({"role":"assistant","content":"![chart](#media:%2Ftmp%2Fmonthly%20chart.png)"}),
            json!({"role":"tool","tool_name":"browser_vision","content":"Screenshot path: C:\\temp\\screen.png"}),
            json!({"role":"tool","tool_name":"generate_image","content":{"result":{"generated_file":"/tmp/generated.webp"}}}),
            json!({"role":"tool","tool_name":"search","content":"unrelated output at /tmp/not-an-artifact.csv"}),
        ];
        let items = collect_artifacts_for_session(&session(), &messages);
        let values: Vec<_> = items.iter().map(|item| item.value.as_str()).collect();
        assert_eq!(values.len(), 3);
        assert!(values.contains(&"/tmp/monthly chart.png"));
        assert!(values.contains(&r"C:\temp\screen.png"));
        assert!(values.contains(&"/tmp/generated.webp"));
    }

    #[test]
    fn pages_oldest_transcript_until_short_page_and_collects_all_artifacts() {
        let sessions = vec![session()];
        let mut offsets = Vec::new();
        let snapshot = load_artifacts_for_sessions(&sessions, |_, limit, offset| {
            offsets.push((limit, offset));
            let messages = if offset == 0 {
                let mut first_page = vec![json!({"role":"assistant","content":"/tmp/first.txt"})];
                first_page.extend(
                    (1..100)
                        .map(|index| json!({"role":"user","content":format!("message {index}")})),
                );
                first_page
            } else {
                vec![json!({"role":"assistant","content":"/tmp/second.txt"})]
            };
            Ok(ArtifactMessagePage {
                pagination: Some(ArtifactMessagePagination {
                    limit,
                    offset,
                    order: "oldest".into(),
                    returned: messages.len(),
                }),
                messages,
            })
        });
        assert_eq!(snapshot.artifacts.len(), 2);
        assert_eq!(snapshot.failures.len(), 0);
        assert_eq!(offsets, vec![(100, 0), (100, 100)]);
    }

    #[test]
    fn filters_and_query_values_are_canonical() {
        assert_eq!(ArtifactFilter::parse("image").as_str(), "image");
        assert!(ArtifactFilter::Image.matches(ArtifactKind::Image));
        assert!(!ArtifactFilter::Image.matches(ArtifactKind::File));
        assert_eq!(
            label("https://example.test/report.pdf?download=1"),
            "report.pdf"
        );
    }
}
