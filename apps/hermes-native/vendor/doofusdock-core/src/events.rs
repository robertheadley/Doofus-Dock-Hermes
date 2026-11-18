//! Bounded, ordered notifications about committed workspace facts.
//!
//! Consumers poll immutable event batches. No callback runs inside a canonical
//! transaction, so any follow-up mutation must use a fresh service revision.
use crate::{CommandId, DockError, DockResult, ErrorCode, PanelId, TransitionId, WindowId};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, VecDeque};

const MAX_EVENT_QUEUE: usize = 4096;
const MAX_SUBSCRIBERS: usize = 256;
const MAX_EVENT_PAYLOAD_FIELDS: usize = 32;
const MAX_EVENT_KEY_BYTES: usize = 64;
const MAX_EVENT_VALUE_BYTES: usize = 512;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EventId(pub u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EventSubscriptionId(pub u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FrameworkEventKind {
    LayoutCommitted,
    LayoutRestored,
    PanelCreated,
    PanelClosed,
    PanelVisibilityChanged,
    PanelWorkspaceMembershipChanged,
    GroupMinimizedChanged,
    ScratchpadChanged,
    PanelMoved,
    PanelDrawerChanged,
    GroupChanged,
    WindowFloated,
    WindowDocked,
    WindowGeometryChanged,
    PictureInPictureChanged,
    WindowZOrderChanged,
    WindowMonitorChanged,
    FocusChanged,
    NativeViewportFocused,
    NativeViewportCloseRequested,
    NativeViewportGeometryObserved,
    NativeRootViewportGeometryObserved,
    WorkspaceActivated,
    ModalityChanged,
    MonitorChanged,
    TransitionStarted,
    TransitionCompleted,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FrameworkEvent {
    pub id: EventId,
    pub sequence: u64,
    pub revision: u64,
    pub kind: FrameworkEventKind,
    pub command: CommandId,
    pub payload: BTreeMap<String, String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EventBatch {
    pub events: Vec<FrameworkEvent>,
    /// Number of older events removed before this subscriber read its cursor.
    pub lost_events: u64,
}

/// A validated observation reported by a renderer or native host. These facts
/// describe host state and do not mutate the canonical layout document.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HostObservation {
    PanelFocused {
        panel: PanelId,
        floating_window: Option<WindowId>,
    },
    /// The root viewport is `None`; detached native viewports carry their
    /// canonical floating-window identity.
    NativeViewportFocused { window: Option<WindowId> },
    /// A user or window manager requested closure of a native floating view.
    /// This observation does not itself mutate or close canonical content.
    NativeViewportCloseRequested { window: WindowId },
    /// Geometry reported by the native host before canonical snapping or workspace policy is applied.
    NativeViewportGeometryObserved {
        window: WindowId,
        geometry: crate::Rect,
    },
    /// Stable geometry reported by the native root viewport before layout policy.
    NativeRootViewportGeometryObserved { geometry: crate::Rect },
    /// Host display topology changed independently of canonical window placement.
    MonitorChanged {
        monitor: String,
        work_area: crate::Rect,
        scale_milli: u32,
        primary: bool,
        connected: bool,
    },
    /// A renderer or animation provider began a presentation transition.
    TransitionStarted { transition: TransitionId },
    /// A renderer or animation provider completed a presentation transition.
    TransitionCompleted { transition: TransitionId },
}

#[derive(Clone, Debug)]
struct Subscriber {
    cursor: u64,
}

/// Single-writer event buffer with bounded subscriber cursors and duplicate
/// suppression for a repeated `(revision, command, kind)` publication.
#[derive(Clone, Debug)]
pub struct EventHub {
    capacity: usize,
    next_sequence: u64,
    next_subscription: u64,
    events: VecDeque<FrameworkEvent>,
    subscribers: BTreeMap<EventSubscriptionId, Subscriber>,
}

impl EventHub {
    pub fn new(capacity: usize) -> DockResult<Self> {
        if !(1..=MAX_EVENT_QUEUE).contains(&capacity) {
            return Err(event_error(
                ErrorCode::ResourceLimit,
                "Event queue capacity must be between 1 and 4096.",
                capacity.to_string(),
            ));
        }
        Ok(Self {
            capacity,
            next_sequence: 1,
            next_subscription: 1,
            events: VecDeque::with_capacity(capacity.min(256)),
            subscribers: BTreeMap::new(),
        })
    }

    /// Start receiving events published after this call.
    pub fn subscribe(&mut self) -> DockResult<EventSubscriptionId> {
        if self.subscribers.len() >= MAX_SUBSCRIBERS {
            return Err(event_error(
                ErrorCode::ResourceLimit,
                "Event subscriber limit has been reached.",
                self.subscribers.len().to_string(),
            ));
        }
        let id = EventSubscriptionId(self.next_subscription);
        self.next_subscription = self.next_subscription.checked_add(1).ok_or_else(|| {
            event_error(
                ErrorCode::ResourceLimit,
                "Event subscription ID overflow.",
                "subscribe",
            )
        })?;
        self.subscribers.insert(
            id,
            Subscriber {
                cursor: self.next_sequence.saturating_sub(1),
            },
        );
        Ok(id)
    }

    pub fn unsubscribe(&mut self, id: EventSubscriptionId) -> bool {
        self.subscribers.remove(&id).is_some()
    }

    pub fn publish(
        &mut self,
        revision: u64,
        kind: FrameworkEventKind,
        command: CommandId,
        payload: BTreeMap<String, String>,
    ) -> DockResult<Option<FrameworkEvent>> {
        if (revision == 0
            && !matches!(
                kind,
                FrameworkEventKind::FocusChanged
                    | FrameworkEventKind::NativeViewportFocused
                    | FrameworkEventKind::NativeViewportCloseRequested
                    | FrameworkEventKind::NativeViewportGeometryObserved
                    | FrameworkEventKind::NativeRootViewportGeometryObserved
                    | FrameworkEventKind::MonitorChanged
            ))
            || command.is_empty()
            || command.0.len() > 128
        {
            return Err(event_error(
                ErrorCode::SchemaError,
                "Event requires a valid revision for its kind and a bounded command ID.",
                format!("revision={revision}:command={}", command.0),
            ));
        }
        validate_payload(&payload)?;
        if self.events.iter().any(|event| {
            event.revision == revision
                && event.command == command
                && event.kind == kind
                && event.payload == payload
        }) {
            return Ok(None);
        }
        let sequence = self.next_sequence;
        self.next_sequence = self.next_sequence.checked_add(1).ok_or_else(|| {
            event_error(
                ErrorCode::ResourceLimit,
                "Event sequence overflow.",
                "publish",
            )
        })?;
        let event = FrameworkEvent {
            id: EventId(sequence),
            sequence,
            revision,
            kind,
            command,
            payload,
        };
        if self.events.len() == self.capacity {
            self.events.pop_front();
        }
        self.events.push_back(event.clone());
        Ok(Some(event))
    }

    pub fn read(&mut self, id: EventSubscriptionId, limit: usize) -> DockResult<EventBatch> {
        if !(1..=256).contains(&limit) {
            return Err(event_error(
                ErrorCode::ResourceLimit,
                "Event read limit must be between 1 and 256.",
                limit.to_string(),
            ));
        }
        let subscriber = self.subscribers.get_mut(&id).ok_or_else(|| {
            event_error(
                ErrorCode::NotFound,
                "Event subscription does not exist.",
                id.0.to_string(),
            )
        })?;
        let oldest = self.events.front().map(|event| event.sequence);
        let lost_events = oldest
            .map(|oldest| oldest.saturating_sub(subscriber.cursor.saturating_add(1)))
            .unwrap_or(0);
        if lost_events > 0 {
            subscriber.cursor = oldest.unwrap().saturating_sub(1);
        }
        let events = self
            .events
            .iter()
            .filter(|event| event.sequence > subscriber.cursor)
            .take(limit)
            .cloned()
            .collect::<Vec<_>>();
        if let Some(last) = events.last() {
            subscriber.cursor = last.sequence;
        }
        Ok(EventBatch {
            events,
            lost_events,
        })
    }
}

fn validate_payload(payload: &BTreeMap<String, String>) -> DockResult<()> {
    if payload.len() > MAX_EVENT_PAYLOAD_FIELDS
        || payload.iter().any(|(key, value)| {
            key.is_empty()
                || key.len() > MAX_EVENT_KEY_BYTES
                || !key.chars().all(|character| {
                    character.is_ascii_alphanumeric() || "_.:-".contains(character)
                })
                || value.len() > MAX_EVENT_VALUE_BYTES
                || value.chars().any(char::is_control)
        })
    {
        return Err(event_error(
            ErrorCode::ResourceLimit,
            "Event payload contains invalid or unbounded fields.",
            format!("fields={}", payload.len()),
        ));
    }
    Ok(())
}

fn event_error(code: ErrorCode, message: &str, context: impl Into<String>) -> DockError {
    DockError::new(code, message, context)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subscriptions_receive_ordered_events_once_and_can_be_removed() {
        let mut hub = EventHub::new(8).unwrap();
        let subscription = hub.subscribe().unwrap();
        let event = hub
            .publish(
                1,
                FrameworkEventKind::LayoutCommitted,
                "command-1".into(),
                BTreeMap::from([("operation".into(), "split_panel".into())]),
            )
            .unwrap()
            .unwrap();
        assert_eq!(event.sequence, 1);
        assert!(
            hub.publish(
                1,
                FrameworkEventKind::LayoutCommitted,
                "command-1".into(),
                BTreeMap::from([("operation".into(), "split_panel".into())]),
            )
            .unwrap()
            .is_none()
        );
        let batch = hub.read(subscription, 16).unwrap();
        assert_eq!(batch.events, vec![event]);
        assert_eq!(hub.read(subscription, 16).unwrap(), EventBatch::default());
        assert!(hub.unsubscribe(subscription));
        assert!(hub.read(subscription, 1).is_err());
    }

    #[test]
    fn overflow_reports_lost_events_and_preserves_stable_order() {
        let mut hub = EventHub::new(2).unwrap();
        let subscription = hub.subscribe().unwrap();
        for revision in 1..=4 {
            hub.publish(
                revision,
                FrameworkEventKind::LayoutCommitted,
                format!("command-{revision}").into(),
                BTreeMap::new(),
            )
            .unwrap();
        }
        let batch = hub.read(subscription, 8).unwrap();
        assert_eq!(batch.lost_events, 2);
        assert_eq!(
            batch
                .events
                .iter()
                .map(|event| event.sequence)
                .collect::<Vec<_>>(),
            vec![3, 4]
        );
    }

    #[test]
    fn same_kind_facts_keep_distinct_payloads_but_suppress_exact_retries() {
        let mut hub = EventHub::new(8).unwrap();
        let subscription = hub.subscribe().unwrap();
        let command = CommandId::from("move-two-panels");
        let kind = FrameworkEventKind::PanelMoved;
        let first = BTreeMap::from([("panel".into(), "alpha".into())]);
        let second = BTreeMap::from([("panel".into(), "beta".into())]);
        assert!(
            hub.publish(1, kind, command.clone(), first.clone())
                .unwrap()
                .is_some()
        );
        assert!(
            hub.publish(1, kind, command.clone(), second.clone())
                .unwrap()
                .is_some()
        );
        assert!(hub.publish(1, kind, command, first).unwrap().is_none());
        let batch = hub.read(subscription, 8).unwrap();
        assert_eq!(batch.events.len(), 2);
        assert_eq!(batch.events[0].payload.get("panel").unwrap(), "alpha");
        assert_eq!(batch.events[1].payload.get("panel").unwrap(), "beta");
    }

    #[test]
    fn invalid_event_payloads_and_bounds_fail_closed() {
        assert!(EventHub::new(MAX_EVENT_QUEUE + 1).is_err());
        let mut hub = EventHub::new(1).unwrap();
        let subscription = hub.subscribe().unwrap();
        assert!(hub.read(subscription, 0).is_err());
        assert!(
            hub.publish(
                1,
                FrameworkEventKind::LayoutCommitted,
                "command".into(),
                BTreeMap::from([("bad key".into(), "value".into())]),
            )
            .is_err()
        );
        assert!(
            hub.publish(
                1,
                FrameworkEventKind::LayoutCommitted,
                "command".into(),
                BTreeMap::from([("field".into(), "bad\u{7}".into())]),
            )
            .is_err()
        );
    }

    #[test]
    fn initial_revision_focus_observation_is_valid_but_commits_require_a_positive_revision() {
        let mut hub = EventHub::new(4).unwrap();
        let subscription = hub.subscribe().unwrap();
        let focus = hub
            .publish(
                0,
                FrameworkEventKind::FocusChanged,
                "initial-focus".into(),
                BTreeMap::from([("panel".into(), "editor".into())]),
            )
            .unwrap()
            .unwrap();
        assert_eq!(focus.revision, 0);
        assert!(
            hub.publish(
                0,
                FrameworkEventKind::LayoutCommitted,
                "invalid-zero-commit".into(),
                BTreeMap::new(),
            )
            .is_err()
        );
        assert_eq!(hub.read(subscription, 4).unwrap().events, vec![focus]);
    }
}
