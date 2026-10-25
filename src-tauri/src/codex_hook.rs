//! Approval transport for an existing Codex client. Never starts or resumes a thread.
use crate::{
    model::{Choice, Interaction},
    runtime::Runtime,
};
use anyhow::{ensure, Context, Result};
use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    Json,
};
use base64::{engine::general_purpose::STANDARD, Engine};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    io::{Read, Write},
    path::Path as FilePath,
    sync::Arc,
};

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Connection {
    pub url: String,
    pub token: String,
    pub session_id: String,
    pub thread_id: String,
    pub cwd: String,
    pub timeout_seconds: u64,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Setup {
    pub session_id: String,
    pub configuration: Value,
    pub install_command: String,
}
pub fn validate_input(c: &Connection, input: &Value) -> Result<()> {
    ensure!(
        input["hook_event_name"] == "PermissionRequest",
        "Not an approval hook"
    );
    ensure!(
        input["session_id"].as_str() == Some(c.thread_id.as_str()),
        "Another Codex thread"
    );
    let cwd = input["cwd"].as_str().context("Missing project directory")?;
    ensure!(
        std::fs::canonicalize(cwd)? == std::fs::canonicalize(&c.cwd)?,
        "Another project directory"
    );
    ensure!(
        input["tool_name"].as_str().is_some_and(|s| !s.is_empty()),
        "Missing tool name"
    );
    Ok(())
}
pub fn proof(secret: &str, direction: &str, nonce: &str, value: &Value) -> String {
    let key = ring::hmac::Key::new(ring::hmac::HMAC_SHA256, secret.as_bytes());
    let data = format!("{direction}\0{nonce}\0{}", value);
    STANDARD.encode(ring::hmac::sign(&key, data.as_bytes()).as_ref())
}
pub fn verify_proof(
    secret: &str,
    direction: &str,
    nonce: &str,
    value: &Value,
    signature: &str,
) -> Result<()> {
    let key = ring::hmac::Key::new(ring::hmac::HMAC_SHA256, secret.as_bytes());
    let data = format!("{direction}\0{nonce}\0{}", value);
    ring::hmac::verify(&key, data.as_bytes(), &STANDARD.decode(signature)?)
        .map_err(|_| anyhow::anyhow!("Invalid approval proof"))
}
pub fn decision(allow: bool) -> Value {
    json!({"hookSpecificOutput":{"hookEventName":"PermissionRequest","decision":if allow {json!({"behavior":"allow"})} else {json!({"behavior":"deny","message":"The approval request was denied or cancelled."})}}})
}
fn flattened(value: &Value, prefix: &str, out: &mut Vec<String>) {
    match value {
        Value::Object(map) => {
            for (key, value) in map {
                flattened(value, &format!("{prefix}{key}."), out);
            }
        }
        Value::Array(items) => {
            for (i, value) in items.iter().enumerate() {
                flattened(value, &format!("{prefix}{i}."), out);
            }
        }
        Value::String(text) => out.push(format!("{}: {text}", prefix.trim_end_matches('.'))),
        _ => out.push(format!("{}: {value}", prefix.trim_end_matches('.'))),
    }
}
pub async fn receive(
    State(runtime): State<Arc<Runtime>>,
    Path(session): Path<String>,
    headers: HeaderMap,
    Json(input): Json<Value>,
) -> Result<Json<Value>, StatusCode> {
    if headers.contains_key("origin") {
        return Err(StatusCode::FORBIDDEN);
    }
    let nonce = headers
        .get("x-orbit-nonce")
        .and_then(|v| v.to_str().ok())
        .ok_or(StatusCode::UNAUTHORIZED)?;
    uuid::Uuid::parse_str(nonce).map_err(|_| StatusCode::UNAUTHORIZED)?;
    let signature = headers
        .get("x-orbit-proof")
        .and_then(|v| v.to_str().ok())
        .ok_or(StatusCode::UNAUTHORIZED)?;
    let secret = runtime
        .sessions
        .hook_secret(&session)
        .ok_or(StatusCode::UNAUTHORIZED)?;
    verify_proof(&secret, "request", nonce, &input, signature)
        .map_err(|_| StatusCode::UNAUTHORIZED)?;
    let info = runtime
        .sessions
        .authenticate(&session, &secret)
        .ok_or(StatusCode::UNAUTHORIZED)?;
    if info.provider_id != "codex" || info.mode != "hook" {
        return Err(StatusCode::FORBIDDEN);
    }
    let (current, _) = runtime
        .profile(&info.profile_id)
        .map_err(|_| StatusCode::NOT_FOUND)?;
    if !runtime.sessions.profile_matches(&session, &current) {
        return Err(StatusCode::CONFLICT);
    }
    let seconds = runtime.settings.read().unwrap().interaction_timeout_seconds;
    let connection = Connection {
        url: String::new(),
        token: String::new(),
        thread_id: info.id.rsplit(':').next().unwrap_or("").into(),
        session_id: session.clone(),
        cwd: info.cwd.clone(),
        timeout_seconds: seconds,
    };
    validate_input(&connection, &input).map_err(|_| StatusCode::BAD_REQUEST)?;
    runtime
        .sessions
        .hook_connected(&session)
        .map_err(|_| StatusCode::CONFLICT)?;
    let tool = input["tool_name"].as_str().unwrap_or("Tool");
    let action = if let Some(command) = input["tool_input"]["command"].as_str() {
        command.to_owned()
    } else {
        let mut fields = vec![tool.to_string()];
        flattened(&input["tool_input"], "", &mut fields);
        fields.join("\n")
    };
    let request = Interaction {profile_label:current.label, cwd:info.cwd, id:uuid::Uuid::new_v4().to_string(),session_id:session,provider_id:"codex".into(),title:tool.into(),detail:json!({"reason":input["tool_input"]["description"].as_str().or_else(||input["reason"].as_str()),"command":action}).to_string(),kind:"permission".into(),choices:vec![Choice{id:"allow".into(),label:"Allow once".into()},Choice{id:"deny".into(),label:"Deny".into()}],questions:vec![],created_at:chrono::Utc::now().timestamp(),expires_at:chrono::Utc::now().timestamp()+seconds as i64};
    let id = request.id.clone();
    let reply = runtime.broker.insert(request);
    runtime.emit("dashboard");
    let result = tokio::time::timeout(std::time::Duration::from_secs(seconds), reply).await;
    runtime.broker.remove(&id);
    runtime.emit("dashboard");
    let binding_valid = runtime
        .sessions
        .authenticate(&connection.session_id, &secret)
        .is_some()
        && runtime.profile(&info.profile_id).is_ok_and(|(profile, _)| {
            runtime
                .sessions
                .profile_matches(&connection.session_id, &profile)
        });
    let result = match result {
        Ok(Ok(reply)) if binding_valid => decision(reply.choice.as_deref() == Some("allow")),
        _ => json!({}),
    };
    Ok(Json(
        json!({"result":result,"nonce":nonce,"proof":proof(&secret,"response",nonce,&result)}),
    ))
}
pub(crate) fn quote(value: &str) -> String {
    #[cfg(not(windows))]
    {
        format!("'{}'", value.replace('\'', "'\\''"))
    }
    #[cfg(windows)]
    {
        format!("\"{}\"", value.replace('"', "\"\""))
    }
}
pub(crate) fn private_json(path: &FilePath, value: &Value) -> Result<()> {
    let parent = path.parent().context("Missing configuration directory")?;
    std::fs::create_dir_all(parent)?;
    let temp = parent.join(format!(".orbit-{}.tmp", uuid::Uuid::new_v4()));
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let result = (|| {
        let mut file = options.open(&temp)?;
        file.write_all(serde_json::to_string_pretty(value)?.as_bytes())?;
        file.sync_all()?;
        std::fs::rename(&temp, path)?;
        Ok::<_, anyhow::Error>(())
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(temp);
    }
    result
}
pub fn merge_config(mut value: Value, command: &str, seconds: u64) -> Result<Value> {
    ensure!(value.is_object(), "Hook configuration must be an object");
    let object = value.as_object_mut().unwrap();
    let hooks = object
        .entry("hooks")
        .or_insert_with(|| json!({}))
        .as_object_mut()
        .context("Invalid hooks object")?;
    let groups = hooks
        .entry("PermissionRequest")
        .or_insert_with(|| json!([]))
        .as_array_mut()
        .context("Invalid permission hooks")?;
    if !groups.iter().any(|g| {
        g["hooks"]
            .as_array()
            .is_some_and(|h| h.iter().any(|h| h["command"] == command))
    }) {
        groups.push(json!({"hooks":[{"type":"command","command":command,"timeout":seconds}]}));
    }
    Ok(value)
}
fn descriptor_path(runtime: &Runtime, id: &str) -> std::path::PathBuf {
    let name = STANDARD
        .encode(ring::digest::digest(&ring::digest::SHA256, id.as_bytes()).as_ref())
        .replace('/', "_")
        .replace('+', "-")
        .trim_end_matches('=')
        .to_string();
    runtime
        .data_dir
        .join("codex-hooks")
        .join(format!("{name}.json"))
}
fn setup_for(
    runtime: &Runtime,
    info: &crate::model::SessionInfo,
    executable: &FilePath,
) -> Result<Setup> {
    let path = descriptor_path(runtime, &info.id);
    let connection: Connection = serde_json::from_slice(&std::fs::read(&path)?)?;
    let command = format!(
        "{} codex-hook {}",
        quote(&executable.to_string_lossy()),
        quote(&path.to_string_lossy())
    );
    let configuration = merge_config(json!({}), &command, connection.timeout_seconds + 10)?;
    let config_path = path.with_extension("hooks.json");
    private_json(&config_path, &configuration)?;
    let target = FilePath::new(&info.cwd).join(".codex/hooks.json");
    let install_command = format!(
        "{} install-codex-hook {} {}",
        quote(&executable.to_string_lossy()),
        quote(&config_path.to_string_lossy()),
        quote(&target.to_string_lossy())
    );
    Ok(Setup {
        session_id: info.id.clone(),
        configuration,
        install_command,
    })
}
pub fn prepare(
    runtime: &Arc<Runtime>,
    info: crate::model::SessionInfo,
    executable: &FilePath,
) -> Result<Setup> {
    let base = runtime.bridge_url.read().unwrap().clone();
    ensure!(!base.is_empty(), "Local approval listener is not ready");
    let (profile, _) = runtime.profile(&info.profile_id)?;
    if runtime.sessions.hook_secret(&info.id).is_some() {
        ensure!(
            runtime.sessions.profile_matches(&info.id, &profile),
            "Account binding changed; disconnect before reconnecting"
        );
        let current = runtime
            .sessions
            .list()
            .into_iter()
            .find(|s| s.id == info.id)
            .context("Connection unavailable")?;
        return setup_for(runtime, &current, executable);
    }
    let settings = runtime.settings.read().unwrap().clone();
    let (info, token) = runtime
        .sessions
        .attach_codex_hook(&profile, info, &settings)?;
    let result = (|| {
        let c = Connection {
            url: base,
            token,
            session_id: info.id.clone(),
            thread_id: info
                .id
                .rsplit(':')
                .next()
                .context("Missing original thread")?
                .into(),
            cwd: info.cwd.clone(),
            timeout_seconds: settings.interaction_timeout_seconds,
        };
        private_json(
            &descriptor_path(runtime, &info.id),
            &serde_json::to_value(c)?,
        )?;
        setup_for(runtime, &info, executable)
    })();
    if result.is_err() {
        let _ = runtime.sessions.stop(&info.id);
    }
    result
}

pub fn install(config: &FilePath, target: &FilePath) -> Result<()> {
    ensure!(
        target.file_name().is_some_and(|n| n == "hooks.json"),
        "Expected a hooks.json target"
    );
    let generated: Value = serde_json::from_slice(&std::fs::read(config)?)?;
    let handler = &generated["hooks"]["PermissionRequest"][0]["hooks"][0];
    let command = handler["command"]
        .as_str()
        .context("Missing hook command")?;
    let timeout = handler["timeout"]
        .as_u64()
        .context("Missing hook timeout")?;
    let existing = if target.exists() {
        serde_json::from_slice(&std::fs::read(target)?)?
    } else {
        json!({})
    };
    let merged = merge_config(existing, command, timeout)?;
    if target.exists() {
        std::fs::copy(
            target,
            target.with_extension(format!("json.backup-{}", uuid::Uuid::new_v4())),
        )?;
    }
    private_json(target, &merged)?;
    if let Some(migrations) = generated["orbitMigration"].as_array() {
        for migration in migrations {
            let old = std::path::PathBuf::from(
                migration["target"]
                    .as_str()
                    .context("Invalid migration target")?,
            );
            ensure!(
                old.file_name().is_some_and(|n| n == "hooks.json"),
                "Invalid migration hook file"
            );
            let old_command = migration["command"]
                .as_str()
                .context("Invalid migration command")?;
            if !old.exists() {
                continue;
            }
            let mut config: Value = serde_json::from_slice(&std::fs::read(&old)?)?;
            let before = config.clone();
            if let Some(groups) = config["hooks"]["PermissionRequest"].as_array_mut() {
                for group in groups.iter_mut() {
                    if let Some(hooks) = group["hooks"].as_array_mut() {
                        hooks.retain(|h| h["command"].as_str() != Some(old_command));
                    }
                }
                groups.retain(|g| !g["hooks"].as_array().is_some_and(Vec::is_empty));
            }
            if config != before {
                std::fs::copy(
                    &old,
                    old.with_extension(format!("json.backup-{}", uuid::Uuid::new_v4())),
                )?;
                private_json(&old, &config)?;
            }
        }
    }
    Ok(())
}
pub async fn run_helper(path: &FilePath) -> Value {
    let result = async {
        let metadata = std::fs::symlink_metadata(path)?;
        ensure!(
            !metadata.file_type().is_symlink(),
            "Connection descriptors must not be symlinks"
        );
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            ensure!(
                metadata.permissions().mode() & 0o077 == 0,
                "Connection file must be owner-only"
            );
        }
        let c: Connection = serde_json::from_slice(&std::fs::read(path)?)?;
        if crate::codex_dispatcher::supersedes_legacy(path, &c) {
            return Ok(json!({}));
        }
        ensure!(
            c.timeout_seconds > 0 && c.timeout_seconds <= 86400,
            "Invalid hook timeout"
        );
        let mut data = vec![];
        std::io::stdin()
            .take(2 * 1024 * 1024 + 1)
            .read_to_end(&mut data)?;
        ensure!(data.len() <= 2 * 1024 * 1024, "Hook input too large");
        let input: Value = serde_json::from_slice(&data)?;
        validate_input(&c, &input)?;
        let mut url = reqwest::Url::parse(&c.url)?;
        ensure!(
            url.scheme() == "http"
                && url.host_str() == Some("127.0.0.1")
                && url.username().is_empty()
                && url.password().is_none()
                && url.query().is_none()
                && url.fragment().is_none(),
            "Hook listener must be local"
        );
        url.path_segments_mut()
            .map_err(|_| anyhow::anyhow!("Invalid listener URL"))?
            .clear()
            .push("codex")
            .push(&c.session_id)
            .push("permission");
        let nonce = uuid::Uuid::new_v4().to_string();
        let signature = proof(&c.token, "request", &nonce, &input);
        let response = reqwest::Client::builder()
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(std::time::Duration::from_secs(c.timeout_seconds + 5))
            .build()?
            .post(url)
            .header("x-orbit-nonce", &nonce)
            .header("x-orbit-proof", signature)
            .json(&input)
            .send()
            .await?;
        ensure!(
            response.status().is_success(),
            "Approval listener unavailable"
        );
        let envelope: Value = response.json().await?;
        ensure!(
            envelope["nonce"].as_str() == Some(nonce.as_str()),
            "Another approval response"
        );
        let answer = &envelope["result"];
        verify_proof(
            &c.token,
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
        ensure!(["allow", "deny"].contains(&behavior), "Unknown decision");
        Ok::<_, anyhow::Error>(decision(behavior == "allow"))
    }
    .await;
    // No decision means the original client retains its normal approval prompt.
    result.unwrap_or_else(|_| json!({}))
}
