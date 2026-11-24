use doofusdock_core::{UpgradeWriterLease, checksum_layout_json};
use serde_json::json;
use std::{
    fs,
    path::PathBuf,
    process::{Command, Output},
    sync::atomic::{AtomicU64, Ordering},
};

static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

struct TempTree(PathBuf);

impl TempTree {
    fn new() -> Self {
        let sequence = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "doofusdock-upgrade-cli-{}-{sequence}",
            std::process::id()
        ));
        fs::create_dir_all(&path).expect("temporary test root should be created");
        Self(path)
    }

    fn path(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }
}

impl Drop for TempTree {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn run_cli(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_doofusdock-migrate"))
        .args(args)
        .output()
        .expect("migration CLI should start")
}

#[test]
fn migration_cli_refuses_to_recover_while_a_host_holds_the_writer_lease() {
    let tree = TempTree::new();
    let data = tree.path("data");
    fs::create_dir_all(&data).unwrap();
    let data_arg = data.to_string_lossy().into_owned();

    let lease = UpgradeWriterLease::try_acquire(&data).unwrap();
    assert_eq!(lease.data_root(), fs::canonicalize(&data).unwrap());
    assert!(lease.lock_path().is_file());

    let blocked = run_cli(&["recover", "--data-root", &data_arg]);
    assert!(!blocked.status.success());
    let error = String::from_utf8_lossy(&blocked.stderr);
    assert!(error.contains("writer coordination failed"), "{error}");
    assert!(error.contains("cooperating DoofusDock host"), "{error}");

    drop(lease);
    let recovered = run_cli(&["recover", "--data-root", &data_arg]);
    assert!(
        recovered.status.success(),
        "{}",
        String::from_utf8_lossy(&recovered.stderr)
    );
    let summary: serde_json::Value = serde_json::from_slice(&recovered.stdout).unwrap();
    assert_eq!(summary["recovered"], false);
    assert_eq!(summary["reason"], "no pending restore");
}

#[test]
fn migration_cli_mutations_share_the_host_lease_while_inspect_stays_read_only() {
    let tree = TempTree::new();
    let data = tree.path("data");
    let application = tree.path("application");
    fs::create_dir_all(&data).unwrap();
    fs::create_dir_all(&application).unwrap();
    let manifest = tree.path("manifest.json");
    fs::write(
        &manifest,
        serde_json::to_vec(&json!({
            "schema_version": 1,
            "stores": [{
                "relative_path": "optional-layout.json",
                "format": "layout",
                "optional": true
            }]
        }))
        .unwrap(),
    )
    .unwrap();
    let data_arg = data.to_string_lossy().into_owned();
    let application_arg = application.to_string_lossy().into_owned();
    let manifest_arg = manifest.to_string_lossy().into_owned();
    let checkpoint_arg = tree.path("checkpoint").to_string_lossy().into_owned();
    let journal_arg = tree
        .path("restore-journal.json")
        .to_string_lossy()
        .into_owned();

    let lease = UpgradeWriterLease::try_acquire(&data).unwrap();
    let inspect = run_cli(&[
        "inspect",
        "--data-root",
        &data_arg,
        "--manifest",
        &manifest_arg,
    ]);
    assert!(
        inspect.status.success(),
        "{}",
        String::from_utf8_lossy(&inspect.stderr)
    );
    let inspect_summary: serde_json::Value = serde_json::from_slice(&inspect.stdout).unwrap();
    assert_eq!(inspect_summary["action"], "inspect");
    assert_eq!(inspect_summary["migration_count"], 0);

    let mutation_args: [Vec<&str>; 5] = [
        vec![
            "apply",
            "--data-root",
            &data_arg,
            "--manifest",
            &manifest_arg,
            "--checkpoint",
            &checkpoint_arg,
        ],
        vec![
            "restore",
            "--data-root",
            &data_arg,
            "--checkpoint",
            &checkpoint_arg,
        ],
        vec!["recover", "--data-root", &data_arg],
        vec![
            "checkpoint-release",
            "--application-root",
            &application_arg,
            "--data-root",
            &data_arg,
            "--checkpoint",
            &checkpoint_arg,
        ],
        vec![
            "restore-release",
            "--application-root",
            &application_arg,
            "--data-root",
            &data_arg,
            "--checkpoint",
            &checkpoint_arg,
            "--journal",
            &journal_arg,
        ],
    ];
    for args in mutation_args {
        let blocked = run_cli(&args);
        let error = String::from_utf8_lossy(&blocked.stderr);
        assert!(
            !blocked.status.success(),
            "{} unexpectedly succeeded",
            args[0]
        );
        assert!(
            error.contains("upgrade writer coordination failed"),
            "{} returned an unexpected error: {error}",
            args[0]
        );
    }
    assert_eq!(fs::read_dir(&data).unwrap().count(), 0);
    drop(lease);
}

#[test]
fn headless_cli_checkpoints_and_restores_application_with_compatible_data() {
    let tree = TempTree::new();
    let application = tree.path("application");
    let data = tree.path("data");
    fs::create_dir_all(&application).unwrap();
    fs::create_dir_all(&data).unwrap();
    fs::write(application.join("host.exe"), b"old-host-binary").unwrap();
    fs::write(data.join("preferences.json"), b"old-compatible-state").unwrap();
    let checkpoint = tree.path("release-checkpoint");
    let app_arg = application.to_string_lossy().into_owned();
    let data_arg = data.to_string_lossy().into_owned();
    let checkpoint_arg = checkpoint.to_string_lossy().into_owned();
    let created = run_cli(&[
        "checkpoint-release",
        "--application-root",
        &app_arg,
        "--data-root",
        &data_arg,
        "--checkpoint",
        &checkpoint_arg,
    ]);
    assert!(
        created.status.success(),
        "{}",
        String::from_utf8_lossy(&created.stderr)
    );
    let checkpoint_json: serde_json::Value = serde_json::from_slice(&created.stdout).unwrap();
    assert_eq!(checkpoint_json["action"], "checkpoint-release");
    assert_eq!(checkpoint_json["application_files"], 1);
    assert_eq!(checkpoint_json["data_files"], 1);

    fs::write(application.join("host.exe"), b"new-host-binary").unwrap();
    fs::write(application.join("new.dll"), b"new-binary").unwrap();
    fs::write(data.join("preferences.json"), b"new-state").unwrap();
    let journal = tree.path("release-restore.json");
    let journal_arg = journal.to_string_lossy().into_owned();
    let restored = run_cli(&[
        "restore-release",
        "--application-root",
        &app_arg,
        "--data-root",
        &data_arg,
        "--checkpoint",
        &checkpoint_arg,
        "--journal",
        &journal_arg,
    ]);
    assert!(
        restored.status.success(),
        "{}",
        String::from_utf8_lossy(&restored.stderr)
    );
    let restore_json: serde_json::Value = serde_json::from_slice(&restored.stdout).unwrap();
    assert_eq!(restore_json["action"], "restore-release");
    assert_eq!(restore_json["resumed"], false);
    assert_eq!(
        fs::read(application.join("host.exe")).unwrap(),
        b"old-host-binary"
    );
    assert!(!application.join("new.dll").exists());
    assert_eq!(
        fs::read(data.join("preferences.json")).unwrap(),
        b"old-compatible-state"
    );
    assert!(!journal.exists());

    let recovered = run_cli(&["recover-release", "--journal", &journal_arg]);
    assert!(
        recovered.status.success(),
        "{}",
        String::from_utf8_lossy(&recovered.stderr)
    );
    let recovery_json: serde_json::Value = serde_json::from_slice(&recovered.stdout).unwrap();
    assert_eq!(recovery_json["action"], "recover-release");
    assert_eq!(recovery_json["recovered"], false);
}

#[test]
fn headless_cli_resumes_interrupted_release_restore_from_coordinator_journal() {
    let tree = TempTree::new();
    let application = tree.path("application");
    let data = tree.path("data");
    fs::create_dir_all(&application).unwrap();
    fs::create_dir_all(&data).unwrap();
    fs::write(application.join("host.exe"), b"old-host-binary").unwrap();
    fs::write(data.join("preferences.json"), b"old-compatible-state").unwrap();
    let checkpoint = tree.path("release-checkpoint");
    let app_arg = application.to_string_lossy().into_owned();
    let data_arg = data.to_string_lossy().into_owned();
    let checkpoint_arg = checkpoint.to_string_lossy().into_owned();
    let created = run_cli(&[
        "checkpoint-release",
        "--application-root",
        &app_arg,
        "--data-root",
        &data_arg,
        "--checkpoint",
        &checkpoint_arg,
    ]);
    assert!(
        created.status.success(),
        "{}",
        String::from_utf8_lossy(&created.stderr)
    );

    fs::write(application.join("host.exe"), b"new-host-binary").unwrap();
    fs::write(data.join("preferences.json"), b"new-state").unwrap();
    let app_checkpoint_arg = checkpoint
        .join("application")
        .to_string_lossy()
        .into_owned();
    let app_restore = run_cli(&[
        "restore",
        "--data-root",
        &app_arg,
        "--checkpoint",
        &app_checkpoint_arg,
    ]);
    assert!(
        app_restore.status.success(),
        "{}",
        String::from_utf8_lossy(&app_restore.stderr)
    );
    let app_restore_json: serde_json::Value = serde_json::from_slice(&app_restore.stdout).unwrap();

    let journal_path = tree.path("recovery/release-restore.json");
    fs::create_dir_all(journal_path.parent().unwrap()).unwrap();
    let mut journal = json!({
        "schema_version": 1,
        "checkpoint_root": fs::canonicalize(&checkpoint).unwrap(),
        "application_destination": fs::canonicalize(&application).unwrap(),
        "data_destination": fs::canonicalize(&data).unwrap(),
        "application_replaced_tree": app_restore_json["retained_replaced_tree"],
        "data_replaced_tree": null,
        "phase": "application_restored",
        "checksum": ""
    });
    let mut checksum_payload = journal.clone();
    checksum_payload.as_object_mut().unwrap().remove("checksum");
    let checksum = checksum_layout_json(&checksum_payload.to_string()).unwrap();
    journal["checksum"] = json!(checksum);
    fs::write(&journal_path, serde_json::to_vec(&journal).unwrap()).unwrap();

    let journal_arg = journal_path.to_string_lossy().into_owned();
    let data_lease = UpgradeWriterLease::try_acquire(&data).unwrap();
    let blocked_recovery = run_cli(&["recover-release", "--journal", &journal_arg]);
    assert!(!blocked_recovery.status.success());
    assert!(
        String::from_utf8_lossy(&blocked_recovery.stderr)
            .contains("upgrade writer coordination failed")
    );
    drop(data_lease);
    let recovered = run_cli(&["recover-release", "--journal", &journal_arg]);
    assert!(
        recovered.status.success(),
        "{}",
        String::from_utf8_lossy(&recovered.stderr)
    );
    let recovery_json: serde_json::Value = serde_json::from_slice(&recovered.stdout).unwrap();
    assert_eq!(recovery_json["action"], "recover-release");
    assert_eq!(recovery_json["recovered"], true);
    assert_eq!(recovery_json["resumed"], true);
    assert_eq!(
        fs::read(application.join("host.exe")).unwrap(),
        b"old-host-binary"
    );
    assert_eq!(
        fs::read(data.join("preferences.json")).unwrap(),
        b"old-compatible-state"
    );
    assert!(!journal_path.exists());
}

#[test]
fn headless_cli_inspects_applies_reapplies_and_restores_legacy_layout() {
    let tree = TempTree::new();
    let data = tree.path("data");
    fs::create_dir_all(&data).unwrap();

    let mut legacy_layout: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/layout_schema_v0.json")).unwrap();
    legacy_layout["checksum"] = json!(checksum_layout_json(&legacy_layout.to_string()).unwrap());
    let original = serde_json::to_vec(&legacy_layout).unwrap();
    fs::write(data.join("workspace.json"), &original).unwrap();

    let manifest_path = tree.path("upgrade-manifest.json");
    fs::write(
        &manifest_path,
        r#"{
          "schema_version": 1,
          "stores": [{ "relative_path": "workspace.json", "format": "layout" }]
        }"#,
    )
    .unwrap();
    let data_arg = data.to_string_lossy().into_owned();
    let manifest_arg = manifest_path.to_string_lossy().into_owned();

