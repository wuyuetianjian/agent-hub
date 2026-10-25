//! A user-level Codex hook routes each callback to its original session.
use crate::{
    codex_hook::{self, proof, verify_proof},
    model::{Profile, SessionInfo},
    runtime::Runtime,
};
use anyhow::{ensure, Context, Result};
use axum::{
    extract::{Path, State},
    http::{HeaderMap, HeaderValue, StatusCode},
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    collections::HashMap,
    io::Read,
    path::{Path as FilePath, PathBuf},
    sync::Arc,
};

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Registration {
    pub profile: Profile,
    pub home: PathBuf,
    pub executable: PathBuf,
    pub token: String,
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Descriptor {
    pub profile_id: String,
    pub home: PathBuf,
    pub url: String,
    pub token: String,
    pub timeout_seconds: u64,
    pub config_env: Option<String>,
    pub default_home: PathBuf,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Setup {
    pub configuration: Value,
    pub install_command: String,
    pub descriptor: PathBuf,
    pub target: PathBuf,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Coverage {
    pub profile_id: String,
    pub installed: bool,
}
pub fn coverage(runtime: &Runtime) -> Vec<Coverage> {
    runtime
        .store
        .dispatchers()
        .unwrap_or_default()
        .into_iter()
        .filter(|r| validate_binding(runtime, r).is_ok())
        .map(|r| {
            let installed = (|| -> Option<bool> {
                let generated: Value = serde_json::from_slice(
                    &std::fs::read(
                        descriptor_path(runtime, &r.profile.id).with_extension("hooks.json"),
                    )
                    .ok()?,
                )
                .ok()?;
                let user: Value =
                    serde_json::from_slice(&std::fs::read(r.home.join("hooks.json")).ok()?).ok()?;
                let command =
                    generated["hooks"]["PermissionRequest"][0]["hooks"][0]["command"].as_str()?;
                Some(
                    user["hooks"]["PermissionRequest"]
                        .as_array()?
                        .iter()
                        .any(|g| {
                            g["hooks"].as_array().is_some_and(|hs| {
                                hs.iter().any(|h| h["command"].as_str() == Some(command))
                            })
                        }),
                )
            })()
            .unwrap_or(false);
            Coverage {
                profile_id: r.profile.id,
                installed,
            }
        })
        .collect()
}

fn home(runtime: &Runtime, profile: &Profile) -> Result<PathBuf> {
    let (_, provider) = runtime.profile(&profile.id)?;
    let env: HashMap<_, _> = provider
        .config_env
        .as_ref()
        .and_then(|key| std::env::var(key).ok().map(|value| (key.clone(), value)))
        .into_iter()
        .collect();
    let roots = crate::external_sessions::profile_roots(
        profile,
        &provider,
        &dirs::home_dir().context("No home directory")?,
        &env,
    );
    ensure!(
        roots.len() == 1,
        "Configure one Codex home per account before connecting all windows"
    );
    Ok(std::fs::canonicalize(&roots[0]).context("Codex home does not exist")?)
}
fn validate_binding(runtime: &Runtime, r: &Registration) -> Result<()> {
    let (profile, _) = runtime.profile(&r.profile.id)?;
    ensure!(
        profile.provider_id == "codex"
            && profile.enabled
            && serde_json::to_value(&profile)? == serde_json::to_value(&r.profile)?,
        "Codex account binding changed; reconnect all-window approvals"
    );
    ensure!(home(runtime, &profile)? == r.home, "Codex home changed");
    for other in runtime.store.profiles()? {
        if other.id != profile.id && other.provider_id == "codex" && other.enabled {
            ensure!(home(runtime,&other).ok().as_ref()!=Some(&r.home), "Multiple accounts share this Codex home; configure distinct homes before connecting approvals");
        }
    }
    Ok(())
}
fn registration(runtime: &Runtime, profile: &str) -> Result<Registration> {
    let r = runtime
        .store
        .dispatchers()?
        .into_iter()
        .find(|r| r.profile.id == profile)
        .context("All-window approvals are disabled")?;
    validate_binding(runtime, &r)?;
    Ok(r)
}
fn dispatcher_file(data_dir: &FilePath, profile: &str) -> PathBuf {
    let hash = ring::digest::digest(&ring::digest::SHA256, profile.as_bytes());
    let name: String = hash.as_ref().iter().map(|b| format!("{b:02x}")).collect();
    data_dir
        .join("codex-dispatchers")
        .join(format!("{name}.json"))
}
pub fn descriptor_path(runtime: &Runtime, profile: &str) -> PathBuf {
    dispatcher_file(&runtime.data_dir, profile)
}
fn publish(runtime: &Runtime, r: &Registration) -> Result<()> {
    let url = runtime.bridge_url.read().unwrap().clone();
    ensure!(!url.is_empty(), "Local approval listener is not ready");
    let (_, provider) = runtime.profile(&r.profile.id)?;
    let mut default_profile = r.profile.clone();
    default_profile.config_dir.clear();
    let defaults = crate::external_sessions::profile_roots(
        &default_profile,
        &provider,
        &dirs::home_dir().context("No home directory")?,
        &HashMap::new(),
    );
    ensure!(defaults.len() == 1, "Configure a single default Codex home");
    let descriptor = Descriptor {
        profile_id: r.profile.id.clone(),
        home: r.home.clone(),
        url,
        token: r.token.clone(),
        timeout_seconds: runtime.settings.read().unwrap().interaction_timeout_seconds,
        config_env: provider.config_env,
        default_home: defaults[0].clone(),
    };
    codex_hook::private_json(
        &descriptor_path(runtime, &r.profile.id),
        &serde_json::to_value(descriptor)?,
    )
}
/// The hook inherits the original client's environment, not Orbit's environment.
/// This prevents a hook left in an old Codex home from using a rebound descriptor.
pub fn validate_client_home(
    descriptor: &Descriptor,
    environment: &HashMap<String, String>,
) -> Result<()> {
    let configured = descriptor
        .config_env
        .as_ref()
        .and_then(|key| environment.get(key))
        .filter(|s| !s.is_empty());
    let path = if let Some(configured) = configured {
        crate::discovery::expand_pattern(
            configured,
            &dirs::home_dir().context("No home directory")?,
            environment,
        )
        .context("Invalid client Codex home")?
    } else {
        descriptor.default_home.clone()
    };
    ensure!(
        std::fs::canonicalize(path)? == descriptor.home,
        "This hook belongs to another Codex home"
    );
    Ok(())
}
pub fn prepare(runtime: &Arc<Runtime>, profile_id: &str, executable: &FilePath) -> Result<Setup> {
    let _guard = runtime.dispatcher_gate.lock().unwrap();
    let (profile, _) = runtime.profile(profile_id)?;
    let previous = runtime
        .store
        .dispatchers()?
        .into_iter()
        .find(|r| r.profile.id == profile_id);
    let mut r = if let Some(existing) = previous.filter(|r| validate_binding(runtime, r).is_ok()) {
        existing
    } else {
        // Explicit preparation is the user's reconnect action for a changed home.
        let next = Registration {
            home: home(runtime, &profile)?,
            profile,
            executable: executable.into(),
            token: uuid::Uuid::new_v4().to_string(),
        };
        validate_binding(runtime, &next)?;
        disable_locked(runtime, profile_id)?;
        next
    };
    validate_binding(runtime, &r)?;
    r.executable = executable.into();
    publish(runtime, &r)?;
    runtime.store.save_dispatcher(&r)?;
    let descriptor = descriptor_path(runtime, profile_id);
    let command = format!(
        "{} codex-dispatch {}",
        codex_hook::quote(&executable.to_string_lossy()),
        codex_hook::quote(&descriptor.to_string_lossy())
    );
    let old_configuration: Option<Value> = std::fs::read(descriptor.with_extension("hooks.json"))
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok());
    let mut configuration = codex_hook::merge_config(
        json!({}),
        &command,
        runtime.settings.read().unwrap().interaction_timeout_seconds + 10,
    )?;
    let mut migrations = legacy_handlers(runtime, profile_id)?
        .as_array()
        .cloned()
        .unwrap_or_default();
    if let Some(old) = old_configuration {
        migrations.extend(
            old["orbitMigration"]
                .as_array()
                .cloned()
                .unwrap_or_default(),
        );
        if let Some(previous_command) =
            old["hooks"]["PermissionRequest"][0]["hooks"][0]["command"].as_str()
        {
            if previous_command != command {
                migrations
                    .push(json!({"target":r.home.join("hooks.json"),"command":previous_command}));
            }
        }
    }
    // Migration metadata survives repeated preparations before installation.
    let mut unique = vec![];
    for migration in migrations {
        if !unique.contains(&migration) {
            unique.push(migration);
        }
    }
    configuration["orbitMigration"] = json!(unique);
    let config_path = descriptor.with_extension("hooks.json");
    codex_hook::private_json(&config_path, &configuration)?;
    let target = r.home.join("hooks.json");
    let install_command = format!(
        "{} install-codex-hook {} {}",
        codex_hook::quote(&executable.to_string_lossy()),
        codex_hook::quote(&config_path.to_string_lossy()),
        codex_hook::quote(&target.to_string_lossy())
    );
    Ok(Setup {
        configuration,
        install_command,
        descriptor,
        target,
    })
}
fn legacy_handlers(runtime: &Runtime, profile: &str) -> Result<Value> {
    let mut migrations = vec![];
    let Ok(files) = std::fs::read_dir(runtime.data_dir.join("codex-hooks")) else {
        return Ok(json!([]));
    };
    for entry in files.flatten() {
        let path = entry.path();
        if path
            .file_name()
            .is_some_and(|p| p.to_string_lossy().ends_with(".hooks.json"))
        {
            continue;
        }
        let Ok(bytes) = std::fs::read(&path) else {
            continue;
        };
        let Ok(c) = serde_json::from_slice::<codex_hook::Connection>(&bytes) else {
            continue;
        };
        if c.session_id != format!("external:{profile}:{}", c.thread_id) {
            continue;
        }
        let Ok(bytes) = std::fs::read(path.with_extension("hooks.json")) else {
            continue;
        };
        let Ok(config) = serde_json::from_slice::<Value>(&bytes) else {
            continue;
        };
        let Some(command) = config["hooks"]["PermissionRequest"][0]["hooks"][0]["command"].as_str()
        else {
            continue;
        };
        migrations.push(
            json!({"target":FilePath::new(&c.cwd).join(".codex/hooks.json"),"command":command}),
        );
    }
    Ok(json!(migrations))
}
pub(crate) fn supersedes_legacy(path: &FilePath, connection: &codex_hook::Connection) -> bool {
    let Some((profile, _)) = connection
        .session_id
        .strip_prefix("external:")
        .and_then(|s| s.rsplit_once(':'))
    else {
        return false;
    };
    let Some(data_dir) = path.parent().and_then(FilePath::parent) else {
        return false;
    };
    let descriptor = dispatcher_file(data_dir, profile);
    let check = || -> Option<bool> {
        let d: Descriptor = serde_json::from_slice(&std::fs::read(&descriptor).ok()?).ok()?;
        let generated: Value =
            serde_json::from_slice(&std::fs::read(descriptor.with_extension("hooks.json")).ok()?)
                .ok()?;
        let user: Value =
            serde_json::from_slice(&std::fs::read(d.home.join("hooks.json")).ok()?).ok()?;
        let command = generated["hooks"]["PermissionRequest"][0]["hooks"][0]["command"].as_str()?;
        Some(
            user["hooks"]["PermissionRequest"]
                .as_array()?
                .iter()
                .any(|group| {
                    group["hooks"].as_array().is_some_and(|hooks| {
                        hooks
                            .iter()
                            .any(|hook| hook["command"].as_str() == Some(command))
                    })
                }),
        )
    };
    check().unwrap_or(false)
}
pub fn restore(runtime: &Arc<Runtime>) -> Result<()> {
    let _guard = runtime.dispatcher_gate.lock().unwrap();
    for mut r in runtime.store.dispatchers()? {
        if validate_binding(runtime, &r).is_err() {
            continue;
        }
        // A stale process cannot sign an approval for the new listener instance.
        r.token = uuid::Uuid::new_v4().to_string();
        runtime.store.save_dispatcher(&r)?;
        publish(runtime, &r)?;
    }
    Ok(())
}
pub fn disable(runtime: &Runtime, profile: &str) -> Result<()> {
    let _guard = runtime.dispatcher_gate.lock().unwrap();
    disable_locked(runtime, profile)
}
pub(crate) fn disable_locked(runtime: &Runtime, profile: &str) -> Result<()> {
    runtime.store.remove_dispatcher(profile)?;
    for session in runtime.sessions.list() {
        if session.profile_id == profile && session.mode == "hook" {
            let _ = runtime.sessions.stop(&session.id);
        }
    }
    // Keeping an offline descriptor lets trusted hooks fail back to the client.
    runtime.emit("dashboard");
    Ok(())
}
pub fn disconnect_session(runtime: &Runtime, session: &str) -> Result<()> {
    let _guard = runtime.dispatcher_gate.lock().unwrap();
    let info = runtime
        .sessions
        .list()
        .into_iter()
        .find(|s| s.id == session)
        .context("Unknown session")?;
    runtime.store.block_hook_thread(
        &info.profile_id,
        info.id.rsplit(':').next().unwrap_or(""),
        true,
    )?;
    runtime.sessions.stop(session)
}
pub fn reconnect_session(runtime: &Runtime, info: SessionInfo) -> Result<()> {
    let _guard = runtime.dispatcher_gate.lock().unwrap();
    let r = registration(runtime, &info.profile_id)?;
    runtime.store.block_hook_thread(
        &info.profile_id,
        info.id.rsplit(':').next().unwrap_or(""),
        false,
    )?;
    if runtime.sessions.hook_secret(&info.id).is_none() {
        runtime
            .sessions
            .attach_codex_hook(&r.profile, info, &runtime.settings.read().unwrap())?;
    }
    runtime.emit("dashboard");
    Ok(())
}
fn validate_event(input: &Value) -> Result<(String, String)> {
    ensure!(
        input["hook_event_name"] == "PermissionRequest",
        "Not an approval request"
    );
    let thread = input["session_id"]
        .as_str()
        .context("Missing original session id")?;
    uuid::Uuid::parse_str(thread)?;
    ensure!(
        input["tool_name"].as_str().is_some_and(|s| !s.is_empty()),
        "Missing tool name"
    );
    let cwd = std::fs::canonicalize(
        input["cwd"]
            .as_str()
            .context("Missing original working directory")?,
    )?;
    ensure!(cwd.is_dir(), "Working directory is not a directory");
    Ok((thread.into(), cwd.to_string_lossy().into_owned()))
}
fn envelope(secret: &str, nonce: &str, result: Value) -> Json<Value> {
    Json(json!({"nonce":nonce,"proof":proof(secret,"response",nonce,&result),"result":result}))
}
pub async fn receive(
    State(runtime): State<Arc<Runtime>>,
    Path(profile): Path<String>,
    headers: HeaderMap,
    Json(input): Json<Value>,
) -> Result<Json<Value>, StatusCode> {
    if headers.contains_key("origin") {
        return Err(StatusCode::FORBIDDEN);
    }
    let nonce = headers
        .get("x-orbit-nonce")
        .and_then(|h| h.to_str().ok())
        .ok_or(StatusCode::UNAUTHORIZED)?;
    uuid::Uuid::parse_str(nonce).map_err(|_| StatusCode::UNAUTHORIZED)?;
    let signature = headers
        .get("x-orbit-proof")
        .and_then(|h| h.to_str().ok())
        .ok_or(StatusCode::UNAUTHORIZED)?;
    let (r, id, session_secret) = {
        let _guard = runtime.dispatcher_gate.lock().unwrap();
        let r = registration(&runtime, &profile).map_err(|_| StatusCode::CONFLICT)?;
        verify_proof(&r.token, "request", nonce, &input, signature)
            .map_err(|_| StatusCode::UNAUTHORIZED)?;
        let (thread, cwd) = validate_event(&input).map_err(|_| StatusCode::BAD_REQUEST)?;
        let now = std::time::Instant::now();
        let seconds = runtime.settings.read().unwrap().interaction_timeout_seconds;
        let mut seen = runtime.dispatcher_nonces.lock().unwrap();
        seen.retain(|_, at| now.duration_since(*at).as_secs() <= seconds + 10);
        let key = format!("{profile}:{nonce}");
        if seen.contains_key(&key) {
            return Err(StatusCode::CONFLICT);
        }
        seen.insert(key, now);
        drop(seen);
        if runtime
            .store
            .hook_thread_blocked(&profile, &thread)
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        {
            return Ok(envelope(&r.token, nonce, json!({})));
        }
        let id = format!("external:{profile}:{thread}");
        if runtime.sessions.hook_secret(&id).is_none() {
            let info = SessionInfo {
                id: id.clone(),
                profile_id: profile.clone(),
                provider_id: "codex".into(),
                cwd: cwd.clone(),
                mode: "external".into(),
                status: "external".into(),
                read_only: true,
                interaction_channel: "history".into(),
                title: format!(
                    "Codex · {}",
                    FilePath::new(&cwd)
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                ),
                source: "user-level approval hook".into(),
                started_at: chrono::Utc::now().timestamp(),
                updated_at: chrono::Utc::now().timestamp(),
            };
            runtime
                .sessions
                .attach_codex_hook(&r.profile, info, &runtime.settings.read().unwrap())
                .map_err(|_| StatusCode::CONFLICT)?;
        }
        let secret = runtime
            .sessions
            .hook_secret(&id)
            .ok_or(StatusCode::CONFLICT)?;
        (r, id, secret)
    };
    let mut internal = HeaderMap::new();
    internal.insert(
        "x-orbit-nonce",
        HeaderValue::from_str(nonce).map_err(|_| StatusCode::BAD_REQUEST)?,
    );
    internal.insert(
        "x-orbit-proof",
        HeaderValue::from_str(&proof(&session_secret, "request", nonce, &input))
            .map_err(|_| StatusCode::BAD_REQUEST)?,
    );
    let response =
        codex_hook::receive(State(runtime.clone()), Path(id), internal, Json(input)).await?;
    let _guard = runtime.dispatcher_gate.lock().unwrap();
    let still_valid =
        registration(&runtime, &profile).is_ok_and(|current| current.token == r.token);
    let result = if still_valid {
        response.0["result"].clone()
    } else {
        json!({})
    };
    Ok(envelope(&r.token, nonce, result))
}

pub async fn helper(path: &FilePath) -> Value {
    let result = async {
        let metadata = std::fs::symlink_metadata(path)?;
        ensure!(
            !metadata.file_type().is_symlink(),
            "Unsafe dispatcher descriptor"
        );
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            ensure!(
                metadata.permissions().mode() & 0o077 == 0,
                "Dispatcher descriptor must be owner-only"
            );
        }
        let d: Descriptor = serde_json::from_slice(&std::fs::read(path)?)?;
        validate_client_home(&d, &std::env::vars().collect())?;
        ensure!(
            d.timeout_seconds > 0 && d.timeout_seconds <= 86400,
            "Invalid dispatcher timeout"
        );
        let mut data = vec![];
        std::io::stdin()
            .take(2 * 1024 * 1024 + 1)
            .read_to_end(&mut data)?;
        ensure!(data.len() <= 2 * 1024 * 1024, "Hook input too large");
        let input: Value = serde_json::from_slice(&data)?;
        validate_event(&input)?;
        let mut url = reqwest::Url::parse(&d.url)?;
        ensure!(
            url.scheme() == "http"
                && url.host_str() == Some("127.0.0.1")
                && url.username().is_empty()
                && url.password().is_none()
                && url.query().is_none()
                && url.fragment().is_none(),
            "Dispatcher listener must be local"
        );
        url.path_segments_mut()
            .map_err(|_| anyhow::anyhow!("Invalid listener URL"))?
            .clear()
            .push("codex-dispatch")
            .push(&d.profile_id)
            .push("permission");
        let nonce = uuid::Uuid::new_v4().to_string();
        let response = reqwest::Client::builder()
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(std::time::Duration::from_secs(d.timeout_seconds + 5))
            .build()?
            .post(url)
            .header("x-orbit-nonce", &nonce)
            .header("x-orbit-proof", proof(&d.token, "request", &nonce, &input))
            .json(&input)
            .send()
            .await?;
        ensure!(
            response.status().is_success(),
            "Dispatcher unavailable or account binding revoked"
        );
        let envelope: Value = response.json().await?;
        ensure!(
            envelope["nonce"].as_str() == Some(nonce.as_str()),
            "Wrong response nonce"
        );
        let answer = &envelope["result"];
        verify_proof(
            &d.token,
            "response",
            &nonce,
            answer,
            envelope["proof"]
                .as_str()
                .context("Missing response proof")?,
        )?;
        if answer == &json!({}) {
            return Ok(json!({}));
        }
        let behavior = answer["hookSpecificOutput"]["decision"]["behavior"]
            .as_str()
            .context("Missing decision")?;
        ensure!(["allow", "deny"].contains(&behavior), "Invalid decision");
        Ok::<_, anyhow::Error>(codex_hook::decision(behavior == "allow"))
    }
    .await;
    match result {
        Ok(value) => value,
        Err(_) => {
            eprintln!("Agent Orbit: approval dispatcher unavailable or request rejected; using original client approval.");
            json!({})
        }
    }
}
