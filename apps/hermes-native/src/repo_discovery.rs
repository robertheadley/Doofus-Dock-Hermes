//! Bounded filesystem scanner for Hermes Desktop's configured repo roots.
//!
//! This module never guesses a root. Callers must pass the active profile's
//! configured paths; an empty list is an intentional no-op.

use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs::{self, File},
    path::{Component, Path, PathBuf},
};

const DEFAULT_MAX_DEPTH: usize = 3;
const JUNK_DIRS: &[&str] = &[
    "Applications",
    "Library",
    "node_modules",
    "site-packages",
    "vendor",
    "venv",
];
const MACOS_MEDIA_ROOTS: &[&str] = &["Movies", "Music", "Pictures", "Public"];
const MACOS_LIBRARY_SUFFIXES: &[&str] = &[
    ".photoslibrary",
    ".musiclibrary",
    ".tvlibrary",
    ".aplibrary",
];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiscoveredRepo {
    pub label: String,
    pub root: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct RepoScanPolicy {
    pub enabled: bool,
    pub roots: Vec<String>,
    pub exclude_paths: Vec<String>,
}

impl Default for RepoScanPolicy {
    fn default() -> Self {
        Self {
            enabled: true,
            roots: Vec::new(),
            exclude_paths: Vec::new(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct RepoScanOptions {
    pub policy: RepoScanPolicy,
    pub home_dir: PathBuf,
    pub max_depth: usize,
}

impl RepoScanOptions {
    pub fn new(policy: RepoScanPolicy, home_dir: PathBuf) -> Self {
        Self {
            policy,
            home_dir,
            max_depth: DEFAULT_MAX_DEPTH,
        }
    }
}

/// Discover valid Git roots using only explicitly configured paths. Filesystem
/// read failures are skipped, matching the pinned Desktop scanner.
pub fn scan_git_repos(options: &RepoScanOptions) -> Vec<DiscoveredRepo> {
    if !options.policy.enabled || options.policy.roots.is_empty() {
        return Vec::new();
    }

    let max_depth = options.max_depth.min(DEFAULT_MAX_DEPTH);
    let roots = options
        .policy
        .roots
        .iter()
        .filter_map(|raw| normalize_repo_scan_path(raw, &options.home_dir))
        .collect::<BTreeMap<_, _>>();
    let exclusions = options
        .policy
        .exclude_paths
        .iter()
        .filter_map(|raw| normalize_repo_scan_path(raw, &options.home_dir))
        .map(|(key, path)| (key, path))
        .collect::<Vec<_>>();

    let mut found = BTreeMap::<String, DiscoveredRepo>::new();

    for (_root_key, root) in roots {
        if is_excluded(&root, &exclusions) {
            continue;
        }
        let mut pending = vec![(root, 0usize)];
        while let Some((directory, depth)) = pending.pop() {
            if is_excluded(&directory, &exclusions) || !directory.is_dir() {
                continue;
            }

            if has_git_directory(&directory) {
                if is_valid_git_root(&directory) {
                    let key = normalized_key(&directory);
                    let label = directory
                        .file_name()
                        .map(|name| name.to_string_lossy().into_owned())
                        .filter(|name| !name.is_empty())
                        .unwrap_or_else(|| directory.to_string_lossy().into_owned());
                    found.insert(
                        key,
                        DiscoveredRepo {
                            label,
                            root: directory.to_string_lossy().into_owned(),
                        },
                    );
                }
                continue; // Preserve nested repositories as their own boundary.
            }
            if depth >= max_depth {
                continue;
            }

            let Ok(entries) = fs::read_dir(&directory) else {
                continue;
            };
            let mut children = entries
                .filter_map(Result::ok)
                .filter_map(|entry| {
                    let file_type = entry.file_type().ok()?;
                    if !file_type.is_dir() {
                        return None;
                    }
                    let name = entry.file_name().to_string_lossy().into_owned();
                    if name.starts_with('.') || JUNK_DIRS.contains(&name.as_str()) {
                        return None;
                    }
                    if cfg!(target_os = "macos")
                        && depth == 0
                        && MACOS_MEDIA_ROOTS.contains(&name.as_str())
                    {
                        return None;
                    }
                    if cfg!(target_os = "macos")
                        && MACOS_LIBRARY_SUFFIXES
                            .iter()
                            .any(|suffix| name.to_lowercase().ends_with(suffix))
                    {
                        return None;
                    }
                    let path = entry.path();
                    (!is_excluded(&path, &exclusions)).then_some(path)
                })
                .collect::<Vec<_>>();
            children.sort_by_key(|path| normalized_key(path));
            pending.extend(children.into_iter().rev().map(|child| (child, depth + 1)));
        }
    }
    found.into_values().collect()
}

/// Expand only the source-supported tilde forms and resolve relative roots
/// against the user's home directory. Returns a comparison key and path.
pub fn normalize_repo_scan_path(raw: &str, home_dir: &Path) -> Option<(String, PathBuf)> {
    let raw = raw.trim();
    if raw.is_empty() {
        return None;
    }
    let expanded = if raw == "~" {
        home_dir.to_path_buf()
    } else if let Some(rest) = raw.strip_prefix("~/").or_else(|| raw.strip_prefix("~\\")) {
        home_dir.join(rest)
    } else {
        PathBuf::from(raw)
    };
    let absolute = if expanded.is_absolute() {
        expanded
    } else {
        home_dir.join(expanded)
    };
    let normalized = normalize_components(&absolute);
    Some((normalized_key(&normalized), normalized))
}

fn normalize_components(path: &Path) -> PathBuf {
    let mut output = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                if !matches!(
                    output.components().next_back(),
                    Some(Component::RootDir | Component::Prefix(_))
                ) {
                    output.pop();
                }
            }
            other => output.push(other.as_os_str()),
        }
    }
    output
}

fn normalized_key(path: &Path) -> String {
    let value = normalize_components(path).to_string_lossy().into_owned();
    if cfg!(windows) {
        value.to_lowercase()
    } else {
        value
    }
}

fn path_is_within(candidate: &Path, parent: &Path) -> bool {
    let candidate = normalize_components(candidate);
    let parent = normalize_components(parent);
    let mut candidate_components = candidate.components();
    for parent_component in parent.components() {
        let Some(candidate_component) = candidate_components.next() else {
            return false;
        };
        let left = candidate_component.as_os_str().to_string_lossy();
        let right = parent_component.as_os_str().to_string_lossy();
        if cfg!(windows) {
            if !left.eq_ignore_ascii_case(&right) {
                return false;
            }
        } else if left != right {
            return false;
        }
    }
    true
}

fn is_excluded(path: &Path, exclusions: &[(String, PathBuf)]) -> bool {
    exclusions
        .iter()
        .any(|(_, excluded)| path_is_within(path, excluded))
}

fn has_git_directory(path: &Path) -> bool {
    fs::symlink_metadata(path.join(".git")).is_ok_and(|metadata| metadata.file_type().is_dir())
}

fn is_valid_git_root(path: &Path) -> bool {
    let git_dir = path.join(".git");
    File::open(git_dir.join("HEAD")).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_FIXTURE_ID: AtomicU64 = AtomicU64::new(0);

    fn fixture(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "hermes-repo-{name}-{}-{}",
            std::process::id(),
            NEXT_FIXTURE_ID.fetch_add(1, Ordering::Relaxed),
        ))
    }

    fn make_repo(path: &Path, valid: bool) {
        fs::create_dir_all(path.join(".git")).unwrap();
        if valid {
            fs::write(path.join(".git").join("HEAD"), "ref: refs/heads/main\n").unwrap();
        }
    }

    #[test]
    fn empty_or_disabled_policy_never_scans_or_guesses_home() {
        let home = PathBuf::from("a-home-that-does-not-exist");
        let empty = RepoScanOptions::new(
            RepoScanPolicy {
                enabled: true,
                roots: vec![],
                exclude_paths: vec![],
            },
            home.clone(),
        );
        assert!(scan_git_repos(&empty).is_empty());
        let disabled = RepoScanOptions::new(
            RepoScanPolicy {
                enabled: false,
                roots: vec![home.to_string_lossy().into_owned()],
                exclude_paths: vec![],
            },
            home,
        );
        assert!(scan_git_repos(&disabled).is_empty());
    }

    #[test]
    fn resolves_tilde_relative_and_parent_components_against_home() {
        let home = PathBuf::from(if cfg!(windows) {
            "C:/Users/tester"
        } else {
            "/home/tester"
        });
        let (_, tilde) = normalize_repo_scan_path("~/src/../code", &home).unwrap();
        let (_, relative) = normalize_repo_scan_path("code", &home).unwrap();
        assert_eq!(tilde, relative);
    }

    #[test]
    fn scans_only_configured_roots_and_prunes_exclusions_and_junk() {
        let temp = fixture("scan");
        let _ = fs::remove_dir_all(&temp);
        let root = temp.join("root");
        let include = root.join("include");
        let exclude = root.join("excluded").join("nested");
        let junk = root.join("vendor").join("hidden-repo");
        make_repo(&include, true);
        make_repo(&exclude, true);
        make_repo(&junk, true);
        let options = RepoScanOptions::new(
            RepoScanPolicy {
                enabled: true,
                roots: vec![root.to_string_lossy().into_owned()],
                exclude_paths: vec![root.join("excluded").to_string_lossy().into_owned()],
            },
            temp.clone(),
        );
        let repos = scan_git_repos(&options);
        assert_eq!(
            repos,
            vec![DiscoveredRepo {
                label: "include".into(),
                root: include.to_string_lossy().into_owned(),
            }]
        );
        let _ = fs::remove_dir_all(temp);
    }

    #[test]
    fn deduplicates_overlapping_roots_and_stops_at_nested_repo_boundaries() {
        let temp = fixture("boundary");
        let _ = fs::remove_dir_all(&temp);
        let root = temp.join("root");
        let outer = root.join("outer");
        let nested = outer.join("nested");
        make_repo(&outer, true);
        make_repo(&nested, true);
        let options = RepoScanOptions::new(
            RepoScanPolicy {
                enabled: true,
                roots: vec![
                    root.to_string_lossy().into_owned(),
                    outer.to_string_lossy().into_owned(),
                ],
                exclude_paths: vec![],
            },
            temp.clone(),
        );
        assert_eq!(
            scan_git_repos(&options),
            vec![DiscoveredRepo {
                label: "outer".into(),
                root: outer.to_string_lossy().into_owned(),
            }]
        );
        let _ = fs::remove_dir_all(temp);
    }

    #[test]
    fn rejects_malformed_git_metadata_and_respects_depth_three() {
        let temp = fixture("depth");
        let _ = fs::remove_dir_all(&temp);
        let root = temp.join("root");
        make_repo(&root.join("bad"), false);
        make_repo(&root.join("bad").join("nested-valid"), true);
        let within = root.join("one").join("two").join("three");
        let outside = root.join("one").join("two").join("three").join("four");
        make_repo(&within, true);
        make_repo(&outside, true);
        let options = RepoScanOptions::new(
            RepoScanPolicy {
                enabled: true,
                roots: vec![root.to_string_lossy().into_owned()],
                exclude_paths: vec![],
            },
            temp.clone(),
        );
        assert_eq!(
            scan_git_repos(&options),
            vec![DiscoveredRepo {
                label: "three".into(),
                root: within.to_string_lossy().into_owned(),
            }]
        );
        let _ = fs::remove_dir_all(temp);
    }

    #[cfg(unix)]
    #[test]
    fn git_directory_symlink_does_not_become_a_repository() {
        use std::os::unix::fs::symlink;

        let temp = fixture("symlink");
        let root = temp.join("root");
        let target = temp.join("git-metadata-target");
        fs::create_dir_all(&root).unwrap();
        fs::create_dir_all(&target).unwrap();
        fs::write(target.join("HEAD"), "ref: refs/heads/main\n").unwrap();
        symlink(&target, root.join(".git")).unwrap();
        let options = RepoScanOptions::new(
            RepoScanPolicy {
                enabled: true,
                roots: vec![root.to_string_lossy().into_owned()],
                exclude_paths: vec![],
            },
            temp.clone(),
        );
        assert!(scan_git_repos(&options).is_empty());
        let _ = fs::remove_dir_all(temp);
    }

    #[test]
    fn exclusion_uses_path_components_and_windows_case_folding() {
        let parent = if cfg!(windows) {
            PathBuf::from("C:/work/vendor")
        } else {
            PathBuf::from("/work/vendor")
        };
        let child = parent.join("repo");
        let sibling = if cfg!(windows) {
            PathBuf::from("c:/work/vendorized/repo")
        } else {
            PathBuf::from("/work/vendorized/repo")
        };
        assert!(path_is_within(&child, &parent));
        assert!(!path_is_within(&sibling, &parent));
    }
}
