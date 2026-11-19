//! Exact owner identity for stored Hermes sessions.
//!
//! This module models source ownership without implementing connection
//! discovery, credential lookup, dialing, or network dispatch. Callers must
//! resolve a tagged connection through a registry before sending a request.

use std::{
    collections::{BTreeMap, HashSet},
    fmt,
};

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

/// Registry for resolving session owners to their configured connection state.
///
/// Tagged identities resolve only by their exact validated connection ID. An
/// untagged legacy identity resolves only when one legacy connection has been
/// explicitly configured with [`configure_legacy_connection`](Self::configure_legacy_connection).
/// A missing or removed tagged owner never falls back to the legacy entry.
#[derive(Debug)]
pub struct SessionOwnerRegistry<T> {
    connections: BTreeMap<String, T>,
    legacy_connection_id: Option<String>,
}

impl<T> Default for SessionOwnerRegistry<T> {
    fn default() -> Self {
        Self {
            connections: BTreeMap::new(),
            legacy_connection_id: None,
        }
    }
}

impl<T> SessionOwnerRegistry<T> {
    /// Registers a connection under its normalized stable ID.
    ///
    /// Registering the same normalized ID twice is rejected rather than
    /// replacing a possibly live connection silently.
    pub fn register(
        &mut self,
        connection_id: &str,
        connection: T,
    ) -> Result<(), SessionOwnerError> {
        let connection_id = normalize_component(connection_id, OwnerComponent::ConnectionId)?;
        if self.connections.contains_key(&connection_id) {
            return Err(SessionOwnerError::DuplicateConnectionId(connection_id));
        }
        self.connections.insert(connection_id, connection);
        Ok(())
    }

    /// Removes a connection. Removing the explicitly configured legacy
    /// connection also clears that legacy designation.
    pub fn remove(&mut self, connection_id: &str) -> Result<Option<T>, SessionOwnerError> {
        let connection_id = normalize_component(connection_id, OwnerComponent::ConnectionId)?;
        let removed = self.connections.remove(&connection_id);
        if self.legacy_connection_id.as_deref() == Some(connection_id.as_str()) {
            self.legacy_connection_id = None;
        }
        Ok(removed)
    }

    /// Marks one already registered connection as the sole legacy route.
    ///
    /// Repeating the same configuration is idempotent. A different second
    /// legacy connection is rejected so untagged rows cannot become ambiguous.
    pub fn configure_legacy_connection(
        &mut self,
        connection_id: &str,
    ) -> Result<(), SessionOwnerError> {
        let connection_id = normalize_component(connection_id, OwnerComponent::ConnectionId)?;
        if !self.connections.contains_key(&connection_id) {
            return Err(SessionOwnerError::LegacyConnectionNotRegistered(
                connection_id,
            ));
        }
        match self.legacy_connection_id.as_deref() {
            None => self.legacy_connection_id = Some(connection_id),
            Some(current) if current == connection_id => {}
            Some(current) => {
                return Err(SessionOwnerError::LegacyConnectionAlreadyConfigured(
                    current.to_owned(),
                ));
            }
        }
        Ok(())
    }

