//! Optional native egui software host and bounded bitmap chrome. Canonical state remains host-owned.
pub mod chrome;
pub mod modal;
pub mod raster;
pub mod scene;
pub mod skin;
pub mod software;
pub mod tabs;
pub mod topology;
pub mod transient;
pub mod transition;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Capabilities {
    pub software: bool,
    pub raster_skin: bool,
    pub shaders: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RendererCapabilityAvailability {
    Available,
    Unavailable,
    NotImplemented,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct RendererCapabilityObservation {
    pub capability_id: String,
    pub availability: RendererCapabilityAvailability,
    pub details: String,
}

/// Closure-free renderer facts serialized with the core URNE report schema.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct RendererCapabilityReport {
    pub renderer_id: String,
    pub observations: Vec<RendererCapabilityObservation>,
}

impl Capabilities {
    pub const SOFTWARE: Self = Self {
        software: true,
        raster_skin: true,
        shaders: false,
    };
    pub const GLOW: Self = Self {
        software: false,
        raster_skin: true,
        shaders: false,
    };
    /// Describe implemented rendering paths without probing or invoking UI callbacks.
    pub fn capability_report(self) -> RendererCapabilityReport {
        use RendererCapabilityAvailability::{Available, NotImplemented, Unavailable};
        RendererCapabilityReport {
            renderer_id: "doofusdock-render-egui".into(),
            observations: vec![
                RendererCapabilityObservation {
                    capability_id: "software_canvas".into(),
                    availability: if self.software {
                        Available
                    } else {
                        Unavailable
                    },
                    details: if self.software {
                        "The software canvas renderer is available.".into()
                    } else {
                        "This renderer configuration does not provide the software canvas.".into()
                    },
                },
                RendererCapabilityObservation {
                    capability_id: "raster_skin_chrome".into(),
                    availability: if self.raster_skin {
                        Available
                    } else {
                        Unavailable
                    },
                    details: if self.raster_skin {
                        "Raster nine-slice chrome is supported when a valid skin is supplied."
                            .into()
                    } else {
                        "This renderer configuration does not support raster skin chrome.".into()
                    },
                },
                RendererCapabilityObservation {
                    capability_id: "vector_chrome".into(),
                    availability: Available,
                    details: "Vector chrome is available as the baseline and fallback path.".into(),
                },
                RendererCapabilityObservation {
                    capability_id: "shader_chrome".into(),
                    availability: NotImplemented,
                    details: "The current chrome selector never emits shader chrome.".into(),
                },
                RendererCapabilityObservation {
                    capability_id: "application_shader_effects".into(),
                    availability: NotImplemented,
                    details: "No application shader effect pipeline is implemented.".into(),
                },
            ],
        }
    }
    /// Neither adapter currently implements application shader chrome.
    pub fn chrome(self, requested: ChromeKind, skin_available: bool) -> ChromeKind {
        match requested {
            ChromeKind::Vector => ChromeKind::Vector,
            _ if self.raster_skin && skin_available => ChromeKind::NineSlice,
            _ => ChromeKind::Vector,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChromeKind {
    Vector,
    NineSlice,
    Shader,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unsupported_shaders_fall_back_to_skin_or_vector() {
        for capabilities in [Capabilities::SOFTWARE, Capabilities::GLOW] {
            assert_eq!(
                capabilities.chrome(ChromeKind::Shader, true),
                ChromeKind::NineSlice
            );
            assert_eq!(
                capabilities.chrome(ChromeKind::Shader, false),
                ChromeKind::Vector
            );
        }
    }
    #[test]
    fn capability_report_exposes_actual_fallback_and_unimplemented_paths() {
        let report = Capabilities::SOFTWARE.capability_report();
        let shader = report
            .observations
            .iter()
            .find(|entry| entry.capability_id == "shader_chrome")
            .unwrap();
        assert_eq!(
            shader.availability,
            RendererCapabilityAvailability::NotImplemented
        );
        let effects = report
            .observations
            .iter()
            .find(|entry| entry.capability_id == "application_shader_effects")
            .unwrap();
        assert_eq!(
            effects.availability,
            RendererCapabilityAvailability::NotImplemented
        );
        assert!(
            effects
                .details
                .contains("No application shader effect pipeline")
        );
        let vector = report
            .observations
            .iter()
            .find(|entry| entry.capability_id == "vector_chrome")
            .unwrap();
        assert_eq!(
            vector.availability,
            RendererCapabilityAvailability::Available
        );
        let serialized = serde_json::to_value(&report).unwrap();
        assert_eq!(serialized["renderer_id"], "doofusdock-render-egui");
        assert_eq!(
            serialized["observations"][0]["capability_id"],
            "software_canvas"
        );
    }
    #[test]
    fn broken_skin_is_visible_vector_fallback() {
        let ctx = egui::Context::default();
        let error = skin::configure(
            &ctx,
            false,
            Some(std::path::Path::new("missing-skin-package")),
        )
        .unwrap();
        assert!(error.contains("using vector chrome"));
    }
}