    let inspect = run_cli(&[
        "inspect",
        "--data-root",
        &data_arg,
        "--manifest",
        &manifest_arg,
    ]);
    assert!(
        inspect.status.success(),
        "{}",
        String::from_utf8_lossy(&inspect.stderr)
    );
    let inspect_json: serde_json::Value = serde_json::from_slice(&inspect.stdout).unwrap();
    assert_eq!(inspect_json["action"], "inspect");
    assert_eq!(inspect_json["migration_count"], 1);
    assert_eq!(fs::read(data.join("workspace.json")).unwrap(), original);

    let checkpoint_path = tree.path("checkpoint");
    let checkpoint_arg = checkpoint_path.to_string_lossy().into_owned();
    let apply = run_cli(&[
        "apply",
        "--data-root",
        &data_arg,
        "--manifest",
        &manifest_arg,
        "--checkpoint",
        &checkpoint_arg,
    ]);
    assert!(
        apply.status.success(),
        "{}",
        String::from_utf8_lossy(&apply.stderr)
    );
    let apply_json: serde_json::Value = serde_json::from_slice(&apply.stdout).unwrap();
    assert_eq!(apply_json["action"], "apply");
    assert_eq!(apply_json["migration_count"], 1);
    assert_eq!(apply_json["applied_paths"][0], "workspace.json");
    assert!(checkpoint_path.exists());
    let migrated: serde_json::Value =
        serde_json::from_slice(&fs::read(data.join("workspace.json")).unwrap()).unwrap();
    assert_eq!(migrated["schema_version"], 2);

