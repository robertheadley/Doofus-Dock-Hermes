//! Reusable deterministic tween tracks. Tracks are transient presentation data;
//! sampling never mutates canonical layout state.
use crate::{Box2, Easing, Visual};

pub trait TweenValue: Clone {
    fn interpolate(from: &Self, to: &Self, progress: f64) -> Self;
}

impl TweenValue for f64 {
    fn interpolate(from: &Self, to: &Self, progress: f64) -> Self {
        from + (to - from) * progress
    }
}

impl TweenValue for f32 {
    fn interpolate(from: &Self, to: &Self, progress: f64) -> Self {
        (*from as f64 + (*to as f64 - *from as f64) * progress) as f32
    }
}

impl TweenValue for Box2 {
    fn interpolate(from: &Self, to: &Self, progress: f64) -> Self {
        from.mix(*to, progress.clamp(0.0, 1.0))
    }
}

impl TweenValue for Visual {
    fn interpolate(from: &Self, to: &Self, progress: f64) -> Self {
        Self {
            bounds: from.bounds.mix(to.bounds, progress.clamp(0.0, 1.0)),
            clip: from.clip.mix(to.clip, progress.clamp(0.0, 1.0)),
            opacity: from.opacity + (to.opacity - from.opacity) * progress.clamp(0.0, 1.0),
            z: if progress < 0.5 { from.z } else { to.z },
        }
    }
}

#[derive(Clone, Debug)]
pub struct Tween<T> {
    from: T,
    to: T,
    start: f64,
    duration: f64,
    easing: Easing,
    reduced_motion: bool,
    settled: bool,
}

impl<T: TweenValue> Tween<T> {
    pub fn new(
        from: T,
        to: T,
        start: f64,
        duration: f64,
        easing: Easing,
        reduced_motion: bool,
    ) -> Result<Self, String> {
        validate_time(start, duration, easing)?;
        Ok(Self {
            from,
            to,
            start,
            duration: if reduced_motion { 0.0 } else { duration },
            easing,
            reduced_motion,
            settled: false,
        })
    }

    pub fn progress(&self, time: f64) -> Result<f64, String> {
        if !time.is_finite() {
            return Err("Tween time must be finite".into());
        }
        Ok(if self.duration == 0.0 {
            if time < self.start { 0.0 } else { 1.0 }
        } else {
            ((time - self.start) / self.duration).clamp(0.0, 1.0)
        })
    }

    pub fn sample(&self, time: f64) -> Result<T, String> {
        let t = self.easing.sample(self.progress(time)?);
        Ok(T::interpolate(&self.from, &self.to, t))
    }

    /// Start a new segment at the exact currently sampled value.
    pub fn retarget(
        &mut self,
        to: T,
        time: f64,
        duration: f64,
        easing: Easing,
    ) -> Result<(), String> {
        if self.settled {
            return Err("Tween is settled".into());
        }
        validate_time(time, duration, easing)?;
        let current = self.sample(time)?;
        self.from = current;
        self.to = to;
        self.start = time;
        self.duration = if self.reduced_motion { 0.0 } else { duration };
        self.easing = easing;
        Ok(())
    }

    pub fn reverse(&mut self, time: f64, duration: f64) -> Result<(), String> {
        self.retarget(self.from.clone(), time, duration, self.easing)
    }

    pub fn is_complete(&self, time: f64) -> Result<bool, String> {
        Ok(self.progress(time)? >= 1.0)
    }

    pub fn cancel(&mut self) {
        self.settled = true;
    }

    pub fn is_settled(&self) -> bool {
        self.settled
    }

    pub fn start_time(&self) -> f64 {
        self.start
    }

    pub fn duration(&self) -> f64 {
        self.duration
    }
}

fn validate_time(start: f64, duration: f64, easing: Easing) -> Result<(), String> {
    if !start.is_finite() || !duration.is_finite() || !(0.0..=60.0).contains(&duration) {
        return Err("Tween time must be finite and duration within 0..60 seconds".into());
    }
    if !easing.valid() {
        return Err("Tween easing parameters are invalid".into());
    }
    Ok(())
}

/// Parallel/sequence-ready group over existing tween tracks. Sampling delegates
/// to each track; this type adds no interpolation path and owns no layout state.
#[derive(Clone, Debug)]
pub struct TransitionGroup<T> {
    tracks: Vec<Tween<T>>,
    cancelled: bool,
}
impl<T: TweenValue> TransitionGroup<T> {
    const MAX_SCHEDULED_TRACKS: usize = 256;

    pub fn new(tracks: Vec<Tween<T>>) -> Result<Self, String> {
        if tracks.is_empty() {
            return Err("Transition group requires at least one track".into());
        }
        Ok(Self {
            tracks,
            cancelled: false,
        })
    }

    /// Schedule tracks one after another, using the first track's start as the
    /// sequence origin. The total schedule is bounded to 60 seconds.
    pub fn sequence(mut tracks: Vec<Tween<T>>) -> Result<Self, String> {
        if tracks.is_empty() || tracks.len() > Self::MAX_SCHEDULED_TRACKS {
            return Err("Transition sequence must contain 1..=256 tracks".into());
        }
        let origin = tracks[0].start;
        let mut cursor = origin;
        for track in &mut tracks {
            track.start = cursor;
            cursor += track.duration;
            if !cursor.is_finite() || cursor - origin > 60.0 {
                return Err("Transition sequence duration exceeds 60 seconds".into());
            }
        }
        Self::new(tracks)
    }

    /// Start tracks at a fixed interval from the first track's start. The
    /// complete staggered schedule is bounded to 60 seconds.
    pub fn staggered(mut tracks: Vec<Tween<T>>, interval: f64) -> Result<Self, String> {
        if tracks.is_empty() || tracks.len() > Self::MAX_SCHEDULED_TRACKS {
            return Err("Staggered transition must contain 1..=256 tracks".into());
        }
        if !interval.is_finite() || !(0.0..=60.0).contains(&interval) {
            return Err(
                "Transition stagger interval must be finite and within 0..=60 seconds".into(),
            );
        }
        let origin = tracks[0].start;
        for (index, track) in tracks.iter_mut().enumerate() {
            let offset = interval * index as f64;
            let start = origin + offset;
            if !start.is_finite() || offset + track.duration > 60.0 {
                return Err("Staggered transition duration exceeds 60 seconds".into());
            }
            track.start = start;
        }
        Self::new(tracks)
    }

    pub fn sample(&self, time: f64) -> Result<Vec<T>, String> {
        if !time.is_finite() {
            return Err("Transition time must be finite".into());
        }
        self.tracks.iter().map(|track| track.sample(time)).collect()
    }
    pub fn is_complete(&self, time: f64) -> Result<bool, String> {
        self.tracks
            .iter()
            .try_fold(true, |done, track| Ok(done && track.is_complete(time)?))
    }
    pub fn cancel(&mut self) {
        self.cancelled = true;
        self.tracks.iter_mut().for_each(Tween::cancel);
    }
    pub fn is_cancelled(&self) -> bool {
        self.cancelled
    }
    pub fn tracks(&self) -> &[Tween<T>] {
        &self.tracks
    }
}
