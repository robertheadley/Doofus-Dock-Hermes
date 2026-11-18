//! Typed bounded scenario and cue validation. Evaluation is a pure function of time.
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeSet,
    io::Read,
    path::{Component, Path, PathBuf},
};
pub const MAX_JSON_BYTES: u64 = 1024 * 1024;
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Scenario {
    pub schema_version: u32,
    pub title: String,
    pub bpm: f64,
    pub beats_per_bar: u32,
    pub music: Music,
    pub timeline_origin_seconds: f64,
    pub seed: u64,
    pub mode: String,
    #[serde(rename = "loop")]
    pub looping: bool,
    pub reduced_motion_policy: String,
    pub renderer: Renderer,
    pub scenes: Vec<Scene>,
    pub stress_panel_counts: Vec<usize>,
    pub notes: String,
    #[serde(default)]
    pub timing_status: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Music {
    pub master: String,
    pub stems_archive: String,
    pub status: String,
    #[serde(default)]
    pub midi_archive: Option<String>,
    #[serde(default)]
    pub tempo_map: Option<String>,
    #[serde(default)]
    pub midi_note_events: Option<String>,
    #[serde(default)]
    pub timing_note: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Renderer {
    pub preferred: String,
    pub fallback: Vec<String>,
    pub gpu_required: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Scene {
    pub id: String,
    pub bar_start: u32,
    pub bar_end: u32,
    pub profile: String,
    pub effects: Vec<Effect>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Effect {
    Materialize,
    Dematerialize,
    VirtualizedWall,
    Extract,
    MagneticDock,
    LayoutMorph,
    BallisticLaunch,
    GhostTrail,
    SplineSwarm,
    Fission,
    Fusion,
    Reflow,
    Converge,
    FinalMorph,
    Teleport,
    Cascade,
}
impl Scenario {
    pub fn parse(json: &str) -> Result<Self, String> {
        if json.len() > MAX_JSON_BYTES as usize {
            return Err("Scenario exceeds 1 MiB".into());
        }
        let s: Self = serde_json::from_str(json).map_err(|e| format!("Scenario JSON: {e}"))?;
        s.validate()?;
        Ok(s)
    }
    pub fn load(path: &Path) -> Result<Self, String> {
        Self::parse(&read_json(path)?)
    }
    pub fn duration(&self) -> f64 {
        self.timeline_origin_seconds
            + self.scenes.last().map_or(0, |s| s.bar_end) as f64 * self.bar_seconds()
    }
    pub fn bar_seconds(&self) -> f64 {
        60. / self.bpm * self.beats_per_bar as f64
    }
    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != 1
            || !self.bpm.is_finite()
            || !(30.0..=300.0).contains(&self.bpm)
            || !(1..=16).contains(&self.beats_per_bar)
        {
            return Err("Unsupported scenario version or tempo".into());
        }
        if !self.timeline_origin_seconds.is_finite()
            || !(0.0..=60.).contains(&self.timeline_origin_seconds)
            || self.scenes.is_empty()
            || self.scenes.len() > 64
        {
            return Err("Invalid timeline origin or scene count".into());
        }
        if self.title.len() > 256 || self.notes.len() > 4096 || self.music.status.len() > 4096 {
            return Err("Scenario text limit exceeded".into());
        }
        validate_relative(&self.music.master)?;
        validate_relative(&self.music.stems_archive)?;
        if let Some(archive) = &self.music.midi_archive {
            validate_relative(archive)?;
        }
        if let Some(map) = &self.music.tempo_map {
            validate_relative(map)?;
        }
        if let Some(events) = &self.music.midi_note_events {
            validate_relative(events)?;
        }
        if !["cinematic", "playground", "benchmark"].contains(&self.mode.as_str())
            || !["respect_system", "always", "never"].contains(&self.reduced_motion_policy.as_str())
        {
            return Err("Unknown mode or reduced-motion policy".into());
        }
        let backends = ["auto", "gpu_skin", "software_skin", "software_vector"];
        if self.renderer.gpu_required
            || !backends.contains(&self.renderer.preferred.as_str())
            || self.renderer.fallback.is_empty()
            || self.renderer.fallback.len() > 4
            || self
                .renderer
                .fallback
                .iter()
                .any(|s| !backends.contains(&s.as_str()))
        {
            return Err("Renderer must support a bounded CPU fallback".into());
        }
        let mut end = 0;
        let mut ids = BTreeSet::new();
        for scene in &self.scenes {
            if scene.id.is_empty()
                || scene.id.len() > 64
                || !scene
                    .id
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '_')
                || !ids.insert(&scene.id)
                || scene.bar_start != end
                || scene.bar_end <= scene.bar_start
                || scene.bar_end > 1024
                || scene.effects.len() > 16
            {
                return Err(format!("Invalid or noncontiguous scene: {}", scene.id));
            }
            if canonical_profile(&scene.profile).is_none() {
                return Err(format!("Unknown profile/visual recipe: {}", scene.profile));
            }
            end = scene.bar_end;
        }
        if self.duration() > 600.
            || self.stress_panel_counts.is_empty()
            || self.stress_panel_counts.len() > 5
            || self
                .stress_panel_counts
                .iter()
                .any(|n| ![10, 50, 100, 250, 500].contains(n))
        {
            return Err("Scenario exceeds duration or stress limits".into());
        }
        Ok(())
    }
    pub fn evaluate(&self, seconds: f64) -> Result<TimelineState, String> {
        if !seconds.is_finite() {
            return Err("Non-finite timeline position".into());
        }
        let time = seconds.clamp(0., self.duration());
        let beat = ((time - self.timeline_origin_seconds).max(0.)) * self.bpm / 60.;
        let bar = beat / self.beats_per_bar as f64;
        let index = self
            .scenes
            .partition_point(|s| {
                self.timeline_origin_seconds + s.bar_end as f64 * self.bar_seconds() <= time
            })
            .min(self.scenes.len() - 1);
        let scene = &self.scenes[index];
        let progress = ((bar - scene.bar_start as f64) / (scene.bar_end - scene.bar_start) as f64)
            .clamp(0., 1.);
        Ok(TimelineState {
            seconds: time,
            beat,
            bar,
            scene_index: index,
            scene_progress: progress,
            finished: time >= self.duration(),
        })
    }
}
/// Virtual recipes explicitly map back to a canonical recovery layout; they are not core profile IDs.
pub fn canonical_profile(id: &str) -> Option<String> {
    let canonical = match id {
        "single" => "single-pane",
        "wall" | "cascade" | "storm" => "workbench",
        other => other,
    };
    doofusdock_core::builtin_workspace_profiles()
        .iter()
        .any(|p| p.id == canonical)
        .then(|| canonical.to_owned())
}
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct TimelineState {
    pub seconds: f64,
    pub beat: f64,
    pub bar: f64,
    pub scene_index: usize,
    pub scene_progress: f64,
    pub finished: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CueMap {
    pub title: String,
    pub audio: String,
    pub bpm: f64,
    pub beats_per_bar: u32,
    pub beat_duration_s: f64,
    pub bar_duration_s: f64,
    pub audio_decoded_duration_s: f64,
    pub performance_end_s: f64,
    pub sync_policy: String,
    pub segments: Vec<Segment>,
    pub rhythm_transients: Vec<Cue>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Segment {
    pub index: usize,
    pub name: String,
    pub system: String,
    pub start_bar: u32,
    pub start_s: f64,
    pub end_s: f64,
    pub duration_s: f64,
    pub instructions: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Cue {
    pub time_s: f64,
    pub source: String,
    pub strength: f64,
    pub beat: f64,
    pub segment: String,
}
impl CueMap {
    pub fn parse(json: &str, scenario: &Scenario) -> Result<Self, String> {
        if json.len() > MAX_JSON_BYTES as usize {
            return Err("Cue map exceeds 1 MiB".into());
        }
        let map: Self = serde_json::from_str(json).map_err(|e| format!("Cue JSON: {e}"))?;
        if map.bpm != scenario.bpm
            || map.beats_per_bar != scenario.beats_per_bar
            || !map.audio_decoded_duration_s.is_finite()
            || map.audio_decoded_duration_s < scenario.duration()
            || map.audio_decoded_duration_s > 600.
            || !map.performance_end_s.is_finite()
            || (map.performance_end_s - scenario.duration()).abs() > 0.002
            || !map.beat_duration_s.is_finite()
            || (map.beat_duration_s - 60. / scenario.bpm).abs() > 1e-6
            || !map.bar_duration_s.is_finite()
            || (map.bar_duration_s - scenario.bar_seconds()).abs() > 1e-6
        {
            return Err("Cue tempo/duration disagrees with scenario".into());
        }
        if map.rhythm_transients.len() > 8192 || map.segments.len() != scenario.scenes.len() {
            return Err("Cue/segment count invalid".into());
        }
        for (i, (segment, scene)) in map.segments.iter().zip(&scenario.scenes).enumerate() {
            let start =
                scenario.timeline_origin_seconds + scene.bar_start as f64 * scenario.bar_seconds();
            let end =
                scenario.timeline_origin_seconds + scene.bar_end as f64 * scenario.bar_seconds();
            if segment.index != i + 1
                || segment.start_bar != scene.bar_start + 1
                || ![segment.start_s, segment.end_s, segment.duration_s]
                    .iter()
                    .all(|v| v.is_finite())
                || (segment.start_s - start).abs() > 0.002
                || (segment.end_s - end).abs() > 0.002
                || (segment.duration_s - (end - start)).abs() > 0.002
            {
                return Err(format!("Segment {} grid mismatch", i + 1));
            }
        }
        let mut last = -1.;
        for cue in &map.rhythm_transients {
            if !cue.time_s.is_finite()
                || cue.time_s < 0.
                || cue.time_s < last
                || cue.time_s > map.audio_decoded_duration_s
                || !cue.strength.is_finite()
                || !(0.0..=1.).contains(&cue.strength)
                || !cue.beat.is_finite()
                || (cue.beat - (cue.time_s - scenario.timeline_origin_seconds) * scenario.bpm / 60.)
                    .abs()
                    > 0.01
                || cue.source.len() > 64
                || cue.segment.len() > 256
            {
                return Err("Invalid, unordered or out-of-range rhythm cue".into());
            }
            last = cue.time_s;
        }
        Ok(map)
    }
    /// Stateless range selection: no double fire after seek. Callers reset the previous position on discontinuities.
    pub fn between(&self, from: f64, to: f64, limit: usize) -> Result<(&[Cue], usize), String> {
        if !from.is_finite() || !to.is_finite() || limit > 256 {
            return Err("Invalid cue dispatch range or budget".into());
        }
        if to <= from {
            return Ok((&self.rhythm_transients[0..0], 0));
        }
        let first = self.rhythm_transients.partition_point(|c| c.time_s <= from);
        let last = self.rhythm_transients.partition_point(|c| c.time_s <= to);
        let omitted = (last - first).saturating_sub(limit);
        Ok((&self.rhythm_transients[first + omitted..last], omitted))
    }
}
/// Result of one bounded dispatch step.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Dispatch {
    /// Index range into `CueMap::rhythm_transients` to fire this frame.
    pub first: usize,
    pub last: usize,
    /// Cues inside the elapsed interval that exceeded the per-frame budget.
    pub omitted: usize,
    /// True when the step was a discontinuity (seek, loop wrap, new generation) and fired nothing.
    pub discontinuity: bool,
}
/// Edge-triggered cue dispatch driven only by observed clock positions.
/// Each cue in `(previous, current]` fires exactly once regardless of frame partitioning.
/// Seeks, backward motion and clock generation changes reset without firing.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CueDispatcher {
    previous: Option<(f64, u64)>,
}
impl CueDispatcher {
    pub fn step(
        &mut self,
        map: &CueMap,
        position: f64,
        generation: u64,
        limit: usize,
    ) -> Result<Dispatch, String> {
        if !position.is_finite() {
            return Err("Non-finite dispatch position".into());
        }
        let previous = self.previous.replace((position, generation));
        let from = match previous {
            Some((from, g)) if g == generation && from <= position => from,
            _ => {
                return Ok(Dispatch {
                    first: 0,
                    last: 0,
                    omitted: 0,
                    discontinuity: true,
                });
            }
        };
        let (fired, omitted) = map.between(from, position, limit)?;
        let last = map
            .rhythm_transients
            .partition_point(|c| c.time_s <= position);
        Ok(Dispatch {
            first: last - fired.len(),
            last,
            omitted,
            discontinuity: false,
        })
    }
}

pub const MAX_CSV_BYTES: u64 = 2 * 1024 * 1024;

/// Single entry in a MIDI variable-tempo map.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct MidiTempoPoint {
    pub tick: u64,
    pub time_seconds: f64,
    pub tempo_us_per_beat: u32,
    pub bpm: f64,
}

/// Variable-tempo map from MIDI tempo events (tick <-> second conversion).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct MidiTempoMap {
    pub points: Vec<MidiTempoPoint>,
}

impl MidiTempoMap {
    pub fn parse_csv(csv: &str) -> Result<Self, String> {
        if csv.len() > MAX_CSV_BYTES as usize {
            return Err("MIDI tempo map CSV exceeds limit".into());
        }
        let mut lines = csv.lines();
        let header = lines.next().ok_or("Empty MIDI tempo map CSV")?;
        if header.trim() != "tick,time_seconds,tempo_microseconds_per_beat,bpm" {
            return Err("Invalid MIDI tempo map header".into());
        }
        let mut points = Vec::new();
        let mut prev_tick = None;
        let mut prev_time = None;
        for (line_num, line) in lines.enumerate() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let mut parts = line.split(',');
            let tick = parts
                .next()
                .and_then(|s| s.parse::<u64>().ok())
                .ok_or_else(|| format!("Invalid tick at line {}", line_num + 2))?;
            let time_seconds = parts
                .next()
                .and_then(|s| s.parse::<f64>().ok())
                .ok_or_else(|| format!("Invalid time_seconds at line {}", line_num + 2))?;
            let tempo_us = parts
                .next()
                .and_then(|s| s.parse::<u32>().ok())
                .ok_or_else(|| format!("Invalid tempo at line {}", line_num + 2))?;
            let bpm = parts
                .next()
                .and_then(|s| s.parse::<f64>().ok())
                .ok_or_else(|| format!("Invalid bpm at line {}", line_num + 2))?;
            if parts.next().is_some() {
                return Err(format!("Unexpected column at line {}", line_num + 2));
            }
            if !time_seconds.is_finite() || !bpm.is_finite() || tempo_us == 0 {
                return Err(format!("Non-finite tempo data at line {}", line_num + 2));
            }
            if let Some(pt) = prev_tick
                && tick < pt
            {
                return Err(format!("Non-monotonic tick at line {}", line_num + 2));
            }
            if let Some(ptm) = prev_time
                && time_seconds < ptm
            {
                return Err(format!("Non-monotonic time at line {}", line_num + 2));
            }
            prev_tick = Some(tick);
            prev_time = Some(time_seconds);
            points.push(MidiTempoPoint {
                tick,
                time_seconds,
                tempo_us_per_beat: tempo_us,
                bpm,
            });
        }
        if points.is_empty() {
            return Err("MIDI tempo map contains no points".into());
        }
        Ok(Self { points })
    }

    pub fn bpm_at(&self, seconds: f64) -> f64 {
        if self.points.is_empty() {
            return 127.0;
        }
        let idx = self.points.partition_point(|p| p.time_seconds <= seconds);
        if idx == 0 {
            self.points[0].bpm
        } else {
            self.points[idx - 1].bpm
        }
    }

    pub fn seconds_to_tick(&self, seconds: f64) -> u64 {
        if self.points.is_empty() {
            return 0;
        }
        let idx = self.points.partition_point(|p| p.time_seconds <= seconds);
        if idx == 0 {
            0
        } else {
            let p = &self.points[idx - 1];
            let elapsed_sec = seconds - p.time_seconds;
            let us_per_tick = p.tempo_us_per_beat as f64 / 480.0;
            let elapsed_ticks = (elapsed_sec * 1_000_000.0 / us_per_tick).round() as u64;
            p.tick + elapsed_ticks
        }
    }

    pub fn tick_to_seconds(&self, tick: u64) -> f64 {
        if self.points.is_empty() {
            return 0.0;
        }
        let idx = self.points.partition_point(|p| p.tick <= tick);
        if idx == 0 {
            0.0
        } else {
            let p = &self.points[idx - 1];
            let delta_ticks = tick - p.tick;
            let us_per_tick = p.tempo_us_per_beat as f64 / 480.0;
            p.time_seconds + (delta_ticks as f64 * us_per_tick / 1_000_000.0)
        }
    }
}

