use agent_orbit::{config, model::QuotaSnapshot, store::Store};
#[test]
fn settings_survive_restart_and_override_file_defaults() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("state.db");
    let cfg = config::load().unwrap();
    {
        let s = Store::open(&path, &cfg).unwrap();
        let mut settings = cfg.settings.clone();
        settings.refresh_seconds = 75;
        s.save_settings(&settings).unwrap();
    }
    let mut next = cfg.clone();
    next.settings.refresh_seconds = 300;
    let s = Store::open(&path, &next).unwrap();
    assert_eq!(s.load_settings(&next.settings).unwrap().refresh_seconds, 75);
}
#[test]
fn profile_change_invalidates_prior_account_snapshot() {
    let t = tempfile::tempdir().unwrap();
    let cfg = config::load().unwrap();
    let s = Store::open(&t.path().join("s.db"), &cfg).unwrap();
    s.save_snapshot(&QuotaSnapshot::state("codex", "ready", "old account"))
        .unwrap();
    let mut p = s
        .profiles()
        .unwrap()
        .into_iter()
        .find(|p| p.id == "codex")
        .unwrap();
    p.config_dir = "/new-account".into();
    s.save_profile(&p).unwrap();
    assert!(s.snapshot("codex").unwrap().is_none());
}
#[test]
fn invalid_poll_interval_cannot_be_persisted() {
    let t = tempfile::tempdir().unwrap();
    let cfg = config::load().unwrap();
    let s = Store::open(&t.path().join("s.db"), &cfg).unwrap();
    let mut settings = cfg.settings;
    settings.refresh_seconds = 0;
    assert!(s.save_settings(&settings).is_err());
}
#[test]
fn old_account_cannot_write_after_profile_changed() {
    use agent_orbit::model::QuotaSnapshot;
    let t = tempfile::tempdir().unwrap();
    let cfg = config::load().unwrap();
    let s = Store::open(&t.path().join("s.db"), &cfg).unwrap();
    let old = s.profiles().unwrap().remove(0);
    let mut edited = old.clone();
    edited.config_dir = "account-B".into();
    s.save_profile(&edited).unwrap();
    assert!(s
        .save_bound_snapshot(&QuotaSnapshot::state(&old.id, "ready", "account A"), &old)
        .is_err());
    assert!(s.snapshot(&old.id).unwrap().is_none());
}
#[test]
fn legacy_dock_defaults_migrate_without_losing_other_preferences() {
    let t = tempfile::tempdir().unwrap();
    let cfg = config::load().unwrap();
    let path = t.path().join("legacy.db");
    let store = Store::open(&path, &cfg).unwrap();
    let db = rusqlite::Connection::open(&path).unwrap();
    db.execute(
        "INSERT INTO settings(id,value) VALUES(1,?1)",
        [r#"{"dockSize":60,"locale":"en","dockSide":"left","dockEdgeInset":12}"#],
    )
    .unwrap();
    let settings = store.load_settings(&cfg.settings).unwrap();
    assert_eq!(settings.dock_size, cfg.settings.dock_size);
    assert!(settings.dock_hover_size >= settings.dock_size);
    assert_eq!(settings.locale, "en");
    assert_eq!(settings.dock_side, "left");
    assert_eq!(settings.dock_edge_inset, cfg.settings.dock_edge_inset);
    db.execute(
        "UPDATE settings SET value=?1 WHERE id=1",
        [r#"{"dockSize":80,"dockEdgeInset":28}"#],
    )
    .unwrap();
    let custom = store.load_settings(&cfg.settings).unwrap();
    assert_eq!(custom.dock_size, 80.0);
    assert_eq!(custom.dock_hover_size, 80.0);
    assert_eq!(custom.dock_edge_inset, 28.0);
}
#[test]
fn dock_sizes_are_persisted_and_invalid_hover_size_is_rejected() {
    let t = tempfile::tempdir().unwrap();
    let cfg = config::load().unwrap();
    let store = Store::open(&t.path().join("sizes.db"), &cfg).unwrap();
    let mut settings = cfg.settings.clone();
    settings.dock_size = 28.0;
    settings.dock_locked = true;
    settings.dock_hover_size = 48.0;
    store.save_settings(&settings).unwrap();
    let loaded = store.load_settings(&cfg.settings).unwrap();
    assert!(loaded.dock_locked);
    assert_eq!((loaded.dock_size, loaded.dock_hover_size), (28.0, 48.0));
    settings.dock_hover_size = 20.0;
    assert!(store.save_settings(&settings).is_err());
}
