use agent_orbit::{config, external_sessions::scan_in, model::Profile};
use rusqlite::Connection;
fn profile() -> Profile {
    Profile {
        id: "codex".into(),
        provider_id: "codex".into(),
        label: "Personal".into(),
        executable: String::new(),
        args: vec![],
        config_dir: String::new(),
        quota_command: String::new(),
        quota_args: vec![],
        quota_file: String::new(),
        enabled: true,
    }
}
#[test]
fn existing_cli_and_desktop_threads_are_imported_as_read_only() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join(".codex");
    std::fs::create_dir_all(&root).unwrap();
    let db = Connection::open(root.join("state_5.sqlite")).unwrap();
    db.execute_batch("CREATE TABLE threads(id TEXT,cwd TEXT,title TEXT,source TEXT,created_at INTEGER,updated_at INTEGER,archived INTEGER);INSERT INTO threads VALUES('cli-id','/projects/cli','CLI task','cli',100,200,0);INSERT INTO threads VALUES('desktop-id','/projects/app','Desktop task','desktop',100,300,0);INSERT INTO threads VALUES('archived','/old','Old task','cli',1,2,1);").unwrap();
    drop(db);
    let provider = config::load()
        .unwrap()
        .providers
        .into_iter()
        .find(|p| p.id == "codex")
        .unwrap();
    let rows = scan_in(&profile(), &provider, 10, temp.path());
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].title, "Desktop task");
    assert!(rows
        .iter()
        .all(|r| r.read_only && r.mode == "external" && r.status != "running"));
    assert_eq!(rows[1].cwd, "/projects/cli");
}
#[test]
fn session_metadata_falls_back_to_jsonl_without_reading_conversation_body() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join(".codex/sessions/2026/10/25");
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(root.join("rollout-test.jsonl"),"{\"type\":\"session_meta\",\"timestamp\":\"2026-10-25T00:00:00Z\",\"payload\":{\"id\":\"thread-id\",\"cwd\":\"/project\",\"source\":\"cli\"}}\nNOT_VALID_JSON_CONVERSATION_BODY").unwrap();
    let provider = config::load()
        .unwrap()
        .providers
        .into_iter()
        .find(|p| p.id == "codex")
        .unwrap();
    let rows = scan_in(&profile(), &provider, 10, temp.path());
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].cwd, "/project");
    assert!(rows[0].read_only);
}
#[test]
fn configured_account_directory_is_used_instead_of_default_account() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("other");
    std::fs::create_dir_all(&root).unwrap();
    let db = Connection::open(root.join("state_5.sqlite")).unwrap();
    db.execute_batch("CREATE TABLE threads(id TEXT,cwd TEXT,source TEXT,created_at INTEGER,updated_at INTEGER);INSERT INTO threads VALUES('other','/other','cli',1,2);").unwrap();
    drop(db);
    let mut p = profile();
    p.config_dir = root.to_string_lossy().into_owned();
    let provider = config::load()
        .unwrap()
        .providers
        .into_iter()
        .find(|p| p.id == "codex")
        .unwrap();
    let rows = scan_in(&p, &provider, 10, temp.path());
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].cwd, "/other");
}
#[test]
fn an_empty_authoritative_index_does_not_resurrect_archived_logs() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join(".codex");
    std::fs::create_dir_all(root.join("sessions/2026/10/25")).unwrap();
    let db = Connection::open(root.join("state_5.sqlite")).unwrap();
    db.execute_batch("CREATE TABLE threads(id TEXT,cwd TEXT,source TEXT,created_at INTEGER,updated_at INTEGER,archived INTEGER);INSERT INTO threads VALUES('archived','/old','cli',1,2,1);").unwrap();
    drop(db);
    std::fs::write(
        root.join("sessions/2026/10/25/old.jsonl"),
        "{\"type\":\"session_meta\",\"payload\":{\"id\":\"archived\",\"cwd\":\"/old\"}}\n",
    )
    .unwrap();
    let provider = config::load()
        .unwrap()
        .providers
        .into_iter()
        .find(|p| p.id == "codex")
        .unwrap();
    assert!(scan_in(&profile(), &provider, 10, temp.path()).is_empty());
}
#[test]
fn bracketed_account_directory_is_literal() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("work[1]");
    std::fs::create_dir_all(&root).unwrap();
    let db = Connection::open(root.join("state_5.sqlite")).unwrap();
    db.execute_batch("CREATE TABLE threads(id TEXT,cwd TEXT,source TEXT,created_at INTEGER,updated_at INTEGER);INSERT INTO threads VALUES('literal','/correct','cli',1,2);").unwrap();
    drop(db);
    let mut p = profile();
    p.config_dir = root.to_string_lossy().into_owned();
    let provider = config::load()
        .unwrap()
        .providers
        .into_iter()
        .find(|p| p.id == "codex")
        .unwrap();
    let rows = scan_in(&p, &provider, 10, temp.path());
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].cwd, "/correct");
}
#[test]
fn inherited_account_home_overrides_default_session_directory() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("work-account");
    std::fs::create_dir_all(&root).unwrap();
    let db = Connection::open(root.join("state_5.sqlite")).unwrap();
    db.execute_batch("CREATE TABLE threads(id TEXT,cwd TEXT,source TEXT,created_at INTEGER,updated_at INTEGER);INSERT INTO threads VALUES('work','/work','cli',1,2);").unwrap();
    drop(db);
    let provider = config::load()
        .unwrap()
        .providers
        .into_iter()
        .find(|p| p.id == "codex")
        .unwrap();
    let env = std::collections::HashMap::from([(
        "CODEX_HOME".into(),
        root.to_string_lossy().into_owned(),
    )]);
    let rows = agent_orbit::external_sessions::scan_with_environment(
        &profile(),
        &provider,
        10,
        temp.path(),
        &env,
    );
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].cwd, "/work");
}
