use doofusdock_presentation::timeline::*;
const SCENARIO: &str = include_str!("../../../assets/kinetic-chaos/scenario.json");
#[test]
fn exact_boundaries_seek_and_late_frame_are_deterministic() {
    let s = Scenario::parse(SCENARIO).unwrap();
    assert!((s.duration() - 102.04724409448819).abs() < 1e-8);
    for (i, scene) in s.scenes.iter().enumerate() {
        let t = scene.bar_start as f64 * s.bar_seconds();
        let a = s.evaluate(t).unwrap();
        assert_eq!(a.scene_index, i);
        assert_eq!(s.evaluate(t).unwrap(), a);
    }
    assert!(s.evaluate(s.duration()).unwrap().finished);
    assert_eq!(s.evaluate(65.).unwrap(), s.evaluate(65.).unwrap());
    assert!(s.evaluate(f64::NAN).is_err());
}
#[test]
fn malformed_scenario_and_unknown_effects_fail_before_playback() {
    let bad = SCENARIO.replace("\"materialize\"", "\"execute_shell\"");
    assert!(Scenario::parse(&bad).is_err());
    let mut s = Scenario::parse(SCENARIO).unwrap();
    s.scenes[1].bar_start += 1;
    assert!(s.validate().is_err());
    s.scenes[1].bar_start -= 1;
    s.stress_panel_counts = vec![1000000];
    assert!(s.validate().is_err());
    s.stress_panel_counts = vec![10];
    s.music.master = "../secret.mp3".into();
    assert!(s.validate().is_err());
}
#[test]
fn reference_cues_validate_and_bounded_dispatch_never_repeats_endpoints() {
    let s = Scenario::parse(SCENARIO).unwrap();
    let m = CueMap::parse(
        include_str!("../../../assets/kinetic-chaos/original-stem-cues.json"),
        &s,
    )
    .unwrap();
    let (all, missed) = m.between(-1., s.duration(), 256).unwrap();
    assert_eq!(all.len(), 219);
    assert_eq!(missed, 0);
    let (bounded, missed) = m.between(-1., s.duration(), 16).unwrap();
    assert_eq!(bounded.len(), 16);
    assert_eq!(missed, 203);
    let t = all[0].time_s;
    assert!(m.between(t, t, 16).unwrap().0.is_empty());
    assert!(m.between(50., 10., 16).unwrap().0.is_empty());
    assert!(m.between(0., 1., 257).is_err());
}

#[test]
fn edge_dispatch_is_partition_invariant_and_resets_on_discontinuity() {
    let s = Scenario::parse(SCENARIO).unwrap();
    let m = CueMap::parse(
        include_str!("../../../assets/kinetic-chaos/master-cues.json"),
        &s,
    )
    .unwrap();
    let fire = |steps: &[f64], generation: &dyn Fn(usize) -> u64| {
        let mut d = CueDispatcher::default();
        let mut fired = Vec::new();
        for (i, &t) in steps.iter().enumerate() {
            let r = d.step(&m, t, generation(i), 256).unwrap();
            assert_eq!(r.omitted, 0);
            fired.extend(r.first..r.last);
        }
        fired
    };
    let coarse: Vec<f64> = (0..=40).map(|i| i as f64 * 0.5).collect();
    let fine: Vec<f64> = (0..=1200).map(|i| i as f64 / 60.).collect();
    let jitter: Vec<f64> = (0..=700)
        .map(|i| (i as f64 * 0.0287 + (i % 7) as f64 * 0.0011).min(20.))
        .collect();
    let expected: Vec<usize> = (0..m.rhythm_transients.len())
        .filter(|&i| (0.0..=20.).contains(&m.rhythm_transients[i].time_s))
        .filter(|&i| m.rhythm_transients[i].time_s > 0.)
        .collect();
    assert!(!expected.is_empty());
    assert_eq!(fire(&coarse, &|_| 0), expected);
    assert_eq!(fire(&fine, &|_| 0), expected);
    assert_eq!(fire(&jitter, &|_| 0), expected);
    let mut d = CueDispatcher::default();
    assert!(d.step(&m, 10., 0, 16).unwrap().discontinuity);
    let late = d.step(&m, 60., 0, 4).unwrap();
    assert_eq!(late.last - late.first, 4);
    assert!(late.omitted > 0);
    assert!(
        d.step(&m, 59., 0, 16).unwrap().discontinuity,
        "backward/loop"
    );
    assert!(
        d.step(&m, 70., 1, 16).unwrap().discontinuity,
        "seek generation"
    );
    assert!(!d.step(&m, 70.5, 1, 16).unwrap().discontinuity);
    assert!(d.step(&m, f64::NAN, 1, 16).is_err());
}

#[test]
fn selected_master_candidates_validate_and_match_asset() {
    let s = Scenario::parse(SCENARIO).unwrap();
    let m = CueMap::parse(
        include_str!("../../../assets/kinetic-chaos/master-cues.json"),
        &s,
    )
    .unwrap();
    assert_eq!(m.audio, s.music.master);
    assert!(m.rhythm_transients.len() > 100);
    assert!(
        m.rhythm_transients
            .iter()
            .all(|c| c.source == "master_onset_candidate")
    );
}

#[test]
fn midi_tempo_map_and_note_schedule_contract() {
    let tempo_csv = include_str!("../../../assets/kinetic-chaos/midi_tempo_map.csv");
    let tempo_map = MidiTempoMap::parse_csv(tempo_csv).unwrap();
    assert_eq!(tempo_map.points.len(), 196);
    assert_eq!(tempo_map.points[0].tick, 0);
    assert_eq!(tempo_map.points[0].time_seconds, 0.0);
    for p in &tempo_map.points {
        assert!((125.0..=128.0).contains(&p.bpm));
    }
    // Round-trip conversion tick <-> seconds
    for p in &tempo_map.points {
        let sec = tempo_map.tick_to_seconds(p.tick);
        assert!((sec - p.time_seconds).abs() < 1e-4);
        let tick = tempo_map.seconds_to_tick(p.time_seconds);
        assert_eq!(tick, p.tick);
    }

    let notes_csv = include_str!("../../../assets/kinetic-chaos/midi_note_events.csv");
    let notes = MidiNoteSchedule::parse_csv(notes_csv).unwrap();
    assert_eq!(notes.events.len(), 7819);

    let mut stems = std::collections::BTreeSet::new();
    for ev in &notes.events {
        stems.insert(ev.stem.as_str());
    }
    assert_eq!(stems.len(), 9);
    assert!(stems.contains("Drums"));
    assert!(stems.contains("Bass"));
    assert!(stems.contains("Synth"));
    assert!(stems.contains("Keyboard"));

    // Edge dispatcher test on MIDI notes
    let mut d = MidiDispatcher::default();
    let r1 = d.step(&notes, 0.5, 0, 1024).unwrap();
    assert!(r1.discontinuity);
    let r2 = d.step(&notes, 1.0, 0, 1024).unwrap();
    assert!(!r2.discontinuity);
    assert!(r2.last > r2.first);
    let fired_count = r2.last - r2.first;
    assert!(fired_count > 0);

    // Discontinuity resets without duplicate
    let r3 = d.step(&notes, 0.8, 1, 1024).unwrap();
    assert!(r3.discontinuity);
}