    let inspect_current = run_cli(&[
        "inspect",
        "--data-root",
        &data_arg,
        "--manifest",
        &manifest_arg,
    ]);
    assert!(inspect_current.status.success());
    let current_json: serde_json::Value = serde_json::from_slice(&inspect_current.stdout).unwrap();
    assert_eq!(current_json["migration_count"], 0);

    let retry_checkpoint = tree.path("retry-checkpoint");
    let retry_checkpoint_arg = retry_checkpoint.to_string_lossy().into_owned();
    let retry = run_cli(&[
        "apply",
        "--data-root",
        &data_arg,
        "--manifest",
        &manifest_arg,
        "--checkpoint",
        &retry_checkpoint_arg,
    ]);
    assert!(
        retry.status.success(),
        "{}",
        String::from_utf8_lossy(&retry.stderr)
    );
    let retry_json: serde_json::Value = serde_json::from_slice(&retry.stdout).unwrap();
    assert_eq!(retry_json["already_current"], true);
    assert!(!retry_checkpoint.exists());

    let restore = run_cli(&[
        "restore",
        "--data-root",
        &data_arg,
        "--checkpoint",
        &checkpoint_arg,
    ]);
    assert!(
        restore.status.success(),
        "{}",
        String::from_utf8_lossy(&restore.stderr)
    );
    assert_eq!(fs::read(data.join("workspace.json")).unwrap(), original);
    assert!(restore.status.success());
}

