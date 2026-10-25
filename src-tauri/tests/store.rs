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
