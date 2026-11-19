//! Exact owner identity for stored Hermes sessions.
//!
//! This module models source ownership without implementing connection
//! discovery, credential lookup, dialing, or network dispatch. Callers must
//! resolve a tagged connection through a registry before sending a request.

use std::{collections::HashSet, fmt};

const DEFAULT_PROFILE: &str = "default";
const MAX_OWNER_COMPONENT_BYTES: usize = 256;

/// Stable identity of a session's backend owner.
///
/// `connection_id == None` represents a truly untagged legacy row and may use
/// the legacy profile route. A present connection ID is an exact routing
/// requirement; it is never interchangeable with the active/ambient backend.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct SessionOwnerIdentity {
    connection_id: Option<String>,
    profile: String,
}

impl SessionOwnerIdentity {
    /// Creates a normalized owner identity.
    ///
    /// Profile names are trimmed and empty/missing values normalize to
    /// `default`. A present connection ID is trimmed but must remain
    /// non-empty. Components reject control characters and values over 256
    /// UTF-8 bytes so malformed owner tags fail before routing.
    pub fn new(
        connection_id: Option<&str>,
        profile: Option<&str>,
    ) -> Result<Self, SessionOwnerError> {
        let connection_id = connection_id
            .map(|value| normalize_component(value, OwnerComponent::ConnectionId))
            .transpose()?;

        let profile = profile
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or(DEFAULT_PROFILE);
        let profile = normalize_component(profile, OwnerComponent::Profile)?;
        if profile == "all" {
            return Err(SessionOwnerError::InvalidProfile);
        }

        Ok(Self {
            connection_id,
            profile,
        })
    }

    pub fn connection_id(&self) -> Option<&str> {
        self.connection_id.as_deref()
    }

    pub fn profile(&self) -> &str {
        &self.profile
    }

    /// Produces a collision-safe key for a session under this exact owner.
    ///
    /// Each UTF-8 component is length-prefixed, and an explicit owner marker
    /// distinguishes an untagged legacy row from every tagged row.
    pub fn session_identity_key(&self, session_id: &str) -> Result<String, SessionOwnerError> {
        if session_id.is_empty()
            || session_id.len() > MAX_OWNER_COMPONENT_BYTES
            || session_id.chars().any(char::is_control)
        {
            return Err(SessionOwnerError::InvalidSessionId);
        }

        let (owner_marker, connection_id) = match self.connection_id.as_deref() {
            Some(connection_id) => ("tagged", connection_id),
            None => ("legacy", ""),
        };
        Ok(format!(
            "v1|{owner_marker}|{}:{}|{}:{}|{}:{}",
            connection_id.len(),
            connection_id,
            self.profile.len(),
            self.profile,
            session_id.len(),
            session_id,
        ))
    }

    /// Resolves the safe route class against registered connection IDs.
    ///
    /// A tagged owner must be present in the registry. It cannot fall back to
    /// the legacy/ambient route when missing. Only a row that was genuinely
    /// untagged receives the legacy profile decision.
    pub fn route_decision<'a>(
        &'a self,
        registered_connection_ids: &HashSet<String>,
    ) -> Result<SessionRouteDecision<'a>, SessionOwnerError> {
        match self.connection_id.as_deref() {
            Some(connection_id) if registered_connection_ids.contains(connection_id) => {
                Ok(SessionRouteDecision::RegisteredConnection {
                    connection_id,
                    profile: &self.profile,
                })
            }
            Some(connection_id) => Err(SessionOwnerError::UnregisteredConnectionId(
                connection_id.to_owned(),
            )),
            None => Ok(SessionRouteDecision::LegacyProfile {
                profile: &self.profile,
            }),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SessionRouteDecision<'a> {
    /// The exact owner is known and registered; dispatch must use this pair.
    RegisteredConnection {
        connection_id: &'a str,
        profile: &'a str,
    },
    /// The source row had no connection tag; callers may use legacy profile routing.
    LegacyProfile { profile: &'a str },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SessionOwnerError {
    InvalidConnectionId,
    InvalidProfile,
    InvalidSessionId,
    UnregisteredConnectionId(String),
}

impl fmt::Display for SessionOwnerError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidConnectionId => {
                formatter.write_str("session owner connection ID is invalid")
            }
            Self::InvalidProfile => formatter.write_str("session owner profile is invalid"),
            Self::InvalidSessionId => formatter.write_str("session ID is invalid"),
            Self::UnregisteredConnectionId(connection_id) => write!(
                formatter,
                "session owner connection is not registered: {connection_id}"
            ),
        }
    }
}

