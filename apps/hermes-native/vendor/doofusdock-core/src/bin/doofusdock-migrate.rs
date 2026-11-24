use doofusdock_core::{
    UpgradeCheckpointLimits, UpgradeManifest, UpgradeManifestApplyReport, UpgradePlan,
    UpgradeReleaseCheckpointReport, UpgradeReleaseRestoreReport, UpgradeRestoreReport,
    UpgradeWriterLease, apply_upgrade_manifest, create_upgrade_release_checkpoint,
    inspect_upgrade_manifest, recover_upgrade_release_restore, recover_upgrade_restore,
    restore_upgrade_checkpoint, restore_upgrade_release_checkpoint,
    try_acquire_release_restore_writer_lease,
};
use serde_json::{Value, json};
use std::{collections::BTreeMap, env, fs, path::PathBuf, process};

const MAX_MANIFEST_BYTES: u64 = 1024 * 1024;

fn main() {
    match run() {
        Ok(output) => println!("{output}"),
        Err(error) => {
            eprintln!("doofusdock-migrate: {error}");
            process::exit(2);
        }
    }
}

fn run() -> Result<String, String> {
    let mut args = env::args().skip(1);
    let action = args.next().ok_or_else(usage)?;
    if action == "--help" || action == "help" {
        return Ok(usage());
    }
    if ![
        "inspect",
        "apply",
        "restore",
        "recover",
        "checkpoint-release",
        "restore-release",
        "recover-release",
    ]
    .contains(&action.as_str())
    {
        return Err(format!("unknown action `{action}`\n{}", usage()));
    }

    let mut options = BTreeMap::<String, PathBuf>::new();
    while let Some(name) = args.next() {
        if !name.starts_with("--") {
            return Err(format!("expected an option name, received `{name}`"));
        }
        let value = args
            .next()
            .ok_or_else(|| format!("option `{name}` requires a path value"))?;
        if options.insert(name.clone(), PathBuf::from(value)).is_some() {
            return Err(format!("option `{name}` was supplied more than once"));
        }
    }

    match action.as_str() {
        "inspect" => {
            require_options(&options, &["--data-root", "--manifest"])?;
            let root = option(&options, "--data-root")?;
            let manifest = read_manifest(option(&options, "--manifest")?)?;
            let plan =
                inspect_upgrade_manifest(root, &manifest, UpgradeCheckpointLimits::default())
                    .map_err(|error| format!("inspection failed: {error}"))?;
            serde_json::to_string_pretty(&plan_summary(&plan))
                .map_err(|error| format!("serialize inspection summary: {error}"))
        }
        "apply" => {
            require_options(&options, &["--data-root", "--manifest", "--checkpoint"])?;
            let root = option(&options, "--data-root")?;
            let _writer_lease = UpgradeWriterLease::try_acquire(root)
                .map_err(|error| format!("upgrade writer coordination failed: {error}"))?;
            let checkpoint = option(&options, "--checkpoint")?;
            let manifest = read_manifest(option(&options, "--manifest")?)?;
            let report = apply_upgrade_manifest(
                root,
                checkpoint,
                &manifest,
                UpgradeCheckpointLimits::default(),
            )
            .map_err(|error| format!("upgrade apply failed: {error}"))?;
            serde_json::to_string_pretty(&apply_summary(&report))
                .map_err(|error| format!("serialize apply summary: {error}"))
        }
        "restore" => {
            require_options(&options, &["--data-root", "--checkpoint"])?;
            let _writer_lease =
                UpgradeWriterLease::try_acquire(option(&options, "--data-root")?)
                    .map_err(|error| format!("upgrade writer coordination failed: {error}"))?;
            let report = restore_upgrade_checkpoint(
                option(&options, "--checkpoint")?,
                option(&options, "--data-root")?,
                UpgradeCheckpointLimits::default(),
            )
            .map_err(|error| format!("data restore failed: {error}"))?;
            serde_json::to_string_pretty(&restore_summary(&report))
                .map_err(|error| format!("serialize restore summary: {error}"))
        }
        "recover" => {
            require_options(&options, &["--data-root"])?;
            let _writer_lease =
                UpgradeWriterLease::try_acquire(option(&options, "--data-root")?)
                    .map_err(|error| format!("upgrade writer coordination failed: {error}"))?;
            let report = recover_upgrade_restore(
                option(&options, "--data-root")?,
                UpgradeCheckpointLimits::default(),
            )
            .map_err(|error| format!("restore recovery failed: {error}"))?;
            let value = report
                .as_ref()
                .map(restore_summary)
                .unwrap_or_else(|| json!({ "recovered": false, "reason": "no pending restore" }));
            serde_json::to_string_pretty(&value)
                .map_err(|error| format!("serialize recovery summary: {error}"))
        }
        "checkpoint-release" => {
            require_options(
                &options,
                &["--application-root", "--data-root", "--checkpoint"],
            )?;
            let _writer_lease =
                UpgradeWriterLease::try_acquire(option(&options, "--data-root")?)
                    .map_err(|error| format!("upgrade writer coordination failed: {error}"))?;
            let report = create_upgrade_release_checkpoint(
                option(&options, "--application-root")?,
                option(&options, "--data-root")?,
                option(&options, "--checkpoint")?,
                UpgradeCheckpointLimits::default(),
            )
            .map_err(|error| format!("release checkpoint failed: {error}"))?;
            serde_json::to_string_pretty(&release_checkpoint_summary(&report))
                .map_err(|error| format!("serialize release checkpoint summary: {error}"))
        }
        "restore-release" => {
            require_options(
                &options,
                &[
                    "--application-root",
                    "--data-root",
                    "--checkpoint",
                    "--journal",
                ],
            )?;
            let _writer_lease =
                UpgradeWriterLease::try_acquire(option(&options, "--data-root")?)
                    .map_err(|error| format!("upgrade writer coordination failed: {error}"))?;
            let report = restore_upgrade_release_checkpoint(
                option(&options, "--checkpoint")?,
                option(&options, "--application-root")?,
                option(&options, "--data-root")?,
                option(&options, "--journal")?,
                UpgradeCheckpointLimits::default(),
            )
            .map_err(|error| format!("release restore failed: {error}"))?;
            serde_json::to_string_pretty(&release_restore_summary(&report))
                .map_err(|error| format!("serialize release restore summary: {error}"))
        }
        "recover-release" => {
            require_options(&options, &["--journal"])?;
            let _writer_lease = try_acquire_release_restore_writer_lease(
                option(&options, "--journal")?,
                UpgradeCheckpointLimits::default(),
            )
            .map_err(|error| format!("upgrade writer coordination failed: {error}"))?;
            let report = recover_upgrade_release_restore(
                option(&options, "--journal")?,
                UpgradeCheckpointLimits::default(),
            )
            .map_err(|error| format!("release restore recovery failed: {error}"))?;
            let value = report
                .as_ref()
                .map(|report| {
                    let mut summary = release_restore_summary(report);
                    summary["action"] = json!("recover-release");
                    summary["recovered"] = json!(true);
                    summary
                })
                .unwrap_or_else(|| {
                    json!({
                        "action": "recover-release",
                        "recovered": false,
                        "reason": "no pending release restore"
                    })
                });
            serde_json::to_string_pretty(&value)
                .map_err(|error| format!("serialize release recovery summary: {error}"))
        }
        _ => unreachable!(),
    }
}