    /// Resolves a tagged owner exactly, or an untagged owner through the
    /// explicitly configured single legacy connection.
    pub fn resolve<'a>(
        &'a self,
        owner: &'a SessionOwnerIdentity,
    ) -> Result<ResolvedSessionOwner<'a, T>, SessionOwnerError> {
        let (connection_id, route) = match owner.connection_id() {
            Some(connection_id) => (connection_id, SessionOwnerRouteKind::Exact),
            None => (
                self.legacy_connection_id
                    .as_deref()
                    .ok_or(SessionOwnerError::LegacyConnectionNotConfigured)?,
                SessionOwnerRouteKind::Legacy,
            ),
        };
        let connection = self
            .connections
            .get(connection_id)
            .ok_or_else(|| SessionOwnerError::UnregisteredConnectionId(connection_id.to_owned()))?;
        Ok(ResolvedSessionOwner {
            connection_id,
            profile: owner.profile(),
            connection,
            route,
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SessionOwnerRouteKind {
    Exact,
    Legacy,
}

/// Borrowed result of resolving a session owner through a registry.
#[derive(Debug)]
pub struct ResolvedSessionOwner<'a, T> {
    pub connection_id: &'a str,
    pub profile: &'a str,
    pub connection: &'a T,
    route: SessionOwnerRouteKind,
}

impl<T> ResolvedSessionOwner<'_, T> {
    /// Whether the row had an exact connection tag or used the configured
    /// legacy route for an untagged row.
    pub fn is_legacy_route(&self) -> bool {
        self.route == SessionOwnerRouteKind::Legacy
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
    DuplicateConnectionId(String),
    LegacyConnectionNotConfigured,
    LegacyConnectionNotRegistered(String),
    LegacyConnectionAlreadyConfigured(String),
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
            Self::DuplicateConnectionId(connection_id) => write!(
                formatter,
                "session owner connection is already registered: {connection_id}"
            ),
            Self::LegacyConnectionNotConfigured => {
                formatter.write_str("no legacy session owner connection is configured")
            }
            Self::LegacyConnectionNotRegistered(connection_id) => write!(
                formatter,
                "legacy session owner connection is not registered: {connection_id}"
            ),
            Self::LegacyConnectionAlreadyConfigured(connection_id) => write!(
                formatter,
                "a different legacy session owner connection is already configured: {connection_id}"
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
    use super::{
        SessionOwnerError, SessionOwnerIdentity, SessionOwnerRegistry, SessionRouteDecision,
    };
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

    #[test]
    fn registry_resolves_tagged_owner_to_exact_registered_connection() {
        let mut registry = SessionOwnerRegistry::default();
        registry.register("remote-a", "connection A").unwrap();
        registry.register("remote-b", "connection B").unwrap();
        let owner = SessionOwnerIdentity::new(Some(" remote-b "), Some(" worker ")).unwrap();

        let resolved = registry.resolve(&owner).unwrap();
        assert_eq!(resolved.connection_id, "remote-b");
        assert_eq!(resolved.profile, "worker");
        assert_eq!(*resolved.connection, "connection B");
        assert!(!resolved.is_legacy_route());
    }

    #[test]
    fn registry_rejects_duplicate_normalized_connection_registration() {
        let mut registry = SessionOwnerRegistry::default();
        registry.register(" remote-a ", "first").unwrap();

        assert_eq!(
            registry.register("remote-a", "replacement"),
            Err(SessionOwnerError::DuplicateConnectionId(
                "remote-a".to_owned()
            ))
        );
        assert_eq!(
            *registry
                .resolve(&SessionOwnerIdentity::new(Some("remote-a"), None).unwrap())
                .unwrap()
                .connection,
            "first"
        );
    }

    #[test]
    fn registry_fails_closed_for_unknown_tagged_owner() {
        let mut registry = SessionOwnerRegistry::default();
        registry.register("remote-a", "connection A").unwrap();
        registry.configure_legacy_connection("remote-a").unwrap();
        let unknown = SessionOwnerIdentity::new(Some("remote-missing"), Some("worker"))
            .expect("valid but unregistered owner");

        assert_eq!(
            registry.resolve(&unknown).unwrap_err(),
            SessionOwnerError::UnregisteredConnectionId("remote-missing".to_owned())
        );
    }

    #[test]
    fn legacy_resolution_requires_one_explicitly_configured_registered_connection() {
        let legacy_owner = SessionOwnerIdentity::new(None, Some("worker")).unwrap();
        let mut registry = SessionOwnerRegistry::default();
        registry.register("remote-a", "connection A").unwrap();
        registry.register("remote-b", "connection B").unwrap();

        assert_eq!(
            registry.resolve(&legacy_owner).unwrap_err(),
            SessionOwnerError::LegacyConnectionNotConfigured
        );
        assert_eq!(
            registry.configure_legacy_connection("remote-missing"),
            Err(SessionOwnerError::LegacyConnectionNotRegistered(
                "remote-missing".to_owned()
            ))
        );

        registry.configure_legacy_connection(" remote-a ").unwrap();
        registry.configure_legacy_connection("remote-a").unwrap();
        assert_eq!(
            registry.configure_legacy_connection("remote-b"),
            Err(SessionOwnerError::LegacyConnectionAlreadyConfigured(
                "remote-a".to_owned()
            ))
        );

        let resolved = registry.resolve(&legacy_owner).unwrap();
        assert_eq!(resolved.connection_id, "remote-a");
        assert_eq!(resolved.profile, "worker");
        assert_eq!(*resolved.connection, "connection A");
        assert!(resolved.is_legacy_route());

        registry.remove("remote-a").unwrap();
        assert_eq!(
            registry.resolve(&legacy_owner).unwrap_err(),
            SessionOwnerError::LegacyConnectionNotConfigured
        );
    }
}
