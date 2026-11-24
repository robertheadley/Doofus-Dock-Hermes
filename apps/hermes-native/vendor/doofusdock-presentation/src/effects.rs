//! Deterministic, bounded presentation effects and scene graph.
//! All effects are pure functions of (time, seed, viewport) and never mutate canonical layouts.

use crate::{Box2, Visual};
use doofusdock_core::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Deterministic, zero-allocation PCG32 pseudo-random number generator.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DeterministicRng {
    state: u64,
    inc: u64,
}

impl DeterministicRng {
    pub const fn new(seed: u64) -> Self {
        Self {
            state: seed
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407),
            inc: 1442695040888963407 | 1,
        }
    }

    pub fn next_u32(&mut self) -> u32 {
        let oldstate = self.state;
        self.state = oldstate
            .wrapping_mul(6364136223846793005)
            .wrapping_add(self.inc);
        let xorshifted = (((oldstate >> 18) ^ oldstate) >> 27) as u32;
        let rot = (oldstate >> 59) as u32;
        (xorshifted >> rot) | (xorshifted << ((32u32.wrapping_sub(rot)) & 31))
    }

    pub fn next_f64(&mut self) -> f64 {
        (self.next_u32() as f64) / (u32::MAX as f64 + 1.0)
    }

    pub fn range_f64(&mut self, min: f64, max: f64) -> f64 {
        min + (max - min) * self.next_f64()
    }
}

/// Visual category of a decorative element.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DecorativeKind {
    WindowCard,
    GhostTrail,
    SplineParticle,
    PanelFragment,
}

/// Lightweight decorative visual item (virtualized wall, Solitaire trail, swarm particle).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DecorativeItem {
    pub id: String,
    pub title: String,
    pub bounds: Box2,
    pub clip: Box2,
    pub opacity: f64,
    pub z: i32,
    pub is_extracted: bool,
    pub kind: DecorativeKind,
}

/// Complete presentation scene graph rendered by host adapters.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct SceneGraph {
    /// Active interactive canonical panel visuals.
    pub panels: BTreeMap<PanelId, Visual>,
    /// Lightweight decorative visual items (virtualized wall, ghost trails, swarm).
    pub decorative: Vec<DecorativeItem>,
    /// Active viewport.
    pub viewport: Box2,
    /// Bounded LOD panel budget (e.g. 10, 50, 100, 250, 500).
    pub lod_panel_budget: usize,
}

impl SceneGraph {
    pub fn new(viewport: Box2, lod_panel_budget: usize) -> Self {
        Self {
            panels: BTreeMap::new(),
            decorative: Vec::with_capacity(lod_panel_budget.min(512)),
            viewport,
            lod_panel_budget,
        }
    }

    /// Hit test: interactive panels take priority over decorative items.
    pub fn hit_test(&self, x: f64, y: f64) -> Option<PanelId> {
        self.panels
            .iter()
            .filter(|(_, v)| v.opacity >= 0.5 && v.bounds.contains(x, y) && v.clip.contains(x, y))
            .max_by_key(|(_, v)| v.z)
            .map(|(id, _)| id.clone())
    }

    /// Check if an extracted decorative item was clicked (promotion candidate).
    pub fn hit_test_extracted(&self, x: f64, y: f64) -> Option<&DecorativeItem> {
        self.decorative
            .iter()
            .filter(|item| {
                item.is_extracted
                    && item.opacity >= 0.5
                    && item.bounds.contains(x, y)
                    && item.clip.contains(x, y)
            })
            .max_by_key(|item| item.z)
    }
}

/// Context passed to each pure effect evaluation function.
#[derive(Clone, Copy, Debug)]
pub struct EffectContext<'a> {
    pub time_seconds: f64,
    pub progress: f64,
    pub viewport: Box2,
    pub seed: u64,
    pub lod_budget: usize,
    pub reduced_motion: bool,
    pub pulse: f64,
    pub canonical_doc: &'a LayoutDocument,
}