fn usage() -> String {
    "Usage:\n  doofusdock-migrate inspect --data-root PATH --manifest PATH\n  doofusdock-migrate apply --data-root PATH --manifest PATH --checkpoint PATH\n  doofusdock-migrate restore --data-root PATH --checkpoint PATH\n  doofusdock-migrate recover --data-root PATH\n  doofusdock-migrate checkpoint-release --application-root PATH --data-root PATH --checkpoint PATH\n  doofusdock-migrate restore-release --application-root PATH --data-root PATH --checkpoint PATH --journal PATH\n  doofusdock-migrate recover-release --journal PATH\n\nThe manifest is host-authored JSON listing relative persisted-store paths and formats. Inspect is read-only. Hosts should hold `UpgradeWriterLease` while writing their data root. Apply, restore, recovery, and release operations acquire that cooperative lease and fail with a conflict while a cooperating host is active; stop older or non-cooperating writers before running them. Apply checkpoints and atomically upgrades the listed stores. Restore replaces the selected data directory from a verified checkpoint and retains the replaced tree.".into()
}

fn release_checkpoint_summary(report: &UpgradeReleaseCheckpointReport) -> Value {
    json!({
        "action": "checkpoint-release",
        "checkpoint": report.checkpoint_path,
        "application_fingerprint": report.application.tree_fingerprint,
        "application_files": report.application.file_count,
        "data_fingerprint": report.data.tree_fingerprint,
        "data_files": report.data.file_count,
    })
}

