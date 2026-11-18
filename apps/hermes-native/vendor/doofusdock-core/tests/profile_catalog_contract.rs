use doofusdock_core::*;

#[test]
fn declarative_spec_round_trips_and_rejects_unknown_schema() {
    let profile = builtin_workspace_profiles().remove(0);
    let spec = WorkspaceSpecification::new(profile.clone()).unwrap();
    let json = spec.to_json().unwrap();
    let restored = WorkspaceSpecification::from_json(&json, 1_000_000).unwrap();
    assert_eq!(restored, spec);
    assert_eq!(validate_profile(&restored.profile), Ok(()));
    assert_eq!(
        WorkspaceSpecification::from_json(&json, 1),
        Err(DockError::new(
            ErrorCode::ResourceLimit,
            "Workspace specification exceeds the configured byte limit.",
            json.len().to_string(),
        ))
    );

    let mut invalid = spec;
    invalid.schema_version += 1;
    assert_eq!(
        validate_workspace_specification(&invalid).unwrap_err().code,
        ErrorCode::SchemaError
    );
}

#[test]
fn profile_catalog_schema_zero_migrates_to_current_schema() {
    let profile = builtin_workspace_profiles().remove(0);
    let legacy = serde_json::json!({"schema_version": 0, "profiles": [profile]});
    let catalog = WorkspaceProfileCatalog::from_json(&legacy.to_string(), 1_000_000).unwrap();
    assert_eq!(
        catalog.schema_version,
        WorkspaceProfileCatalog::CURRENT_SCHEMA
    );
    assert_eq!(catalog.profiles.len(), 1);
}

#[test]
fn profile_catalog_lifecycle_is_validated_and_persisted_atomically() {
    let mut defaults = builtin_workspace_profiles();
    let original = defaults[0].clone();
    let mut catalog = WorkspaceProfileCatalog::from_profiles(defaults.clone()).unwrap();
    catalog.set_default(Some(&original.id)).unwrap();
    catalog
        .duplicate(&original.id, "user-copy", "User copy")
        .unwrap();
    catalog
        .rename("user-copy", "renamed-copy", "Renamed copy")
        .unwrap();
    assert_eq!(
        catalog.default_profile.as_deref(),
        Some(original.id.as_str())
    );
    let exported = catalog.export("renamed-copy").unwrap();

    let mut imported = WorkspaceProfileCatalog::default();
    imported.import(&exported, 1_000_000).unwrap();
    assert_eq!(imported.profiles["renamed-copy"].title_key, "Renamed copy");
    assert!(
        imported
            .compare("renamed-copy", "renamed-copy")
            .unwrap()
            .same
    );

    let root = std::env::temp_dir().join(format!(
        "doofusdock-profile-catalog-{}-{}",
        std::process::id(),
        original
            .id
            .replace(|c: char| !c.is_ascii_alphanumeric(), "-")
    ));
    let store = WorkspacePreferencesStore::new(&root, PersistenceLimits::default());
    let workspace = WorkspaceId::from("profile-catalog-contract");
    save_profile_catalog(&store, &workspace, &catalog).unwrap();
    let loaded = load_profile_catalog(&store, &workspace).unwrap();
    assert_eq!(loaded, catalog);

    let primary = root.join("profile-catalog-contract.json");
    let prior_good_bytes = std::fs::read(&primary).unwrap();
    let mut newer = loaded.clone();
    newer.set_default(Some("renamed-copy")).unwrap();
    save_profile_catalog(&store, &workspace, &newer).unwrap();
    let backup = root.join("profile-catalog-contract.json.bak");
    assert_eq!(std::fs::read(&backup).unwrap(), prior_good_bytes);
    std::fs::write(&primary, b"corrupt profile catalog preferences").unwrap();
    store.recover(&workspace).unwrap();
    assert_eq!(load_profile_catalog(&store, &workspace).unwrap(), loaded);

    let current_default = loaded.default_profile.clone().unwrap();
    let mut guarded = loaded.clone();
    assert_eq!(
        guarded.remove(&current_default).unwrap_err().code,
        ErrorCode::ConstraintViolation
    );
    guarded.reset(std::mem::take(&mut defaults)).unwrap();
    assert_eq!(guarded.profiles.len(), 71);
    let _ = std::fs::remove_dir_all(root);
}
