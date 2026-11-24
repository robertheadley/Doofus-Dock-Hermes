//! Explicit whole-directory data checkpoints for host upgrade workflows.
//!
//! This module never discovers a host data directory or replaces application
//! files. The host supplies both paths and decides when to restore a verified
//! checkpoint.

use crate::{DockError, DockResult, ErrorCode, MigrationPreview};
use fs4::FileExt;
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Component, Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

const CHECKPOINT_MANIFEST: &str = "doofusdock-upgrade-checkpoint.json";
const CHECKPOINT_SCHEMA: u32 = 1;
const RESTORE_JOURNAL_SCHEMA: u32 = 1;
const RELEASE_CHECKPOINT_SCHEMA: u32 = 1;
const RELEASE_RESTORE_JOURNAL_SCHEMA: u32 = 1;
const RELEASE_CHECKPOINT_MANIFEST: &str = "doofusdock-release-checkpoint.json";
const MAX_RELEASE_JOURNAL_BYTES: usize = 128 * 1024;
const MAX_CHECKPOINT_FILES: usize = 100_000;
const MAX_CHECKPOINT_MANIFEST_BYTES: usize = 16 * 1024 * 1024;
const MAX_UPGRADE_MANIFEST_BYTES: usize = 1024 * 1024;
const MAX_UPGRADE_STORES: usize = 512;
static CHECKPOINT_SEQUENCE: AtomicU64 = AtomicU64::new(0);

/// Cooperative exclusive lease for a host's persisted-data directory.
///
/// Hosts should hold this guard for as long as they can write the data tree.
/// Migration, restore, and checkpoint commands acquire the same lease before
/// mutating state. The lock file lives next to the data root, outside the tree
/// being checkpointed or atomically replaced.
#[derive(Debug)]
pub struct UpgradeWriterLease {
    _file: File,
    data_root: PathBuf,
    lock_path: PathBuf,
}

impl UpgradeWriterLease {
    /// Acquire the host-writer lease for an existing, real data directory.
    /// A competing process receives `Conflict`; the OS releases the lease if
    /// this process exits unexpectedly.
    pub fn try_acquire(data_root: &Path) -> DockResult<Self> {
        let metadata = fs::symlink_metadata(data_root)
            .map_err(|error| fs_error("Inspect upgrade writer data root", error, data_root))?;
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            return Err(upgrade_error(
                ErrorCode::UnsupportedOperation,
                "Upgrade writer lease requires a real directory, not a symlink or file.",
                data_root.display().to_string(),
            ));
        }
        let root = fs::canonicalize(data_root)
            .map_err(|error| fs_error("Resolve upgrade writer data root", error, data_root))?;
        let lock_path = upgrade_writer_lock_path(&root)?;
        if let Ok(metadata) = fs::symlink_metadata(&lock_path)
            && (metadata.file_type().is_symlink() || !metadata.is_file())
        {
            return Err(upgrade_error(
                ErrorCode::UnsupportedOperation,
                "Upgrade writer lock path must be a regular file.",
                lock_path.display().to_string(),
            ));
        }
        let file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(&lock_path)
            .map_err(|error| fs_error("Open upgrade writer lock", error, &lock_path))?;
        match FileExt::try_lock(&file) {
            Ok(()) => Ok(Self {
                _file: file,
                data_root: root,
                lock_path,
            }),
            Err(fs4::TryLockError::WouldBlock) => Err(upgrade_error(
                ErrorCode::Conflict,
                "Another cooperating DoofusDock host or upgrade process holds the data writer lease.",
                lock_path.display().to_string(),
            )),
            Err(fs4::TryLockError::Error(error)) => {
                Err(fs_error("Acquire upgrade writer lease", error, &lock_path))
            }
        }
    }

    pub fn data_root(&self) -> &Path {
        &self.data_root
    }

    /// Returns the sidecar lock path used for this canonical data root.
    pub fn lock_path(&self) -> &Path {
        &self.lock_path
    }
}

