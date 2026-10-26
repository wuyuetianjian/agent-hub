//! Private stdio transport to the separately signed macOS accessory application.
//! No listener, shell, environment-selected executable, or duplicate command dispatcher.
use crate::overlay_protocol::{self as protocol, MAX_FRAME, MAX_JS_ID, MAX_PENDING};
use base64::Engine;
use serde_json::{json, Value};
use std::{
    collections::{HashMap, HashSet},
    io::{BufReader, Write},
    process::{Child, Command, Stdio},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{self, SyncSender},
        Arc, Mutex,
    },
    time::{Duration, Instant},
};
use tauri::{Emitter, Manager};

const INVOKE_TTL: Duration = Duration::from_secs(120);
struct Pending {
    value: Value,
    started: Instant,
    dispatched: bool,
}
#[derive(Default)]
struct Hosts {
    ready: HashSet<String>,
    pending: HashMap<(String, u64), Pending>,
}
struct Inner {
    app: tauri::AppHandle,
    stopped: AtomicBool,
    failure: Mutex<Option<String>>,
    child: Mutex<Option<Child>>,
    writer: Mutex<Option<SyncSender<Vec<u8>>>>,
    hosts: Mutex<Hosts>,
}
pub struct OverlayTransport {
    inner: Arc<Inner>,
}