/// Describe the built-in deterministic scene-effect family for capability
/// discovery. This reports CPU scene-graph effects only; it makes no claim
/// about GPU shaders, surface presentation, or frame pacing.
pub fn built_in_effect_capability_report() -> EffectCapabilityReport {
    const EFFECT_IDS: [&str; 15] = [
        "awakening",
        "wall_of_windows",
        "magnetic_assembly",
        "great_morph",
        "solitaire_victory",
        "kinetic_storm",
        "infinite_regeneration",
        "impossible_recovery",
        "tile_assembly",
        "fly_in",
        "unfold",
        "fly_out",
        "collapse",
        "card_stack",
        "fluid_ripple",
    ];
    let schema = serde_json::json!({
        "type": "object",
        "required": [
            "time_seconds", "progress", "viewport", "seed", "lod_budget",
            "reduced_motion", "pulse", "canonical_doc"
        ],
        "additionalProperties": false,
        "properties": {
            "time_seconds": {"type": "number", "finite": true},
            "progress": {"type": "number", "minimum": 0.0, "maximum": 1.0},
            "viewport": {"type": "object", "required": ["x", "y", "w", "h"]},
            "seed": {"type": "integer", "minimum": 0},
            "lod_budget": {"type": "integer", "minimum": 0, "maximum": 500},
            "reduced_motion": {"type": "boolean"},
            "pulse": {"type": "number", "minimum": 0.0, "maximum": 1.0},
            "canonical_doc": {"type": "object", "description": "validated canonical LayoutDocument"}
        }
    });
    EffectCapabilityReport {
        provider_id: "doofusdock-presentation:scene_effects".into(),
        observations: EFFECT_IDS
            .into_iter()
            .map(|effect_id| EffectCapabilityObservation {
                effect_id: effect_id.into(),
                availability: RendererCapabilityAvailability::Available,
                schema: schema.clone(),
                provided_capabilities: vec![
                    "scene_graph".into(),
                    format!("scene_effect:{effect_id}"),
                ],
                required_capabilities: vec!["validated_layout".into(), "bounded_lod".into()],
                details:
                    "Pure deterministic CPU scene graph evaluation; canonical layout is read-only."
                        .into(),
            })
            .collect(),
    }
}

fn smooth(t: f64) -> f64 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

// Fractions describe the center and extent within the stage.
fn placed(v: Box2, cx: f64, cy: f64, w: f64, h: f64) -> Box2 {
    Box2 {
        x: v.x + (cx - w / 2.0) * v.w,
        y: v.y + (cy - h / 2.0) * v.h,
        w: w * v.w,
        h: h * v.h,
    }
}

