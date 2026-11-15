use std::collections::BTreeSet;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AppRoute {
    NewChat,
    Session(String),
    Capabilities,
    Messaging,
    Artifacts,
    SessionImport,
    Agents,
    CommandCenter,
    Cron,
    Profiles,
    Settings,
    Starmap,
    Webhooks,
    Extension(String),
    Unknown(String),
}

impl AppRoute {
    pub fn parse(target: &str, contributed_paths: &BTreeSet<String>) -> Self {
        let path = route_pathname(target);
        match path {
            "/" => return Self::NewChat,
            "/capabilities" => return Self::Capabilities,
            "/messaging" => return Self::Messaging,
            "/artifacts" => return Self::Artifacts,
            "/session-import" => return Self::SessionImport,
            "/agents" => return Self::Agents,
            "/command-center" => return Self::CommandCenter,
            "/cron" => return Self::Cron,
            "/profiles" => return Self::Profiles,
            "/settings" => return Self::Settings,
            "/starmap" => return Self::Starmap,
            "/webhooks" => return Self::Webhooks,
            _ => {}
        }

        if contributed_paths.contains(path) {
            return Self::Extension(path.to_owned());
        }

        if let Some(encoded_id) = path.strip_prefix('/')
            && !encoded_id.is_empty()
            && !encoded_id.contains('/')
        {
            return decode_component(encoded_id)
                .map(Self::Session)
                .unwrap_or_else(|| Self::Unknown(path.to_owned()));
        }

        Self::Unknown(path.to_owned())
    }

    pub fn path(&self) -> String {
        match self {
            Self::NewChat => "/".into(),
            Self::Session(id) => format!("/{}", encode_component(id)),
            Self::Capabilities => "/capabilities".into(),
            Self::Messaging => "/messaging".into(),
            Self::Artifacts => "/artifacts".into(),
            Self::SessionImport => "/session-import".into(),
            Self::Agents => "/agents".into(),
            Self::CommandCenter => "/command-center".into(),
            Self::Cron => "/cron".into(),
            Self::Profiles => "/profiles".into(),
            Self::Settings => "/settings".into(),
            Self::Starmap => "/starmap".into(),
            Self::Webhooks => "/webhooks".into(),
            Self::Extension(path) | Self::Unknown(path) => path.clone(),
        }
    }

    pub fn is_overlay(&self) -> bool {
        matches!(
            self,
            Self::SessionImport
                | Self::Agents
                | Self::CommandCenter
                | Self::Cron
                | Self::Profiles
                | Self::Settings
                | Self::Starmap
                | Self::Webhooks
        )
    }

    pub fn title(&self) -> &str {
        match self {
            Self::NewChat => "New session",
            Self::Session(_) => "Session",
            Self::Capabilities => "Capabilities",
            Self::Messaging => "Messaging",
            Self::Artifacts => "Artifacts",
            Self::SessionImport => "Import sessions",
            Self::Agents => "Agents",
            Self::CommandCenter => "Command center",
            Self::Cron => "Scheduled jobs",
            Self::Profiles => "Profiles",
            Self::Settings => "Settings",
            Self::Starmap => "Star map",
            Self::Webhooks => "Webhooks",
            Self::Extension(_) => "Extension",
            Self::Unknown(_) => "Hermes",
        }
    }
}

pub fn route_pathname(target: &str) -> &str {
    let end = target.find(['?', '#']).unwrap_or(target.len());
    &target[..end]
}

fn encode_component(value: &str) -> String {
    let mut encoded = String::with_capacity(value.len());
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
            encoded.push(char::from(byte));
        } else {
            encoded.push('%');
            encoded.push(char::from(hex_digit(byte >> 4)));
            encoded.push(char::from(hex_digit(byte & 0x0f)));
        }
    }
    encoded
}

fn decode_component(value: &str) -> Option<String> {
    let bytes = value.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' {
            let high = hex_value(*bytes.get(index + 1)?)?;
            let low = hex_value(*bytes.get(index + 2)?)?;
            decoded.push((high << 4) | low);
            index += 3;
        } else {
            decoded.push(bytes[index]);
            index += 1;
        }
    }
    String::from_utf8(decoded).ok()
}

fn hex_digit(value: u8) -> u8 {
    match value {
        0..=9 => b'0' + value,
        _ => b'A' + (value - 10),
    }
}

fn hex_value(value: u8) -> Option<u8> {
    match value {
        b'0'..=b'9' => Some(value - b'0'),
        b'a'..=b'f' => Some(value - b'a' + 10),
        b'A'..=b'F' => Some(value - b'A' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reserved_pages_and_overlays_are_not_session_ids() {
        let contributed = BTreeSet::new();
        for (path, expected_overlay) in [
            ("/capabilities", false),
            ("/messaging", false),
            ("/artifacts", false),
            ("/session-import", true),
            ("/agents", true),
            ("/command-center", true),
            ("/cron", true),
            ("/profiles", true),
            ("/settings", true),
            ("/starmap", true),
            ("/webhooks", true),
        ] {
            let route = AppRoute::parse(path, &contributed);
            assert_ne!(
                route,
                AppRoute::Session(path.trim_start_matches('/').to_owned())
            );
            assert_eq!(route.is_overlay(), expected_overlay, "{path}");
        }
    }

    #[test]
    fn query_and_hash_are_removed_before_route_classification() {
        assert_eq!(
            AppRoute::parse("/capabilities?tab=connectors#active", &BTreeSet::new()),
            AppRoute::Capabilities
        );
    }

    #[test]
    fn contributed_paths_are_reserved_and_session_ids_round_trip() {
        let contributed = BTreeSet::from(["/reports".to_owned()]);
        assert_eq!(
            AppRoute::parse("/reports", &contributed),
            AppRoute::Extension("/reports".into())
        );
        assert_eq!(
            AppRoute::parse("/reports", &BTreeSet::new()),
            AppRoute::Session("reports".into())
        );
        let session = AppRoute::Session("room/one λ".into());
        assert_eq!(AppRoute::parse(&session.path(), &contributed), session);
    }

    #[test]
    fn malformed_multi_segment_paths_are_not_sessions() {
        assert_eq!(
            AppRoute::parse("/bad%2", &BTreeSet::new()),
            AppRoute::Unknown("/bad%2".into())
        );
        assert_eq!(
            AppRoute::parse("/one/two", &BTreeSet::new()),
            AppRoute::Unknown("/one/two".into())
        );
    }
}