pub fn start(app: &tauri::AppHandle) -> Result<OverlayTransport, String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let helper = exe
        .parent()
        .ok_or("application executable has no directory")?
        .join("agent-orbit-overlay");
    let mut child = Command::new(&helper)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .map_err(|e| format!("cannot start {}: {e}", helper.display()))?;
    let stdin = child.stdin.take().expect("requested piped stdin");
    let stdout = child.stdout.take().expect("requested piped stdout");
    let (tx, rx) = mpsc::sync_channel::<Vec<u8>>(128);
    let inner = Arc::new(Inner {
        app: app.clone(),
        stopped: AtomicBool::new(false),
        failure: Mutex::new(None),
        child: Mutex::new(Some(child)),
        writer: Mutex::new(Some(tx)),
        hosts: Mutex::new(Hosts::default()),
    });
    let weak = Arc::downgrade(&inner);
    std::thread::spawn(move || {
        let mut stdin = stdin;
        while let Ok(frame) = rx.recv() {
            if let Err(e) = stdin.write_all(&frame).and_then(|_| stdin.flush()) {
                if let Some(inner) = weak.upgrade() {
                    inner.fail(format!("overlay writer: {e}"));
                }
                break;
            }
        }
    });
    let weak = Arc::downgrade(&inner);
    std::thread::spawn(move || {
        let mut reader = BufReader::new(stdout);
        loop {
            let frame = protocol::read_frame(&mut reader);
            let Some(inner) = weak.upgrade() else {
                break;
            };
            if inner.stopped.load(Ordering::Acquire) {
                break;
            }
            let result = match frame {
                Ok(Some(bytes)) => serde_json::from_slice::<Value>(&bytes)
                    .map_err(|e| format!("invalid overlay JSON: {e}"))
                    .and_then(|v| inner.receive(v)),
                Ok(None) => Err("overlay helper closed its pipe".into()),
                Err(e) => Err(format!("overlay reader: {e}")),
            };
            if let Err(e) = result {
                inner.fail(e);
                break;
            }
        }
    });
    let weak = Arc::downgrade(&inner);
    std::thread::spawn(move || loop {
        std::thread::sleep(Duration::from_secs(1));
        let Some(inner) = weak.upgrade() else {
            break;
        };
        if inner.stopped.load(Ordering::Acquire) {
            break;
        }
        let expired = {
            let mut hosts = inner.hosts.lock().unwrap();
            let keys: Vec<_> = hosts
                .pending
                .iter()
                .filter(|(_, p)| {
                    // Even a modal request must expire if its hidden host never becomes ready.
                    let command = p.value.get("command").and_then(Value::as_str).unwrap_or("");
                    p.started.elapsed() >= INVOKE_TTL
                        && (!p.dispatched || protocol::invocation_has_deadline(command))
                })
                .map(|(k, _)| k.clone())
                .collect();
            for k in &keys {
                hosts.pending.remove(k);
            }
            keys
        };
        for (label, id) in expired {
            if let Err(e) = inner.send(json!({"type":"reply","label":label,"id":id,"ok":false,"value":"overlay invocation timed out"})) { inner.fail(e); break; }
        }
    });
    let transport = OverlayTransport { inner };
    for (label, suffix) in [
        ("main", ""),
        ("dock-panel", "?panel=1"),
        ("task-completion", "?completion=1"),
    ] {
        transport.send(json!({"type":"create","label":label,"url":format!("orbit-asset://localhost/index.html{suffix}")}))?;
    }
    Ok(transport)
}
impl OverlayTransport {
    pub fn send(&self, value: Value) -> Result<(), String> {
        self.inner.send(value)
    }
    pub fn status(&self) -> Option<String> {
        if self.inner.stopped.load(Ordering::Acquire) {
            Some(
                self.inner
                    .failure
                    .lock()
                    .unwrap()
                    .clone()
                    .unwrap_or_else(|| "overlay helper stopped".into()),
            )
        } else {
            None
        }
    }
    pub fn stop(&self) {
        self.inner.stop();
    }
    pub fn host_ready(&self, label: &str) -> Result<(), String> {
        if !protocol::valid_label(label) {
            return Err("invalid overlay host".into());
        }
        let queued = {
            let mut hosts = self.inner.hosts.lock().unwrap();
            hosts.ready.insert(label.into());
            hosts
                .pending
                .iter_mut()
                .filter_map(|((l, id), p)| {
                    if l == label && !p.dispatched {
                        p.dispatched = true;
                        Some((*id, p.value.clone()))
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>()
        };
        for (id, value) in queued {
            self.inner.dispatch(label, id, value);
        }
        Ok(())
    }
    pub fn reply(&self, label: &str, id: u64, ok: bool, value: Value) -> Result<(), String> {
        let pending = self
            .inner
            .hosts
            .lock()
            .unwrap()
            .pending
            .remove(&(label.to_owned(), id));
        if !pending.is_some_and(|p| p.dispatched) {
            return Err("unknown overlay invocation".into());
        }
        self.send(json!({"type":"reply","label":label,"id":id,"ok":ok,"value":value}))
    }
}
impl Drop for OverlayTransport {
    fn drop(&mut self) {
        self.stop();
    }
}
impl Inner {
    fn send(&self, value: Value) -> Result<(), String> {
        if self.stopped.load(Ordering::Acquire) {
            return Err("overlay helper unavailable".into());
        }
        let mut bytes = serde_json::to_vec(&value).map_err(|e| e.to_string())?;
        if bytes.len() > MAX_FRAME {
            return Err("overlay frame exceeds limit".into());
        }
        bytes.push(b'\n');
        let result = {
            let writer = self.writer.lock().unwrap();
            writer
                .as_ref()
                .ok_or("overlay helper unavailable")?
                .try_send(bytes)
                .map_err(|e| format!("overlay writer unavailable or full: {e}"))
        };
        // Never continue after dropping a layout/reply frame: native and proxy state diverge.
        if let Err(error) = &result {
            self.fail(error.clone());
        }
        result
    }
    fn stop(&self) {
        if self.stopped.swap(true, Ordering::AcqRel) {
            return;
        }
        self.writer.lock().unwrap().take();
        self.hosts.lock().unwrap().pending.clear();
        // Kill before wait: a stuck child cannot block application shutdown indefinitely.
        if let Some(mut child) = self.child.lock().unwrap().take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
    fn fail(&self, error: String) {
        if !self.stopped.load(Ordering::Acquire) {
            *self.failure.lock().unwrap() = Some(error.clone());
            eprintln!("{error}");
            let _ = self.app.emit("orbit:overlay-error", &error);
            let app = self.app.clone();
            let _ = self.app.run_on_main_thread(move || {
                if let Some(workspace) = app.get_webview_window("workspace") {
                    let _ = workspace.show();
                    let _ = workspace.set_focus();
                }
            });
        }
        self.stop();
    }
    fn dispatch(&self, label: &str, id: u64, value: Value) {
        // Never hold our mutex across webview getters or eval: they may schedule on the UI thread.
        let result = self
            .app
            .get_webview_window(label)
            .ok_or_else(|| "overlay host webview missing".to_string())
            .and_then(|window| {
                window
                    .eval(&format!("window.__ORBIT_HOST_INVOKE__({value})"))
                    .map_err(|e| e.to_string())
            });
        if let Err(error) = result {
            self.hosts
                .lock()
                .unwrap()
                .pending
                .remove(&(label.to_owned(), id));
            if let Err(e) =
                self.send(json!({"type":"reply","label":label,"id":id,"ok":false,"value":error}))
            {
                self.fail(e);
            }
        }
    }
    fn receive(&self, value: Value) -> Result<(), String> {
        match value.get("type").and_then(Value::as_str) {
            Some("ready") => Ok(()),
            Some("content-error") => {
                let label = value
                    .get("label")
                    .and_then(Value::as_str)
                    .ok_or("invalid content error label")?;
                let error = value
                    .get("error")
                    .and_then(Value::as_str)
                    .ok_or("invalid content error diagnostic")?;
                if !protocol::valid_content_error(label, error) {
                    return Err("invalid overlay content error".into());
                }
                Err(format!("Overlay {label} content unavailable: {error}"))
            }
            Some("closed") => {
                let label = value
                    .get("label")
                    .and_then(Value::as_str)
                    .filter(|l| protocol::valid_label(l))
                    .ok_or("invalid closed overlay label")?
                    .to_owned();
                let app = self.app.clone();
                self.app
                    .run_on_main_thread(move || crate::desktop::overlay_closed(&app, &label))
                    .map_err(|e| e.to_string())
            }
            Some("geometry") => {
                let label = value
                    .get("label")
                    .and_then(Value::as_str)
                    .filter(|l| protocol::valid_label(l))
                    .ok_or("invalid overlay geometry label")?
                    .to_owned();
                let number = |key| {
                    value
                        .get(key)
                        .and_then(Value::as_f64)
                        .ok_or("invalid overlay geometry")
                };
                let (x, y, width, height) = (
                    number("x")?,
                    number("y")?,
                    number("width")?,
                    number("height")?,
                );
                if !protocol::valid_geometry(x, y, width, height) {
                    return Err("invalid overlay geometry".into());
                }
                let app = self.app.clone();
                self.app
                    .run_on_main_thread(move || {
                        // Raw Tauri webview, deliberately not the mirroring facade.
                        if let Some(window) = app.get_webview_window(&label) {
                            let _ = window.set_position(tauri::LogicalPosition::new(x, y));
                            let _ = window.set_size(tauri::LogicalSize::new(width, height));
                        }
                    })
                    .map_err(|e| e.to_string())
            }
            Some("asset") => {
                let id = value
                    .get("id")
                    .and_then(Value::as_str)
                    .filter(|s| {
                        !s.is_empty()
                            && s.len() <= 80
                            && s.bytes()
                                .all(|b| b.is_ascii_alphanumeric() || b"-:".contains(&b))
                    })
                    .ok_or("invalid overlay asset id")?;
                let path = value
                    .get("path")
                    .and_then(Value::as_str)
                    .ok_or("invalid overlay asset path")?;
                let path = path.strip_prefix('/').unwrap_or(path);
                if !protocol::valid_asset_path(path) {
                    return self.send(json!({"type":"asset","id":id,"error":"invalid asset path"}));
                }
                match self.app.asset_resolver().get(path.to_owned()) {
                    Some(asset) if asset.bytes.len() <= (MAX_FRAME - 4096) * 3 / 4 => self.send(json!({"type":"asset","id":id,"mime":asset.mime_type,"body":base64::engine::general_purpose::STANDARD.encode(asset.bytes)})),
                    _ => self.send(json!({"type":"asset","id":id,"error":"asset missing or too large"})),
                }
            }
            Some("invoke") => {
                let label = value
                    .get("label")
                    .and_then(Value::as_str)
                    .filter(|l| protocol::valid_label(l))
                    .ok_or("invalid overlay label")?;
                let id = value
                    .get("id")
                    .and_then(Value::as_u64)
                    .filter(|id| *id > 0 && *id <= MAX_JS_ID)
                    .ok_or("invalid overlay invocation id")?;
                let command = value
                    .get("command")
                    .and_then(Value::as_str)
                    .ok_or("invalid overlay command")?;
                if !protocol::allowed_command(label, command)
                    || !value.get("args").is_some_and(Value::is_object)
                {
                    return self.send(json!({"type":"reply","label":label,"id":id,"ok":false,"value":"overlay command not allowed or invalid args"}));
                }
                let dispatch = {
                    let mut hosts = self.hosts.lock().unwrap();
                    let key = (label.to_owned(), id);
                    if hosts.pending.contains_key(&key) {
                        return Err("duplicate outstanding overlay invocation".into());
                    }
                    if hosts.pending.len() >= MAX_PENDING {
                        drop(hosts);
                        return self.send(json!({"type":"reply","label":label,"id":id,"ok":false,"value":"too many pending overlay invocations"}));
                    }
                    let ready = hosts.ready.contains(label);
                    hosts.pending.insert(
                        key,
                        Pending {
                            value: value.clone(),
                            started: Instant::now(),
                            dispatched: ready,
                        },
                    );
                    ready
                };
                if dispatch {
                    self.dispatch(label, id, value.clone());
                }
                Ok(())
            }
            _ => Err("unsupported overlay message".into()),
        }
    }
}
