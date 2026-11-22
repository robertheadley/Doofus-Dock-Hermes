use std::collections::{HashSet, VecDeque};

const MAX_RETAINED_EVENT_KEYS: usize = 8_192;

/// Bounded exact deduplication for canonical Gateway event sequence identities.
///
/// Legacy events have neither a replay epoch nor a sequence and intentionally
/// pass through unchanged. Canonical sequence numbers are scoped by connection,
/// session, profile, and replay epoch so equal numbers from different owners do
/// not suppress each other.
#[derive(Default)]
pub(crate) struct GatewayEventDeduplicator {
    retained: HashSet<EventKey>,
    order: VecDeque<EventKey>,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
struct EventKey {
    connection_id: Option<String>,
    session_id: String,
    profile: Option<String>,
    replay_epoch: String,
    sequence: u64,
}

impl GatewayEventDeduplicator {
    /// Returns `true` for an unseen canonical frame or an unsequenced legacy frame.
    pub(crate) fn should_deliver(
        &mut self,
        event: &serde_json::Value,
        connection_id: Option<&str>,
    ) -> bool {
        let (Some(session_id), Some(replay_epoch), Some(sequence)) = (
            event.get("session_id").and_then(serde_json::Value::as_str),
            event
                .get("replay_epoch")
                .and_then(serde_json::Value::as_str),
            event.get("seq").and_then(serde_json::Value::as_u64),
        ) else {
            return true;
        };
        if session_id.is_empty() || replay_epoch.is_empty() {
            return true;
        }

        let key = EventKey {
            connection_id: connection_id.map(str::to_owned),
            session_id: session_id.to_owned(),
            profile: event
                .get("profile")
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned),
            replay_epoch: replay_epoch.to_owned(),
            sequence,
        };
        if !self.retained.insert(key.clone()) {
            return false;
        }
        self.order.push_back(key);
        while self.order.len() > MAX_RETAINED_EVENT_KEYS {
            if let Some(expired) = self.order.pop_front() {
                self.retained.remove(&expired);
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::GatewayEventDeduplicator;
    use serde_json::json;

    fn event(profile: &str, epoch: &str, sequence: u64) -> serde_json::Value {
        json!({
            "type": "message.delta",
            "session_id": "runtime-1",
            "profile": profile,
            "replay_epoch": epoch,
            "seq": sequence,
            "payload": {"text": "hello"}
        })
    }

    #[test]
    fn suppresses_duplicate_sequence_only_for_same_owner_epoch_and_connection() {
        let mut deduplicator = GatewayEventDeduplicator::default();
        let frame = event("work", "epoch-a", 17);

        assert!(deduplicator.should_deliver(&frame, Some("gateway-a")));
        assert!(!deduplicator.should_deliver(&frame, Some("gateway-a")));
        assert!(deduplicator.should_deliver(&frame, Some("gateway-b")));
        assert!(deduplicator.should_deliver(&event("personal", "epoch-a", 17), Some("gateway-a")));
        assert!(deduplicator.should_deliver(&event("work", "epoch-b", 17), Some("gateway-a")));
        assert!(deduplicator.should_deliver(&event("work", "epoch-a", 18), Some("gateway-a")));
    }

    #[test]
    fn unsequenced_legacy_events_are_never_suppressed() {
        let mut deduplicator = GatewayEventDeduplicator::default();
        let legacy =
            json!({"type":"message.delta", "session_id":"runtime-1", "payload":{"text":"x"}});

        assert!(deduplicator.should_deliver(&legacy, Some("gateway-a")));
        assert!(deduplicator.should_deliver(&legacy, Some("gateway-a")));
    }

    #[test]
    fn retention_is_bounded_and_old_keys_can_be_relearned() {
        let mut deduplicator = GatewayEventDeduplicator::default();
        let first = event("work", "epoch-a", 0);
        assert!(deduplicator.should_deliver(&first, Some("gateway-a")));
        for sequence in 1..=8_192 {
            assert!(
                deduplicator.should_deliver(&event("work", "epoch-a", sequence), Some("gateway-a"))
            );
        }
        assert!(deduplicator.should_deliver(&first, Some("gateway-a")));
    }
}
