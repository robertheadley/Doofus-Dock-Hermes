use doofusdock_presentation::*;
#[test]
fn group_samples_tracks_in_parallel_and_cancels_atomically() {
    let linear = Tween::new(0.0, 10.0, 0.0, 1.0, Easing::Linear, false).unwrap();
    let delayed = Tween::new(10.0, 20.0, 0.5, 1.0, Easing::Linear, false).unwrap();
    let mut group = TransitionGroup::new(vec![linear, delayed]).unwrap();
    assert_eq!(group.sample(0.5).unwrap(), vec![5.0, 10.0]);
    assert!(!group.is_complete(1.0).unwrap());
    assert!(group.is_complete(1.5).unwrap());
    group.cancel();
    assert!(group.is_cancelled());
    assert!(group.tracks().iter().all(Tween::is_settled));
}
#[test]
fn group_rejects_empty_tracks_and_invalid_sample_time() {
    assert!(TransitionGroup::<f64>::new(vec![]).is_err());
    let t = Tween::new(1.0, 2.0, 0.0, 1.0, Easing::Linear, true).unwrap();
    let group = TransitionGroup::new(vec![t]).unwrap();
    assert!(group.sample(f64::NAN).is_err());
}