/// Single musical note-on event from stem-separated MIDI.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MidiNoteEvent {
    pub stem: String,
    pub time_seconds: f64,
    pub tick: u64,
    pub midi_track: u8,
    pub channel: u8,
    pub note: u8,
    pub velocity: u8,
}

/// Monotonically ordered sequence of note-on events across all stems.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct MidiNoteSchedule {
    pub events: Vec<MidiNoteEvent>,
}

impl MidiNoteSchedule {
    pub fn parse_csv(csv: &str) -> Result<Self, String> {
        if csv.len() > MAX_CSV_BYTES as usize {
            return Err("MIDI note events CSV exceeds limit".into());
        }
        let mut lines = csv.lines();
        let header = lines.next().ok_or("Empty MIDI note events CSV")?;
        if header.trim() != "stem,time_seconds,tick,midi_track,channel,note,velocity" {
            return Err("Invalid MIDI note events header".into());
        }
        let mut events = Vec::new();
        let mut prev_time = 0.0;
        for (line_num, line) in lines.enumerate() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let mut parts = line.split(',');
            let stem = parts
                .next()
                .ok_or_else(|| format!("Missing stem at line {}", line_num + 2))?
                .to_string();
            let time_seconds = parts
                .next()
                .and_then(|s| s.parse::<f64>().ok())
                .ok_or_else(|| format!("Invalid time_seconds at line {}", line_num + 2))?;
            let tick = parts
                .next()
                .and_then(|s| s.parse::<u64>().ok())
                .ok_or_else(|| format!("Invalid tick at line {}", line_num + 2))?;
            let midi_track = parts
                .next()
                .and_then(|s| s.parse::<u8>().ok())
                .ok_or_else(|| format!("Invalid track at line {}", line_num + 2))?;
            let channel = parts
                .next()
                .and_then(|s| s.parse::<u8>().ok())
                .ok_or_else(|| format!("Invalid channel at line {}", line_num + 2))?;
            let note = parts
                .next()
                .and_then(|s| s.parse::<u8>().ok())
                .ok_or_else(|| format!("Invalid note at line {}", line_num + 2))?;
            let velocity = parts
                .next()
                .and_then(|s| s.parse::<u8>().ok())
                .ok_or_else(|| format!("Invalid velocity at line {}", line_num + 2))?;
            if parts.next().is_some() {
                return Err(format!("Unexpected column at line {}", line_num + 2));
            }
            if !time_seconds.is_finite() || time_seconds < 0.0 || time_seconds < prev_time {
                return Err(format!(
                    "Invalid or non-monotonic time at line {}",
                    line_num + 2
                ));
            }
            prev_time = time_seconds;
            events.push(MidiNoteEvent {
                stem,
                time_seconds,
                tick,
                midi_track,
                channel,
                note,
                velocity,
            });
        }
        Ok(Self { events })
    }

    /// Stateless range selection: no double fire after seek.
    pub fn between(
        &self,
        from: f64,
        to: f64,
        limit: usize,
    ) -> Result<(&[MidiNoteEvent], usize), String> {
        if !from.is_finite() || !to.is_finite() || limit > 1024 {
            return Err("Invalid MIDI dispatch range or budget".into());
        }
        if to <= from {
            return Ok((&self.events[0..0], 0));
        }
        let first = self.events.partition_point(|c| c.time_seconds <= from);
        let last = self.events.partition_point(|c| c.time_seconds <= to);
        let omitted = (last - first).saturating_sub(limit);
        Ok((&self.events[first + omitted..last], omitted))
    }
}