fn release_restore_summary(report: &UpgradeReleaseRestoreReport) -> Value {
    json!({
        "action": "restore-release",
        "resumed": report.resumed,
        "application": restore_summary(&report.application),
        "data": restore_summary(&report.data),
    })
}

fn option<'a>(options: &'a BTreeMap<String, PathBuf>, name: &str) -> Result<&'a PathBuf, String> {
    options
        .get(name)
        .ok_or_else(|| format!("required option `{name}` is missing"))
}

fn require_options(options: &BTreeMap<String, PathBuf>, expected: &[&str]) -> Result<(), String> {
    for name in expected {
        option(options, name)?;
    }
    for name in options.keys() {
        if !expected.contains(&name.as_str()) {
            return Err(format!("option `{name}` is not supported for this action"));
        }
    }
    Ok(())
}

fn read_manifest(path: &std::path::Path) -> Result<UpgradeManifest, String> {
    let metadata = fs::metadata(path)
        .map_err(|error| format!("inspect manifest `{}`: {error}", path.display()))?;
    if metadata.len() > MAX_MANIFEST_BYTES {
        return Err(format!(
            "manifest `{}` exceeds the 1 MiB byte limit",
            path.display()
        ));
    }
    let bytes =
        fs::read(path).map_err(|error| format!("read manifest `{}`: {error}", path.display()))?;
    if bytes.len() as u64 > MAX_MANIFEST_BYTES {
        return Err(format!(
            "manifest `{}` grew beyond the 1 MiB byte limit while being read",
            path.display()
        ));
    }
    let text = std::str::from_utf8(&bytes)
        .map_err(|error| format!("manifest `{}` is not UTF-8 JSON: {error}", path.display()))?;
    UpgradeManifest::from_json(text)
        .map_err(|error| format!("parse manifest `{}`: {error}", path.display()))
}

fn plan_summary(plan: &UpgradePlan) -> Value {
    json!({
        "action": "inspect",
        "manifest_schema": plan.manifest_schema,
        "migration_count": plan.migration_count(),
        "stores": plan.stores,
    })
}

fn apply_summary(report: &UpgradeManifestApplyReport) -> Value {
    let application = report.application.as_ref();
    json!({
        "action": "apply",
        "migration_count": report.inspection.migration_count(),
        "stores": report.inspection.stores,
        "already_current": application.is_none(),
        "applied_paths": application.map(|value| &value.applied_paths),
        "source_checkpoint": application.and_then(|value| value.source_checkpoint_path.as_ref()),
        "target_tree_checkpoint": application.and_then(|value| value.target_tree_checkpoint_path.as_ref()),
        "retained_replaced_tree": application.and_then(|value| value.replaced_tree_path.as_ref()),
        "target_tree_fingerprint": application.map(|value| &value.target_tree_fingerprint),
    })
}

fn restore_summary(report: &UpgradeRestoreReport) -> Value {
    json!({
        "restored_path": report.restored_path,
        "retained_replaced_tree": report.replaced_tree_path,
        "file_count": report.file_count,
        "total_bytes": report.total_bytes,
    })
}
