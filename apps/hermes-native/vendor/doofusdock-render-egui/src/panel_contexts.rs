//! Persistent egui contexts for independently rendered scene panels.
//!
//! Each panel gets a separate widget-memory namespace and produces its own
//! `FullOutput`. Hosts remain responsible for routing validated input, painting
//! the output, handling platform commands, and deciding when a panel is gone.

use std::collections::{BTreeMap, BTreeSet};

use doofusdock_core::PanelId;
use egui::{Context, FullOutput, RawInput, Ui};

/// Owns one persistent egui context per host-managed panel.
///
/// The store deliberately does not infer visibility or prune contexts from a
/// scene frame: temporary hiding or clipping must not erase `TextEdit`, scroll,
/// or other widget state. Call [`Self::remove_panel`] or [`Self::retain_panels`]
/// when the host decides that panels have been closed or removed.
#[derive(Default)]
pub struct SceneEguiPanelContexts {
    contexts: BTreeMap<PanelId, Context>,
}

impl SceneEguiPanelContexts {
    /// Get the stable context for a panel, creating it on first use.
    pub fn context(&mut self, panel: &PanelId) -> &Context {
        self.contexts.entry(panel.clone()).or_default()
    }

    /// Run one panel's widget tree and return its context-local output.
    ///
    /// The supplied `RawInput` should already contain host-validated,
    /// panel-local events and a panel-local `screen_rect`. The returned output
    /// belongs only to this panel's context; texture IDs and platform commands
    /// must be handled by the host without merging contexts implicitly.
    pub fn run_panel_ui(
        &mut self,
        panel: &PanelId,
        input: RawInput,
        mut ui: impl FnMut(&mut Ui),
    ) -> FullOutput {
        self.context(panel).run_ui(input, &mut ui)
    }

    /// Remove one panel's context and all widget memory owned by it.
    pub fn remove_panel(&mut self, panel: &PanelId) -> bool {
        self.contexts.remove(panel).is_some()
    }

    /// Keep only contexts for IDs the host considers live.
    pub fn retain_panels(&mut self, live_panels: &BTreeSet<PanelId>) {
        self.contexts.retain(|panel, _| live_panels.contains(panel));
    }

    pub fn len(&self) -> usize {
        self.contexts.len()
    }

    pub fn is_empty(&self) -> bool {
        self.contexts.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::{Event, Id, TextEdit};

    fn text_frame(
        contexts: &mut SceneEguiPanelContexts,
        panel: &PanelId,
        text: &mut String,
        events: Vec<Event>,
    ) -> FullOutput {
        let mut output = contexts.run_panel_ui(
            panel,
            RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::Vec2::new(320.0, 120.0),
                )),
                events,
                ..Default::default()
            },
            |ui| {
                ui.add(TextEdit::singleline(text).id(Id::unique("shared-editor-id")));
            },
        );
        output.textures_delta.clear();
        output
    }

    #[test]
    fn panel_contexts_keep_widget_memory_independent() {
        let first = PanelId::from("first");
        let second = PanelId::from("second");
        let mut contexts = SceneEguiPanelContexts::default();
        let mut first_text = String::new();
        let mut second_text = String::new();

        let _ = text_frame(&mut contexts, &first, &mut first_text, vec![]);
        let _ = text_frame(&mut contexts, &second, &mut second_text, vec![]);
        contexts
            .context(&first)
            .memory_mut(|memory| memory.request_focus(Id::unique("shared-editor-id")));
        contexts
            .context(&second)
            .memory_mut(|memory| memory.request_focus(Id::unique("shared-editor-id")));

        let first_output = text_frame(
            &mut contexts,
            &first,
            &mut first_text,
            vec![Event::Text("alpha".into())],
        );
        let second_output = text_frame(&mut contexts, &second, &mut second_text, vec![]);

        assert_eq!(first_text, "alpha");
        assert!(second_text.is_empty());
        assert!(!first_output.shapes.is_empty());
        assert!(!second_output.shapes.is_empty());
        assert_eq!(contexts.len(), 2);
    }

    #[test]
    fn panel_context_lifecycle_is_explicit_and_recreates_fresh_state() {
        let first = PanelId::from("first");
        let second = PanelId::from("second");
        let third = PanelId::from("third");
        let mut contexts = SceneEguiPanelContexts::default();
        let mut text = String::new();
        let _ = text_frame(&mut contexts, &first, &mut text, vec![]);
        let _ = text_frame(&mut contexts, &second, &mut text, vec![]);
        assert_eq!(contexts.len(), 2);
        let focus_id = Id::unique("lifecycle-focus");
        contexts
            .context(&first)
            .memory_mut(|memory| memory.request_focus(focus_id));
        assert!(
            contexts
                .context(&first)
                .memory(|memory| memory.has_focus(focus_id))
        );
        assert!(contexts.remove_panel(&first));
        assert!(!contexts.remove_panel(&first));

        assert!(
            !contexts
                .context(&first)
                .memory(|memory| memory.has_focus(focus_id))
        );
        assert_eq!(contexts.len(), 2);
        let live = BTreeSet::from([second.clone(), third.clone()]);
        contexts.retain_panels(&live);
        assert_eq!(contexts.len(), 1);
        assert!(contexts.contexts.contains_key(&second));
        assert!(!contexts.contexts.contains_key(&first));
        assert!(!contexts.is_empty());

        contexts.retain_panels(&BTreeSet::new());
        assert!(contexts.is_empty());
    }
}