#[test]
fn generic_cli_rejects_mixed_extension_manifest_without_partial_upgrade() {
    let tree = TempTree::new();
    let data = tree.path("data");
    fs::create_dir_all(&data).unwrap();

    let mut legacy_layout: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/layout_schema_v0.json")).unwrap();
    legacy_layout["checksum"] = json!(checksum_layout_json(&legacy_layout.to_string()).unwrap());
    let layout_source = serde_json::to_vec(&legacy_layout).unwrap();
    fs::write(data.join("workspace.json"), &layout_source).unwrap();

    let mut preferences = json!({
        "schema_version": 1,
        "workspace_id": "host",
        "preferences": {
            "extensions": {
                "sample:host-state": {
                    "schema_version": 1,
                    "data": { "value": 4 }
                }
            }
        }
    });
    preferences["checksum"] = json!(checksum_layout_json(&preferences.to_string()).unwrap());
    let extension_source = serde_json::to_vec(&preferences).unwrap();
    fs::write(data.join("preferences.json"), &extension_source).unwrap();

    let manifest_path = tree.path("upgrade-manifest.json");
    fs::write(
        &manifest_path,
        r#"{
          "schema_version": 1,
          "stores": [
            { "relative_path": "workspace.json", "format": "layout" },
            { "relative_path": "preferences.json", "format": "extension_state" }
          ]
        }"#,
    )
    .unwrap();
    let data_root = data.to_string_lossy().into_owned();
    let manifest_arg = manifest_path.to_string_lossy().into_owned();
    let inspect = run_cli(&[
        "inspect",
        "--data-root",
        &data_root,
        "--manifest",
        &manifest_arg,
    ]);
    assert!(!inspect.status.success());
    let inspect_stderr = String::from_utf8_lossy(&inspect.stderr);
    assert!(
        inspect_stderr.contains("activated ExtensionRegistry"),
        "unexpected inspect diagnostic: {inspect_stderr}"
    );

    let checkpoint_path = tree.path("checkpoint");
    let checkpoint_arg = checkpoint_path.to_string_lossy().into_owned();
    let apply = run_cli(&[
        "apply",
        "--data-root",
        &data_root,
        "--manifest",
        &manifest_arg,
        "--checkpoint",
        &checkpoint_arg,
    ]);
    assert!(!apply.status.success());
    let apply_stderr = String::from_utf8_lossy(&apply.stderr);
    assert!(
        apply_stderr.contains("activated ExtensionRegistry"),
        "unexpected apply diagnostic: {apply_stderr}"
    );
    assert_eq!(
        fs::read(data.join("workspace.json")).unwrap(),
        layout_source
    );
    assert_eq!(
        fs::read(data.join("preferences.json")).unwrap(),
        extension_source
    );
    assert!(!checkpoint_path.exists());
}

