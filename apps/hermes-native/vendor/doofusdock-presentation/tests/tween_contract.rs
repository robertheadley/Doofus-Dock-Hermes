use doofusdock_presentation::*;

#[test]
fn retargeting_starts_at_the_current_sample_without_a_jump() {
    let mut tween = Tween::new(0.0_f64, 100.0, 0.0, 2.0, Easing::EaseInOut, false).unwrap();
    let before = tween.sample(0.8).unwrap();
    tween
        .retarget(
            40.0,
            0.8,
            1.0,
            Easing::CubicBezier {
                x1: 0.2,
                y1: 0.0,
                x2: 0.4,
                y2: 1.0,
            },
        )
        .unwrap();
    assert_eq!(tween.sample(0.8).unwrap(), before);
    assert!((tween.sample(1.8).unwrap() - 40.0).abs() < f64::EPSILON);
    assert!(tween.is_complete(1.8).unwrap());
}

#[test]
fn reduced_motion_is_immediate_and_invalid_timing_is_rejected() {
    let tween = Tween::new(
        2.0_f64,
        5.0,
        10.0,
        2.0,
        Easing::Spring {
            damping: 6.0,
            frequency: 2.0,
        },
        true,
    )
    .unwrap();
    assert_eq!(tween.sample(9.0).unwrap(), 2.0);
    assert_eq!(tween.sample(10.0).unwrap(), 5.0);
    assert!(
        Tween::new(
            0.0_f64,
            1.0,
            0.0,
            1.0,
            Easing::CubicBezier {
                x1: f64::NAN,
                y1: 0.0,
                x2: 1.0,
                y2: 1.0
            },
            false
        )
        .is_err()
    );
    assert!(Tween::new(0.0_f64, 1.0, f64::NAN, 1.0, Easing::Linear, false).is_err());
}

#[test]
fn box_tween_is_deterministic_and_keeps_geometry_nonnegative() {
    let from = Box2 {
        x: 0.0,
        y: 0.0,
        w: 10.0,
        h: 10.0,
    };
    let to = Box2 {
        x: 100.0,
        y: 50.0,
        w: 400.0,
        h: 200.0,
    };
    let tween = Tween::new(
        from,
        to,
        0.0,
        1.0,
        Easing::Spring {
            damping: 0.2,
            frequency: 12.0,
        },
        false,
    )
    .unwrap();
    let a = tween.sample(0.4).unwrap();
    let b = tween.sample(0.4).unwrap();
    assert_eq!(a, b);
    assert!(a.w >= 0.0 && a.h >= 0.0);
}

#[test]
fn transition_groups_schedule_sequence_and_stagger_without_new_interpolation() {
    let track =
        |to, start| Tween::new(to - 1.0_f64, to, start, 1.0, Easing::Linear, false).unwrap();
    let sequence = TransitionGroup::sequence(vec![track(1.0, 4.0), track(2.0, 90.0)]).unwrap();
    assert_eq!(sequence.tracks()[0].start_time(), 4.0);
    assert_eq!(sequence.tracks()[1].start_time(), 5.0);
    assert_eq!(sequence.sample(4.5).unwrap(), vec![0.5, 1.0]);
    assert_eq!(sequence.sample(5.5).unwrap(), vec![1.0, 1.5]);
    assert!(sequence.is_complete(6.0).unwrap());

    let staggered =
        TransitionGroup::staggered(vec![track(1.0, 8.0), track(2.0, 80.0)], 0.25).unwrap();
    assert_eq!(staggered.tracks()[0].start_time(), 8.0);
    assert_eq!(staggered.tracks()[1].start_time(), 8.25);
    assert_eq!(staggered.sample(8.5).unwrap(), vec![0.5, 1.25]);
}

#[test]
fn transition_schedules_reject_unbounded_track_counts_and_durations() {
    assert!(TransitionGroup::<f64>::sequence(Vec::new()).is_err());
    assert!(TransitionGroup::<f64>::staggered(Vec::new(), 0.1).is_err());
    let tracks = (0..257)
        .map(|index| Tween::new(0.0_f64, 1.0, index as f64, 0.0, Easing::Linear, false).unwrap())
        .collect();
    assert!(TransitionGroup::sequence(tracks).is_err());
    let tracks = vec![
        Tween::new(0.0_f64, 1.0, 0.0, 1.0, Easing::Linear, false).unwrap(),
        Tween::new(0.0_f64, 1.0, 0.0, 1.0, Easing::Linear, false).unwrap(),
    ];
    assert!(TransitionGroup::staggered(tracks, f64::NAN).is_err());
    let tracks = vec![
        Tween::new(0.0_f64, 1.0, 0.0, 60.0, Easing::Linear, false).unwrap(),
        Tween::new(0.0_f64, 1.0, 0.0, 60.0, Easing::Linear, false).unwrap(),
    ];
    assert!(TransitionGroup::sequence(tracks).is_err());
}