fn card(
    id: String,
    title: String,
    bounds: Box2,
    clip: Box2,
    z: i32,
    kind: DecorativeKind,
) -> DecorativeItem {
    DecorativeItem {
        id,
        title,
        bounds,
        clip,
        z,
        kind,
        opacity: 1.0,
        is_extracted: false,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MorphPose {
    Ide,
    Chat,
    Focus,
    Quad,
    Media,
    Browser,
    Timeline,
    Dashboard,
}

impl MorphPose {
    /// Eight compositions advance one at a time over the 32-beat score.
    pub fn at(progress: f64) -> (Self, Self, f64) {
        let poses = [
            Self::Ide,
            Self::Chat,
            Self::Focus,
            Self::Quad,
            Self::Media,
            Self::Browser,
            Self::Timeline,
            Self::Dashboard,
        ];
        let score = progress * 32.0 / 4.0;
        let beat = score.floor() as usize;
        (
            poses[(beat + poses.len() - 1) % poses.len()],
            poses[beat % poses.len()],
            smooth(score.fract()),
        )
    }

    fn pane(self, v: Box2, slot: usize) -> (Box2, f64) {
        // Slot order: editor, explorer, console, inspector.
        let rect = match self {
            Self::Ide => [
                (0.52, 0.36, 0.64, 0.72),
                (0.10, 0.50, 0.20, 1.0),
                (0.52, 0.86, 0.64, 0.28),
                (0.92, 0.50, 0.16, 1.0),
            ][slot],
            Self::Chat => [
                (0.62, 0.39, 0.76, 0.78),
                (0.12, 0.50, 0.24, 1.0),
                (0.62, 0.89, 0.76, 0.22),
                (1.0, 0.5, 0.0, 0.0),
            ][slot],
            Self::Focus if slot == 0 => (0.5, 0.5, 1.0, 1.0),
            Self::Focus => (0.5, 0.5, 0.0, 0.0),
            Self::Quad => (
                0.25 + 0.5 * (slot % 2) as f64,
                0.25 + 0.5 * (slot / 2) as f64,
                0.5,
                0.5,
            ),
            Self::Media => [
                (0.50, 0.47, 0.64, 0.76),
                (0.12, 0.50, 0.24, 0.92),
                (0.50, 0.91, 0.64, 0.18),
                (0.91, 0.48, 0.18, 0.92),
            ][slot],
            Self::Browser => [
                (0.56, 0.52, 0.88, 0.86),
                (0.08, 0.52, 0.14, 0.86),
                (0.56, 0.96, 0.88, 0.08),
                (1.0, 0.5, 0.0, 0.0),
            ][slot],
            Self::Timeline => [
                (0.50, 0.32, 0.78, 0.60),
                (0.10, 0.52, 0.20, 0.92),
                (0.50, 0.82, 0.78, 0.32),
                (0.90, 0.52, 0.20, 0.92),
            ][slot],
            Self::Dashboard => (
                0.25 + 0.5 * (slot % 2) as f64,
                0.25 + 0.5 * (slot / 2) as f64,
                0.5,
                0.5,
            ),
        };
        (
            placed(v, rect.0, rect.1, rect.2, rect.3),
            if rect.2 > 0.0 { 1.0 } else { 0.0 },
        )
    }
}

fn solitaire_bounds(v: Box2, index: usize, beats: f64) -> Box2 {
    // Fold horizontal travel at both rails. Each arch returns to the same floor.
    let lane = (index % 4) as f64;
    let age = beats - (index / 4) as f64 * 0.24;
    let travel = age * (0.17 + lane * 0.021) + lane * 0.51;
    let x = 1.0 - (travel.rem_euclid(2.0) - 1.0).abs();
    let phase = (age * (0.30 + lane * 0.035) + lane * 0.21).rem_euclid(1.0);
    let width = (v.w * 0.071).min(v.h * 0.107);
    let height = width * 1.39;
    Box2 {
        x: v.x + x * (v.w - width),
        y: v.y + (v.h - height) * (1.0 - 4.0 * phase * (1.0 - phase) * (0.55 + lane * 0.14)),
        w: width,
        h: height,
    }
}

/// Pure effect evaluation producing a SceneGraph.
pub fn evaluate_scene_effects(scene_id: &str, ctx: EffectContext) -> Result<SceneGraph, String> {
    if !ctx.viewport.valid() {
        return Err("Invalid viewport for effect evaluation".into());
    }
    if !ctx.time_seconds.is_finite() {
        return Err("Effect time must be finite".into());
    }
    if !ctx.progress.is_finite() || !(0.0..=1.0).contains(&ctx.progress) {
        return Err("Effect progress outside 0..1".into());
    }
    if !ctx.pulse.is_finite() || !(0.0..=1.0).contains(&ctx.pulse) {
        return Err("Effect pulse outside 0..1".into());
    }

    let mut graph = SceneGraph::new(ctx.viewport, ctx.lod_budget);
    let base_snapshot = crate::project(ctx.canonical_doc, ctx.viewport)?;

    match scene_id {
        "tile_assembly" => {
            // Panels begin as a compact grid of preview tiles and assemble into
            // their canonical projected bounds. The source document stays read-only.
            let count = base_snapshot.panels.len().max(1);
            let columns = (count as f64).sqrt().ceil() as usize;
            let rows = count.div_ceil(columns);
            let cell_w = ctx.viewport.w / columns as f64;
            let cell_h = ctx.viewport.h / rows as f64;
            let stagger = 0.08;
            let schedule_span = 1.0 + stagger * count.saturating_sub(1) as f64;
            for (index, (id, visual)) in base_snapshot.panels.into_iter().enumerate() {
                let tile_w = (visual.bounds.w * 0.58).min(cell_w * 0.82);
                let tile_h = (visual.bounds.h * 0.58).min(cell_h * 0.82);
                let col = index % columns;
                let row = index / columns;
                let start = Box2 {
                    x: ctx.viewport.x + col as f64 * cell_w + (cell_w - tile_w) * 0.5,
                    y: ctx.viewport.y + row as f64 * cell_h + (cell_h - tile_h) * 0.5,
                    w: tile_w,
                    h: tile_h,
                };
                let local = if ctx.reduced_motion {
                    1.0
                } else {
                    smooth((ctx.progress * schedule_span - index as f64 * stagger).clamp(0.0, 1.0))
                };
                graph.panels.insert(
                    id,
                    Visual {
                        bounds: start.mix(visual.bounds, local),
                        opacity: visual.opacity * (0.72 + 0.28 * local),
                        ..visual
                    },
                );
            }
        }
        "fly_in" => {
            // Panels enter from alternating viewport edges in stable ID order.
            // The stateless progress input lets hosts reverse or scrub the motion.
            let count = base_snapshot.panels.len().max(1);
            let stagger = 0.07;
            let schedule_span = 1.0 + stagger * count.saturating_sub(1) as f64;
            for (index, (id, visual)) in base_snapshot.panels.into_iter().enumerate() {
                let mut start = visual.bounds;
                match index % 4 {
                    0 => start.x = ctx.viewport.x - start.w - ctx.viewport.w,
                    1 => start.y = ctx.viewport.y - start.h - ctx.viewport.h,
                    2 => start.x = ctx.viewport.x + ctx.viewport.w,
                    _ => start.y = ctx.viewport.y + ctx.viewport.h,
                }
                let local = if ctx.reduced_motion {
                    1.0
                } else {
                    smooth((ctx.progress * schedule_span - index as f64 * stagger).clamp(0.0, 1.0))
                };
                graph.panels.insert(
                    id,
                    Visual {
                        bounds: start.mix(visual.bounds, local),
                        opacity: visual.opacity * local,
                        ..visual
                    },
                );
            }
        }
        "unfold" => {
            // Panels open from a narrow center seam into canonical bounds.
            // Sampling is stateless so hosts may reverse, scrub, or retarget.
            let count = base_snapshot.panels.len().max(1);
            let stagger = 0.06;
            let schedule_span = 1.0 + stagger * count.saturating_sub(1) as f64;
            for (index, (id, visual)) in base_snapshot.panels.into_iter().enumerate() {
                let center_x = visual.bounds.x + visual.bounds.w * 0.5;
                let start = Box2 {
                    x: center_x,
                    y: visual.bounds.y,
                    w: 0.0,
                    h: visual.bounds.h,
                };
                let local = if ctx.reduced_motion {
                    1.0
                } else {
                    smooth((ctx.progress * schedule_span - index as f64 * stagger).clamp(0.0, 1.0))
                };
                graph.panels.insert(
                    id,
                    Visual {
                        bounds: start.mix(visual.bounds, local),
                        opacity: visual.opacity * (0.55 + 0.45 * local),
                        ..visual
                    },
                );
            }
        }
        "fly_out" => {
            // Panels exit beyond alternating viewport edges. In reduced-motion
            // mode the host receives the canonical projection and can dismiss
            // the surface immediately without animating its content.
            let count = base_snapshot.panels.len().max(1);
            let stagger = 0.07;
            let schedule_span = 1.0 + stagger * count.saturating_sub(1) as f64;
            for (index, (id, visual)) in base_snapshot.panels.into_iter().enumerate() {
                let mut end = visual.bounds;
                match index % 4 {
                    0 => end.x = ctx.viewport.x - end.w - ctx.viewport.w,
                    1 => end.y = ctx.viewport.y - end.h - ctx.viewport.h,
                    2 => end.x = ctx.viewport.x + ctx.viewport.w,
                    _ => end.y = ctx.viewport.y + ctx.viewport.h,
                }
                let local = if ctx.reduced_motion {
                    0.0
                } else {
                    smooth((ctx.progress * schedule_span - index as f64 * stagger).clamp(0.0, 1.0))
                };
                graph.panels.insert(
                    id,
                    Visual {
                        bounds: visual.bounds.mix(end, local),
                        opacity: visual.opacity * (1.0 - local),
                        ..visual
                    },
                );
            }
        }
        "collapse" => {
            // Panels close toward a vertical center seam. Reduced motion
            // returns canonical bounds so the host may switch state directly.
            let count = base_snapshot.panels.len().max(1);
            let stagger = 0.06;
            let schedule_span = 1.0 + stagger * count.saturating_sub(1) as f64;
            for (index, (id, visual)) in base_snapshot.panels.into_iter().enumerate() {
                let center_x = visual.bounds.x + visual.bounds.w * 0.5;
                let end = Box2 {
                    x: center_x,
                    y: visual.bounds.y,
                    w: 0.0,
                    h: visual.bounds.h,
                };
                let local = if ctx.reduced_motion {
                    0.0
                } else {
                    smooth((ctx.progress * schedule_span - index as f64 * stagger).clamp(0.0, 1.0))
                };
                graph.panels.insert(
                    id,
                    Visual {
                        bounds: visual.bounds.mix(end, local),
                        opacity: visual.opacity * (1.0 - local),
                        ..visual
                    },
                );
            }
        }
        "card_stack" => {
            // Panels converge to a compact, ordered stack. Small bounded
            // offsets keep each card edge visible and stable across samples.
            let count = base_snapshot.panels.len().max(1);
            let stagger = 0.05;
            let schedule_span = 1.0 + stagger * count.saturating_sub(1) as f64;
            for (index, (id, visual)) in base_snapshot.panels.into_iter().enumerate() {
                let depth = index as f64;
                let offset = (depth * 6.0).min(ctx.viewport.w.min(ctx.viewport.h) * 0.12);
                let target = Box2 {
                    x: ctx.viewport.x + (ctx.viewport.w - visual.bounds.w * 0.62) * 0.5 + offset,
                    y: ctx.viewport.y + (ctx.viewport.h - visual.bounds.h * 0.62) * 0.5 + offset,
                    w: visual.bounds.w * 0.62,
                    h: visual.bounds.h * 0.62,
                };
                let local = if ctx.reduced_motion {
                    0.0
                } else {
                    smooth((ctx.progress * schedule_span - depth * stagger).clamp(0.0, 1.0))
                };
                graph.panels.insert(
                    id,
                    Visual {
                        bounds: visual.bounds.mix(target, local),
                        z: if local > 0.5 {
                            visual.z.saturating_add(index.min(i32::MAX as usize) as i32)
                        } else {
                            visual.z
                        },
                        ..visual
                    },
                );
            }
        }
        "fluid_ripple" => {
            // Panels flow from a compact center-stage cluster along staggered
            // ripple paths, then settle exactly onto canonical bounds.
            let count = base_snapshot.panels.len().max(1);
            let stagger = 0.045;
            let schedule_span = 1.0 + stagger * count.saturating_sub(1) as f64;
            let seed_phase = (ctx.seed % 1024) as f64 * 0.006_135_923_151_542_565;
            for (index, (id, visual)) in base_snapshot.panels.into_iter().enumerate() {
                let index_phase = index as f64 * 2.399_963_229_728_653;
                let start = Box2 {
                    x: ctx.viewport.x
                        + ctx.viewport.w * 0.5
                        + index_phase.cos() * ctx.viewport.w * 0.025
                        - visual.bounds.w * 0.35,
                    y: ctx.viewport.y
                        + ctx.viewport.h * 0.5
                        + index_phase.sin() * ctx.viewport.h * 0.025
                        - visual.bounds.h * 0.35,
                    w: visual.bounds.w * 0.7,
                    h: visual.bounds.h * 0.7,
                };
                let local = if ctx.reduced_motion {
                    1.0
                } else {
                    (ctx.progress * schedule_span - index as f64 * stagger).clamp(0.0, 1.0)
                };
                if ctx.reduced_motion || ctx.progress >= 1.0 || local >= 1.0 {
                    graph.panels.insert(id, visual);
                    continue;
                }
                let eased = smooth(local);
                let wave = (std::f64::consts::PI * local).sin()
                    * (std::f64::consts::TAU * local + index_phase + seed_phase).sin()
                    * 0.025;
                let flow = Box2 {
                    x: start.x + (visual.bounds.x - start.x) * eased + wave * ctx.viewport.w,
                    y: start.y + (visual.bounds.y - start.y) * eased - wave * ctx.viewport.h * 0.7,
                    w: start.w + (visual.bounds.w - start.w) * eased,
                    h: start.h + (visual.bounds.h - start.h) * eased,
                };
                graph.panels.insert(
                    id,
                    Visual {
                        bounds: flow,
                        opacity: visual.opacity * eased,
                        ..visual
                    },
                );
            }
        }
        "awakening" => {
            // Materialize: panels expand smoothly from center with opacity fade
            let t = if ctx.reduced_motion {
                1.0
            } else {
                1.0 - (1.0 - ctx.progress).powi(14)
            };
            let opacity = t.clamp(0.0, 1.0);
            let scale = if ctx.reduced_motion {
                1.0
            } else {
                // Keep window geometry monotonic and independent of beat transients.
                // Audio pulses belong in panel content and decorative effects; applying
                // them to window scale makes the first scene visibly vibrate.
                0.42 + 0.58 * t
            };
            for (id, visual) in base_snapshot.panels {
                let center_x = visual.bounds.x + visual.bounds.w * 0.5;
                let center_y = visual.bounds.y + visual.bounds.h * 0.5;
                let w = visual.bounds.w * scale;
                let h = visual.bounds.h * scale;
                graph.panels.insert(
                    id,
                    Visual {
                        bounds: Box2 {
                            x: center_x - w * 0.5,
                            y: center_y - h * 0.5,
                            w,
                            h,
                        },
                        clip: visual.clip,
                        opacity: visual.opacity * opacity,
                        z: visual.z,
                    },
                );
            }
        }
        "wall_of_windows" => {
            // Virtualized Wall: grid of decorative cards up to LOD budget.
            // One extracted window promotes to center stage.
            let count = ctx.lod_budget.clamp(10, 500);
            let cols = ((count as f64).sqrt().ceil() as usize).max(4);
            let rows = count.div_ceil(cols);
            let card_w = (ctx.viewport.w / cols as f64).max(60.0);
            let card_h = (ctx.viewport.h / rows as f64).max(40.0);
            let beats = ctx.progress * 32.0;
            let wave_offset = if ctx.reduced_motion {
                0.0
            } else {
                (beats * 2.4).sin() * card_h * 0.22
            };
            // Spotlight a new feed every four beats. Hold long enough to click.
            let turn = if ctx.reduced_motion {
                0.5
            } else {
                (beats / 4.0).fract()
            };
            let handoff = smooth(turn);
            let selection = (count / 2 + (beats / 4.0).floor() as usize * 37) % count;
            let next_selection = (selection + 37) % count;

            for i in 0..count {
                let col = i % cols;
                let row = i / cols;
                let base_x = ctx.viewport.x + col as f64 * card_w;
                let base_y =
                    ctx.viewport.y + row as f64 * card_h + wave_offset * ((col as f64 * 0.2).sin());

                if i == selection || i == next_selection {
                    // Keep the featured window's dimensions fixed. Handoff moves
                    // the old feed off stage as the next feed moves onto the same
                    // center point, avoiding the repeated shrink and pop to center.
                    let next = i == next_selection;
                    let destination = if next { next_selection } else { selection };
                    let col = destination % cols;
                    let row = destination / cols;
                    let from_x = ctx.viewport.x + col as f64 * card_w;
                    let from_y = ctx.viewport.y
                        + row as f64 * card_h
                        + wave_offset * ((col as f64 * 0.2).sin());
                    let target_w = (ctx.viewport.w * 0.64).min(ctx.viewport.h * 16.0 / 9.0);
                    let target_h = target_w * 9.0 / 16.0;
                    let target_x = ctx.viewport.x + (ctx.viewport.w - target_w) * 0.5;
                    let target_y = ctx.viewport.y + (ctx.viewport.h - target_h) * 0.5;
                    let travel = handoff;
                    let start = Box2 {
                        x: from_x,
                        y: from_y,
                        w: card_w,
                        h: card_h,
                    };
                    let center = Box2 {
                        x: target_x,
                        y: target_y,
                        w: target_w,
                        h: target_h,
                    };
                    let bounds = if next {
                        start.mix(center, travel)
                    } else {
                        center.mix(start, travel)
                    };
                    let opacity = if next { handoff } else { 1.0 - handoff };

                    graph.decorative.push(DecorativeItem {
                        id: format!("wall_item_{destination}"),
                        title: format!("Promoted Window #{destination:03}"),
                        bounds,
                        clip: ctx.viewport,
                        opacity,
                        z: i32::MAX,
                        is_extracted: true,
                        kind: DecorativeKind::WindowCard,
                    });
                } else {
                    graph.decorative.push(DecorativeItem {
                        id: format!("wall_item_{i}"),
                        title: format!("Window #{i:03}"),
                        bounds: Box2 {
                            x: base_x + 2.0,
                            y: base_y + 2.0,
                            w: (card_w - 4.0).max(10.0),
                            h: (card_h - 4.0).max(10.0),
                        },
                        clip: ctx.viewport,
                        opacity: 0.75 + 0.25 * ctx.pulse,
                        z: (row * cols + col) as i32,
                        is_extracted: false,
                        kind: DecorativeKind::WindowCard,
                    });
                }
            }
        }
        "magnetic_assembly" => {
            let beats = ctx.progress * 20.0;
            for (idx, (id, mut panel)) in base_snapshot.panels.into_iter().enumerate() {
                let polarity = if idx % 2 == 0 { 1.0 } else { -1.0 };
                let phase = (beats * 0.25 - idx as f64 * 0.075).rem_euclid(1.0);
                let angle = idx as f64 * 2.4 + polarity * beats * 0.83;
                let orbit = placed(
                    ctx.viewport,
                    0.5 + 0.33 * angle.cos(),
                    0.5 + 0.30 * angle.sin(),
                    0.27,
                    0.26,
                );
                // Opposite poles circle, snap together, then repel on the next bar.
                let pull = smooth((phase - 0.26) / 0.25) * (1.0 - smooth((phase - 0.70) / 0.30));
                let settle = if ctx.reduced_motion {
                    1.0
                } else {
                    pull.max(smooth(beats - 19.0))
                };
                panel.bounds = orbit.mix(panel.bounds, settle);
                panel.clip = ctx.viewport;
                graph.panels.insert(id, panel);
            }
        }
        "great_morph" => {
            let (from, to, mix) = MorphPose::at(ctx.progress);
            let slots: Vec<_> = ["editor", "explorer", "console", "inspector"]
                .iter()
                .enumerate()
                .filter_map(|(slot, id)| {
                    base_snapshot
                        .panels
                        .get(&PanelId::from(*id))
                        .map(|v| (v.bounds, slot))
                })
                .collect();
            for (idx, (id, mut panel)) in base_snapshot.panels.into_iter().enumerate() {
                let slot = slots
                    .iter()
                    .find(|(b, _)| *b == panel.bounds)
                    .map_or(idx % 4, |(_, s)| *s);
                let (a, alpha) = from.pane(ctx.viewport, slot);
                let (b, beta) = to.pane(ctx.viewport, slot);
                let t = if ctx.reduced_motion { 1.0 } else { mix };
                panel.bounds = a.mix(b, t);
                panel.opacity *= alpha + (beta - alpha) * t;
                panel.clip = ctx.viewport;
                graph.panels.insert(id, panel);
            }
        }
        "solitaire_victory" => {
            let count = ctx.lod_budget.clamp(1, 52);
            let beats = if ctx.reduced_motion {
                0.0
            } else {
                ctx.progress * 32.0
            };
            let spare = ctx.lod_budget.saturating_sub(count).min(128);
            for i in 0..count {
                let trail_count = if ctx.reduced_motion {
                    0
                } else {
                    (spare / count + usize::from(i < spare % count)).min(4)
                };
                let rank = [
                    "A", "2", "3", "4", "5", "6", "7", "8", "9", "10", "J", "Q", "K",
                ][i / 4];
                let suit = ["♠", "♥", "♣", "♦"][i % 4];
                let title = format!("{rank}{suit}");
                for step in (1..=trail_count).rev() {
                    let bounds = solitaire_bounds(ctx.viewport, i, beats - step as f64 * 0.075);
                    let mut trail = card(
                        format!("trail_{i}_{step}"),
                        title.clone(),
                        bounds,
                        ctx.viewport,
                        -(step as i32),
                        DecorativeKind::GhostTrail,
                    );
                    trail.opacity = 0.44 / step as f64;
                    graph.decorative.push(trail);
                }
                graph.decorative.push(card(
                    format!("deck_{i}"),
                    title,
                    solitaire_bounds(ctx.viewport, i, beats),
                    ctx.viewport,
                    i as i32,
                    DecorativeKind::WindowCard,
                ));
            }
        }
        "kinetic_storm" => {
            let base_panels: Vec<_> = base_snapshot.panels.into_iter().collect();
            let count = (base_panels.len() * 4).min(ctx.lod_budget);
            let v = ctx.viewport;
            let beats = if ctx.reduced_motion {
                0.0
            } else {
                ctx.progress * 32.0
            };
            let fission = if ctx.reduced_motion {
                0.0
            } else {
                smooth(ctx.progress / 0.18)
            };
            let fusion = if ctx.reduced_motion {
                0.0
            } else {
                smooth((ctx.progress - 0.72) / 0.28)
            };

            // Fade each live pane out as its four fragments scatter, then restore it
            // while those same fragments converge into their original quadrants.
            for (id, mut panel) in base_panels.iter().cloned() {
                panel.opacity *= (1.0 - fission + fusion).clamp(0.0, 1.0);
                graph.panels.insert(id, panel);
            }

            for i in 0..count {
                let panel_index = i / 4;
                let piece = i % 4;
                let parent = base_panels[panel_index].1.bounds;
                let target = Box2 {
                    x: parent.x + (piece % 2) as f64 * parent.w * 0.5,
                    y: parent.y + (piece / 2) as f64 * parent.h * 0.5,
                    w: parent.w * 0.5,
                    h: parent.h * 0.5,
                };
                // Opposing satellites share their size and radius, centering the ensemble.
                let pair = i / 2;
                let theta = pair as f64 * 2.39996
                    + beats * (0.52 + 0.08 * (pair % 3) as f64)
                    + (i % 2) as f64 * std::f64::consts::PI;
                let depth = (beats * 0.81 + pair as f64 * 1.7).sin() * 0.5 + 0.5;
                let w = (v.w * (0.15 + 0.12 * depth)).min(v.h * 0.55);
                let h = w * 9.0 / 16.0;
                let radius = 0.57 + 0.31 * depth + 0.10 * ctx.pulse;
                let orbit = Box2 {
                    x: v.x + (v.w - w) * (0.5 + 0.5 * radius * theta.cos()),
                    y: v.y + (v.h - h) * (0.5 + 0.5 * radius * theta.sin()),
                    w,
                    h,
                };
                let bounds = target.mix(orbit, fission).mix(target, fusion);
                graph.decorative.push(card(
                    format!("storm_{i}"),
                    format!("Signal {:02}", i + 1),
                    bounds,
                    v,
                    (depth * 100.0) as i32,
                    DecorativeKind::PanelFragment,
                ));
            }
        }
        "infinite_regeneration" => {
            // A parent divides along alternating axes. After four divisions, zoom into
            // one daughter cell until it fills the stage and starts the next generation.
            let beats = ctx.progress * 32.0;
            let generation = (beats / 8.0).floor() as usize;
            let phase = beats % 8.0;
            let level = (phase.floor() as usize).min(3);
            let count = (2usize << level).min(ctx.lod_budget);
            let split = if ctx.reduced_motion {
                1.0
            } else {
                smooth(phase.fract() / 0.58)
            };
            let zoom = if ctx.reduced_motion {
                0.0
            } else {
                smooth((phase - 4.0) / 4.0)
            };
            let v = ctx.viewport;
            let focus = (generation * 7 + 5) % 16;
            let tile = |i: usize, depth: usize| {
                let mut b = v;
                for d in 0..depth {
                    let bit = ((i >> (depth - 1 - d)) & 1) as f64;
                    if d % 2 == 0 {
                        b.w *= 0.5;
                        b.x += bit * b.w;
                    } else {
                        b.h *= 0.5;
                        b.y += bit * b.h;
                    }
                }
                b
            };
            let camera = v.mix(tile(focus, 4), zoom);
            for i in 0..count {
                let end = tile(i, level + 1);
                let start = tile(i / 2, level);
                let mut b = start.mix(end, if phase >= 4.0 { 1.0 } else { split });
                if camera.w <= 0.0 || camera.h <= 0.0 {
                    continue;
                }
                b = Box2 {
                    x: v.x + (b.x - camera.x) * v.w / camera.w,
                    y: v.y + (b.y - camera.y) * v.h / camera.h,
                    w: b.w * v.w / camera.w,
                    h: b.h * v.h / camera.h,
                };
                if b.x + b.w <= v.x || b.x >= v.x + v.w || b.y + b.h <= v.y || b.y >= v.y + v.h {
                    continue;
                }
                graph.decorative.push(card(
                    format!("cell_{generation}_{level}_{i}"),
                    format!("Culture {}.{:02}", generation + 1, i + 1),
                    b,
                    v,
                    i as i32,
                    DecorativeKind::WindowCard,
                ));
            }
        }
        "impossible_recovery" => {
            // Four quadrants of each live pane fly in from different bearings, latch
            // into their sockets, then reveal the actual editable pane underneath.
            let beats = ctx.progress * 20.0;
            let n = base_snapshot.panels.len().max(1);
            for (idx, (id, mut panel)) in base_snapshot.panels.into_iter().enumerate() {
                let landing = 4.0 + 11.5 * idx as f64 / n.saturating_sub(1).max(1) as f64;
                let local = beats - landing;
                if !ctx.reduced_motion && local < 4.0 && panel.opacity > 0.0 {
                    for piece in 0..4 {
                        let theta = idx as f64 * 2.4 + piece as f64 * 1.57 + beats * 0.66;
                        let orbit = placed(
                            ctx.viewport,
                            0.5 + 0.37 * theta.cos(),
                            0.5 + 0.36 * theta.sin(),
                            0.20,
                            0.19,
                        );
                        let dest = placed(
                            panel.bounds,
                            0.25 + 0.5 * (piece % 2) as f64,
                            0.25 + 0.5 * (piece / 2) as f64,
                            0.5,
                            0.5,
                        );
                        let lock = smooth((local - piece as f64 * 0.6) / 1.5);
                        graph.decorative.push(card(
                            format!("repair_{}_{piece}", id.0),
                            format!("{} · {}/4", id.0, piece + 1),
                            orbit.mix(dest, lock),
                            ctx.viewport,
                            idx as i32,
                            DecorativeKind::WindowCard,
                        ));
                    }
                    panel.opacity = 0.0;
                }
                graph.panels.insert(id, panel);
            }
        }
        _ => {
            // Default: pure baseline projection
            graph.panels = base_snapshot.panels;
        }
    }

    graph.decorative.sort_by_key(|item| item.z);
    Ok(graph)
}
