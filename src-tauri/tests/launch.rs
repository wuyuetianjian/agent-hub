use agent_orbit::launch::resolve;
#[test]
fn npm_batch_shim_resolves_to_node_and_script_without_shell_interpolation() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    std::fs::create_dir_all(root.join("node_modules/vendor")).unwrap();
    std::fs::write(root.join("node.exe"), "").unwrap();
    std::fs::write(root.join("node_modules/vendor/cli.js"), "").unwrap();
    let shim = root.join("agent.cmd");
    std::fs::write(
        &shim,
        r#"@ECHO off
"%_prog%" "%dp0%\node_modules\vendor\cli.js" %*
"#,
    )
    .unwrap();
    let (program, args) =
        resolve(shim.to_str().unwrap(), &["path with spaces & text".into()]).unwrap();
    assert_eq!(program, root.join("node.exe").to_string_lossy());
    assert_eq!(
        std::path::Path::new(&args[0]),
        root.join("node_modules/vendor/cli.js")
    );
    assert_eq!(args[1], "path with spaces & text");
}
#[test]
fn arbitrary_batch_file_is_not_executed_as_a_shell_command() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("custom.cmd");
    std::fs::write(&file, "echo unsupported").unwrap();
    assert!(resolve(file.to_str().unwrap(), &[]).is_err());
}
#[cfg(unix)]
#[test]
fn unix_node_launcher_uses_its_sibling_node() {
    let dir = tempfile::tempdir().unwrap();
    let cli = dir.path().join("codex");
    let node = dir.path().join("node");
    std::fs::write(&cli, "#!/usr/bin/env node\nconsole.log('test')").unwrap();
    std::fs::write(&node, "").unwrap();
    let (program, args) = resolve(cli.to_str().unwrap(), &["app-server".into()]).unwrap();
    assert_eq!(program, node.to_string_lossy());
    assert_eq!(
        args,
        vec![cli.to_string_lossy().into_owned(), "app-server".into()]
    );
}