#[test]
fn generic_cli_rejects_corrupt_layout_without_rewriting_or_checkpointing() {
    let tree = TempTree::new();
    let data = tree.path("data");
    fs::create_dir_all(&data).unwrap();
    let mut corrupt: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/layout_schema_v0.json")).unwrap();
    corrupt["checksum"] = json!(checksum_layout_json(&corrupt.to_string()).unwrap());
    corrupt["layout_id"] = json!("tampered-after-checksum");
    let original = serde_json::to_vec(&corrupt).unwrap();
    fs::write(data.join("workspace.json"), &original).unwrap();

    let manifest_path = tree.path("upgrade-manifest.json");
    fs::write(
        &manifest_path,
        r#"{
          "schema_version": 1,
          "stores": [{ "relative_path": "workspace.json", "format": "layout" }]
        }"#,
    )
    .unwrap();
    let data_arg = data.to_string_lossy().into_owned();
    let manifest_arg = manifest_path.to_string_lossy().into_owned();
    let inspect = run_cli(&[
        "inspect",
        "--data-root",
        &data_arg,
        "--manifest",
        &manifest_arg,
    ]);
    assert!(!inspect.status.success());
    assert!(String::from_utf8_lossy(&inspect.stderr).contains("checksum"));
    assert_eq!(fs::read(data.join("workspace.json")).unwrap(), original);

    let checkpoint_path = tree.path("checkpoint");
    let checkpoint_arg = checkpoint_path.to_string_lossy().into_owned();
    let apply = run_cli(&[
        "apply",
        "--data-root",
        &data_arg,
        "--manifest",
        &manifest_arg,
        "--checkpoint",
        &checkpoint_arg,
    ]);
    assert!(!apply.status.success());
    assert_eq!(fs::read(data.join("workspace.json")).unwrap(), original);
    assert!(!checkpoint_path.exists());
}