impl std::error::Error for SessionOwnerError {}

#[derive(Clone, Copy)]
enum OwnerComponent {
    ConnectionId,
    Profile,
}

fn normalize_component(
    value: &str,
    component: OwnerComponent,
) -> Result<String, SessionOwnerError> {
    let value = value.trim();
    let invalid = value.is_empty()
        || value.len() > MAX_OWNER_COMPONENT_BYTES
        || value.chars().any(char::is_control);
    if invalid {
        return Err(match component {
            OwnerComponent::ConnectionId => SessionOwnerError::InvalidConnectionId,
            OwnerComponent::Profile => SessionOwnerError::InvalidProfile,
        });
    }
    Ok(value.to_owned())
}

#[cfg(test)]
mod tests {
    use super::{SessionOwnerError, SessionOwnerIdentity, SessionRouteDecision};
    use std::collections::HashSet;

    #[test]
    fn identical_session_and_profile_on_different_connections_have_distinct_keys() {
        let first =
            SessionOwnerIdentity::new(Some("remote-a"), Some("worker")).expect("valid owner A");
        let second =
            SessionOwnerIdentity::new(Some("remote-b"), Some("worker")).expect("valid owner B");

        assert_ne!(
            first.session_identity_key("same-session").unwrap(),
            second.session_identity_key("same-session").unwrap()
        );
    }

    #[test]
    fn profile_is_trimmed_and_missing_or_empty_profile_uses_default() {
        let trimmed = SessionOwnerIdentity::new(Some(" remote "), Some(" worker "))
            .expect("valid normalized owner");
        assert_eq!(trimmed.connection_id(), Some("remote"));
        assert_eq!(trimmed.profile(), "worker");

        assert_eq!(
            SessionOwnerIdentity::new(None, None).unwrap().profile(),
            "default"
        );
        assert_eq!(
            SessionOwnerIdentity::new(None, Some("  "))
                .unwrap()
                .profile(),
            "default"
        );
    }

    #[test]
    fn malformed_present_connection_owner_is_rejected() {
        assert_eq!(
            SessionOwnerIdentity::new(Some("  "), Some("worker")),
            Err(SessionOwnerError::InvalidConnectionId)
        );
        assert_eq!(
            SessionOwnerIdentity::new(Some("remote\nowner"), Some("worker")),
            Err(SessionOwnerError::InvalidConnectionId)
        );
        assert_eq!(
            SessionOwnerIdentity::new(Some(&"x".repeat(257)), Some("worker")),
            Err(SessionOwnerError::InvalidConnectionId)
        );
    }

    #[test]
    fn tagged_owner_selects_only_the_exact_registered_connection() {
        let owner = SessionOwnerIdentity::new(Some("remote-b"), Some("worker"))
            .expect("valid tagged owner");
        let registered = HashSet::from(["remote-a".to_owned(), "remote-b".to_owned()]);

        assert_eq!(
            owner.route_decision(&registered),
            Ok(SessionRouteDecision::RegisteredConnection {
                connection_id: "remote-b",
                profile: "worker"
            })
        );
        assert_eq!(
            owner.route_decision(&HashSet::from(["remote-a".to_owned()])),
            Err(SessionOwnerError::UnregisteredConnectionId(
                "remote-b".to_owned()
            ))
        );
    }

    #[test]
    fn only_untagged_owner_uses_legacy_profile_routing() {
        let untagged = SessionOwnerIdentity::new(None, Some("worker")).unwrap();
        assert_eq!(
            untagged.route_decision(&HashSet::new()),
            Ok(SessionRouteDecision::LegacyProfile { profile: "worker" })
        );
        assert_eq!(
            SessionOwnerIdentity::new(Some("missing"), Some("worker"))
                .unwrap()
                .route_decision(&HashSet::new()),
            Err(SessionOwnerError::UnregisteredConnectionId(
                "missing".to_owned()
            ))
        );
    }

    #[test]
    fn length_prefixed_keys_do_not_collide_on_delimiters() {
        let first =
            SessionOwnerIdentity::new(Some("a|2:b"), Some("c")).expect("valid tagged identity");
        let second =
            SessionOwnerIdentity::new(Some("a"), Some("2:b|c")).expect("valid tagged identity");

        assert_ne!(
            first.session_identity_key("session").unwrap(),
            second.session_identity_key("session").unwrap()
        );
    }
}