/// Edge-triggered MIDI note dispatch driven by observed clock positions.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct MidiDispatcher {
    previous: Option<(f64, u64)>,
}

impl MidiDispatcher {
    pub fn step(
        &mut self,
        schedule: &MidiNoteSchedule,
        position: f64,
        generation: u64,
        limit: usize,
    ) -> Result<Dispatch, String> {
        if !position.is_finite() {
            return Err("Non-finite dispatch position".into());
        }
        let previous = self.previous.replace((position, generation));
        let from = match previous {
            Some((from, g)) if g == generation && from <= position => from,
            _ => {
                return Ok(Dispatch {
                    first: 0,
                    last: 0,
                    omitted: 0,
                    discontinuity: true,
                });
            }
        };
        let (fired, omitted) = schedule.between(from, position, limit)?;
        let last = schedule
            .events
            .partition_point(|c| c.time_seconds <= position);
        Ok(Dispatch {
            first: last - fired.len(),
            last,
            omitted,
            discontinuity: false,
        })
    }
}
pub fn read_json(path: &Path) -> Result<String, String> {
    let mut bytes = Vec::new();
    std::fs::File::open(path)
        .map_err(|e| format!("{}: {e}", path.display()))?
        .take(MAX_JSON_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > MAX_JSON_BYTES as usize {
        return Err("JSON exceeds 1 MiB".into());
    }
    String::from_utf8(bytes).map_err(|e| e.to_string())
}
fn validate_relative(name: &str) -> Result<(), String> {
    if name.len() > 1024
        || name.is_empty()
        || name.contains(':')
        || name.contains('\\')
        || Path::new(name)
            .components()
            .any(|c| !matches!(c, Component::Normal(_)))
    {
        Err("Asset path must be a safe package-relative path".into())
    } else {
        Ok(())
    }
}
pub fn resolve_asset(root: &Path, name: &str) -> Result<PathBuf, String> {
    validate_relative(name)?;
    let base = root.canonicalize().map_err(|e| e.to_string())?;
    let path = base
        .join(name)
        .canonicalize()
        .map_err(|e| format!("Asset {name}: {e}"))?;
    if !path.starts_with(&base) {
        return Err("Asset escapes package root".into());
    }
    Ok(path)
}
