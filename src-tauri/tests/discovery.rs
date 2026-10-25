use agent_orbit::{config, discovery::discover_in, model::Profile};
use std::{collections::HashMap, ffi::OsStr, path::Path};
fn profile() -> Profile {
    Profile {
        id: "codex".into(),
        provider_id: "codex".into(),
        label: "Codex".into(),
        executable: String::new(),
        args: vec![],
        config_dir: String::new(),
        quota_command: String::new(),
        quota_args: vec![],
        quota_file: String::new(),
        enabled: true,
    }
}
fn executable(path: &Path) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, "#!/bin/sh\nexit 0\n").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
}
#[cfg(unix)]
#[test]
fn nvm_install_is_found_without_a_shell_path() {
    let temp = tempfile::tempdir().unwrap();
    let exe = temp.path().join(".nvm/versions/node/v24.20.0/bin/codex");
    executable(&exe);
    let mut provider = config::load()
        .unwrap()
        .providers
        .into_iter()
        .find(|p| p.id == "codex")
        .unwrap();
    provider.executable_paths = vec!["~/.nvm/versions/node/*/bin/codex".into()];
    let result = discover_in(
        &profile(),
        &provider,
        temp.path(),
        Some(OsStr::new("")),
        &HashMap::new(),
    );
    assert_eq!(result.executable.as_deref(), Some(exe.to_str().unwrap()));
}
#[cfg(unix)]
#[test]
fn newest_nvm_version_is_selected_numerically() {
    let temp = tempfile::tempdir().unwrap();
    for version in ["v24.9.0", "v24.20.0"] {
        executable(
            &temp
                .path()
                .join(format!(".nvm/versions/node/{version}/bin/codex")),
        );
    }
    let mut provider = config::load()
        .unwrap()
        .providers
        .into_iter()
        .find(|p| p.id == "codex")
        .unwrap();
    provider.executable_paths = vec!["~/.nvm/versions/node/*/bin/codex".into()];
    let result = discover_in(
        &profile(),
        &provider,
        temp.path(),
        Some(OsStr::new("")),
        &HashMap::new(),
    );
    assert!(result.executable.unwrap().contains("v24.20.0"));
}
#[test]
fn desktop_only_installation_is_distinct_from_cli() {
    let temp = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(temp.path().join("Applications/Codex.app")).unwrap();
    let mut provider = config::load()
        .unwrap()
        .providers
        .into_iter()
        .find(|p| p.id == "codex")
        .unwrap();
    provider.desktop_paths = vec!["~/Applications/Codex.app".into()];
    provider.executable_paths = vec![];
    let result = discover_in(
        &profile(),
        &provider,
        temp.path(),
        Some(OsStr::new("")),
        &HashMap::new(),
    );
    assert!(result.installed && result.desktop_found);
    assert!(result.executable.is_none());
}
#[test]
fn invalid_explicit_path_is_not_replaced_by_another_account_install() {
    let temp = tempfile::tempdir().unwrap();
    let exe = temp.path().join("bin/codex");
    executable(&exe);
    let mut p = profile();
    p.executable = temp
        .path()
        .join("missing/codex")
        .to_string_lossy()
        .into_owned();
    let provider = config::load()
        .unwrap()
        .providers
        .into_iter()
        .find(|p| p.id == "codex")
        .unwrap();
    let result = discover_in(
        &p,
        &provider,
        temp.path(),
        Some(temp.path().join("bin").as_os_str()),
        &HashMap::new(),
    );
    assert!(result.executable.is_none());
}
