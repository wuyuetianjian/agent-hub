use crate::{
    model::{Choice, Interaction, Question},
    providers::normalized,
    runtime::Runtime,
};
use anyhow::{ensure, Result};
use axum::{
    extract::{DefaultBodyLimit, Path, State},
    http::{HeaderMap, StatusCode},
    routing::post,
    Json, Router,
};
use serde_json::{json, Value};
use std::sync::Arc;

pub async fn serve(runtime: Arc<Runtime>) -> Result<()> {
    let listener = tokio::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0)).await?;
    *runtime.bridge_url.write().unwrap() = format!("http://{}", listener.local_addr()?);
    let router = Router::new()
        .route("/claude/{session}/{kind}", post(hook))
        .layer(DefaultBodyLimit::max(2 * 1024 * 1024))
        .with_state(runtime);
    tokio::spawn(async move {
        let _ = axum::serve(listener, router).await;
    });
    Ok(())
}
async fn hook(
    State(runtime): State<Arc<Runtime>>,
    Path((session, kind)): Path<(String, String)>,
    headers: HeaderMap,
    Json(input): Json<Value>,
) -> std::result::Result<Json<Value>, StatusCode> {
    if headers.contains_key("origin") {
        return Err(StatusCode::FORBIDDEN);
    }
    let token = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or(StatusCode::UNAUTHORIZED)?;
    let info = runtime
        .sessions
        .authenticate(&session, token)
        .ok_or(StatusCode::UNAUTHORIZED)?;
    if info.provider_id != "claude" {
        return Err(StatusCode::FORBIDDEN);
    }
    let profile = runtime
        .sessions
        .original_profile(&session)
        .map_err(|_| StatusCode::NOT_FOUND)?;
    if kind == "status" {
        let (current, _) = runtime
            .profile(&info.profile_id)
            .map_err(|_| StatusCode::NOT_FOUND)?;
        if !runtime.sessions.profile_matches(&session, &current) {
            return Err(StatusCode::CONFLICT);
        }
        let mut snapshot = normalized(&profile, "claude", &input, "Claude Code status-line bridge");
        if snapshot.windows.is_empty() {
            return Ok(Json(json!({})));
        }
        snapshot.account = input["account"]["email"].as_str().map(str::to_owned);
        runtime
            .store
            .save_bound_snapshot(&snapshot, &profile)
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        runtime.emit("dashboard");
        return Ok(Json(json!({})));
    }
    if !["permission", "question"].contains(&kind.as_str()) {
        return Err(StatusCode::NOT_FOUND);
    }
    let seconds = runtime.settings.read().unwrap().interaction_timeout_seconds;
    let mut request = Interaction {
        profile_label: profile.label.clone(),
        cwd: info.cwd.clone(),
        id: uuid::Uuid::new_v4().to_string(),
        session_id: session,
        provider_id: "claude".into(),
        title: input["tool_name"]
            .as_str()
            .unwrap_or("Claude approval")
            .into(),
        detail: serde_json::to_string_pretty(&input["tool_input"]).unwrap_or_default(),
        kind: kind.clone(),
        choices: vec![],
        questions: vec![],
        created_at: chrono::Utc::now().timestamp(),
        expires_at: chrono::Utc::now().timestamp() + seconds as i64,
    };
    if kind == "permission" {
        request.choices = vec![
            Choice {
                id: "allow".into(),
                label: "Allow once".into(),
            },
            Choice {
                id: "deny".into(),
                label: "Deny".into(),
            },
        ];
    } else {
        for q in input["tool_input"]["questions"]
            .as_array()
            .into_iter()
            .flatten()
        {
            let title = q["question"].as_str().unwrap_or("Question");
            request.questions.push(Question {
                id: title.into(),
                title: title.into(),
                multiple: q["multiSelect"].as_bool().unwrap_or(false),
                options: q["options"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(|o| {
                        o["label"].as_str().map(|v| Choice {
                            id: v.into(),
                            label: v.into(),
                        })
                    })
                    .collect(),
            });
        }
        if request.questions.is_empty() {
            return Ok(Json(json!({})));
        }
    }
    let id = request.id.clone();
    let rx = runtime.broker.insert(request);
    runtime.emit("dashboard");
    let result = tokio::time::timeout(std::time::Duration::from_secs(seconds), rx).await;
    runtime.broker.remove(&id);
    runtime.emit("dashboard");
    let response = match result {
        Ok(Ok(reply)) if kind == "permission" => {
            json!({"hookSpecificOutput":{"hookEventName":"PermissionRequest","decision":{"behavior":reply.choice.unwrap_or("deny".into())}}})
        }
        Ok(Ok(reply)) => {
            let mut updated = input["tool_input"].clone();
            let answers: serde_json::Map<String, Value> = reply
                .answers
                .into_iter()
                .map(|(k, v)| (k, Value::String(v.join(", "))))
                .collect();
            updated["answers"] = Value::Object(answers);
            json!({"hookSpecificOutput":{"hookEventName":"PreToolUse","permissionDecision":"allow","updatedInput":updated}})
        }
        _ => denied(&kind),
    };
    Ok(Json(response))
}
fn denied(kind: &str) -> Value {
    if kind == "permission" {
        json!({"hookSpecificOutput":{"hookEventName":"PermissionRequest","decision":{"behavior":"deny","message":"Desktop approval timed out or disconnected"}}})
    } else {
        json!({"hookSpecificOutput":{"hookEventName":"PreToolUse","permissionDecision":"deny","permissionDecisionReason":"Desktop question timed out or disconnected"}})
    }
}
pub async fn helper(kind: &str) -> Result<()> {
    use std::io::Read;
    let mut data = vec![];
    std::io::stdin()
        .take(2 * 1024 * 1024 + 1)
        .read_to_end(&mut data)?;
    ensure!(
        data.len() <= 2 * 1024 * 1024,
        "Hook input exceeds size limit"
    );
    let result = async {
        let base = std::env::var("AGENT_ORBIT_BRIDGE_URL")?;
        let parsed = reqwest::Url::parse(&base)?;
        ensure!(
            parsed.scheme() == "http" && parsed.host_str() == Some("127.0.0.1"),
            "Hook bridge must be local"
        );
        let token = std::env::var("AGENT_ORBIT_BRIDGE_TOKEN")?;
        let session = std::env::var("AGENT_ORBIT_SESSION_ID")?;
        let timeout = std::env::var("AGENT_ORBIT_BRIDGE_TIMEOUT")?.parse::<u64>()?;
        let value: Value = serde_json::from_slice(&data)?;
        let res = reqwest::Client::builder()
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(std::time::Duration::from_secs(timeout + 5))
            .build()?
            .post(format!("{base}/claude/{session}/{kind}"))
            .bearer_auth(token)
            .json(&value)
            .send()
            .await?;
        ensure!(res.status().is_success(), "Bridge unavailable");
        Ok::<Value, anyhow::Error>(res.json().await?)
    }
    .await;
    if kind == "status" {
        if let Ok(v) = serde_json::from_slice::<Value>(&data) {
            if let Some(name) = v["model"]["display_name"].as_str() {
                println!("{name}");
            }
        }
    } else {
        println!("{}", result.unwrap_or_else(|_| denied(kind)));
    }
    Ok(())
}