fn upgrade_writer_lock_path(root: &Path) -> DockResult<PathBuf> {
    let parent = root.parent().ok_or_else(|| {
        upgrade_error(
            ErrorCode::InvalidId,
            "Upgrade data root has no parent for its writer lock.",
            root.display().to_string(),
        )
    })?;
    #[cfg(windows)]
    let identity = root.to_string_lossy().to_lowercase();
    #[cfg(not(windows))]
    let identity = root.to_string_lossy().into_owned();
    let fingerprint = identity
        .as_bytes()
        .iter()
        .fold(0xcbf29ce484222325u64, |hash, byte| {
            (hash ^ u64::from(*byte)).wrapping_mul(0x100000001b3)
        });
    Ok(parent.join(format!(".doofusdock-writer-{fingerprint:016x}.lock")))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UpgradeCheckpointLimits {
    pub maximum_files: usize,
    pub maximum_total_bytes: u64,
    pub maximum_file_bytes: u64,
}

impl Default for UpgradeCheckpointLimits {
    fn default() -> Self {
        Self {
            maximum_files: MAX_CHECKPOINT_FILES,
            maximum_total_bytes: 16 * 1024 * 1024 * 1024,
            maximum_file_bytes: 2 * 1024 * 1024 * 1024,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UpgradeCheckpointReport {
    pub checkpoint_path: PathBuf,
    pub file_count: usize,
    pub directory_count: usize,
    pub total_bytes: u64,
    pub tree_fingerprint: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UpgradeRestoreReport {
    pub restored_path: PathBuf,
    /// If the destination existed, its contents are retained here for audit or
    /// a subsequent forward recovery. The caller may remove it after acceptance.
    pub replaced_tree_path: Option<PathBuf>,
    pub file_count: usize,
    pub total_bytes: u64,
}

/// A matched checkpoint of the installed application tree and its user data.
/// Both trees are verified and published together under one bundle directory.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UpgradeReleaseCheckpointReport {
    pub checkpoint_path: PathBuf,
    pub application: UpgradeCheckpointReport,
    pub data: UpgradeCheckpointReport,
}

/// Result of restoring both application and user data from one release bundle.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UpgradeReleaseRestoreReport {
    pub application: UpgradeRestoreReport,
    pub data: UpgradeRestoreReport,
    /// True when this call resumed an existing durable release-restore journal.
    pub resumed: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UpgradeFileChange {
    /// Safe path relative to the host-selected data directory.
    pub relative_path: String,
    /// Read-only framework preview produced from the current source file.
    pub preview: MigrationPreview,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UpgradeApplyReport {
    pub source_checkpoint_path: Option<PathBuf>,
    pub replaced_tree_path: Option<PathBuf>,
    pub target_tree_checkpoint_path: Option<PathBuf>,
    pub applied_paths: Vec<String>,
    pub target_tree_fingerprint: String,
    pub already_applied: bool,
}

/// Host-owned inventory of persisted stores that the framework may inspect.
/// Paths are relative to an explicitly supplied data root.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpgradeManifest {
    pub schema_version: u32,
    pub stores: Vec<UpgradeStoreSpec>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpgradeStoreSpec {
    pub relative_path: String,
    pub format: crate::PersistedDataFormat,
    #[serde(default)]
    pub optional: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpgradeStoreInspection {
    pub relative_path: String,
    pub format: crate::PersistedDataFormat,
    pub present: bool,
    pub source_version: Option<u64>,
    pub target_version: Option<u64>,
    pub requires_migration: bool,
    pub source_fingerprint: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UpgradePlan {
    pub manifest_schema: u32,
    pub stores: Vec<UpgradeStoreInspection>,
    changes: Vec<UpgradeFileChange>,
}

impl UpgradePlan {
    pub fn migration_count(&self) -> usize {
        self.changes.len()
    }

    pub fn changes(&self) -> &[UpgradeFileChange] {
        &self.changes
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UpgradeManifestApplyReport {
    pub inspection: UpgradePlan,
    pub application: Option<UpgradeApplyReport>,
}

impl UpgradeManifest {
    pub const CURRENT_SCHEMA: u32 = 1;

    pub fn from_json(json: &str) -> DockResult<Self> {
        if json.len() > MAX_UPGRADE_MANIFEST_BYTES {
            return Err(upgrade_error(
                ErrorCode::ResourceLimit,
                "Upgrade manifest exceeds the configured byte limit.",
                json.len().to_string(),
            ));
        }
        let manifest: Self = serde_json::from_str(json).map_err(|error| {
            upgrade_error(
                ErrorCode::SerializationError,
                "Upgrade manifest is malformed or contains unsupported fields.",
                error.to_string(),
            )
        })?;
        manifest.validate()?;
        Ok(manifest)
    }

    pub fn to_json(&self) -> DockResult<String> {
        self.validate()?;
        serde_json::to_string_pretty(self).map_err(|error| {
            upgrade_error(
                ErrorCode::SerializationError,
                "Upgrade manifest serialization failed.",
                error.to_string(),
            )
        })
    }

    pub fn validate(&self) -> DockResult<()> {
        if self.schema_version != Self::CURRENT_SCHEMA {
            return Err(upgrade_error(
                ErrorCode::SchemaError,
                "Unsupported upgrade manifest schema version.",
                self.schema_version.to_string(),
            ));
        }
        if self.stores.is_empty() || self.stores.len() > MAX_UPGRADE_STORES {
            return Err(upgrade_error(
                ErrorCode::ResourceLimit,
                "Upgrade manifest must list between one and 512 persisted stores.",
                self.stores.len().to_string(),
            ));
        }
        let mut seen: Vec<String> = Vec::with_capacity(self.stores.len());
        for store in &self.stores {
            let path = safe_relative_path(&store.relative_path)?;
            let normalized = normalized_upgrade_path(&path);
            if upgrade_paths_equal(&normalized, CHECKPOINT_MANIFEST)
                || seen
                    .iter()
                    .any(|existing| upgrade_paths_equal(existing, &normalized))
            {
                return Err(upgrade_error(
                    ErrorCode::InvalidId,
                    "Upgrade manifest contains a reserved or duplicate path.",
                    normalized,
                ));
            }
            seen.push(normalized);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct CheckpointManifest {
    schema_version: u32,
    files: Vec<CheckpointFile>,
    directories: Vec<String>,
    total_bytes: u64,
    tree_fingerprint: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct CheckpointFile {
    relative_path: String,
    byte_length: u64,
    fingerprint: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ReleaseCheckpointManifest {
    schema_version: u32,
    application_fingerprint: String,
    data_fingerprint: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum ReleaseRestorePhase {
    Prepared,
    ApplicationRestored,
    DataRestored,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ReleaseRestoreJournal {
    schema_version: u32,
    checkpoint_root: PathBuf,
    application_destination: PathBuf,
    data_destination: PathBuf,
    application_replaced_tree: Option<PathBuf>,
    data_replaced_tree: Option<PathBuf>,
    phase: ReleaseRestorePhase,
    checksum: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum RestorePhase {
    Prepared,
    DestinationMoved,
    RestorePublished,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct RestoreJournal {
    schema_version: u32,
    checkpoint_root: PathBuf,
    destination_root: PathBuf,
    staging_root: PathBuf,
    replaced_root: Option<PathBuf>,
    tree_fingerprint: String,
    file_count: usize,
    total_bytes: u64,
    phase: RestorePhase,
}

/// Copy a complete data tree to a new checkpoint directory.
///
/// The checkpoint path must not exist. Source symlinks and special files are
/// rejected. Files are bounded, synced, verified in staging, and the staged
/// directory is published with a same-parent rename.
pub fn create_upgrade_checkpoint(
    source_root: &Path,
    checkpoint_path: &Path,
    limits: UpgradeCheckpointLimits,
) -> DockResult<UpgradeCheckpointReport> {
    validate_limits(limits)?;
    let source = fs::canonicalize(source_root)
        .map_err(|error| fs_error("Resolve checkpoint source", error, source_root))?;
    if !source.is_dir() {
        return Err(upgrade_error(
            ErrorCode::PersistenceError,
            "Checkpoint source is not a directory.",
            source.display().to_string(),
        ));
    }
    let (checkpoint_parent, checkpoint_final) = resolve_new_directory(checkpoint_path)?;
    if checkpoint_final.starts_with(&source) || checkpoint_parent.starts_with(&source) {
        return Err(upgrade_error(
            ErrorCode::InvalidId,
            "Checkpoint destination cannot be inside its source tree.",
            checkpoint_final.display().to_string(),
        ));
    }
    if checkpoint_final == source {
        return Err(upgrade_error(
            ErrorCode::InvalidId,
            "Checkpoint source and destination must differ.",
            source.display().to_string(),
        ));
    }
    if path_entry_exists(&checkpoint_final) {
        return Err(upgrade_error(
            ErrorCode::Conflict,
            "Checkpoint destination already exists; existing checkpoints are never overwritten.",
            checkpoint_final.display().to_string(),
        ));
    }
    let staging = unique_sibling(&checkpoint_parent, "doofusdock-checkpoint-stage")?;
    fs::create_dir(&staging)
        .map_err(|error| fs_error("Create checkpoint staging directory", error, &staging))?;
    let result = (|| {
        let mut manifest = CheckpointManifest {
            schema_version: CHECKPOINT_SCHEMA,
            files: Vec::new(),
            directories: Vec::new(),
            total_bytes: 0,
            tree_fingerprint: String::new(),
        };
        copy_tree_into(&source, &source, &staging, limits, &mut manifest)?;
        manifest
            .files
            .sort_by(|left, right| left.relative_path.cmp(&right.relative_path));
        manifest.directories.sort();
        let source_after_copy = inventory_tree(&source, limits)?;
        if source_after_copy.files != manifest.files
            || source_after_copy.directories != manifest.directories
            || source_after_copy.total_bytes != manifest.total_bytes
        {
            return Err(upgrade_error(
                ErrorCode::Conflict,
                "Checkpoint source changed while it was being copied; no checkpoint was published.",
                source.display().to_string(),
            ));
        }
        manifest.tree_fingerprint = manifest_fingerprint(&manifest);
        write_manifest(&staging, &manifest)?;
        verify_checkpoint_tree(&staging, limits)?;
        fs::rename(&staging, &checkpoint_final)
            .map_err(|error| fs_error("Publish verified checkpoint", error, &checkpoint_final))?;
        Ok(report(&checkpoint_final, &manifest))
    })();
    if result.is_err() {
        let _ = fs::remove_dir_all(&staging);
    }
    result
}

/// Restore a verified checkpoint into an explicitly chosen data directory.
///
/// Replacement is staged beside the destination. An existing destination is
/// first renamed to a unique sibling and retained after successful restore.
/// If publishing the staged tree fails, the previous destination is restored.
pub fn restore_upgrade_checkpoint(
    checkpoint_path: &Path,
    destination_root: &Path,
    limits: UpgradeCheckpointLimits,
) -> DockResult<UpgradeRestoreReport> {
    validate_limits(limits)?;
    let checkpoint = fs::canonicalize(checkpoint_path)
        .map_err(|error| fs_error("Resolve upgrade checkpoint", error, checkpoint_path))?;
    let (destination_parent, destination_final) = resolve_destination(destination_root)?;
    if destination_final == checkpoint
        || destination_final.starts_with(&checkpoint)
        || checkpoint.starts_with(&destination_final)
    {
        return Err(upgrade_error(
            ErrorCode::InvalidId,
            "Checkpoint and restore destination must be separate directory trees.",
            format!(
                "checkpoint={};destination={}",
                checkpoint.display(),
                destination_final.display()
            ),
        ));
    }
    let manifest = verify_checkpoint_tree(&checkpoint, limits)?;
    let staging = unique_sibling(&destination_parent, "doofusdock-restore-stage")?;
    fs::create_dir(&staging)
        .map_err(|error| fs_error("Create restore staging directory", error, &staging))?;
    let restore_result = (|| {
        for directory in &manifest.directories {
            let relative = safe_relative_path(directory)?;
            let target = staging.join(relative);
            fs::create_dir_all(&target)
                .map_err(|error| fs_error("Create restored directory", error, &target))?;
        }
        for entry in &manifest.files {
            let relative = safe_relative_path(&entry.relative_path)?;
            let source = checkpoint.join(&relative);
            let target = staging.join(&relative);
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent)
                    .map_err(|error| fs_error("Create restored parent", error, parent))?;
            }
            copy_verified_file(&source, &target, entry)?;
        }
        let staged = inventory_tree(&staging, limits)?;
        if staged.files != manifest.files || staged.total_bytes != manifest.total_bytes {
            return Err(upgrade_error(
                ErrorCode::SerializationError,
                "Staged restore does not match the checkpoint manifest.",
                staging.display().to_string(),
            ));
        }
        Ok(())
    })();
    if let Err(error) = restore_result {
        let _ = fs::remove_dir_all(&staging);
        return Err(error);
    }

    if path_entry_exists(&destination_final) {
        let metadata = fs::symlink_metadata(&destination_final)
            .map_err(|error| fs_error("Inspect restore destination", error, &destination_final))?;
        if !metadata.is_dir() || metadata.file_type().is_symlink() {
            let _ = fs::remove_dir_all(&staging);
            return Err(upgrade_error(
                ErrorCode::InvalidId,
                "Restore destination must be a regular directory.",
                destination_final.display().to_string(),
            ));
        }
    }
    let journal_path = restore_journal_path(&destination_final)?;
    if path_entry_exists(&journal_path) {
        let _ = fs::remove_dir_all(&staging);
        return Err(upgrade_error(
            ErrorCode::Conflict,
            "An unfinished restore journal exists; call recover_upgrade_restore before starting another restore.",
            journal_path.display().to_string(),
        ));
    }
    let replaced = if path_entry_exists(&destination_final) {
        Some(unique_sibling(
            &destination_parent,
            "doofusdock-replaced-data",
        )?)
    } else {
        None
    };
    let journal = RestoreJournal {
        schema_version: RESTORE_JOURNAL_SCHEMA,
        checkpoint_root: checkpoint,
        destination_root: destination_final,
        staging_root: staging,
        replaced_root: replaced,
        tree_fingerprint: manifest.tree_fingerprint,
        file_count: manifest.files.len(),
        total_bytes: manifest.total_bytes,
        phase: RestorePhase::Prepared,
    };
    if let Err(error) = write_restore_journal(&journal_path, &journal) {
        let _ = fs::remove_dir_all(&journal.staging_root);
        return Err(error);
    }
    finish_restore_from_journal(&journal_path, journal, limits)
}

/// Resume a directory restore after process interruption. The journal and
/// checkpoint are verified before any rename. If the restored destination is
/// already complete, recovery only clears the journal and reports success.
pub fn recover_upgrade_restore(
    destination_root: &Path,
    limits: UpgradeCheckpointLimits,
) -> DockResult<Option<UpgradeRestoreReport>> {
    validate_limits(limits)?;
    let (parent, destination) = resolve_destination(destination_root)?;
    let journal_path = restore_journal_path(&destination)?;
    if !path_entry_exists(&journal_path) {
        return Ok(None);
    }
    let journal = read_restore_journal(&journal_path, &parent, &destination)?;
    let report = finish_restore_from_journal(&journal_path, journal, limits)?;
    Ok(Some(report))
}

/// Checkpoint the application installation and its user data into one
/// immutable, integrity-verified release bundle. Both source roots must be
/// separate regular directory trees, and the bundle must live outside them.
pub fn create_upgrade_release_checkpoint(
    application_root: &Path,
    data_root: &Path,
    checkpoint_path: &Path,
    limits: UpgradeCheckpointLimits,
) -> DockResult<UpgradeReleaseCheckpointReport> {
    validate_limits(limits)?;
    let application = fs::canonicalize(application_root).map_err(|error| {
        fs_error(
            "Resolve application checkpoint source",
            error,
            application_root,
        )
    })?;
    let data = fs::canonicalize(data_root)
        .map_err(|error| fs_error("Resolve data checkpoint source", error, data_root))?;
    validate_separate_roots(&application, &data)?;
    for root in [&application, &data] {
        let metadata = fs::symlink_metadata(root)
            .map_err(|error| fs_error("Inspect release checkpoint source", error, root))?;
        if !metadata.is_dir() || metadata.file_type().is_symlink() {
            return Err(upgrade_error(
                ErrorCode::UnsupportedOperation,
                "Release checkpoint sources must be regular directories.",
                root.display().to_string(),
            ));
        }
    }

    let (parent, final_path) = resolve_new_directory(checkpoint_path)?;
    if final_path.starts_with(&application)
        || final_path.starts_with(&data)
        || application.starts_with(&final_path)
        || data.starts_with(&final_path)
    {
        return Err(upgrade_error(
            ErrorCode::InvalidId,
            "Release checkpoint must be separate from both source trees.",
            final_path.display().to_string(),
        ));
    }
    if path_entry_exists(&final_path) {
        return Err(upgrade_error(
            ErrorCode::Conflict,
            "Release checkpoint destination already exists; existing checkpoints are never overwritten.",
            final_path.display().to_string(),
        ));
    }

    let staging = unique_sibling(&parent, "doofusdock-release-checkpoint")?;
    fs::create_dir(&staging).map_err(|error| {
        fs_error(
            "Create release checkpoint staging directory",
            error,
            &staging,
        )
    })?;
    let staged_result = (|| {
        let application_checkpoint = staging.join("application");
        let data_checkpoint = staging.join("data");
        let application_report =
            create_upgrade_checkpoint(&application, &application_checkpoint, limits)?;
        let data_report = create_upgrade_checkpoint(&data, &data_checkpoint, limits)?;
        let bundle_manifest = ReleaseCheckpointManifest {
            schema_version: RELEASE_CHECKPOINT_SCHEMA,
            application_fingerprint: application_report.tree_fingerprint.clone(),
            data_fingerprint: data_report.tree_fingerprint.clone(),
        };
        write_release_checkpoint_manifest(&staging, &bundle_manifest)?;
        verify_release_checkpoint(&staging, limits)?;
        Ok((application_report, data_report))
    })();
    let (mut application_report, mut data_report) = match staged_result {
        Ok(reports) => reports,
        Err(error) => {
            let _ = fs::remove_dir_all(&staging);
            return Err(error);
        }
    };
    if let Err(error) = fs::rename(&staging, &final_path) {
        let _ = fs::remove_dir_all(&staging);
        return Err(fs_error(
            "Publish verified release checkpoint",
            error,
            &final_path,
        ));
    }
    application_report.checkpoint_path = final_path.join("application");
    data_report.checkpoint_path = final_path.join("data");

    Ok(UpgradeReleaseCheckpointReport {
        checkpoint_path: final_path.clone(),
        application: application_report,
        data: data_report,
    })
}

/// Restore the application directory and data directory from one release
/// bundle. A durable coordinator journal makes a retry complete the pair if
/// the process stops between either tree's independently journaled restore.
/// Stop the application and all writers before calling this function.
pub fn restore_upgrade_release_checkpoint(
    checkpoint_path: &Path,
    application_destination: &Path,
    data_destination: &Path,
    journal_path: &Path,
    limits: UpgradeCheckpointLimits,
) -> DockResult<UpgradeReleaseRestoreReport> {
    validate_limits(limits)?;
    let checkpoint = fs::canonicalize(checkpoint_path)
        .map_err(|error| fs_error("Resolve release checkpoint", error, checkpoint_path))?;
    verify_release_checkpoint(&checkpoint, limits)?;
    let (_, application) = resolve_destination(application_destination)?;
    let (_, data) = resolve_destination(data_destination)?;
    validate_separate_roots(&application, &data)?;
    validate_release_restore_paths(&checkpoint, &application, &data, journal_path)?;
    for destination in [&application, &data] {
        let local_journal = restore_journal_path(destination)?;
        if path_entry_exists(&local_journal) {
            return Err(upgrade_error(
                ErrorCode::Conflict,
                "An unfinished directory restore must be recovered before starting a release rollback.",
                local_journal.display().to_string(),
            ));
        }
    }
    let journal = ReleaseRestoreJournal {
        schema_version: RELEASE_RESTORE_JOURNAL_SCHEMA,
        checkpoint_root: checkpoint,
        application_destination: application,
        data_destination: data,
        application_replaced_tree: None,
        data_replaced_tree: None,
        phase: ReleaseRestorePhase::Prepared,
        checksum: String::new(),
    };
    let journal_file = resolve_journal_file(journal_path)?;
    if path_entry_exists(&journal_file) {
        return Err(upgrade_error(
            ErrorCode::Conflict,
            "Release rollback journal already exists; recover it instead of starting another rollback.",
            journal_file.display().to_string(),
        ));
    }
    write_release_restore_journal(&journal_file, &journal, false)?;
    finish_release_restore_journal(
        &journal_file,
        journal,
        limits,
        false,
        || Ok(()),
        || Ok(()),
        || Ok(()),
    )
}

/// Resume a release rollback after interruption. Each tree is compared with
/// its checkpoint before restore, so a completed first half is not applied a
/// second time and the second half proceeds on retry.
pub fn recover_upgrade_release_restore(
    journal_path: &Path,
    limits: UpgradeCheckpointLimits,
) -> DockResult<Option<UpgradeReleaseRestoreReport>> {
    validate_limits(limits)?;
    let journal_file = resolve_journal_file(journal_path)?;
    if !path_entry_exists(&journal_file) {
        return Ok(None);
    }
    let journal = read_release_restore_journal(&journal_file, limits)?;
    finish_release_restore_journal(
        &journal_file,
        journal,
        limits,
        true,
        || Ok(()),
        || Ok(()),
        || Ok(()),
    )
    .map(Some)
}

/// Acquire the data-tree writer lease recorded by a pending release-restore
/// journal. Returns `None` when no journal exists. This lets recovery tools
/// coordinate with a cooperating host without trusting a separately supplied
/// data-root argument.
pub fn try_acquire_release_restore_writer_lease(
    journal_path: &Path,
    limits: UpgradeCheckpointLimits,
) -> DockResult<Option<UpgradeWriterLease>> {
    validate_limits(limits)?;
    let journal_file = resolve_journal_file(journal_path)?;
    if !path_entry_exists(&journal_file) {
        return Ok(None);
    }
    let journal = read_release_restore_journal(&journal_file, limits)?;
    UpgradeWriterLease::try_acquire(&journal.data_destination).map(Some)
}

fn finish_release_restore_journal(
    journal_file: &Path,
    mut journal: ReleaseRestoreJournal,
    limits: UpgradeCheckpointLimits,
    resumed: bool,
    after_application: impl FnOnce() -> DockResult<()>,
    after_data: impl FnOnce() -> DockResult<()>,
    after_verification: impl FnOnce() -> DockResult<()>,
) -> DockResult<UpgradeReleaseRestoreReport> {
    let (application_manifest, data_manifest) =
        verify_release_checkpoint(&journal.checkpoint_root, limits)?;
    validate_release_restore_paths(
        &journal.checkpoint_root,
        &journal.application_destination,
        &journal.data_destination,
        journal_file,
    )?;

    if let Some(report) = recover_one_release_tree(
        &journal.checkpoint_root.join("application"),
        &journal.application_destination,
        &application_manifest,
        limits,
    )? {
        journal.application_replaced_tree = report.replaced_tree_path;
    }
    if journal.phase == ReleaseRestorePhase::Prepared {
        journal.phase = ReleaseRestorePhase::ApplicationRestored;
        write_release_restore_journal(journal_file, &journal, true)?;
    }
    after_application()?;

    if let Some(report) = recover_one_release_tree(
        &journal.checkpoint_root.join("data"),
        &journal.data_destination,
        &data_manifest,
        limits,
    )? {
        journal.data_replaced_tree = report.replaced_tree_path;
    }
    if journal.phase != ReleaseRestorePhase::DataRestored {
        journal.phase = ReleaseRestorePhase::DataRestored;
        write_release_restore_journal(journal_file, &journal, true)?;
    }
    after_data()?;
    if !tree_matches_manifest(
        &journal.application_destination,
        &application_manifest,
        limits,
    )? || !tree_matches_manifest(&journal.data_destination, &data_manifest, limits)?
    {
        return Err(upgrade_error(
            ErrorCode::PersistenceError,
            "Release rollback did not verify both restored trees; coordinator journal was retained.",
            journal_file.display().to_string(),
        ));
    }
    after_verification()?;
    fs::remove_file(journal_file).map_err(|error| {
        fs_error(
            "Remove completed release restore journal",
            error,
            journal_file,
        )
    })?;
    Ok(UpgradeReleaseRestoreReport {
        application: completed_tree_report(
            &journal.application_destination,
            &application_manifest,
            journal.application_replaced_tree,
        ),
        data: completed_tree_report(
            &journal.data_destination,
            &data_manifest,
            journal.data_replaced_tree,
        ),
        resumed,
    })
}

fn recover_one_release_tree(
    checkpoint: &Path,
    destination: &Path,
    expected: &CheckpointManifest,
    limits: UpgradeCheckpointLimits,
) -> DockResult<Option<UpgradeRestoreReport>> {
    let local_journal = restore_journal_path(destination)?;
    let mut restore_report = if path_entry_exists(&local_journal) {
        recover_upgrade_restore(destination, limits)?
    } else {
        None
    };
    if !tree_matches_manifest(destination, expected, limits)? {
        restore_report = Some(restore_upgrade_checkpoint(checkpoint, destination, limits)?);
    }
    if !tree_matches_manifest(destination, expected, limits)? {
        return Err(upgrade_error(
            ErrorCode::PersistenceError,
            "Release rollback tree does not match its verified checkpoint.",
            destination.display().to_string(),
        ));
    }
    Ok(restore_report)
}

fn completed_tree_report(
    destination: &Path,
    manifest: &CheckpointManifest,
    replaced_tree_path: Option<PathBuf>,
) -> UpgradeRestoreReport {
    UpgradeRestoreReport {
        restored_path: destination.to_path_buf(),
        replaced_tree_path,
        file_count: manifest.files.len(),
        total_bytes: manifest.total_bytes,
    }
}

fn write_release_checkpoint_manifest(
    root: &Path,
    manifest: &ReleaseCheckpointManifest,
) -> DockResult<()> {
    let path = root.join(RELEASE_CHECKPOINT_MANIFEST);
    let temporary = root.join(format!(
        "{RELEASE_CHECKPOINT_MANIFEST}.tmp-{}",
        std::process::id()
    ));
    let bytes = serde_json::to_vec(manifest).map_err(|error| {
        upgrade_error(
            ErrorCode::SerializationError,
            "Release checkpoint manifest serialization failed.",
            error.to_string(),
        )
    })?;
    let mut file = File::create(&temporary)
        .map_err(|error| fs_error("Create release checkpoint manifest", error, &temporary))?;
    if let Err(error) = file.write_all(&bytes).and_then(|()| file.sync_all()) {
        let _ = fs::remove_file(&temporary);
        return Err(fs_error(
            "Write release checkpoint manifest",
            error,
            &temporary,
        ));
    }
    fs::rename(&temporary, &path)
        .map_err(|error| fs_error("Publish release checkpoint manifest", error, &path))
}

fn verify_release_checkpoint(
    root: &Path,
    limits: UpgradeCheckpointLimits,
) -> DockResult<(CheckpointManifest, CheckpointManifest)> {
    let root = fs::canonicalize(root)
        .map_err(|error| fs_error("Resolve release checkpoint bundle", error, root))?;
    let metadata = fs::symlink_metadata(&root)
        .map_err(|error| fs_error("Inspect release checkpoint bundle", error, &root))?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err(upgrade_error(
            ErrorCode::UnsupportedOperation,
            "Release checkpoint bundle must be a regular directory.",
            root.display().to_string(),
        ));
    }
    let entries = fs::read_dir(&root)
        .map_err(|error| fs_error("Enumerate release checkpoint bundle", error, &root))?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| fs_error("Read release checkpoint entry", error, &root))?;
    if entries.len() != 3 {
        return Err(upgrade_error(
            ErrorCode::SerializationError,
            "Release checkpoint bundle must contain exactly its manifest and two trees.",
            root.display().to_string(),
        ));
    }
    let manifest_path = root.join(RELEASE_CHECKPOINT_MANIFEST);
    for path in &entries {
        let entry = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default();
        if !["application", "data", RELEASE_CHECKPOINT_MANIFEST].contains(&entry) {
            return Err(upgrade_error(
                ErrorCode::SerializationError,
                "Release checkpoint bundle contains an unexpected entry.",
                path.display().to_string(),
            ));
        }
        let metadata = fs::symlink_metadata(path)
            .map_err(|error| fs_error("Inspect release checkpoint entry", error, path))?;
        if metadata.file_type().is_symlink()
            || (entry == RELEASE_CHECKPOINT_MANIFEST && !metadata.is_file())
            || (entry != RELEASE_CHECKPOINT_MANIFEST && !metadata.is_dir())
        {
            return Err(upgrade_error(
                ErrorCode::UnsupportedOperation,
                "Release checkpoint bundle entries have invalid file types.",
                path.display().to_string(),
            ));
        }
    }
    let manifest_bytes = fs::read(&manifest_path)
        .map_err(|error| fs_error("Read release checkpoint manifest", error, &manifest_path))?;
    if manifest_bytes.len() > 64 * 1024 {
        return Err(upgrade_error(
            ErrorCode::ResourceLimit,
            "Release checkpoint manifest exceeds its byte limit.",
            manifest_bytes.len().to_string(),
        ));
    }
    let manifest: ReleaseCheckpointManifest =
        serde_json::from_slice(&manifest_bytes).map_err(|error| {
            upgrade_error(
                ErrorCode::SerializationError,
                "Release checkpoint manifest is invalid JSON or contains unsupported fields.",
                error.to_string(),
            )
        })?;
    if manifest.schema_version != RELEASE_CHECKPOINT_SCHEMA {
        return Err(upgrade_error(
            ErrorCode::SchemaError,
            "Unsupported release checkpoint manifest schema.",
            manifest.schema_version.to_string(),
        ));
    }
    let application = verify_checkpoint_tree(&root.join("application"), limits)?;
    let data = verify_checkpoint_tree(&root.join("data"), limits)?;
    if application.tree_fingerprint != manifest.application_fingerprint
        || data.tree_fingerprint != manifest.data_fingerprint
    {
        return Err(upgrade_error(
            ErrorCode::SerializationError,
            "Release checkpoint trees do not match their bundle manifest.",
            root.display().to_string(),
        ));
    }
    Ok((application, data))
}

fn validate_separate_roots(application: &Path, data: &Path) -> DockResult<()> {
    if application == data || application.starts_with(data) || data.starts_with(application) {
        return Err(upgrade_error(
            ErrorCode::InvalidId,
            "Application and data directories must be separate non-overlapping trees.",
            format!(
                "application={};data={}",
                application.display(),
                data.display()
            ),
        ));
    }
    Ok(())
}

fn resolve_journal_file(path: &Path) -> DockResult<PathBuf> {
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .ok_or_else(|| {
            upgrade_error(
                ErrorCode::InvalidId,
                "Journal path must include its parent directory.",
                path.display().to_string(),
            )
        })?;
    let name = path.file_name().ok_or_else(|| {
        upgrade_error(
            ErrorCode::InvalidId,
            "Journal path must have a file name.",
            path.display().to_string(),
        )
    })?;
    let parent = fs::canonicalize(parent)
        .map_err(|error| fs_error("Resolve release journal parent", error, parent))?;
    Ok(parent.join(name))
}

fn validate_release_restore_paths(
    checkpoint: &Path,
    application: &Path,
    data: &Path,
    journal_path: &Path,
) -> DockResult<()> {
    validate_separate_roots(application, data)?;
    if checkpoint.starts_with(application)
        || checkpoint.starts_with(data)
        || application.starts_with(checkpoint)
        || data.starts_with(checkpoint)
    {
        return Err(upgrade_error(
            ErrorCode::InvalidId,
            "Release checkpoint and restore destinations must be separate directory trees.",
            checkpoint.display().to_string(),
        ));
    }
    let journal = resolve_journal_file(journal_path)?;
    if journal.starts_with(checkpoint)
        || journal.starts_with(application)
        || journal.starts_with(data)
    {
        return Err(upgrade_error(
            ErrorCode::InvalidId,
            "Release restore journal must be outside all checkpoint and destination trees.",
            journal.display().to_string(),
        ));
    }
    Ok(())
}

fn release_journal_checksum(journal: &ReleaseRestoreJournal) -> DockResult<String> {
    let mut value = serde_json::to_value(journal).map_err(|error| {
        upgrade_error(
            ErrorCode::SerializationError,
            "Release journal serialization failed.",
            error.to_string(),
        )
    })?;
    value
        .as_object_mut()
        .expect("journal serializes to object")
        .remove("checksum");
    crate::checksum_layout_json(&value.to_string())
}

fn write_release_restore_journal(
    path: &Path,
    journal: &ReleaseRestoreJournal,
    replace_existing: bool,
) -> DockResult<()> {
    let mut journal = journal.clone();
    journal.checksum = release_journal_checksum(&journal)?;
    let bytes = serde_json::to_vec(&journal).map_err(|error| {
        upgrade_error(
            ErrorCode::SerializationError,
            "Release restore journal serialization failed.",
            error.to_string(),
        )
    })?;
    if bytes.len() > MAX_RELEASE_JOURNAL_BYTES {
        return Err(upgrade_error(
            ErrorCode::ResourceLimit,
            "Release restore journal exceeds its byte limit.",
            bytes.len().to_string(),
        ));
    }
    let parent = path.parent().ok_or_else(|| {
        upgrade_error(
            ErrorCode::InvalidId,
            "Release journal path has no parent.",
            path.display().to_string(),
        )
    })?;
    let temporary = unique_sibling(parent, "doofusdock-release-journal")?;
    let mut file = File::create(&temporary)
        .map_err(|error| fs_error("Create release journal staging file", error, &temporary))?;
    if let Err(error) = file.write_all(&bytes).and_then(|()| file.sync_all()) {
        let _ = fs::remove_file(&temporary);
        return Err(fs_error("Write release restore journal", error, &temporary));
    }
    let result = if replace_existing {
        crate::persistence::atomic_replace(&temporary, path)
    } else {
        crate::persistence::atomic_publish_new(&temporary, path)
    };
    if let Err(error) = result {
        let _ = fs::remove_file(&temporary);
        return Err(fs_error("Publish release restore journal", error, path));
    }
    Ok(())
}

fn read_release_restore_journal(
    path: &Path,
    limits: UpgradeCheckpointLimits,
) -> DockResult<ReleaseRestoreJournal> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| fs_error("Inspect release restore journal", error, path))?;
    if !metadata.is_file()
        || metadata.file_type().is_symlink()
        || metadata.len() > MAX_RELEASE_JOURNAL_BYTES as u64
    {
        return Err(upgrade_error(
            ErrorCode::UnsupportedOperation,
            "Release restore journal must be a bounded regular file.",
            path.display().to_string(),
        ));
    }
    let bytes =
        fs::read(path).map_err(|error| fs_error("Read release restore journal", error, path))?;
    let mut journal: ReleaseRestoreJournal = serde_json::from_slice(&bytes).map_err(|error| {
        upgrade_error(
            ErrorCode::SerializationError,
            "Release restore journal is invalid JSON or contains unsupported fields.",
            error.to_string(),
        )
    })?;
    if journal.schema_version != RELEASE_RESTORE_JOURNAL_SCHEMA
        || release_journal_checksum(&journal)? != journal.checksum
    {
        return Err(upgrade_error(
            ErrorCode::SerializationError,
            "Release restore journal schema or checksum is invalid.",
            path.display().to_string(),
        ));
    }
    let (application, data) =
        resolve_destination_pair(&journal.application_destination, &journal.data_destination)?;
    let checkpoint = fs::canonicalize(&journal.checkpoint_root).map_err(|error| {
        fs_error(
            "Resolve journal release checkpoint",
            error,
            &journal.checkpoint_root,
        )
    })?;
    verify_release_checkpoint(&checkpoint, limits)?;
    validate_release_restore_paths(&checkpoint, &application, &data, path)?;
    journal.checkpoint_root = checkpoint;
    journal.application_destination = application;
    journal.data_destination = data;
    Ok(journal)
}

fn resolve_destination_pair(application: &Path, data: &Path) -> DockResult<(PathBuf, PathBuf)> {
    let (_, application) = resolve_destination(application)?;
    let (_, data) = resolve_destination(data)?;
    Ok((application, data))
}

/// Apply a set of already-previewed migrations as one whole-directory switch.
///
/// Every source file is re-previewed and compared with the caller's source
/// fingerprint before any write. The whole tree is checkpointed, copied to a
/// sibling staging tree, updated in deterministic path order, and validated.
/// The target tree is then published through the journaled restore path. A
/// repeated plan is a no-op when every target file already matches exactly.
pub fn apply_upgrade_plan(
    data_root: &Path,
    source_checkpoint_path: &Path,
    changes: &[UpgradeFileChange],
    limits: UpgradeCheckpointLimits,
) -> DockResult<UpgradeApplyReport> {
    apply_upgrade_plan_with_resolver(
        data_root,
        source_checkpoint_path,
        changes,
        limits,
        crate::preview_persisted_migration,
    )
}

fn apply_upgrade_plan_with_resolver(
    data_root: &Path,
    source_checkpoint_path: &Path,
    changes: &[UpgradeFileChange],
    limits: UpgradeCheckpointLimits,
    preview_fn: impl Fn(crate::PersistedDataFormat, &str) -> DockResult<MigrationPreview>,
) -> DockResult<UpgradeApplyReport> {
    validate_limits(limits)?;
    if changes.is_empty() {
        return Err(upgrade_error(
            ErrorCode::InvalidId,
            "Upgrade plan must contain at least one file change.",
            "changes=0",
        ));
    }
    if changes.len() > MAX_UPGRADE_STORES {
        return Err(upgrade_error(
            ErrorCode::ResourceLimit,
            "Upgrade plan exceeds the 512-store bound.",
            changes.len().to_string(),
        ));
    }
    let root = fs::canonicalize(data_root)
        .map_err(|error| fs_error("Resolve upgrade data root", error, data_root))?;
    if !root.is_dir() {
        return Err(upgrade_error(
            ErrorCode::PersistenceError,
            "Upgrade data root is not a directory.",
            root.display().to_string(),
        ));
    }
    let journal_path = restore_journal_path(&root)?;
    if path_entry_exists(&journal_path) {
        return Err(upgrade_error(
            ErrorCode::Conflict,
            "An unfinished restore journal exists; recover it before applying another upgrade plan.",
            journal_path.display().to_string(),
        ));
    }

    let mut ordered: Vec<_> = changes.iter().collect();
    ordered.sort_by(|left, right| left.relative_path.cmp(&right.relative_path));
    let mut seen: Vec<String> = Vec::with_capacity(ordered.len());
    let mut any_already_applied = false;
    let mut plan_paths = Vec::with_capacity(ordered.len());
    let mut validated = Vec::with_capacity(ordered.len());
    for change in ordered {
        let relative = safe_relative_path(&change.relative_path)?;
        let normalized = normalized_upgrade_path(&relative);
        if upgrade_paths_equal(&normalized, CHECKPOINT_MANIFEST)
            || seen
                .iter()
                .any(|existing| upgrade_paths_equal(existing, &normalized))
        {
            return Err(upgrade_error(
                ErrorCode::InvalidId,
                "Upgrade plan contains a reserved or duplicate relative path.",
                normalized,
            ));
        }
        seen.push(normalized.clone());
        let source_path = root.join(&relative);
        let metadata = fs::symlink_metadata(&source_path)
            .map_err(|error| fs_error("Inspect upgrade source file", error, &source_path))?;
        if !metadata.is_file() || metadata.file_type().is_symlink() {
            return Err(upgrade_error(
                ErrorCode::UnsupportedOperation,
                "Upgrade plan entries must refer to regular files.",
                source_path.display().to_string(),
            ));
        }
        if metadata.len() > limits.maximum_file_bytes {
            return Err(upgrade_error(
                ErrorCode::ResourceLimit,
                "Upgrade source file exceeds the configured file limit.",
                source_path.display().to_string(),
            ));
        }
        let source_bytes = fs::read(&source_path)
            .map_err(|error| fs_error("Read upgrade source file", error, &source_path))?;
        let source_json = std::str::from_utf8(&source_bytes).map_err(|error| {
            upgrade_error(
                ErrorCode::SerializationError,
                "Upgrade source file is not valid UTF-8 JSON.",
                format!("{}: {error}", source_path.display()),
            )
        })?;
        let current_preview = preview_fn(change.preview.format, source_json)?;
        plan_paths.push(normalized.clone());
        if current_preview == change.preview {
            if change.preview.requires_migration {
                validated.push((normalized, relative, change.preview.clone()));
            }
            continue;
        }
        if source_bytes == change.preview.target_json.as_bytes() {
            any_already_applied = true;
            continue;
        }
        return Err(upgrade_error(
            ErrorCode::Conflict,
            "Upgrade source changed since its preview; no files were written.",
            source_path.display().to_string(),
        ));
    }
    if validated.is_empty() {
        let current = inventory_tree(&root, limits)?;
        return Ok(UpgradeApplyReport {
            source_checkpoint_path: None,
            replaced_tree_path: None,
            target_tree_checkpoint_path: None,
            applied_paths: plan_paths,
            target_tree_fingerprint: current.tree_fingerprint,
            already_applied: true,
        });
    }
    if any_already_applied {
        return Err(upgrade_error(
            ErrorCode::Conflict,
            "Upgrade plan is only partially applied; automatic completion stopped without writing.",
            root.display().to_string(),
        ));
    }

    let source_checkpoint = create_upgrade_checkpoint(&root, source_checkpoint_path, limits)?;
    let original_manifest = verify_checkpoint_tree(&source_checkpoint.checkpoint_path, limits)?;
    if !tree_matches_manifest(&root, &original_manifest, limits)? {
        return Err(upgrade_error(
            ErrorCode::Conflict,
            "Upgrade data changed after checkpoint creation; no migration was published.",
            root.display().to_string(),
        ));
    }
    let parent = root.parent().ok_or_else(|| {
        upgrade_error(
            ErrorCode::InvalidId,
            "Upgrade data root has no parent directory.",
            root.display().to_string(),
        )
    })?;
    let staging = unique_sibling(parent, "doofusdock-upgrade-stage")?;
    fs::create_dir(&staging)
        .map_err(|error| fs_error("Create upgrade staging directory", error, &staging))?;
    let staged_result = (|| {
        let mut copied_manifest = CheckpointManifest {
            schema_version: CHECKPOINT_SCHEMA,
            files: Vec::new(),
            directories: Vec::new(),
            total_bytes: 0,
            tree_fingerprint: String::new(),
        };
        copy_tree_into(&root, &root, &staging, limits, &mut copied_manifest)?;
        copied_manifest
            .files
            .sort_by(|left, right| left.relative_path.cmp(&right.relative_path));
        copied_manifest.directories.sort();
        if copied_manifest.files != original_manifest.files
            || copied_manifest.directories != original_manifest.directories
            || copied_manifest.total_bytes != original_manifest.total_bytes
        {
            return Err(upgrade_error(
                ErrorCode::Conflict,
                "Upgrade data changed while staging; no migration was published.",
                root.display().to_string(),
            ));
        }
        for (normalized, relative, preview) in &validated {
            let target = staging.join(relative);
            let parent = target.parent().ok_or_else(|| {
                upgrade_error(
                    ErrorCode::InvalidId,
                    "Upgrade target has no parent directory.",
                    normalized.clone(),
                )
            })?;
            write_staged_file(parent, &target, preview.target_json.as_bytes())?;
            let written = fs::read_to_string(&target)
                .map_err(|error| fs_error("Read staged upgrade target", error, &target))?;
            let verified = preview_fn(preview.format, &written)?;
            if verified.target_json != preview.target_json || verified.requires_migration {
                return Err(upgrade_error(
                    ErrorCode::SchemaError,
                    "Staged migration output did not validate as the requested target.",
                    normalized.clone(),
                ));
            }
        }
        if !tree_matches_manifest(&root, &original_manifest, limits)? {
            return Err(upgrade_error(
                ErrorCode::Conflict,
                "Upgrade data changed during staging; no migration was published.",
                root.display().to_string(),
            ));
        }
        inventory_tree(&staging, limits)
    })();
    let target_manifest = match staged_result {
        Ok(manifest) => manifest,
        Err(error) => {
            let _ = fs::remove_dir_all(&staging);
            return Err(error);
        }
    };

    let checkpoint_parent = source_checkpoint_path.parent().ok_or_else(|| {
        upgrade_error(
            ErrorCode::InvalidId,
            "Source checkpoint path has no parent directory.",
            source_checkpoint_path.display().to_string(),
        )
    })?;
    let checkpoint_parent = fs::canonicalize(checkpoint_parent)
        .map_err(|error| fs_error("Resolve target checkpoint parent", error, checkpoint_parent))?;
    let target_checkpoint_path = unique_sibling(&checkpoint_parent, "doofusdock-upgrade-target")?;
    if let Err(error) = create_upgrade_checkpoint(&staging, &target_checkpoint_path, limits) {
        let _ = fs::remove_dir_all(&staging);
        return Err(error);
    }
    let _ = fs::remove_dir_all(&staging);
    if !tree_matches_manifest(&root, &original_manifest, limits)? {
        return Err(upgrade_error(
            ErrorCode::Conflict,
            "Upgrade data changed before publish; source checkpoint and target checkpoint were retained.",
            root.display().to_string(),
        ));
    }
    let restored = restore_upgrade_checkpoint(&target_checkpoint_path, &root, limits)?;
    let target_tree_fingerprint = inventory_tree(&root, limits)?.tree_fingerprint;
    if target_tree_fingerprint != target_manifest.tree_fingerprint {
        return Err(upgrade_error(
            ErrorCode::SerializationError,
            "Published upgrade tree does not match the staged migration result.",
            root.display().to_string(),
        ));
    }
    Ok(UpgradeApplyReport {
        source_checkpoint_path: Some(source_checkpoint.checkpoint_path),
        replaced_tree_path: restored.replaced_tree_path,
        target_tree_checkpoint_path: Some(target_checkpoint_path),
        applied_paths: validated.into_iter().map(|(path, _, _)| path).collect(),
        target_tree_fingerprint,
        already_applied: false,
    })
}

/// Inspect every host-declared persisted store without writing files.
///
/// Required stores must exist. Optional missing stores are reported and skipped.
/// Existing files are bounded, checked for symlinked path components, and sent
/// through the same versioned adapters used by `apply_upgrade_plan`.
pub fn inspect_upgrade_manifest(
    data_root: &Path,
    manifest: &UpgradeManifest,
    limits: UpgradeCheckpointLimits,
) -> DockResult<UpgradePlan> {
    inspect_upgrade_manifest_with_resolver(
        data_root,
        manifest,
        limits,
        crate::preview_persisted_migration,
    )
}

/// Registry-aware read-only inspection for manifests containing extension state.
pub fn inspect_upgrade_manifest_with_extensions(
    data_root: &Path,
    manifest: &UpgradeManifest,
    registry: &crate::ExtensionRegistry,
    limits: UpgradeCheckpointLimits,
) -> DockResult<UpgradePlan> {
    inspect_upgrade_manifest_with_resolver(data_root, manifest, limits, |format, source| {
        if format == crate::PersistedDataFormat::ExtensionState {
            crate::preview_extension_state_migration(registry, source)
        } else {
            crate::preview_persisted_migration(format, source)
        }
    })
}

fn inspect_upgrade_manifest_with_resolver(
    data_root: &Path,
    manifest: &UpgradeManifest,
    limits: UpgradeCheckpointLimits,
    preview_fn: impl Fn(crate::PersistedDataFormat, &str) -> DockResult<MigrationPreview>,
) -> DockResult<UpgradePlan> {
    validate_limits(limits)?;
    manifest.validate()?;
    if manifest.stores.len() > limits.maximum_files {
        return Err(upgrade_error(
            ErrorCode::ResourceLimit,
            "Upgrade manifest store count exceeds the configured file limit.",
            format!(
                "stores={};limit={}",
                manifest.stores.len(),
                limits.maximum_files
            ),
        ));
    }
    let root = fs::canonicalize(data_root)
        .map_err(|error| fs_error("Resolve upgrade data root", error, data_root))?;
    if !root.is_dir() {
        return Err(upgrade_error(
            ErrorCode::PersistenceError,
            "Upgrade data root is not a directory.",
            root.display().to_string(),
        ));
    }
    let journal_path = restore_journal_path(&root)?;
    if path_entry_exists(&journal_path) {
        return Err(upgrade_error(
            ErrorCode::Conflict,
            "An unfinished restore journal exists; recover it before inspecting an upgrade.",
            journal_path.display().to_string(),
        ));
    }

    let mut total_bytes = 0u64;
    let mut stores = Vec::with_capacity(manifest.stores.len());
    let mut changes = Vec::new();
    for store in &manifest.stores {
        let normalized = safe_relative_path(&store.relative_path)?
            .to_string_lossy()
            .replace('\\', "/");
        let Some(path) = manifest_store_path(&root, &store.relative_path, store.optional)? else {
            stores.push(UpgradeStoreInspection {
                relative_path: normalized,
                format: store.format,
                present: false,
                source_version: None,
                target_version: None,
                requires_migration: false,
                source_fingerprint: None,
            });
            continue;
        };
        let metadata = fs::metadata(&path)
            .map_err(|error| fs_error("Inspect declared upgrade store", error, &path))?;
        if metadata.len() > limits.maximum_file_bytes {
            return Err(upgrade_error(
                ErrorCode::ResourceLimit,
                "Declared upgrade store exceeds the configured per-file byte limit.",
                format!("path={};bytes={}", path.display(), metadata.len()),
            ));
        }
        total_bytes = total_bytes.checked_add(metadata.len()).ok_or_else(|| {
            upgrade_error(
                ErrorCode::ResourceLimit,
                "Declared upgrade stores exceed the configured total byte limit.",
                root.display().to_string(),
            )
        })?;
        if total_bytes > limits.maximum_total_bytes {
            return Err(upgrade_error(
                ErrorCode::ResourceLimit,
                "Declared upgrade stores exceed the configured total byte limit.",
                format!("bytes={total_bytes};limit={}", limits.maximum_total_bytes),
            ));
        }
        let source_bytes = fs::read(&path)
            .map_err(|error| fs_error("Read declared upgrade store", error, &path))?;
        let source_json = std::str::from_utf8(&source_bytes).map_err(|error| {
            upgrade_error(
                ErrorCode::SerializationError,
                "Declared upgrade store is not valid UTF-8 JSON.",
                format!("{}: {error}", path.display()),
            )
        })?;
        let preview = preview_fn(store.format, source_json)?;
        stores.push(UpgradeStoreInspection {
            relative_path: normalized.clone(),
            format: store.format,
            present: true,
            source_version: Some(preview.source_version),
            target_version: Some(preview.target_version),
            requires_migration: preview.requires_migration,
            source_fingerprint: Some(preview.source_fingerprint.clone()),
        });
        if preview.requires_migration {
            changes.push(UpgradeFileChange {
                relative_path: normalized,
                preview,
            });
        }
    }
    Ok(UpgradePlan {
        manifest_schema: manifest.schema_version,
        stores,
        changes,
    })
}

/// Inspect and, when required, apply every supported migration listed by a host.
///
/// All stores are previewed before the checkpoint is created. A single invalid,
/// future-version, or unreadable required store aborts without writes. The
/// supplied data root and store paths are host-owned and explicit.
pub fn apply_upgrade_manifest(
    data_root: &Path,
    source_checkpoint_path: &Path,
    manifest: &UpgradeManifest,
    limits: UpgradeCheckpointLimits,
) -> DockResult<UpgradeManifestApplyReport> {
    apply_upgrade_manifest_with_resolver(
        data_root,
        source_checkpoint_path,
        manifest,
        limits,
        crate::preview_persisted_migration,
    )
}

/// Registry-aware counterpart for manifests containing extension state.
pub fn apply_upgrade_manifest_with_extensions(
    data_root: &Path,
    source_checkpoint_path: &Path,
    manifest: &UpgradeManifest,
    registry: &crate::ExtensionRegistry,
    limits: UpgradeCheckpointLimits,
) -> DockResult<UpgradeManifestApplyReport> {
    apply_upgrade_manifest_with_resolver(
        data_root,
        source_checkpoint_path,
        manifest,
        limits,
        |format, source| {
            if format == crate::PersistedDataFormat::ExtensionState {
                crate::preview_extension_state_migration(registry, source)
            } else {
                crate::preview_persisted_migration(format, source)
            }
        },
    )
}

fn apply_upgrade_manifest_with_resolver(
    data_root: &Path,
    source_checkpoint_path: &Path,
    manifest: &UpgradeManifest,
    limits: UpgradeCheckpointLimits,
    resolver: impl Fn(crate::PersistedDataFormat, &str) -> DockResult<MigrationPreview> + Copy,
) -> DockResult<UpgradeManifestApplyReport> {
    let inspection = inspect_upgrade_manifest_with_resolver(data_root, manifest, limits, resolver)?;
    let application = if inspection.changes.is_empty() {
        None
    } else {
        Some(apply_upgrade_plan_with_resolver(
            data_root,
            source_checkpoint_path,
            &inspection.changes,
            limits,
            resolver,
        )?)
    };
    Ok(UpgradeManifestApplyReport {
        inspection,
        application,
    })
}

fn manifest_store_path(root: &Path, relative: &str, optional: bool) -> DockResult<Option<PathBuf>> {
    let safe = safe_relative_path(relative)?;
    let components: Vec<_> = safe.components().collect();
    let mut current = root.to_path_buf();
    for (index, component) in components.iter().enumerate() {
        let Component::Normal(part) = component else {
            return Err(upgrade_error(
                ErrorCode::InvalidId,
                "Upgrade store path contains an unsupported component.",
                relative,
            ));
        };
        current.push(part);
        let metadata = match fs::symlink_metadata(&current) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound && optional => {
                return Ok(None);
            }
            Err(error) => {
                return Err(fs_error(
                    "Inspect declared upgrade store path",
                    error,
                    &current,
                ));
            }
        };
        if metadata.file_type().is_symlink() {
            return Err(upgrade_error(
                ErrorCode::UnsupportedOperation,
                "Upgrade store paths cannot contain symbolic links.",
                current.display().to_string(),
            ));
        }
        let is_final = index + 1 == components.len();
        if is_final && !metadata.is_file() {
            return Err(upgrade_error(
                ErrorCode::UnsupportedOperation,
                "Declared upgrade store must be a regular file.",
                current.display().to_string(),
            ));
        }
        if !is_final && !metadata.is_dir() {
            return Err(upgrade_error(
                ErrorCode::UnsupportedOperation,
                "Parent components of an upgrade store must be regular directories.",
                current.display().to_string(),
            ));
        }
    }
    Ok(Some(current))
}

fn validate_limits(limits: UpgradeCheckpointLimits) -> DockResult<()> {
    if limits.maximum_files == 0
        || limits.maximum_files > MAX_CHECKPOINT_FILES
        || limits.maximum_file_bytes == 0
        || limits.maximum_total_bytes == 0
        || limits.maximum_file_bytes > limits.maximum_total_bytes
    {
        return Err(upgrade_error(
            ErrorCode::ResourceLimit,
            "Checkpoint limits are invalid or exceed framework bounds.",
            format!(
                "files={};file_bytes={};total_bytes={}",
                limits.maximum_files, limits.maximum_file_bytes, limits.maximum_total_bytes
            ),
        ));
    }
    Ok(())
}

fn resolve_new_directory(path: &Path) -> DockResult<(PathBuf, PathBuf)> {
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .ok_or_else(|| {
            upgrade_error(
                ErrorCode::InvalidId,
                "Checkpoint path must include an existing parent directory.",
                path.display().to_string(),
            )
        })?;
    let name = path.file_name().ok_or_else(|| {
        upgrade_error(
            ErrorCode::InvalidId,
            "Checkpoint path must have a final directory name.",
            path.display().to_string(),
        )
    })?;
    let parent = fs::canonicalize(parent)
        .map_err(|error| fs_error("Resolve checkpoint parent", error, parent))?;
    Ok((parent.clone(), parent.join(name)))
}

fn resolve_destination(path: &Path) -> DockResult<(PathBuf, PathBuf)> {
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .ok_or_else(|| {
            upgrade_error(
                ErrorCode::InvalidId,
                "Restore destination must include an existing parent directory.",
                path.display().to_string(),
            )
        })?;
    let name = path.file_name().ok_or_else(|| {
        upgrade_error(
            ErrorCode::InvalidId,
            "Restore destination must have a final directory name.",
            path.display().to_string(),
        )
    })?;
    let parent = fs::canonicalize(parent)
        .map_err(|error| fs_error("Resolve restore destination parent", error, parent))?;
    Ok((parent.clone(), parent.join(name)))
}

fn unique_sibling(parent: &Path, prefix: &str) -> DockResult<PathBuf> {
    for _ in 0..32 {
        let sequence = CHECKPOINT_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let candidate = parent.join(format!(".{prefix}-{}-{sequence}", std::process::id()));
        if !path_entry_exists(&candidate) {
            return Ok(candidate);
        }
    }
    Err(upgrade_error(
        ErrorCode::Conflict,
        "Could not allocate a unique staging path after bounded retries.",
        parent.display().to_string(),
    ))
}

fn path_entry_exists(path: &Path) -> bool {
    fs::symlink_metadata(path).is_ok()
}

fn copy_tree_into(
    root: &Path,
    current: &Path,
    staging: &Path,
    limits: UpgradeCheckpointLimits,
    manifest: &mut CheckpointManifest,
) -> DockResult<()> {
    let mut children = fs::read_dir(current)
        .map_err(|error| fs_error("Enumerate checkpoint source", error, current))?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| fs_error("Read checkpoint directory entry", error, current))?;
    children.sort();
    for source in children {
        let metadata = fs::symlink_metadata(&source)
            .map_err(|error| fs_error("Inspect checkpoint source entry", error, &source))?;
        if metadata.file_type().is_symlink() {
            return Err(upgrade_error(
                ErrorCode::UnsupportedOperation,
                "Checkpoint source contains a symbolic link; it was not followed.",
                source.display().to_string(),
            ));
        }
        let relative = source.strip_prefix(root).map_err(|error| {
            upgrade_error(
                ErrorCode::InvalidId,
                "Checkpoint entry escaped its source tree.",
                error.to_string(),
            )
        })?;
        let relative_text = relative
            .to_str()
            .ok_or_else(|| {
                upgrade_error(
                    ErrorCode::InvalidId,
                    "Checkpoint path is not valid UTF-8.",
                    relative.display().to_string(),
                )
            })?
            .replace('\\', "/");
        safe_relative_path(&relative_text)?;
        let target = staging.join(relative);
        if metadata.is_dir() {
            fs::create_dir(&target)
                .map_err(|error| fs_error("Create checkpoint directory", error, &target))?;
            manifest.directories.push(relative_text);
            copy_tree_into(root, &source, staging, limits, manifest)?;
        } else if metadata.is_file() {
            if manifest.files.len() >= limits.maximum_files {
                return Err(upgrade_error(
                    ErrorCode::ResourceLimit,
                    "Checkpoint file count exceeds configured limit.",
                    limits.maximum_files.to_string(),
                ));
            }
            let length = metadata.len();
            if length > limits.maximum_file_bytes
                || manifest.total_bytes.saturating_add(length) > limits.maximum_total_bytes
            {
                return Err(upgrade_error(
                    ErrorCode::ResourceLimit,
                    "Checkpoint data exceeds configured byte limits.",
                    format!(
                        "path={};bytes={length};total={}",
                        source.display(),
                        manifest.total_bytes
                    ),
                ));
            }
            if relative_text == CHECKPOINT_MANIFEST {
                return Err(upgrade_error(
                    ErrorCode::Conflict,
                    "Source contains the reserved checkpoint manifest filename at its root.",
                    source.display().to_string(),
                ));
            }
            let (fingerprint, bytes) = copy_file(&source, &target)?;
            if bytes != length {
                return Err(upgrade_error(
                    ErrorCode::PersistenceError,
                    "Checkpoint source changed while it was being copied.",
                    source.display().to_string(),
                ));
            }
            manifest.total_bytes = manifest.total_bytes.saturating_add(bytes);
            manifest.files.push(CheckpointFile {
                relative_path: relative_text,
                byte_length: bytes,
                fingerprint,
            });
        } else {
            return Err(upgrade_error(
                ErrorCode::UnsupportedOperation,
                "Checkpoint source contains a non-regular file.",
                source.display().to_string(),
            ));
        }
    }
    Ok(())
}

fn copy_file(source: &Path, target: &Path) -> DockResult<(String, u64)> {
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| fs_error("Create checkpoint file parent", error, parent))?;
    }
    let mut input =
        File::open(source).map_err(|error| fs_error("Open checkpoint source", error, source))?;
    let mut output =
        File::create(target).map_err(|error| fs_error("Create checkpoint file", error, target))?;
    let mut hash = Fnv64::new();
    let mut copied = 0u64;
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let read = input
            .read(&mut buffer)
            .map_err(|error| fs_error("Read checkpoint source", error, source))?;
        if read == 0 {
            break;
        }
        output
            .write_all(&buffer[..read])
            .map_err(|error| fs_error("Write checkpoint file", error, target))?;
        hash.update(&buffer[..read]);
        copied = copied.saturating_add(read as u64);
    }
    output
        .sync_all()
        .map_err(|error| fs_error("Sync checkpoint file", error, target))?;
    Ok((hash.finish(), copied))
}

fn write_staged_file(parent: &Path, target: &Path, contents: &[u8]) -> DockResult<()> {
    let temporary = unique_sibling(parent, "doofusdock-upgrade-file")?;
    let result = (|| {
        let mut file = File::create(&temporary)
            .map_err(|error| fs_error("Create staged migration file", error, &temporary))?;
        file.write_all(contents)
            .and_then(|_| file.sync_all())
            .map_err(|error| fs_error("Write staged migration file", error, &temporary))?;
        crate::persistence::atomic_replace(&temporary, target)
            .map_err(|error| fs_error("Publish staged migration file", error, target))
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

fn copy_verified_file(source: &Path, target: &Path, expected: &CheckpointFile) -> DockResult<()> {
    let metadata = fs::symlink_metadata(source)
        .map_err(|error| fs_error("Inspect checkpoint file", error, source))?;
    if !metadata.is_file()
        || metadata.file_type().is_symlink()
        || metadata.len() != expected.byte_length
    {
        return Err(upgrade_error(
            ErrorCode::SerializationError,
            "Checkpoint file type or length does not match its manifest.",
            source.display().to_string(),
        ));
    }
    let (fingerprint, bytes) = copy_file(source, target)?;
    if bytes != expected.byte_length || fingerprint != expected.fingerprint {
        return Err(upgrade_error(
            ErrorCode::SerializationError,
            "Checkpoint file fingerprint does not match its manifest.",
            source.display().to_string(),
        ));
    }
    Ok(())
}

fn inventory_tree(root: &Path, limits: UpgradeCheckpointLimits) -> DockResult<CheckpointManifest> {
    let mut manifest = CheckpointManifest {
        schema_version: CHECKPOINT_SCHEMA,
        files: Vec::new(),
        directories: Vec::new(),
        total_bytes: 0,
        tree_fingerprint: String::new(),
    };
    inventory_directory(root, root, limits, &mut manifest)?;
    manifest
        .files
        .sort_by(|left, right| left.relative_path.cmp(&right.relative_path));
    manifest.directories.sort();
    manifest.tree_fingerprint = manifest_fingerprint(&manifest);
    Ok(manifest)
}

fn inventory_directory(
    root: &Path,
    current: &Path,
    limits: UpgradeCheckpointLimits,
    manifest: &mut CheckpointManifest,
) -> DockResult<()> {
    let mut children = fs::read_dir(current)
        .map_err(|error| fs_error("Enumerate checkpoint", error, current))?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| fs_error("Read checkpoint entry", error, current))?;
    children.sort();
    for path in children {
        let metadata = fs::symlink_metadata(&path)
            .map_err(|error| fs_error("Inspect checkpoint entry", error, &path))?;
        if metadata.file_type().is_symlink() {
            return Err(upgrade_error(
                ErrorCode::UnsupportedOperation,
                "Checkpoint contains a symbolic link.",
                path.display().to_string(),
            ));
        }
        let relative = path.strip_prefix(root).map_err(|error| {
            upgrade_error(
                ErrorCode::InvalidId,
                "Checkpoint entry escaped its root.",
                error.to_string(),
            )
        })?;
        let relative_text = relative
            .to_str()
            .ok_or_else(|| {
                upgrade_error(
                    ErrorCode::InvalidId,
                    "Checkpoint path is not valid UTF-8.",
                    relative.display().to_string(),
                )
            })?
            .replace('\\', "/");
        safe_relative_path(&relative_text)?;
        if metadata.is_dir() {
            manifest.directories.push(relative_text);
            inventory_directory(root, &path, limits, manifest)?;
        } else if metadata.is_file() {
            if relative_text == CHECKPOINT_MANIFEST {
                continue;
            }
            if manifest.files.len() >= limits.maximum_files
                || metadata.len() > limits.maximum_file_bytes
                || manifest.total_bytes.saturating_add(metadata.len()) > limits.maximum_total_bytes
            {
                return Err(upgrade_error(
                    ErrorCode::ResourceLimit,
                    "Checkpoint inventory exceeds configured limits.",
                    path.display().to_string(),
                ));
            }
            let fingerprint = fingerprint_file(&path)?;
            if fs::metadata(&path)
                .map_err(|error| fs_error("Recheck checkpoint file", error, &path))?
                .len()
                != metadata.len()
            {
                return Err(upgrade_error(
                    ErrorCode::PersistenceError,
                    "Checkpoint file changed during verification.",
                    path.display().to_string(),
                ));
            }
            manifest.total_bytes += metadata.len();
            manifest.files.push(CheckpointFile {
                relative_path: relative_text,
                byte_length: metadata.len(),
                fingerprint,
            });
        } else {
            return Err(upgrade_error(
                ErrorCode::UnsupportedOperation,
                "Checkpoint contains a non-regular file.",
                path.display().to_string(),
            ));
        }
    }
    Ok(())
}

fn verify_checkpoint_tree(
    root: &Path,
    limits: UpgradeCheckpointLimits,
) -> DockResult<CheckpointManifest> {
    let manifest_path = root.join(CHECKPOINT_MANIFEST);
    let metadata = fs::symlink_metadata(&manifest_path)
        .map_err(|error| fs_error("Inspect checkpoint manifest", error, &manifest_path))?;
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return Err(upgrade_error(
            ErrorCode::UnsupportedOperation,
            "Checkpoint manifest must be a regular file.",
            manifest_path.display().to_string(),
        ));
    }
    if metadata.len() > MAX_CHECKPOINT_MANIFEST_BYTES as u64 {
        return Err(upgrade_error(
            ErrorCode::ResourceLimit,
            "Checkpoint manifest exceeds its byte limit.",
            manifest_path.display().to_string(),
        ));
    }
    let bytes = fs::read(&manifest_path)
        .map_err(|error| fs_error("Read checkpoint manifest", error, &manifest_path))?;
    let manifest: CheckpointManifest = serde_json::from_slice(&bytes).map_err(|error| {
        upgrade_error(
            ErrorCode::SerializationError,
            "Checkpoint manifest is invalid JSON.",
            error.to_string(),
        )
    })?;
    if manifest.schema_version != CHECKPOINT_SCHEMA
        || manifest.files.len() > limits.maximum_files
        || manifest_fingerprint(&manifest) != manifest.tree_fingerprint
    {
        return Err(upgrade_error(
            ErrorCode::SerializationError,
            "Checkpoint manifest version, size or tree fingerprint is invalid.",
            manifest_path.display().to_string(),
        ));
    }
    let actual = inventory_tree(root, limits)?;
    if actual.files != manifest.files
        || actual.directories != manifest.directories
        || actual.total_bytes != manifest.total_bytes
        || actual.tree_fingerprint != manifest.tree_fingerprint
    {
        return Err(upgrade_error(
            ErrorCode::SerializationError,
            "Checkpoint contents do not match the validated manifest.",
            root.display().to_string(),
        ));
    }
    Ok(manifest)
}

fn restore_journal_path(destination: &Path) -> DockResult<PathBuf> {
    let parent = destination.parent().ok_or_else(|| {
        upgrade_error(
            ErrorCode::InvalidId,
            "Restore destination has no parent directory.",
            destination.display().to_string(),
        )
    })?;
    let canonical = destination.to_str().ok_or_else(|| {
        upgrade_error(
            ErrorCode::InvalidId,
            "Restore destination path is not valid UTF-8.",
            destination.display().to_string(),
        )
    })?;
    let mut hash = Fnv64::new();
    hash.update(canonical.as_bytes());
    Ok(parent.join(format!(
        ".doofusdock-restore-{}.json",
        hash.finish().trim_start_matches("fnv1a64:")
    )))
}

fn write_restore_journal(path: &Path, journal: &RestoreJournal) -> DockResult<()> {
    let bytes = serde_json::to_vec(journal).map_err(|error| {
        upgrade_error(
            ErrorCode::SerializationError,
            "Restore journal serialization failed.",
            error.to_string(),
        )
    })?;
    if bytes.len() > 64 * 1024 {
        return Err(upgrade_error(
            ErrorCode::ResourceLimit,
            "Restore journal exceeds its byte limit.",
            bytes.len().to_string(),
        ));
    }
    let parent = path.parent().ok_or_else(|| {
        upgrade_error(
            ErrorCode::InvalidId,
            "Restore journal path has no parent.",
            path.display().to_string(),
        )
    })?;
    let temporary = unique_sibling(parent, "doofusdock-restore-journal")?;
    let mut file = File::create(&temporary)
        .map_err(|error| fs_error("Create restore journal staging file", error, &temporary))?;
    if let Err(error) = file.write_all(&bytes).and_then(|_| file.sync_all()) {
        let _ = fs::remove_file(&temporary);
        return Err(fs_error("Write restore journal", error, &temporary));
    }
    let publish = if path_entry_exists(path) {
        crate::persistence::atomic_replace(&temporary, path)
    } else {
        // Publish without replacement so a concurrent restore cannot clobber
        // the journal created after this existence check.
        crate::persistence::atomic_publish_new(&temporary, path)
    };
    if let Err(error) = publish {
        let _ = fs::remove_file(&temporary);
        return Err(fs_error("Publish restore journal", error, path));
    }
    Ok(())
}

fn read_restore_journal(
    path: &Path,
    expected_parent: &Path,
    expected_destination: &Path,
) -> DockResult<RestoreJournal> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| fs_error("Inspect restore journal", error, path))?;
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return Err(upgrade_error(
            ErrorCode::UnsupportedOperation,
            "Restore journal must be a regular file.",
            path.display().to_string(),
        ));
    }
    if metadata.len() > 64 * 1024 {
        return Err(upgrade_error(
            ErrorCode::ResourceLimit,
            "Restore journal exceeds its byte limit.",
            path.display().to_string(),
        ));
    }
    let bytes = fs::read(path).map_err(|error| fs_error("Read restore journal", error, path))?;
    let journal: RestoreJournal = serde_json::from_slice(&bytes).map_err(|error| {
        upgrade_error(
            ErrorCode::SerializationError,
            "Restore journal is malformed.",
            error.to_string(),
        )
    })?;
    let checkpoint = fs::canonicalize(&journal.checkpoint_root).map_err(|error| {
        fs_error(
            "Resolve journal checkpoint",
            error,
            &journal.checkpoint_root,
        )
    })?;
    if journal.schema_version != RESTORE_JOURNAL_SCHEMA
        || journal.destination_root != expected_destination
        || journal.destination_root.parent() != Some(expected_parent)
        || journal.staging_root.parent() != Some(expected_parent)
        || journal
            .replaced_root
            .as_ref()
            .is_some_and(|previous| previous.parent() != Some(expected_parent))
        || journal.staging_root == journal.destination_root
        || journal.replaced_root.as_ref().is_some_and(|previous| {
            previous == &journal.destination_root || previous == &journal.staging_root
        })
        || checkpoint != journal.checkpoint_root
        || journal.checkpoint_root.starts_with(expected_destination)
        || expected_destination.starts_with(&journal.checkpoint_root)
    {
        return Err(upgrade_error(
            ErrorCode::InvalidId,
            "Restore journal paths or schema are invalid for the requested destination.",
            path.display().to_string(),
        ));
    }
    Ok(journal)
}

fn finish_restore_from_journal(
    journal_path: &Path,
    mut journal: RestoreJournal,
    limits: UpgradeCheckpointLimits,
) -> DockResult<UpgradeRestoreReport> {
    let manifest = verify_checkpoint_tree(&journal.checkpoint_root, limits)?;
    if manifest.tree_fingerprint != journal.tree_fingerprint
        || manifest.files.len() != journal.file_count
        || manifest.total_bytes != journal.total_bytes
    {
        return Err(upgrade_error(
            ErrorCode::SerializationError,
            "Restore journal does not match its checkpoint manifest.",
            journal_path.display().to_string(),
        ));
    }
    if tree_matches_manifest(&journal.destination_root, &manifest, limits)? {
        journal.phase = RestorePhase::RestorePublished;
        write_restore_journal(journal_path, &journal)?;
    }
    match journal.phase {
        RestorePhase::Prepared => {
            if path_entry_exists(&journal.destination_root) {
                let metadata =
                    fs::symlink_metadata(&journal.destination_root).map_err(|error| {
                        fs_error(
                            "Inspect restore destination during recovery",
                            error,
                            &journal.destination_root,
                        )
                    })?;
                if !metadata.is_dir() || metadata.file_type().is_symlink() {
                    return Err(upgrade_error(
                        ErrorCode::UnsupportedOperation,
                        "Restore recovery will not replace a non-directory or symbolic-link destination.",
                        journal.destination_root.display().to_string(),
                    ));
                }
                let previous = journal.replaced_root.as_ref().ok_or_else(|| {
                    upgrade_error(
                        ErrorCode::Conflict,
                        "Restore destination appeared after staging and has no recorded replacement path.",
                        journal.destination_root.display().to_string(),
                    )
                })?;
                if path_entry_exists(previous) {
                    return Err(upgrade_error(
                        ErrorCode::Conflict,
                        "Both destination and retained pre-restore directory exist; automatic recovery stopped.",
                        journal.destination_root.display().to_string(),
                    ));
                }
                fs::rename(&journal.destination_root, previous).map_err(|error| {
                    fs_error(
                        "Preserve destination during restore recovery",
                        error,
                        &journal.destination_root,
                    )
                })?;
            }
            journal.phase = RestorePhase::DestinationMoved;
            write_restore_journal(journal_path, &journal)?;
            publish_staged_restore(journal_path, &mut journal, &manifest, limits)?;
        }
        RestorePhase::DestinationMoved => {
            if !tree_matches_manifest(&journal.destination_root, &manifest, limits)? {
                if path_entry_exists(&journal.destination_root) {
                    return Err(upgrade_error(
                        ErrorCode::Conflict,
                        "Restore destination contains unexpected data; automatic recovery stopped.",
                        journal.destination_root.display().to_string(),
                    ));
                }
                publish_staged_restore(journal_path, &mut journal, &manifest, limits)?;
            }
        }
        RestorePhase::RestorePublished => {
            if !tree_matches_manifest(&journal.destination_root, &manifest, limits)? {
                return Err(upgrade_error(
                    ErrorCode::SerializationError,
                    "Published restore no longer matches its checkpoint; journal was retained.",
                    journal.destination_root.display().to_string(),
                ));
            }
        }
    }
    if journal.phase != RestorePhase::RestorePublished
        || !tree_matches_manifest(&journal.destination_root, &manifest, limits)?
    {
        return Err(upgrade_error(
            ErrorCode::PersistenceError,
            "Restore recovery did not reach a verified published state.",
            journal.destination_root.display().to_string(),
        ));
    }
    fs::remove_file(journal_path)
        .map_err(|error| fs_error("Remove completed restore journal", error, journal_path))?;
    Ok(UpgradeRestoreReport {
        restored_path: journal.destination_root,
        replaced_tree_path: journal.replaced_root,
        file_count: manifest.files.len(),
        total_bytes: manifest.total_bytes,
    })
}

fn publish_staged_restore(
    journal_path: &Path,
    journal: &mut RestoreJournal,
    manifest: &CheckpointManifest,
    limits: UpgradeCheckpointLimits,
) -> DockResult<()> {
    if !tree_matches_manifest(&journal.staging_root, manifest, limits)? {
        return Err(upgrade_error(
            ErrorCode::SerializationError,
            "Staged restore is missing or does not match its checkpoint; journal was retained.",
            journal.staging_root.display().to_string(),
        ));
    }
    fs::rename(&journal.staging_root, &journal.destination_root).map_err(|error| {
        fs_error(
            "Publish recovered data tree",
            error,
            &journal.destination_root,
        )
    })?;
    journal.phase = RestorePhase::RestorePublished;
    write_restore_journal(journal_path, journal)
}

fn tree_matches_manifest(
    root: &Path,
    expected: &CheckpointManifest,
    limits: UpgradeCheckpointLimits,
) -> DockResult<bool> {
    if !path_entry_exists(root) {
        return Ok(false);
    }
    let metadata = fs::symlink_metadata(root)
        .map_err(|error| fs_error("Inspect restore tree", error, root))?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Ok(false);
    }
    let actual = inventory_tree(root, limits)?;
    Ok(actual.files == expected.files
        && actual.directories == expected.directories
        && actual.total_bytes == expected.total_bytes
        && actual.tree_fingerprint == expected.tree_fingerprint)
}

fn write_manifest(root: &Path, manifest: &CheckpointManifest) -> DockResult<()> {
    let path = root.join(CHECKPOINT_MANIFEST);
    let temporary = root.join(format!("{CHECKPOINT_MANIFEST}.tmp-{}", std::process::id()));
    let bytes = serde_json::to_vec(manifest).map_err(|error| {
        upgrade_error(
            ErrorCode::SerializationError,
            "Checkpoint manifest serialization failed.",
            error.to_string(),
        )
    })?;
    if bytes.len() > MAX_CHECKPOINT_MANIFEST_BYTES {
        return Err(upgrade_error(
            ErrorCode::ResourceLimit,
            "Checkpoint manifest exceeds its byte limit.",
            bytes.len().to_string(),
        ));
    }
    let mut file = File::create(&temporary)
        .map_err(|error| fs_error("Create checkpoint manifest", error, &temporary))?;
    file.write_all(&bytes)
        .and_then(|_| file.sync_all())
        .map_err(|error| fs_error("Write checkpoint manifest", error, &temporary))?;
    fs::rename(&temporary, &path)
        .map_err(|error| fs_error("Publish checkpoint manifest", error, &path))
}

fn normalized_upgrade_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

#[cfg(windows)]
#[link(name = "Kernel32")]
unsafe extern "system" {
    fn CompareStringOrdinal(
        left: *const u16,
        left_length: i32,
        right: *const u16,
        right_length: i32,
        ignore_case: i32,
    ) -> i32;
}

#[cfg(windows)]
fn upgrade_paths_equal(left: &str, right: &str) -> bool {
    let left: Vec<u16> = left.encode_utf16().collect();
    let right: Vec<u16> = right.encode_utf16().collect();
    let Ok(left_length) = i32::try_from(left.len()) else {
        return true;
    };
    let Ok(right_length) = i32::try_from(right.len()) else {
        return true;
    };
    // SAFETY: both slices are valid UTF-16 buffers for their explicit lengths.
    // CompareStringOrdinal does not retain either pointer.
    let result = unsafe {
        CompareStringOrdinal(left.as_ptr(), left_length, right.as_ptr(), right_length, 1)
    };
    result == 2 || result == 0
}

#[cfg(not(windows))]
fn upgrade_paths_equal(left: &str, right: &str) -> bool {
    left == right
}

fn safe_relative_path(relative: &str) -> DockResult<PathBuf> {
    let path = Path::new(relative);
    if relative.is_empty()
        || path.is_absolute()
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(upgrade_error(
            ErrorCode::InvalidId,
            "Checkpoint manifest contains an unsafe relative path.",
            relative.to_owned(),
        ));
    }
    Ok(path.to_path_buf())
}

fn manifest_fingerprint(manifest: &CheckpointManifest) -> String {
    let mut hash = Fnv64::new();
    for directory in &manifest.directories {
        hash.update(b"D\0");
        hash.update(directory.as_bytes());
        hash.update(b"\0");
    }
    for file in &manifest.files {
        hash.update(b"F\0");
        hash.update(file.relative_path.as_bytes());
        hash.update(b"\0");
        hash.update(&file.byte_length.to_le_bytes());
        hash.update(file.fingerprint.as_bytes());
        hash.update(b"\0");
    }
    hash.finish()
}

fn fingerprint_file(path: &Path) -> DockResult<String> {
    let mut file =
        File::open(path).map_err(|error| fs_error("Open file for fingerprint", error, path))?;
    let mut hash = Fnv64::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|error| fs_error("Read file for fingerprint", error, path))?;
        if read == 0 {
            break;
        }
        hash.update(&buffer[..read]);
    }
    Ok(hash.finish())
}

fn report(path: &Path, manifest: &CheckpointManifest) -> UpgradeCheckpointReport {
    UpgradeCheckpointReport {
        checkpoint_path: path.to_path_buf(),
        file_count: manifest.files.len(),
        directory_count: manifest.directories.len(),
        total_bytes: manifest.total_bytes,
        tree_fingerprint: manifest.tree_fingerprint.clone(),
    }
}

struct Fnv64(u64);

impl Fnv64 {
    fn new() -> Self {
        Self(14695981039346656037)
    }
    fn update(&mut self, bytes: &[u8]) {
        for byte in bytes {
            self.0 ^= u64::from(*byte);
            self.0 = self.0.wrapping_mul(1099511628211);
        }
    }
    fn finish(&self) -> String {
        format!("fnv1a64:{:016x}", self.0)
    }
}

fn fs_error(action: &str, error: std::io::Error, path: &Path) -> DockError {
    upgrade_error(
        ErrorCode::PersistenceError,
        format!("{action} failed: {error}"),
        path.display().to_string(),
    )
}

fn upgrade_error(
    code: ErrorCode,
    message: impl Into<String>,
    context: impl Into<String>,
) -> DockError {
    DockError::new(code, message, context)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::WorkspaceProfileCatalog;
    use crate::{PersistedDataFormat, preview_persisted_migration};
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_TEST_DIR: AtomicU64 = AtomicU64::new(0);

    struct TestTree(PathBuf);
    impl TestTree {
        fn new() -> Self {
            let id = NEXT_TEST_DIR.fetch_add(1, Ordering::Relaxed);
            let nonce = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system clock should be after the Unix epoch")
                .as_nanos();
            let root = std::env::temp_dir().join(format!(
                "doofusdock-upgrade-test-{}-{nonce}-{id}",
                std::process::id(),
            ));
            fs::create_dir(&root).expect("test root must be unique for this run");
            Self(root)
        }
        fn path(&self, name: &str) -> PathBuf {
            self.0.join(name)
        }
    }
    impl Drop for TestTree {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn stage_from_checkpoint(checkpoint: &Path, staging: &Path) -> CheckpointManifest {
        let manifest =
            verify_checkpoint_tree(checkpoint, UpgradeCheckpointLimits::default()).unwrap();
        fs::create_dir(staging).unwrap();
        for directory in &manifest.directories {
            fs::create_dir_all(staging.join(safe_relative_path(directory).unwrap())).unwrap();
        }
        for entry in &manifest.files {
            let relative = safe_relative_path(&entry.relative_path).unwrap();
            let target = staging.join(&relative);
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            copy_verified_file(&checkpoint.join(relative), &target, entry).unwrap();
        }
        manifest
    }

    #[test]
    fn checkpoint_verifies_and_restores_tree_while_retaining_replaced_directory() {
        let tree = TestTree::new();
        let source = tree.path("data");
        fs::create_dir_all(source.join("themes/assets")).unwrap();
        fs::write(source.join("layout.json"), b"old-layout").unwrap();
        fs::write(source.join("themes/assets/skin.bin"), b"old-asset").unwrap();
        let checkpoint = tree.path("checkpoint");
        let report =
            create_upgrade_checkpoint(&source, &checkpoint, UpgradeCheckpointLimits::default())
                .unwrap();
        assert_eq!(report.file_count, 2);
        assert!(report.tree_fingerprint.starts_with("fnv1a64:"));

        fs::write(source.join("layout.json"), b"upgraded-layout").unwrap();
        fs::write(source.join("new-state.json"), b"new-state").unwrap();
        let restored =
            restore_upgrade_checkpoint(&checkpoint, &source, UpgradeCheckpointLimits::default())
                .unwrap();
        assert_eq!(fs::read(source.join("layout.json")).unwrap(), b"old-layout");
        assert_eq!(
            fs::read(source.join("themes/assets/skin.bin")).unwrap(),
            b"old-asset"
        );
        assert!(!source.join("new-state.json").exists());
        let replaced = restored.replaced_tree_path.unwrap();
        assert_eq!(
            fs::read(replaced.join("layout.json")).unwrap(),
            b"upgraded-layout"
        );
        assert!(replaced.join("new-state.json").exists());
    }

    #[test]
    fn manifest_applies_extension_state_with_shared_checkpoint_and_restores_it() {
        let tree = TestTree::new();
        let data = tree.path("data");
        fs::create_dir_all(&data).unwrap();
        let gallery_preferences: serde_json::Value = serde_json::from_str(include_str!(
            "../tests/fixtures/doofusimage_gallery_preferences_v0.json"
        ))
        .unwrap();
        let mut envelope = serde_json::json!({
            "schema_version": 1, "workspace_id": "host", "preferences": {
                "extensions": { "doofusimage:gallery-preferences": { "schema_version": 0, "data": gallery_preferences } },
                "host-key": "preserved"
            }
        });
        let checksum =
            crate::checksum_layout_json(&serde_json::to_string(&envelope).unwrap()).unwrap();
        envelope["checksum"] = serde_json::json!(checksum);
        let original = serde_json::to_vec(&envelope).unwrap();
        fs::write(data.join("preferences.json"), &original).unwrap();

        let mut registry = crate::ExtensionRegistry::default();
        registry
            .register_versioned_persistence_codec(crate::VersionedPersistenceCodecExtension {
                codec: crate::PersistenceCodecExtension {
                    descriptor: crate::ExtensionDescriptor {
                        namespaced_id: "doofusimage:gallery-preferences".into(),
                        version: crate::ApiVersion { major: 1, minor: 0 },
                        api: crate::CompatibilityRange {
                            minimum: crate::ApiVersion { major: 1, minor: 0 },
                            maximum: crate::ApiVersion { major: 1, minor: 0 },
                        },
                        on_activate: None,
                        on_shutdown: None,
                    },
                    encode: std::sync::Arc::new(|state| Ok(state.clone())),
                    decode: std::sync::Arc::new(|state| Ok(state.clone())),
                },
                current_schema_version: 1,
                migrations: vec![crate::PersistenceCodecMigration {
                    from_schema_version: 0,
                    to_schema_version: 1,
                    migrate: std::sync::Arc::new(|state| Ok(state.clone())),
                }],
            })
            .unwrap();
        registry.freeze();
        registry
            .activate("doofusimage:gallery-preferences")
            .unwrap();
        let manifest = UpgradeManifest {
            schema_version: UpgradeManifest::CURRENT_SCHEMA,
            stores: vec![UpgradeStoreSpec {
                relative_path: "preferences.json".into(),
                format: crate::PersistedDataFormat::ExtensionState,
                optional: false,
            }],
        };
        let registry_free_error =
            inspect_upgrade_manifest(&data, &manifest, UpgradeCheckpointLimits::default())
                .unwrap_err();
        assert_eq!(registry_free_error.code, ErrorCode::UnsupportedOperation);
        assert_eq!(fs::read(data.join("preferences.json")).unwrap(), original);
        let inspection = inspect_upgrade_manifest_with_extensions(
            &data,
            &manifest,
            &registry,
            UpgradeCheckpointLimits::default(),
        )
        .unwrap();
        assert_eq!(inspection.migration_count(), 1);
        let report = apply_upgrade_manifest_with_extensions(
            &data,
            &tree.path("source-checkpoint"),
            &manifest,
            &registry,
            UpgradeCheckpointLimits::default(),
        )
        .unwrap();
        let target: serde_json::Value =
            serde_json::from_slice(&fs::read(data.join("preferences.json")).unwrap()).unwrap();
        assert_eq!(
            target["preferences"]["extensions"]["doofusimage:gallery-preferences"]["schema_version"],
            1
        );
        assert_eq!(
            target["preferences"]["extensions"]["doofusimage:gallery-preferences"]["data"],
            gallery_preferences
        );
        assert_eq!(target["preferences"]["host-key"], "preserved");
        let mut checksum_input = target.clone();
        let target_checksum = checksum_input["checksum"].as_str().unwrap().to_owned();
        checksum_input.as_object_mut().unwrap().remove("checksum");
        assert_eq!(
            target_checksum,
            crate::checksum_layout_json(&checksum_input.to_string()).unwrap()
        );
        assert_eq!(
            report.application.as_ref().unwrap().applied_paths,
            vec!["preferences.json"]
        );
        let checkpoint = report.application.unwrap().source_checkpoint_path.unwrap();
        assert_eq!(
            fs::read(checkpoint.join("preferences.json")).unwrap(),
            original
        );
        restore_upgrade_checkpoint(&checkpoint, &data, UpgradeCheckpointLimits::default()).unwrap();
        assert_eq!(fs::read(data.join("preferences.json")).unwrap(), original);
    }

    #[test]
    fn generic_manifest_rejects_extension_state_before_applying_other_migrations() {
        let tree = TestTree::new();
        let data = tree.path("data");
        fs::create_dir_all(&data).unwrap();
        let legacy = legacy_layout_json();
        fs::write(data.join("workspace.json"), &legacy).unwrap();

        let mut preferences = serde_json::json!({
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
        preferences["checksum"] = serde_json::Value::String(
            crate::checksum_layout_json(&preferences.to_string()).unwrap(),
        );
        let extension_source = serde_json::to_vec(&preferences).unwrap();
        fs::write(data.join("preferences.json"), &extension_source).unwrap();

        let manifest = UpgradeManifest {
            schema_version: UpgradeManifest::CURRENT_SCHEMA,
            stores: vec![
                UpgradeStoreSpec {
                    relative_path: "workspace.json".into(),
                    format: crate::PersistedDataFormat::Layout,
                    optional: false,
                },
                UpgradeStoreSpec {
                    relative_path: "preferences.json".into(),
                    format: crate::PersistedDataFormat::ExtensionState,
                    optional: false,
                },
            ],
        };
        let checkpoint = tree.path("source-checkpoint");
        let error = apply_upgrade_manifest(
            &data,
            &checkpoint,
            &manifest,
            UpgradeCheckpointLimits::default(),
        )
        .unwrap_err();

        assert_eq!(error.code, ErrorCode::UnsupportedOperation);
        assert_eq!(
            fs::read(data.join("workspace.json")).unwrap(),
            legacy.as_bytes()
        );
        assert_eq!(
            fs::read(data.join("preferences.json")).unwrap(),
            extension_source
        );
        assert!(!checkpoint.exists());
    }

    #[test]
    fn checkpoint_rejects_existing_destinations_and_corrupted_files() {
        let tree = TestTree::new();
        let source = tree.path("data");
        fs::create_dir_all(&source).unwrap();
        fs::write(source.join("layout.json"), b"state").unwrap();
        let checkpoint = tree.path("checkpoint");
        create_upgrade_checkpoint(&source, &checkpoint, UpgradeCheckpointLimits::default())
            .unwrap();
        let conflict =
            create_upgrade_checkpoint(&source, &checkpoint, UpgradeCheckpointLimits::default())
                .unwrap_err();
        assert_eq!(conflict.code, ErrorCode::Conflict);
        fs::write(checkpoint.join("layout.json"), b"tampered").unwrap();
        let error = restore_upgrade_checkpoint(
            &checkpoint,
            &tree.path("restore"),
            UpgradeCheckpointLimits::default(),
        )
        .unwrap_err();
        assert_eq!(error.code, ErrorCode::SerializationError);
        assert!(!tree.path("restore").exists());
    }

    #[test]
    fn checkpoint_rejects_source_inside_checkpoint_parent_and_invalid_limits() {
        let tree = TestTree::new();
        let source = tree.path("data");
        fs::create_dir_all(source.join("nested")).unwrap();
        let inside = source.join("nested/checkpoint");
        let error = create_upgrade_checkpoint(&source, &inside, UpgradeCheckpointLimits::default())
            .unwrap_err();
        assert_eq!(error.code, ErrorCode::InvalidId);
        let limits = UpgradeCheckpointLimits {
            maximum_files: 0,
            ..UpgradeCheckpointLimits::default()
        };
        assert_eq!(
            create_upgrade_checkpoint(&source, &tree.path("cp"), limits)
                .unwrap_err()
                .code,
            ErrorCode::ResourceLimit
        );
    }

    #[test]
    fn upgrade_manifest_inspects_applies_and_repeats_without_touching_unlisted_files() {
        let tree = TestTree::new();
        let data = tree.path("data");
        fs::create_dir_all(&data).unwrap();
        let original = legacy_layout_json();
        fs::write(data.join("workspace.json"), &original).unwrap();
        let profile_workspace = crate::WorkspaceId::from("legacy-profile-store");
        let legacy_profile_catalog: serde_json::Value = serde_json::from_str(include_str!(
            "../tests/fixtures/profile_catalog_schema_v0.json"
        ))
        .unwrap();
        let mut profile_envelope = serde_json::json!({
            "schema_version": 1,
            "workspace_id": profile_workspace.0,
            "preferences": legacy_profile_catalog,
            "host_metadata": { "owner": "host-owned" }
        });
        profile_envelope["checksum"] = serde_json::Value::String(
            crate::checksum_layout_json(&profile_envelope.to_string()).unwrap(),
        );
        let original_profile = serde_json::to_string(&profile_envelope).unwrap();
        let profile_path = "legacy-profile-store.json";
        fs::write(data.join(profile_path), &original_profile).unwrap();
        let catalog_owner = crate::WorkspaceId::from("workspace-catalog-owner");
        let mut workspace_catalog = crate::WorkspaceCatalog::default();
        workspace_catalog
            .create(crate::WorkspaceId::from("default"), "Default")
            .unwrap();
        let mut workspace_envelope = serde_json::json!({
            "schema_version": 1,
            "workspace_id": catalog_owner.0,
            "preferences": serde_json::to_value(&workspace_catalog).unwrap(),
            "host_metadata": { "owner": "workspace-host" }
        });
        workspace_envelope["checksum"] = serde_json::Value::String(
            crate::checksum_layout_json(&workspace_envelope.to_string()).unwrap(),
        );
        let original_workspace_envelope = serde_json::to_string(&workspace_envelope).unwrap();
        let workspace_catalog_path = "workspace-catalog-owner.json";
        fs::write(
            data.join(workspace_catalog_path),
            &original_workspace_envelope,
        )
        .unwrap();
        fs::write(data.join("host-owned.json"), b"{\"custom\":true}").unwrap();
        let opaque_path = "extension-cache/provider-state.bin";
        let opaque_bytes = [0, 255, 17, 0, 128, 42, 254, 10];
        fs::create_dir_all(data.join("extension-cache")).unwrap();
        fs::write(data.join(opaque_path), opaque_bytes).unwrap();
        let manifest = UpgradeManifest::from_json(
            r#"{
              "schema_version": 1,
              "stores": [
                { "relative_path": "workspace.json", "format": "layout" },
                { "relative_path": "legacy-profile-store.json", "format": "profile_catalog" },
                { "relative_path": "workspace-catalog-owner.json", "format": "workspace_catalog" },
                { "relative_path": "themes/custom/theme.json", "format": "theme_package", "optional": true }
              ]
            }"#,
        )
        .unwrap();
        assert_eq!(
            UpgradeManifest::from_json(&manifest.to_json().unwrap()).unwrap(),
            manifest
        );

        let inspection =
            inspect_upgrade_manifest(&data, &manifest, UpgradeCheckpointLimits::default()).unwrap();
        assert_eq!(inspection.migration_count(), 2);
        assert!(
            inspection
                .stores
                .iter()
                .find(|store| store.relative_path == profile_path)
                .unwrap()
                .requires_migration
        );
        assert!(
            !inspection
                .stores
                .iter()
                .find(|store| store.relative_path == workspace_catalog_path)
                .unwrap()
                .requires_migration
        );
        assert!(
            !inspection
                .stores
                .iter()
                .find(|store| store.relative_path == "themes/custom/theme.json")
                .unwrap()
                .present
        );
        assert_eq!(
            fs::read(data.join("workspace.json")).unwrap(),
            original.as_bytes()
        );
        assert_eq!(
            fs::read(data.join(profile_path)).unwrap(),
            original_profile.as_bytes()
        );
        assert_eq!(
            fs::read(data.join(workspace_catalog_path)).unwrap(),
            original_workspace_envelope.as_bytes()
        );
        assert_eq!(
            fs::read(data.join("host-owned.json")).unwrap(),
            b"{\"custom\":true}"
        );
        assert_eq!(fs::read(data.join(opaque_path)).unwrap(), opaque_bytes);

        let report = apply_upgrade_manifest(
            &data,
            &tree.path("source-checkpoint"),
            &manifest,
            UpgradeCheckpointLimits::default(),
        )
        .unwrap();
        let application = report.application.unwrap();
        let source_checkpoint = application.source_checkpoint_path.clone().unwrap();
        assert_eq!(
            application.applied_paths,
            vec!["legacy-profile-store.json", "workspace.json"]
        );
        assert!(
            application
                .source_checkpoint_path
                .as_ref()
                .unwrap()
                .exists()
        );
        assert!(
            application
                .target_tree_checkpoint_path
                .as_ref()
                .unwrap()
                .exists()
        );
        let migrated = fs::read_to_string(data.join("workspace.json")).unwrap();
        let document: crate::LayoutDocument = serde_json::from_str(&migrated).unwrap();
        assert_eq!(document.schema_version, crate::CURRENT_LAYOUT_SCHEMA);
        let profile_preferences =
            crate::WorkspacePreferencesStore::new(&data, crate::PersistenceLimits::default());
        let migrated_profiles =
            crate::load_profile_catalog(&profile_preferences, &profile_workspace).unwrap();
        assert_eq!(
            migrated_profiles.schema_version,
            WorkspaceProfileCatalog::CURRENT_SCHEMA
        );
        assert!(migrated_profiles.profiles.contains_key("legacy-workbench"));
        let migrated_profile_envelope: serde_json::Value =
            serde_json::from_slice(&fs::read(data.join(profile_path)).unwrap()).unwrap();
        assert_eq!(
            migrated_profile_envelope["host_metadata"]["owner"],
            "host-owned"
        );
        let workspace_store =
            crate::WorkspacePreferencesStore::new(&data, crate::PersistenceLimits::default());
        assert_eq!(
            crate::load_workspace_catalog(
                &workspace_store,
                &catalog_owner,
                crate::PersistenceLimits::default().maximum_bytes
            )
            .unwrap(),
            workspace_catalog
        );
        assert_eq!(
            fs::read(source_checkpoint.join("workspace.json")).unwrap(),
            original.as_bytes()
        );
        assert_eq!(
            fs::read(source_checkpoint.join(profile_path)).unwrap(),
            original_profile.as_bytes()
        );
        assert_eq!(
            fs::read(source_checkpoint.join(workspace_catalog_path)).unwrap(),
            original_workspace_envelope.as_bytes()
        );
        assert_eq!(
            fs::read(source_checkpoint.join(opaque_path)).unwrap(),
            opaque_bytes,
            "the whole-tree checkpoint retains unlisted opaque bytes"
        );
        assert_eq!(
            fs::read(data.join("host-owned.json")).unwrap(),
            b"{\"custom\":true}"
        );
        assert_eq!(
            fs::read(data.join(opaque_path)).unwrap(),
            opaque_bytes,
            "unlisted opaque binary state is carried through byte-for-byte"
        );

        let repeated = apply_upgrade_manifest(
            &data,
            &tree.path("unused-checkpoint"),
            &manifest,
            UpgradeCheckpointLimits::default(),
        )
        .unwrap();
        assert!(repeated.application.is_none());
        assert_eq!(repeated.inspection.migration_count(), 0);
        assert!(!tree.path("unused-checkpoint").exists());
    }

    #[test]
    fn upgrade_manifest_migrates_theme_preserves_rules_and_restores_all_source_bytes() {
        let tree = TestTree::new();
        let data = tree.path("data");
        fs::create_dir_all(&data).unwrap();
        let theme_path = "themes/custom/theme.json";
        let rules_path = "window-rules.json";
        let original_theme = include_bytes!("../tests/fixtures/theme_manifest_source_05da1c4.json");
        let original_rules =
            include_bytes!("../tests/fixtures/window_rules_store_source_c7e7804.json");
        fs::create_dir_all(data.join("themes/custom")).unwrap();
        fs::write(data.join(theme_path), original_theme).unwrap();
        fs::write(data.join(rules_path), original_rules).unwrap();

        let manifest = UpgradeManifest {
            schema_version: UpgradeManifest::CURRENT_SCHEMA,
            stores: vec![
                UpgradeStoreSpec {
                    relative_path: theme_path.into(),
                    format: crate::PersistedDataFormat::ThemePackage,
                    optional: false,
                },
                UpgradeStoreSpec {
                    relative_path: rules_path.into(),
                    format: crate::PersistedDataFormat::WindowRules,
                    optional: false,
                },
            ],
        };
        let limits = UpgradeCheckpointLimits::default();
        let initial = inspect_upgrade_manifest(&data, &manifest, limits).unwrap();
        assert_eq!(initial.migration_count(), 1);
        assert!(
            initial
                .stores
                .iter()
                .find(|store| store.relative_path == theme_path)
                .unwrap()
                .requires_migration
        );
        assert!(
            !initial
                .stores
                .iter()
                .find(|store| store.relative_path == rules_path)
                .unwrap()
                .requires_migration
        );
        assert_eq!(fs::read(data.join(theme_path)).unwrap(), original_theme);
        assert_eq!(fs::read(data.join(rules_path)).unwrap(), original_rules);

        let source_checkpoint_path = tree.path("source-checkpoint");
        let applied = apply_upgrade_manifest(&data, &source_checkpoint_path, &manifest, limits)
            .unwrap()
            .application
            .unwrap();
        assert_eq!(applied.applied_paths, vec![theme_path]);
        let source_checkpoint = applied.source_checkpoint_path.unwrap();
        let migrated_theme = fs::read_to_string(data.join(theme_path)).unwrap();
        let theme_manifest = crate::ThemeManifest::parse_json(&migrated_theme).unwrap();
        assert_eq!(theme_manifest.schema_version, crate::THEME_SCHEMA_VERSION);
        assert_eq!(theme_manifest.id, "doofusdock-noctua");
        assert_eq!(fs::read(data.join(rules_path)).unwrap(), original_rules);
        assert_eq!(
            fs::read(source_checkpoint.join(theme_path)).unwrap(),
            original_theme
        );
        assert_eq!(
            fs::read(source_checkpoint.join(rules_path)).unwrap(),
            original_rules
        );

        let repeated =
            apply_upgrade_manifest(&data, &tree.path("unused-checkpoint"), &manifest, limits)
                .unwrap();
        assert!(repeated.application.is_none());
        assert_eq!(repeated.inspection.migration_count(), 0);
        assert!(!tree.path("unused-checkpoint").exists());

        let restored = restore_upgrade_checkpoint(&source_checkpoint, &data, limits).unwrap();
        assert_eq!(fs::read(data.join(theme_path)).unwrap(), original_theme);
        assert_eq!(fs::read(data.join(rules_path)).unwrap(), original_rules);
        let replaced_tree = restored.replaced_tree_path.unwrap();
        assert_eq!(
            fs::read(replaced_tree.join(theme_path)).unwrap(),
            migrated_theme.as_bytes()
        );
        assert_eq!(
            fs::read(replaced_tree.join(rules_path)).unwrap(),
            original_rules
        );
    }

    #[test]
    fn direct_upgrade_plan_enforces_the_manifest_store_bound_before_io() {
        let legacy = legacy_layout_json();
        let preview = preview_persisted_migration(PersistedDataFormat::Layout, &legacy).unwrap();
        let changes = (0..=MAX_UPGRADE_STORES)
            .map(|index| UpgradeFileChange {
                relative_path: format!("store-{index}.json"),
                preview: preview.clone(),
            })
            .collect::<Vec<_>>();
        let error = apply_upgrade_plan(
            Path::new("missing-data-root"),
            Path::new("unused-checkpoint"),
            &changes,
            UpgradeCheckpointLimits::default(),
        )
        .unwrap_err();
        assert_eq!(error.code, ErrorCode::ResourceLimit);
    }

    #[cfg(windows)]
    #[test]
    fn upgrade_manifest_rejects_case_insensitive_aliases_and_reserved_metadata() {
        let duplicate = UpgradeManifest {
            schema_version: UpgradeManifest::CURRENT_SCHEMA,
            stores: vec![
                UpgradeStoreSpec {
                    relative_path: "Config/preferences.json".into(),
                    format: PersistedDataFormat::WorkspacePreferences,
                    optional: false,
                },
                UpgradeStoreSpec {
                    relative_path: "config/PREFERENCES.JSON".into(),
                    format: PersistedDataFormat::WorkspacePreferences,
                    optional: false,
                },
            ],
        };
        assert_eq!(duplicate.validate().unwrap_err().code, ErrorCode::InvalidId);
        let distinct_unicode_case_mappings = UpgradeManifest {
            schema_version: UpgradeManifest::CURRENT_SCHEMA,
            stores: vec![
                UpgradeStoreSpec {
                    relative_path: "Σ.json".into(),
                    format: PersistedDataFormat::WorkspacePreferences,
                    optional: false,
                },
                UpgradeStoreSpec {
                    relative_path: "ς.JSON".into(),
                    format: PersistedDataFormat::WorkspacePreferences,
                    optional: false,
                },
            ],
        };
        // Confirm the filesystem and ordinal comparison agree for these names.
        let tree = TestTree::new();
        let unicode_root = tree.path("unicode-names");
        fs::create_dir_all(&unicode_root).unwrap();
        fs::write(unicode_root.join("Σ.json"), b"capital").unwrap();
        fs::write(unicode_root.join("ς.JSON"), b"final").unwrap();
        assert_eq!(fs::read_dir(&unicode_root).unwrap().count(), 2);
        // Windows ordinal comparison does not equate capital sigma with final
        // sigma, so the migration guard must not apply linguistic folding.
        distinct_unicode_case_mappings.validate().unwrap();

        let reserved = UpgradeManifest {
            schema_version: UpgradeManifest::CURRENT_SCHEMA,
            stores: vec![UpgradeStoreSpec {
                relative_path: CHECKPOINT_MANIFEST.to_ascii_uppercase(),
                format: PersistedDataFormat::WorkspacePreferences,
                optional: false,
            }],
        };
        assert_eq!(reserved.validate().unwrap_err().code, ErrorCode::InvalidId);
    }

    #[cfg(windows)]
    #[test]
    fn direct_upgrade_plan_rejects_case_aliases_and_reserved_metadata_before_writes() {
        let tree = TestTree::new();
        let data = tree.path("host-data");
        fs::create_dir_all(&data).unwrap();
        let original = legacy_layout_json();
        fs::write(data.join("layout.json"), &original).unwrap();
        let preview = preview_persisted_migration(PersistedDataFormat::Layout, &original).unwrap();
        let checkpoint = tree.path("checkpoint");
        let duplicate = apply_upgrade_plan(
            &data,
            &checkpoint,
            &[
                UpgradeFileChange {
                    relative_path: "layout.json".into(),
                    preview: preview.clone(),
                },
                UpgradeFileChange {
                    relative_path: "LAYOUT.JSON".into(),
                    preview: preview.clone(),
                },
            ],
            UpgradeCheckpointLimits::default(),
        )
        .unwrap_err();
        assert_eq!(duplicate.code, ErrorCode::InvalidId);
        assert_eq!(
            fs::read_to_string(data.join("layout.json")).unwrap(),
            original
        );
        assert!(!checkpoint.exists());

        let reserved = apply_upgrade_plan(
            &data,
            &checkpoint,
            &[UpgradeFileChange {
                relative_path: CHECKPOINT_MANIFEST.to_ascii_uppercase(),
                preview,
            }],
            UpgradeCheckpointLimits::default(),
        )
        .unwrap_err();
        assert_eq!(reserved.code, ErrorCode::InvalidId);
        assert!(!checkpoint.exists());
    }

    #[test]
    fn upgrade_manifest_future_and_unsafe_paths_fail_without_writes() {
        let tree = TestTree::new();
        let data = tree.path("data");
        fs::create_dir_all(&data).unwrap();
        fs::write(
            data.join("layout.json"),
            include_str!("../tests/fixtures/layout_schema_v0.json"),
        )
        .unwrap();
        let before = fs::read(data.join("layout.json")).unwrap();
        let future = UpgradeManifest {
            schema_version: 2,
            stores: vec![UpgradeStoreSpec {
                relative_path: "layout.json".into(),
                format: crate::PersistedDataFormat::Layout,
                optional: false,
            }],
        };
        assert_eq!(
            inspect_upgrade_manifest(&data, &future, UpgradeCheckpointLimits::default())
                .unwrap_err()
                .code,
            ErrorCode::SchemaError
        );

        let unsafe_path = UpgradeManifest {
            schema_version: UpgradeManifest::CURRENT_SCHEMA,
            stores: vec![UpgradeStoreSpec {
                relative_path: "../layout.json".into(),
                format: crate::PersistedDataFormat::Layout,
                optional: false,
            }],
        };
        assert_eq!(
            inspect_upgrade_manifest(&data, &unsafe_path, UpgradeCheckpointLimits::default())
                .unwrap_err()
                .code,
            ErrorCode::InvalidId
        );
        assert_eq!(fs::read(data.join("layout.json")).unwrap(), before);
        assert!(!tree.path("source-checkpoint").exists());
    }

    #[test]
    fn restore_recovery_finishes_after_destination_was_moved_and_is_idempotent() {
        let tree = TestTree::new();
        let destination = tree.path("data");
        fs::create_dir_all(&destination).unwrap();
        fs::write(destination.join("layout.json"), b"old").unwrap();
        let checkpoint = tree.path("checkpoint");
        create_upgrade_checkpoint(
            &destination,
            &checkpoint,
            UpgradeCheckpointLimits::default(),
        )
        .unwrap();
        fs::write(destination.join("layout.json"), b"current").unwrap();

        let checkpoint = fs::canonicalize(checkpoint).unwrap();
        let destination = fs::canonicalize(destination).unwrap();
        let parent = destination.parent().unwrap();
        let staging = parent.join("restore-stage");
        let manifest = stage_from_checkpoint(&checkpoint, &staging);
        let previous = parent.join("previous-tree");
        fs::rename(&destination, &previous).unwrap();
        let journal_path = restore_journal_path(&destination).unwrap();
        let journal = RestoreJournal {
            schema_version: RESTORE_JOURNAL_SCHEMA,
            checkpoint_root: checkpoint,
            destination_root: destination.clone(),
            staging_root: staging,
            replaced_root: Some(previous.clone()),
            tree_fingerprint: manifest.tree_fingerprint,
            file_count: manifest.files.len(),
            total_bytes: manifest.total_bytes,
            phase: RestorePhase::Prepared,
        };
        write_restore_journal(&journal_path, &journal).unwrap();

        let restored = recover_upgrade_restore(&destination, UpgradeCheckpointLimits::default())
            .unwrap()
            .unwrap();
        assert_eq!(fs::read(destination.join("layout.json")).unwrap(), b"old");
        assert_eq!(fs::read(previous.join("layout.json")).unwrap(), b"current");
        assert_eq!(restored.replaced_tree_path, Some(previous));
        assert!(
            recover_upgrade_restore(&destination, UpgradeCheckpointLimits::default())
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn restore_recovery_recognizes_publication_before_journal_update() {
        let tree = TestTree::new();
        let destination = tree.path("data");
        fs::create_dir_all(&destination).unwrap();
        fs::write(destination.join("layout.json"), b"old").unwrap();
        let checkpoint = tree.path("checkpoint");
        create_upgrade_checkpoint(
            &destination,
            &checkpoint,
            UpgradeCheckpointLimits::default(),
        )
        .unwrap();
        let checkpoint = fs::canonicalize(checkpoint).unwrap();
        let destination = fs::canonicalize(destination).unwrap();
        let parent = destination.parent().unwrap();
        let manifest =
            verify_checkpoint_tree(&checkpoint, UpgradeCheckpointLimits::default()).unwrap();
        let staging = parent.join("restore-stage");
        stage_from_checkpoint(&checkpoint, &staging);
        let previous = parent.join("previous-tree");
        fs::rename(&destination, &previous).unwrap();
        fs::rename(&staging, &destination).unwrap();
        let journal_path = restore_journal_path(&destination).unwrap();
        write_restore_journal(
            &journal_path,
            &RestoreJournal {
                schema_version: RESTORE_JOURNAL_SCHEMA,
                checkpoint_root: checkpoint,
                destination_root: destination.clone(),
                staging_root: staging,
                replaced_root: Some(previous),
                tree_fingerprint: manifest.tree_fingerprint,
                file_count: manifest.files.len(),
                total_bytes: manifest.total_bytes,
                phase: RestorePhase::DestinationMoved,
            },
        )
        .unwrap();

        assert!(
            recover_upgrade_restore(&destination, UpgradeCheckpointLimits::default())
                .unwrap()
                .is_some()
        );
        assert!(!path_entry_exists(&journal_path));
        assert_eq!(fs::read(destination.join("layout.json")).unwrap(), b"old");
    }

    fn legacy_layout_json() -> String {
        let text = include_str!("../tests/fixtures/layout_schema_v0.json");
        let mut value: serde_json::Value = serde_json::from_str(text).unwrap();
        let checksum = crate::checksum_layout_json(&value.to_string()).unwrap();
        value["checksum"] = serde_json::Value::String(checksum);
        value.to_string()
    }

    #[test]
    fn multi_file_upgrade_checkpoints_stages_publishes_and_repeats_as_noop() {
        let tree = TestTree::new();
        let data = tree.path("host-data");
        fs::create_dir_all(data.join("themes/assets")).unwrap();
        let old_layout = legacy_layout_json();
        fs::write(data.join("layout.json"), old_layout.as_bytes()).unwrap();
        let old_profiles = include_str!("../tests/fixtures/profile_catalog_schema_v0.json");
        fs::write(data.join("profiles.json"), old_profiles).unwrap();
        fs::write(data.join("preferences.json"), b"opaque-host-preferences").unwrap();
        fs::write(data.join("themes/assets/skin.bin"), b"user-skin-bytes").unwrap();

        let mut preferences = serde_json::json!({
            "schema_version": 1,
            "workspace_id": "workspace-1",
            "preferences": {"host.custom": {"keep": true}}
        });
        let checksum = crate::checksum_layout_json(&preferences.to_string()).unwrap();
        preferences["checksum"] = serde_json::Value::String(checksum);
        fs::write(
            data.join("settings.json"),
            preferences.to_string().as_bytes(),
        )
        .unwrap();

        let changes = vec![
            UpgradeFileChange {
                relative_path: "layout.json".into(),
                preview: preview_persisted_migration(PersistedDataFormat::Layout, &old_layout)
                    .unwrap(),
            },
            UpgradeFileChange {
                relative_path: "profiles.json".into(),
                preview: preview_persisted_migration(
                    PersistedDataFormat::ProfileCatalog,
                    old_profiles,
                )
                .unwrap(),
            },
            UpgradeFileChange {
                relative_path: "settings.json".into(),
                preview: preview_persisted_migration(
                    PersistedDataFormat::WorkspacePreferences,
                    &preferences.to_string(),
                )
                .unwrap(),
            },
        ];
        let source_checkpoint = tree.path("source-checkpoint");
        let first = apply_upgrade_plan(
            &data,
            &source_checkpoint,
            &changes,
            UpgradeCheckpointLimits::default(),
        )
        .unwrap();
        assert!(!first.already_applied);
        assert_eq!(first.applied_paths, ["layout.json", "profiles.json"]);
        assert!(first.source_checkpoint_path.as_ref().unwrap().exists());
        assert!(first.replaced_tree_path.as_ref().unwrap().exists());
        assert_eq!(
            fs::read(data.join("themes/assets/skin.bin")).unwrap(),
            b"user-skin-bytes"
        );
        assert_eq!(
            fs::read(data.join("preferences.json")).unwrap(),
            b"opaque-host-preferences"
        );
        let migrated_layout = crate::deserialize_layout(
            &fs::read_to_string(data.join("layout.json")).unwrap(),
            crate::PersistenceLimits::default(),
        )
        .unwrap();
        assert_eq!(migrated_layout.id.0, "legacy-layout");
        let migrated_root = migrated_layout.root.as_ref().unwrap();
        match &migrated_root.kind {
            crate::NodeKind::Extension {
                modality,
                payload,
                children,
            } => {
                assert_eq!(modality.0, "vendor.unknown-layout-v4");
                assert_eq!(
                    payload,
                    r#"{"future_extension":{"opaque":[1,{"keep":"exact"}]}}"#
                );
                assert_eq!(children[0].id.0, "legacy-extension-panel-node");
            }
            other => panic!("expected the opaque extension node, got {other:?}"),
        }
        assert_eq!(
            WorkspaceProfileCatalog::from_json(
                &fs::read_to_string(data.join("profiles.json")).unwrap(),
                8 * 1024 * 1024,
            )
            .unwrap()
            .schema_version,
            WorkspaceProfileCatalog::CURRENT_SCHEMA
        );

        let repeated = apply_upgrade_plan(
            &data,
            &source_checkpoint,
            &changes,
            UpgradeCheckpointLimits::default(),
        )
        .unwrap();
        assert!(repeated.already_applied);
        assert_eq!(
            repeated.target_tree_fingerprint,
            first.target_tree_fingerprint
        );
        let rollback = restore_upgrade_checkpoint(
            first.source_checkpoint_path.as_ref().unwrap(),
            &data,
            UpgradeCheckpointLimits::default(),
        )
        .unwrap();
        assert!(rollback.replaced_tree_path.unwrap().exists());
        assert_eq!(
            fs::read_to_string(data.join("layout.json")).unwrap(),
            legacy_layout_json()
        );
        assert_eq!(
            fs::read(data.join("themes/assets/skin.bin")).unwrap(),
            b"user-skin-bytes"
        );
    }

    #[test]
    fn upgrade_plan_rejects_stale_preview_before_checkpoint_or_write() {
        let tree = TestTree::new();
        let data = tree.path("host-data");
        fs::create_dir_all(&data).unwrap();
        let original = legacy_layout_json();
        let path = data.join("layout.json");
        fs::write(&path, &original).unwrap();
        let preview = preview_persisted_migration(PersistedDataFormat::Layout, &original).unwrap();
        fs::write(&path, format!("{original} ")).unwrap();
        let changed = fs::read(&path).unwrap();
        let checkpoint = tree.path("source-checkpoint");
        let error = apply_upgrade_plan(
            &data,
            &checkpoint,
            &[UpgradeFileChange {
                relative_path: "layout.json".into(),
                preview,
            }],
            UpgradeCheckpointLimits::default(),
        )
        .unwrap_err();
        assert_eq!(error.code, ErrorCode::Conflict);
        assert_eq!(fs::read(path).unwrap(), changed);
        assert!(!checkpoint.exists());
    }

    #[test]
    fn release_checkpoint_restores_application_and_data_together() {
        let tree = TestTree::new();
        let application = tree.path("application");
        let data = tree.path("data");
        fs::create_dir_all(&application).unwrap();
        fs::create_dir_all(&data).unwrap();
        fs::write(application.join("host.exe"), b"old-host").unwrap();
        fs::write(data.join("preferences.json"), b"old-preferences").unwrap();
        let checkpoint = tree.path("release-checkpoint");
        create_upgrade_release_checkpoint(
            &application,
            &data,
            &checkpoint,
            UpgradeCheckpointLimits::default(),
        )
        .unwrap();

        fs::write(application.join("host.exe"), b"new-host").unwrap();
        fs::write(application.join("new.dll"), b"new-binary").unwrap();
        fs::write(data.join("preferences.json"), b"new-preferences").unwrap();
        fs::write(data.join("new-state.json"), b"new-state").unwrap();

        let journal = tree.path("release-restore.json");
        let restored = restore_upgrade_release_checkpoint(
            &checkpoint,
            &application,
            &data,
            &journal,
            UpgradeCheckpointLimits::default(),
        )
        .unwrap();
        assert!(!restored.resumed);
        assert_eq!(fs::read(application.join("host.exe")).unwrap(), b"old-host");
        assert!(!application.join("new.dll").exists());
        assert_eq!(
            fs::read(data.join("preferences.json")).unwrap(),
            b"old-preferences"
        );
        assert!(!data.join("new-state.json").exists());
        assert!(!journal.exists());
        let replaced_app = restored.application.replaced_tree_path.unwrap();
        let replaced_data = restored.data.replaced_tree_path.unwrap();
        assert_eq!(
            fs::read(replaced_app.join("host.exe")).unwrap(),
            b"new-host"
        );
        assert_eq!(
            fs::read(replaced_data.join("preferences.json")).unwrap(),
            b"new-preferences"
        );
    }

    #[test]
    fn release_restore_recovers_after_each_coordinator_boundary() {
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        enum Boundary {
            Application,
            Data,
            Verified,
        }

        for boundary in [Boundary::Application, Boundary::Data, Boundary::Verified] {
            let tree = TestTree::new();
            let application = tree.path("application");
            let data = tree.path("data");
            fs::create_dir_all(&application).unwrap();
            fs::create_dir_all(&data).unwrap();
            fs::write(application.join("host.exe"), b"old-host").unwrap();
            fs::write(data.join("preferences.json"), b"old-preferences").unwrap();
            let checkpoint = tree.path("release-checkpoint");
            create_upgrade_release_checkpoint(
                &application,
                &data,
                &checkpoint,
                UpgradeCheckpointLimits::default(),
            )
            .unwrap();
            fs::write(application.join("host.exe"), b"new-host").unwrap();
            fs::write(data.join("preferences.json"), b"new-preferences").unwrap();

            let journal_file = tree.path("release-restore.json");
            let journal = ReleaseRestoreJournal {
                schema_version: RELEASE_RESTORE_JOURNAL_SCHEMA,
                checkpoint_root: fs::canonicalize(&checkpoint).unwrap(),
                application_destination: fs::canonicalize(&application).unwrap(),
                data_destination: fs::canonicalize(&data).unwrap(),
                application_replaced_tree: None,
                data_replaced_tree: None,
                phase: ReleaseRestorePhase::Prepared,
                checksum: String::new(),
            };
            write_release_restore_journal(&journal_file, &journal, false).unwrap();
            let interrupt = || {
                upgrade_error(
                    ErrorCode::PersistenceError,
                    "Injected process interruption at a release restore boundary.",
                    "test boundary",
                )
            };
            let interrupted = finish_release_restore_journal(
                &journal_file,
                journal,
                UpgradeCheckpointLimits::default(),
                false,
                || {
                    (boundary != Boundary::Application)
                        .then_some(())
                        .ok_or_else(interrupt)
                },
                || {
                    (boundary != Boundary::Data)
                        .then_some(())
                        .ok_or_else(interrupt)
                },
                || {
                    (boundary != Boundary::Verified)
                        .then_some(())
                        .ok_or_else(interrupt)
                },
            );
            assert!(interrupted.is_err(), "boundary {boundary:?}");
            assert!(journal_file.exists());

            let recovered =
                recover_upgrade_release_restore(&journal_file, UpgradeCheckpointLimits::default())
                    .unwrap()
                    .unwrap();
            assert!(recovered.resumed, "boundary {boundary:?}");
            assert_eq!(fs::read(application.join("host.exe")).unwrap(), b"old-host");
            assert_eq!(
                fs::read(data.join("preferences.json")).unwrap(),
                b"old-preferences"
            );
            assert!(!journal_file.exists());
        }
    }

    #[test]
    fn release_restore_catches_up_when_tree_publish_precedes_phase_commit() {
        for (phase, restore_application, restore_data) in [
            (ReleaseRestorePhase::Prepared, true, false),
            (ReleaseRestorePhase::ApplicationRestored, true, true),
        ] {
            let tree = TestTree::new();
            let application = tree.path("application");
            let data = tree.path("data");
            fs::create_dir_all(&application).unwrap();
            fs::create_dir_all(&data).unwrap();
            fs::write(application.join("host.exe"), b"old-host").unwrap();
            fs::write(data.join("preferences.json"), b"old-preferences").unwrap();
            let checkpoint = tree.path("release-checkpoint");
            create_upgrade_release_checkpoint(
                &application,
                &data,
                &checkpoint,
                UpgradeCheckpointLimits::default(),
            )
            .unwrap();
            fs::write(application.join("host.exe"), b"new-host").unwrap();
            fs::write(data.join("preferences.json"), b"new-preferences").unwrap();

            let limits = UpgradeCheckpointLimits::default();
            if restore_application {
                restore_upgrade_checkpoint(&checkpoint.join("application"), &application, limits)
                    .unwrap();
            }
            if restore_data {
                restore_upgrade_checkpoint(&checkpoint.join("data"), &data, limits).unwrap();
            }
            let journal_file = tree.path("release-restore.json");
            let journal = ReleaseRestoreJournal {
                schema_version: RELEASE_RESTORE_JOURNAL_SCHEMA,
                checkpoint_root: fs::canonicalize(&checkpoint).unwrap(),
                application_destination: fs::canonicalize(&application).unwrap(),
                data_destination: fs::canonicalize(&data).unwrap(),
                application_replaced_tree: None,
                data_replaced_tree: None,
                phase,
                checksum: String::new(),
            };
            write_release_restore_journal(&journal_file, &journal, false).unwrap();

            let recovered = recover_upgrade_release_restore(&journal_file, limits)
                .unwrap()
                .unwrap();
            assert!(recovered.resumed);
            assert_eq!(fs::read(application.join("host.exe")).unwrap(), b"old-host");
            assert_eq!(
                fs::read(data.join("preferences.json")).unwrap(),
                b"old-preferences"
            );
            assert!(!journal_file.exists());
        }
    }

    #[test]
    fn release_restore_rejects_corrupt_bundle_before_touching_either_tree() {
        let tree = TestTree::new();
        let application = tree.path("application");
        let data = tree.path("data");
        fs::create_dir_all(&application).unwrap();
        fs::create_dir_all(&data).unwrap();
        fs::write(application.join("host.exe"), b"current-host").unwrap();
        fs::write(data.join("preferences.json"), b"current-preferences").unwrap();
        let checkpoint = tree.path("release-checkpoint");
        create_upgrade_release_checkpoint(
            &application,
            &data,
            &checkpoint,
            UpgradeCheckpointLimits::default(),
        )
        .unwrap();
        fs::write(application.join("host.exe"), b"running-host").unwrap();
        fs::write(data.join("preferences.json"), b"running-preferences").unwrap();
        fs::write(checkpoint.join("application/host.exe"), b"tampered").unwrap();

        let error = restore_upgrade_release_checkpoint(
            &checkpoint,
            &application,
            &data,
            &tree.path("release-restore.json"),
            UpgradeCheckpointLimits::default(),
        )
        .unwrap_err();
        assert_eq!(error.code, ErrorCode::SerializationError);
        assert_eq!(
            fs::read(application.join("host.exe")).unwrap(),
            b"running-host"
        );
        assert_eq!(
            fs::read(data.join("preferences.json")).unwrap(),
            b"running-preferences"
        );
        assert!(!tree.path("release-restore.json").exists());
    }

    #[test]
    fn release_rollback_rejects_overlapping_roots_and_destination_local_journal() {
        let tree = TestTree::new();
        let application = tree.path("application");
        let nested_data = application.join("data");
        fs::create_dir_all(&nested_data).unwrap();
        let overlap_error = create_upgrade_release_checkpoint(
            &application,
            &nested_data,
            &tree.path("invalid-release-checkpoint"),
            UpgradeCheckpointLimits::default(),
        )
        .unwrap_err();
        assert_eq!(overlap_error.code, ErrorCode::InvalidId);
        assert!(!tree.path("invalid-release-checkpoint").exists());

        let data = tree.path("data");
        fs::create_dir_all(&data).unwrap();
        fs::write(application.join("host.exe"), b"old-host").unwrap();
        fs::write(data.join("preferences.json"), b"old-preferences").unwrap();
        let checkpoint = tree.path("release-checkpoint");
        create_upgrade_release_checkpoint(
            &application,
            &data,
            &checkpoint,
            UpgradeCheckpointLimits::default(),
        )
        .unwrap();
        fs::write(application.join("host.exe"), b"current-host").unwrap();
        fs::write(data.join("preferences.json"), b"current-preferences").unwrap();

        let nested_journal = application.join("recovery.json");
        let journal_error = restore_upgrade_release_checkpoint(
            &checkpoint,
            &application,
            &data,
            &nested_journal,
            UpgradeCheckpointLimits::default(),
        )
        .unwrap_err();
        assert_eq!(journal_error.code, ErrorCode::InvalidId);
        assert_eq!(
            fs::read(application.join("host.exe")).unwrap(),
            b"current-host"
        );
        assert_eq!(
            fs::read(data.join("preferences.json")).unwrap(),
            b"current-preferences"
        );
        assert!(!nested_journal.exists());
    }
}
