//! Strict, connection-local contract for Hermes `runtime.describe`.
//!
//! Unknown fields are intentionally ignored so newer runtimes can add metadata without
//! breaking older clients. Fields this client relies on are required and validated.

use serde::Deserialize;
use serde_json::Value;
use std::collections::HashSet;

const MAX_IDENTITY_BYTES: usize = 128;
const MAX_CONTRACT_VALUE_BYTES: usize = 128;
const MAX_CONTRACT_ENTRIES: usize = 256;

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub(crate) struct RuntimeDescription {
    pub(crate) instance_id: String,
    pub(crate) profile_id: String,
    pub(crate) authority_epoch: u64,
    pub(crate) capabilities: Vec<String>,
    pub(crate) session_create: SessionCreateContract,
    #[serde(default)]
    pub(crate) skin: Option<Value>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub(crate) struct SessionCreateContract {
    pub(crate) sources: Vec<String>,
    pub(crate) parameters: Vec<String>,
}

impl RuntimeDescription {
    /// Parse and validate the required negotiated runtime contract.
    pub(crate) fn parse(value: Value) -> Result<Self, String> {
        let description: Self = serde_json::from_value(value).map_err(|error| {
            format!("Hermes returned an invalid runtime.describe result: {error}")
        })?;

        validate_identifier("instance_id", &description.instance_id, MAX_IDENTITY_BYTES)?;
        validate_identifier("profile_id", &description.profile_id, MAX_IDENTITY_BYTES)?;
        if description.authority_epoch == 0 {
            return Err("Hermes runtime.describe authority_epoch must be greater than zero".into());
        }
        validate_string_set("capabilities", &description.capabilities)?;
        validate_string_set(
            "session_create.sources",
            &description.session_create.sources,
        )?;
        validate_string_set(
            "session_create.parameters",
            &description.session_create.parameters,
        )?;
        if let Some(skin) = &description.skin
            && !skin.is_null()
            && !skin.is_object()
        {
            return Err("Hermes runtime.describe skin must be an object or null".into());
        }
        Ok(description)
    }

    pub(crate) fn supports(&self, capability: &str) -> bool {
        self.capabilities.iter().any(|value| value == capability)
    }

    pub(crate) fn supports_session_source(&self, source: &str) -> bool {
        self.session_create
            .sources
            .iter()
            .any(|value| value == source)
    }

    pub(crate) fn accepts_session_create_parameter(&self, parameter: &str) -> bool {
        self.session_create
            .parameters
            .iter()
            .any(|value| value == parameter)
    }

    /// Build the pinned Hermes Desktop session.create payload for this negotiated owner.
    /// The profile is bound by the Gateway attach grant; only advertised fields are sent.
    /// Hermes maps the `gui` policy label to its stored `desktop` surface.
    pub(crate) fn desktop_session_create_params(
        &self,
        request_id: &str,
        model: Option<(&str, &str)>,
        reasoning: Option<&str>,
    ) -> Result<Value, String> {
        if !self.supports_session_source("gui") {
            return Err(format!(
                "This Hermes Gateway does not advertise the desktop launch policy. Supported launch policies: {}",
                self.session_create.sources.join(", ")
            ));
        }
        if !self.accepts_session_create_parameter("request_id")
            || !self.accepts_session_create_parameter("source")
        {
            return Err(
                "This Hermes Gateway does not advertise the required desktop session creation fields."
                    .into(),
            );
        }
        validate_identifier("session.create request_id", request_id, MAX_IDENTITY_BYTES)?;

        let mut params = serde_json::json!({
            "request_id": request_id,
            "source": "gui",
        });
        if let Some((model, provider)) = model {
            if !self.accepts_session_create_parameter("model")
                || !self.accepts_session_create_parameter("provider")
            {
                return Err(
                    "This Hermes Gateway does not support choosing a model for a new desktop session."
                        .into(),
                );
            }
            validate_identifier("session.create model", model, MAX_CONTRACT_VALUE_BYTES)?;
            validate_identifier(
                "session.create provider",
                provider,
                MAX_CONTRACT_VALUE_BYTES,
            )?;
            params["model"] = Value::String(model.into());
            params["provider"] = Value::String(provider.into());
        }
        if let Some(reasoning) = reasoning {
            if !self.accepts_session_create_parameter("reasoning") {
                return Err(
                    "This Hermes Gateway does not support setting reasoning for a new desktop session."
                        .into(),
                );
            }
            validate_identifier(
                "session.create reasoning",
                reasoning,
                MAX_CONTRACT_VALUE_BYTES,
            )?;
            params["reasoning"] = Value::String(reasoning.into());
        }
        Ok(params)
    }
}

fn validate_string_set(label: &str, values: &[String]) -> Result<(), String> {
    if values.len() > MAX_CONTRACT_ENTRIES {
        return Err(format!(
            "Hermes runtime.describe {label} contains too many entries (maximum {MAX_CONTRACT_ENTRIES})"
        ));
    }
    let mut seen = HashSet::with_capacity(values.len());
    for value in values {
        validate_identifier(label, value, MAX_CONTRACT_VALUE_BYTES)?;
        if !seen.insert(value) {
            return Err(format!(
                "Hermes runtime.describe {label} contains a duplicate entry"
            ));
        }
    }
    Ok(())
}

fn validate_identifier(label: &str, value: &str, max_bytes: usize) -> Result<(), String> {
    if value.trim().is_empty() {
        return Err(format!("Hermes runtime.describe {label} must not be empty"));
    }
    if value.len() > max_bytes {
        return Err(format!(
            "Hermes runtime.describe {label} exceeds the {max_bytes}-byte limit"
        ));
    }
    if value.chars().any(char::is_control) {
        return Err(format!(
            "Hermes runtime.describe {label} must not contain control characters"
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn valid_description() -> Value {
        json!({
            "instance_id": "hermes-instance-a",
            "profile_id": "coding",
            "authority_epoch": 4,
            "capabilities": ["clarify-batch-v1", "session-authority-v1"],
            "session_create": {
                "sources": ["gui"],
                "parameters": ["client", "source", "request_id", "model", "provider", "reasoning"]
            },
            "skin": null,
            "future_runtime_field": {"ignored": true}
        })
    }

    #[test]
    fn parses_required_contract_and_ignores_future_fields() {
        let runtime = RuntimeDescription::parse(valid_description()).unwrap();
        assert_eq!(runtime.instance_id, "hermes-instance-a");
        assert_eq!(runtime.profile_id, "coding");
        assert_eq!(runtime.authority_epoch, 4);
        assert!(runtime.supports("clarify-batch-v1"));
        assert!(!runtime.supports("missing-capability"));
        assert!(runtime.supports_session_source("gui"));
        assert!(!runtime.supports_session_source("tui"));
        assert!(runtime.accepts_session_create_parameter("client"));
        assert!(!runtime.accepts_session_create_parameter("clarify_batch"));
    }

    #[test]
    fn desktop_session_create_uses_only_negotiated_fields_and_gui_policy() {
        let runtime = RuntimeDescription::parse(valid_description()).unwrap();
        let params = runtime
            .desktop_session_create_params(
                "request-1",
                Some(("openai/gpt-5", "openai")),
                Some("high"),
            )
            .unwrap();
        assert_eq!(params["source"], "gui");
        assert_eq!(params["request_id"], "request-1");
        assert_eq!(params["model"], "openai/gpt-5");
        assert_eq!(params["provider"], "openai");
        assert_eq!(params["reasoning"], "high");
        assert!(params.get("profile").is_none());
        assert!(params.get("cols").is_none());
        assert!(params.get("reasoning_effort").is_none());
    }

    #[test]
    fn desktop_session_create_fails_closed_when_policy_or_selected_options_are_not_advertised() {
        let mut value = valid_description();
        value["session_create"]["sources"] = json!(["tui"]);
        let runtime = RuntimeDescription::parse(value).unwrap();
        assert!(
            runtime
                .desktop_session_create_params("request-1", None, None)
                .is_err()
        );

        let mut value = valid_description();
        value["session_create"]["parameters"] = json!(["source", "request_id"]);
        let runtime = RuntimeDescription::parse(value).unwrap();
        assert!(
            runtime
                .desktop_session_create_params("request-1", Some(("model-a", "provider-a")), None)
                .is_err()
        );
        assert!(
            runtime
                .desktop_session_create_params("request-1", None, Some("high"))
                .is_err()
        );
    }

    #[test]
    fn rejects_missing_required_runtime_contract_fields() {
        for key in [
            "instance_id",
            "profile_id",
            "authority_epoch",
            "capabilities",
            "session_create",
        ] {
            let mut value = valid_description();
            value.as_object_mut().unwrap().remove(key);
            assert!(
                RuntimeDescription::parse(value).is_err(),
                "accepted missing {key}"
            );
        }
        let mut value = valid_description();
        value["session_create"]
            .as_object_mut()
            .unwrap()
            .remove("sources");
        assert!(RuntimeDescription::parse(value).is_err());
        let mut value = valid_description();
        value["session_create"]
            .as_object_mut()
            .unwrap()
            .remove("parameters");
        assert!(RuntimeDescription::parse(value).is_err());
    }

    #[test]
    fn rejects_invalid_identity_epoch_and_skin_shapes() {
        for (key, invalid) in [
            ("instance_id", json!("  ")),
            ("profile_id", json!("")),
            ("instance_id", json!("id\nwith-control")),
            ("profile_id", json!("p".repeat(MAX_IDENTITY_BYTES + 1))),
            ("authority_epoch", json!(0)),
            ("skin", json!(false)),
        ] {
            let mut value = valid_description();
            value[key] = invalid;
            assert!(
                RuntimeDescription::parse(value).is_err(),
                "accepted invalid {key}"
            );
        }
    }

    #[test]
    fn rejects_malformed_or_oversized_negotiated_lists() {
        for invalid in [
            json!("not-an-array"),
            json!(["", "desktop"]),
            json!(["bad\nsource"]),
            json!(["x".repeat(MAX_CONTRACT_VALUE_BYTES + 1)]),
            json!(vec!["capability"; MAX_CONTRACT_ENTRIES + 1]),
        ] {
            let mut value = valid_description();
            value["capabilities"] = invalid;
            assert!(RuntimeDescription::parse(value).is_err());
        }
        let mut value = valid_description();
        value["session_create"]["sources"] = json!("desktop");
        assert!(RuntimeDescription::parse(value).is_err());
        let mut value = valid_description();
        value["session_create"]["parameters"] = json!(["client", false]);
        assert!(RuntimeDescription::parse(value).is_err());
    }

    #[test]
    fn rejects_duplicate_capabilities_sources_and_parameters() {
        for (path, duplicate) in [
            (
                "capabilities",
                json!(["tool-progress-v1", "tool-progress-v1"]),
            ),
            ("sources", json!(["desktop", "desktop"])),
            ("parameters", json!(["source", "source"])),
        ] {
            let mut value = valid_description();
            match path {
                "capabilities" => value[path] = duplicate,
                "sources" => value["session_create"][path] = duplicate,
                "parameters" => value["session_create"][path] = duplicate,
                _ => unreachable!(),
            }
            assert!(
                RuntimeDescription::parse(value).is_err(),
                "accepted duplicate {path}"
            );
        }
    }
}
