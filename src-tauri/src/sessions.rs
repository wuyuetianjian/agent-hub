use crate::{broker::Broker, model::*, protocol::RpcClient};
use anyhow::{anyhow, ensure, Result};
use base64::{engine::general_purpose::STANDARD, Engine};
use portable_pty::{native_pty_system, ChildKiller, CommandBuilder, MasterPty, PtySize};
use serde_json::{json, Value};
use std::{
    collections::HashMap,
    io::{Read, Write},
    sync::{Arc, Mutex},
};
use tokio::sync::broadcast;

enum Transport {
    Pty {
        writer: Mutex<Box<dyn Write + Send>>,
        master: Mutex<Box<dyn MasterPty + Send>>,
        killer: Mutex<Box<dyn ChildKiller + Send + Sync>>,
    },
    Rpc {
        client: Arc<RpcClient>,
        thread_id: String,
    },
}
struct Entry {
    profile: Profile,
    info: Mutex<SessionInfo>,
    transport: Mutex<Option<Transport>>,
    output: Mutex<(Vec<u8>, u64)>,
    token: String,
    max_output: usize,
}
pub struct Sessions {
    entries: Mutex<HashMap<String, Arc<Entry>>>,
    events: broadcast::Sender<AppEvent>,
    broker: Arc<Broker>,
}
impl Sessions {
    pub fn new(events: broadcast::Sender<AppEvent>, broker: Arc<Broker>) -> Self {
        Self {
            entries: Mutex::new(HashMap::new()),
            events,
            broker,
        }
    }
    fn entry(&self, id: &str) -> Result<Arc<Entry>> {
        self.entries
            .lock()
            .unwrap()
            .get(id)
            .cloned()
            .ok_or_else(|| anyhow!("Unknown session"))
    }
    pub fn list(&self) -> Vec<SessionInfo> {
        let mut v: Vec<_> = self
            .entries
            .lock()
            .unwrap()
            .values()
            .map(|e| e.info.lock().unwrap().clone())
            .collect();
        v.sort_by_key(|e| e.started_at);
        v
    }
    fn changed(&self) {
        let _ = self.events.send(AppEvent {
            kind: "dashboard".into(),
            session_id: None,
            data: Value::Null,
        });
    }
    fn output(&self, e: &Entry, data: &[u8]) {
        let mut out = e.output.lock().unwrap();
        out.0.extend_from_slice(data);
        if out.0.len() > e.max_output {
            let n = out.0.len() - e.max_output;
            out.0.drain(..n);
        }
        out.1 += 1;
        let _ = self.events.send(AppEvent {
            kind: "output".into(),
            session_id: Some(e.info.lock().unwrap().id.clone()),
            data: json!({"bytes":STANDARD.encode(data),"sequence":out.1}),
        });
    }
    pub fn replay(&self, id: &str) -> Result<Value> {
        let e = self.entry(id)?;
        let out = e.output.lock().unwrap();
        Ok(json!({"bytes":STANDARD.encode(&out.0),"sequence":out.1}))
    }
    pub fn original_profile(&self, id: &str) -> Result<Profile> {
        Ok(self.entry(id)?.profile.clone())
    }
    pub fn profile_matches(&self, id: &str, current: &Profile) -> bool {
        self.original_profile(id)
            .ok()
            .is_some_and(|p| serde_json::to_value(p).ok() == serde_json::to_value(current).ok())
    }
    pub fn authenticate(&self, id: &str, token: &str) -> Option<SessionInfo> {
        use subtle::ConstantTimeEq;
        let e = self.entry(id).ok()?;
        if !bool::from(e.token.as_bytes().ct_eq(token.as_bytes())) {
            return None;
        }
        let info = e.info.lock().unwrap().clone();
        (info.status == "running").then_some(info)
    }
    fn finished(&self, e: &Entry) {
        let id = {
            let mut info = e.info.lock().unwrap();
            info.status = "exited".into();
            info.id.clone()
        };
        self.broker.cancel_session(&id);
        self.changed();
    }
    pub async fn start(
        self: &Arc<Self>,
        p: &Profile,
        provider: &Provider,
        cwd: &str,
        mode: &str,
        s: &Settings,
        bridge_url: &str,
    ) -> Result<SessionInfo> {
        ensure!(
            std::path::Path::new(cwd).is_dir(),
            "Choose an existing project directory"
        );
        let exe = crate::discovery::discover(p, provider)
            .executable
            .ok_or_else(|| {
                anyhow!("CLI executable was not found; set its path in agent settings")
            })?;
        let id = uuid::Uuid::new_v4().to_string();
        let info = SessionInfo {
            read_only: false,
            title: p.label.clone(),
            source: "managed".into(),
            updated_at: chrono::Utc::now().timestamp(),
            id: id.clone(),
            profile_id: p.id.clone(),
            provider_id: provider.id.clone(),
            cwd: cwd.into(),
            mode: mode.into(),
            status: "running".into(),
            started_at: chrono::Utc::now().timestamp(),
        };
        let e = Arc::new(Entry {
            profile: p.clone(),
            info: Mutex::new(info.clone()),
            transport: Mutex::new(None),
            output: Mutex::new((vec![], 0)),
            token: format!("{}{}", uuid::Uuid::new_v4(), uuid::Uuid::new_v4()),
            max_output: s.max_output_bytes,
        });
        self.entries.lock().unwrap().insert(id.clone(), e.clone());
        let result = if mode == "structured" && provider.id == "codex" {
            self.start_codex(e.clone(), p, &exe, cwd, s).await
        } else {
            self.start_pty(e.clone(), p, provider, &exe, cwd, s, bridge_url)
        };
        if let Err(error) = result {
            self.entries.lock().unwrap().remove(&id);
            return Err(error);
        }
        self.changed();
        Ok(info)
    }
    fn start_pty(
        self: &Arc<Self>,
        e: Arc<Entry>,
        p: &Profile,
        provider: &Provider,
        exe: &str,
        cwd: &str,
        s: &Settings,
        bridge_url: &str,
    ) -> Result<()> {
        let pair = native_pty_system().openpty(PtySize {
            rows: 24,
            cols: 90,
            pixel_width: 0,
            pixel_height: 0,
        })?;
        let (program, args) = crate::launch::resolve(exe, &p.args)?;
        let mut cmd = CommandBuilder::new(program);
        cmd.args(&args);
        cmd.cwd(cwd);
        cmd.env("TERM", "xterm-256color");
        let id = e.info.lock().unwrap().id.clone();
        cmd.env("AGENT_ORBIT_BRIDGE_URL", bridge_url);
        cmd.env("AGENT_ORBIT_BRIDGE_TOKEN", &e.token);
        cmd.env("AGENT_ORBIT_SESSION_ID", &id);
        cmd.env(
            "AGENT_ORBIT_BRIDGE_TIMEOUT",
            s.interaction_timeout_seconds.to_string(),
        );
        if !p.config_dir.is_empty() {
            let dir = crate::discovery::expand(&p.config_dir);
            match provider.id.as_str() {
                "claude" => cmd.env("CLAUDE_CONFIG_DIR", dir),
                "codex" => cmd.env("CODEX_HOME", dir),
                "gemini" => cmd.env("GEMINI_CLI_HOME", dir),
                _ => {}
            }
        }
        if provider.id == "claude" && !bridge_url.is_empty() {
            let binary = std::env::current_exe()?.to_string_lossy().into_owned();
            let quote = quote_command(&binary);
            let timeout = s.interaction_timeout_seconds + 10;
            let settings = json!({"hooks":{"PermissionRequest":[{"hooks":[{"type":"command","command":format!("{quote} bridge permission"),"timeout":timeout}]}],"PreToolUse":[{"matcher":"AskUserQuestion","hooks":[{"type":"command","command":format!("{quote} bridge question"),"timeout":timeout}]}]},"statusLine":{"type":"command","command":format!("{quote} bridge status")}});
            cmd.arg("--settings");
            cmd.arg(settings.to_string());
        }
        let mut child = pair.slave.spawn_command(cmd).map_err(|_| {
            anyhow!("Cannot start the CLI; check executable, arguments and project path")
        })?;
        drop(pair.slave);
        let killer = child.clone_killer();
        let mut reader = pair.master.try_clone_reader()?;
        let writer = pair.master.take_writer()?;
        *e.transport.lock().unwrap() = Some(Transport::Pty {
            writer: Mutex::new(writer),
            master: Mutex::new(pair.master),
            killer: Mutex::new(killer),
        });
        let manager = self.clone();
        std::thread::spawn(move || {
            let mut buffer = [0u8; 8192];
            loop {
                match reader.read(&mut buffer) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => manager.output(&e, &buffer[..n]),
                }
            }
            let _ = child.wait();
            manager.finished(&e);
        });
        Ok(())
    }
    async fn start_codex(
        self: &Arc<Self>,
        e: Arc<Entry>,
        p: &Profile,
        exe: &str,
        cwd: &str,
        s: &Settings,
    ) -> Result<()> {
        let mut env = vec![];
        if !p.config_dir.is_empty() {
            env.push((
                "CODEX_HOME".into(),
                crate::discovery::expand(&p.config_dir)
                    .to_string_lossy()
                    .into_owned(),
            ));
        }
        let mut rpc_args = p.args.clone();
        rpc_args.push("app-server".into());
        let rpc = RpcClient::spawn(
            exe,
            &rpc_args,
            Some(cwd),
            &env,
            s.interaction_timeout_seconds,
        )
        .await?;
        let mut guard = crate::protocol::RpcCloseGuard::new(rpc.clone());
        let mut events = rpc.events.subscribe();
        let init=async{rpc.call("initialize",json!({"clientInfo":{"name":"agent_orbit","title":"Agent Orbit","version":env!("CARGO_PKG_VERSION")},"capabilities":{"experimentalApi":true}})).await?;rpc.send(json!({"method":"initialized"})).await?;rpc.call("thread/start",json!({"cwd":cwd})).await}.await;
        let result = match init {
            Ok(v) => v,
            Err(err) => {
                rpc.close();
                return Err(err);
            }
        };
        let thread_id = result["thread"]["id"]
            .as_str()
            .ok_or_else(|| anyhow!("Codex did not return a thread ID"))?
            .to_owned();
        *e.transport.lock().unwrap() = Some(Transport::Rpc {
            client: rpc.clone(),
            thread_id,
        });
        guard.disarm();
        let manager = self.clone();
        let seconds = s.interaction_timeout_seconds;
        tokio::spawn(async move {
            while let Ok(message) = events.recv().await {
                let method = message["method"].as_str().unwrap_or("").to_owned();
                let params = &message["params"];
                if method == "orbit/closed" {
                    break;
                }
                if message.get("id").is_some() {
                    manager
                        .broker
                        .register_wire(&e.info.lock().unwrap().id, &message["id"].to_string());
                    let m = manager.clone();
                    let entry = e.clone();
                    let client = rpc.clone();
                    tokio::spawn(async move {
                        m.codex_request(entry, client, message, seconds).await;
                    });
                } else if let Some(delta) = params["delta"].as_str() {
                    manager.output(&e, delta.as_bytes());
                } else if method == "turn/completed" {
                    manager.output(&e, b"\r\n");
                    manager.changed();
                } else if method == "serverRequest/resolved" {
                    manager
                        .broker
                        .resolve_wire(&e.info.lock().unwrap().id, &params["requestId"].to_string());
                    manager.changed();
                }
                if method == "turn/completed" || method == "turn/failed" {
                    manager.broker.cancel_session(&e.info.lock().unwrap().id);
                    manager.changed();
                }
            }
            rpc.close();
            manager.finished(&e);
        });
        Ok(())
    }
    async fn codex_request(
        &self,
        e: Arc<Entry>,
        rpc: Arc<RpcClient>,
        message: Value,
        seconds: u64,
    ) {
        let method = message["method"].as_str().unwrap_or("");
        let params = &message["params"];
        let wire_id = message["id"].clone();
        let info = e.info.lock().unwrap().clone();
        let mut request = Interaction {
            profile_label: e.profile.label.clone(),
            cwd: info.cwd.clone(),
            id: uuid::Uuid::new_v4().to_string(),
            session_id: info.id.clone(),
            provider_id: info.provider_id,
            title: method.rsplit('/').nth(1).unwrap_or("Approval").into(),
            detail: serde_json::to_string_pretty(params).unwrap_or_default(),
            kind: "permission".into(),
            choices: vec![],
            questions: vec![],
            created_at: chrono::Utc::now().timestamp(),
            expires_at: chrono::Utc::now().timestamp() + seconds as i64,
        };
        let mut decisions = HashMap::new();
        match method {
            "item/commandExecution/requestApproval" | "item/fileChange/requestApproval" => {
                let choices = params["availableDecisions"]
                    .as_array()
                    .cloned()
                    .unwrap_or_else(|| vec![json!("accept"), json!("decline")]);
                for (i, v) in choices.into_iter().enumerate() {
                    let id = i.to_string();
                    request.choices.push(Choice {
                        id: id.clone(),
                        label: match v.as_str() {
                            Some("accept") => "Allow once".into(),
                            Some("decline") => "Deny".into(),
                            Some("cancel") => "Cancel".into(),
                            _ => v.to_string(),
                        },
                    });
                    decisions.insert(id, v);
                }
            }
            "item/tool/requestUserInput" => {
                request.kind = "question".into();
                request.title = "Agent needs your answer".into();
                for q in params["questions"].as_array().into_iter().flatten() {
                    request.questions.push(Question {
                        id: q["id"].as_str().unwrap_or("").into(),
                        title: q["question"].as_str().unwrap_or("Question").into(),
                        multiple: false,
                        options: q["options"]
                            .as_array()
                            .into_iter()
                            .flatten()
                            .filter_map(|o| {
                                o["label"].as_str().map(|l| Choice {
                                    id: l.into(),
                                    label: l.into(),
                                })
                            })
                            .collect(),
                    });
                }
            }
            "item/permissions/requestApproval" => {
                request.choices = vec![
                    Choice {
                        id: "grant".into(),
                        label: "Grant requested permissions for this turn".into(),
                    },
                    Choice {
                        id: "deny".into(),
                        label: "Deny".into(),
                    },
                ];
            }
            _ => {
                let _=rpc.send(json!({"jsonrpc":"2.0","id":wire_id,"error":{"code":-32601,"message":"Unsupported interactive request; use terminal mode"}})).await;
                return;
            }
        }
        let id = request.id.clone();
        let Some(rx) = self.broker.insert_wire(request, &wire_id.to_string()) else {
            return;
        };
        self.changed();
        let reply = tokio::time::timeout(std::time::Duration::from_secs(seconds), rx).await;
        self.broker.remove(&id);
        self.changed();
        if !self.broker.wire_active(&info.id, &wire_id.to_string()) {
            return;
        }
        let result = match reply {
            Ok(Ok(reply)) if method == "item/tool/requestUserInput" => {
                let answers: serde_json::Map<String, Value> = reply
                    .answers
                    .into_iter()
                    .map(|(k, v)| (k, json!({"answers":v})))
                    .collect();
                json!({"answers":answers})
            }
            Ok(Ok(reply)) if method == "item/permissions/requestApproval" => {
                json!({"permissions":if reply.choice.as_deref()==Some("grant"){params["permissions"].clone()}else{json!({})},"scope":"turn"})
            }
            Ok(Ok(reply)) => {
                json!({"decision":decisions.get(reply.choice.as_deref().unwrap_or("")).cloned().unwrap_or(json!("decline"))})
            }
            _ if method == "item/permissions/requestApproval" => {
                json!({"permissions":{},"scope":"turn"})
            }
            _ if method == "item/tool/requestUserInput" => json!({"answers":{}}),
            _ => json!({"decision":"decline"}),
        };
        self.broker.resolve_wire(&info.id, &wire_id.to_string());
        let _ = rpc
            .send(json!({"jsonrpc":"2.0","id":wire_id,"result":result}))
            .await;
    }
    pub fn write(&self, id: &str, data: &str) -> Result<()> {
        ensure!(data.len() <= 65536, "Input exceeds size limit");
        let e = self.entry(id)?;
        ensure!(
            e.info.lock().unwrap().status == "running",
            "Session has exited"
        );
        let guard = e.transport.lock().unwrap();
        match guard.as_ref() {
            Some(Transport::Pty { writer, .. }) => {
                writer.lock().unwrap().write_all(data.as_bytes())?;
                Ok(())
            }
            _ => Err(anyhow!("Use the message box for structured sessions")),
        }
    }
    pub async fn prompt(&self, id: &str, text: &str) -> Result<()> {
        ensure!(
            !text.trim().is_empty() && text.len() <= 65536,
            "Enter a message up to 64 KB"
        );
        let e = self.entry(id)?;
        let (client, thread_id) = {
            let guard = e.transport.lock().unwrap();
            match guard.as_ref() {
                Some(Transport::Rpc { client, thread_id }) => (client.clone(), thread_id.clone()),
                _ => return Err(anyhow!("Not a structured session")),
            }
        };
        client
            .call(
                "turn/start",
                json!({"threadId":thread_id,"input":[{"type":"text","text":text}]}),
            )
            .await?;
        Ok(())
    }
    pub fn resize(&self, id: &str, cols: u16, rows: u16) -> Result<()> {
        ensure!(
            (10..=500).contains(&cols) && (3..=200).contains(&rows),
            "Invalid terminal size"
        );
        let e = self.entry(id)?;
        let guard = e.transport.lock().unwrap();
        if let Some(Transport::Pty { master, .. }) = guard.as_ref() {
            master.lock().unwrap().resize(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })?;
        }
        Ok(())
    }
    pub fn stop(&self, id: &str) -> Result<()> {
        let e = self.entry(id)?;
        self.broker.cancel_session(id);
        let transport = e.transport.lock().unwrap();
        match transport.as_ref() {
            Some(Transport::Pty { killer, .. }) => {
                let _ = killer.lock().unwrap().kill();
            }
            Some(Transport::Rpc { client, .. }) => client.close(),
            _ => {}
        }
        drop(transport);
        self.finished(&e);
        Ok(())
    }
    pub fn stop_all(&self) {
        for info in self.list() {
            let _ = self.stop(&info.id);
        }
    }
}
fn quote_command(value: &str) -> String {
    #[cfg(windows)]
    {
        format!("\"{}\"", value.replace('"', "\"\""))
    }
    #[cfg(not(windows))]
    {
        format!("'{}'", value.replace('\'', "'\\''"))
    }
}
