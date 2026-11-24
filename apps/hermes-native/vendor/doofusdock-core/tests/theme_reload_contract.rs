use doofusdock_core::{
    MAX_THEME_SOURCE_BYTES, THEME_SOURCE_POLL_INTERVAL, ThemeManifest, ThemePackageEditor,
    ThemeSourceWatcher, ThemeValue,
};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, Instant},
};

static NEXT_TEMP: AtomicU64 = AtomicU64::new(0);

fn temp_path() -> PathBuf {
    let id = NEXT_TEMP.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "doofusdock-public-theme-reload-{}-{id}.json",
        std::process::id()
    ))
}

fn editor_with_accent(accent: &str) -> ThemePackageEditor {
    let manifest = ThemeManifest::parse_json(include_str!(
        "../../../themes/doofusdock-workbench/theme.json"
    ))
    .unwrap();
    let mut editor = ThemePackageEditor::new(manifest).unwrap();
    editor
        .set_base_token("color.accent", ThemeValue::Color(accent.into()))
        .unwrap();
    editor
}

fn replace_atomically(path: &Path, editor: &ThemePackageEditor) {
    let temp = path.with_extension("public-reload-tmp");
    editor.clone().save(&temp).unwrap();
    fs::rename(temp, path).unwrap();
}

#[test]
fn external_consumer_can_watch_validate_activate_and_revert_a_theme_package() {
    let path = temp_path();
    editor_with_accent("#F28C28").save(&path).unwrap();
    let mut watcher = ThemeSourceWatcher::new(&path).unwrap();
    assert!(!watcher.enabled());
    assert_eq!(watcher.path(), path);
    assert_eq!(MAX_THEME_SOURCE_BYTES, 1024 * 1024);
    assert_eq!(THEME_SOURCE_POLL_INTERVAL, Duration::from_millis(500));

    watcher.set_enabled(true);
    let mut active = ThemePackageEditor::load(&path).unwrap();
    replace_atomically(&path, &editor_with_accent("#00AA44"));
    assert!(watcher.poll(Instant::now() + Duration::from_secs(1)));
    assert_eq!(
        active.manifest().tokens["color.accent"],
        ThemeValue::Color("#F28C28".into()),
        "watching stages a candidate without mutating host-owned active state"
    );
    assert_eq!(
        watcher.candidate().unwrap().manifest().tokens["color.accent"],
        ThemeValue::Color("#00AA44".into())
    );

    assert!(watcher.activate(&mut active));
    assert_eq!(
        active.manifest().tokens["color.accent"],
        ThemeValue::Color("#00AA44".into())
    );
    assert!(watcher.revert_activation(&mut active));
    assert_eq!(
        active.manifest().tokens["color.accent"],
        ThemeValue::Color("#F28C28".into())
    );
    fs::remove_file(path).unwrap();
}
