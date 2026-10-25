use anyhow::{anyhow, bail, Result};
use serde_json::{json, Value};
use std::{
    collections::HashMap,
    process::Stdio,
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc, Mutex,
    },
};
use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    process::{ChildStdin, Command},
    sync::{broadcast, oneshot},
};

pub struct RpcClient {
    stdin: tokio::sync::Mutex<ChildStdin>,
    pending: Mutex<HashMap<u64, oneshot::Sender<std::result::Result<Value, String>>>>,
    next: AtomicU64,
    alive: AtomicBool,
    pub events: broadcast::Sender<Value>,
    stop: Mutex<Option<oneshot::Sender<()>>>,
    timeout: u64,
}
impl RpcClient {
    pub async fn spawn(
        program: &str,
        args: &[String],
        cwd: Option<&str>,
        env: &[(String, String)],
        timeout: u64,
    ) -> Result<Arc<Self>> {
        let (program, args) = crate::launch::resolve(program, args)?;
        let mut cmd = Command::new(program);
        cmd.args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true);
        if let Some(cwd) = cwd {
            cmd.current_dir(cwd);
        }
        for (k, v) in env {
            cmd.env(k, v);
        }
        #[cfg(windows)]
        cmd.creation_flags(0x08000000);
        let mut child = cmd
            .spawn()
            .map_err(|_| anyhow!("Cannot launch agent executable"))?;
        let stdin = child.stdin.take().unwrap();
        let stdout = child.stdout.take().unwrap();
        let (events, _) = broadcast::channel(256);
        let (stop_tx, stop_rx) = oneshot::channel();
        let client = Arc::new(Self {
            stdin: tokio::sync::Mutex::new(stdin),
            pending: Mutex::new(HashMap::new()),
            next: AtomicU64::new(1),
            alive: AtomicBool::new(true),
            events,
            stop: Mutex::new(Some(stop_tx)),
            timeout,
        });
        let c = client.clone();
        tokio::spawn(async move {
            let mut reader = BufReader::new(stdout);
            loop {
                let mut line = String::new();
                let n = (&mut reader)
                    .take(2 * 1024 * 1024 + 1)
                    .read_line(&mut line)
                    .await;
                match n {
                    Ok(0) | Err(_) => break,
                    Ok(n) if n > 2 * 1024 * 1024 => break,
                    _ => {}
                }
                let Ok(message) = serde_json::from_str::<Value>(&line) else {
                    continue;
                };
                if message.get("method").is_none() {
                    if let Some(id) = message["id"].as_u64() {
                        if let Some(reply) = c.pending.lock().unwrap().remove(&id) {
                            let value = if message.get("error").is_some() {
                                Err("Agent rejected the request; check login, account access and CLI version".into())
                            } else {
                                Ok(message["result"].clone())
                            };
                            let _ = reply.send(value);
                        }
                    }
                } else {
                    let _ = c.events.send(message);
                }
            }
            c.alive.store(false, Ordering::SeqCst);
            for (_, reply) in c.pending.lock().unwrap().drain() {
                let _ = reply.send(Err("Agent disconnected".into()));
            }
            let _ = c.events.send(json!({"method":"orbit/closed"}));
            c.close();
        });
        tokio::spawn(async move {
            tokio::select! {_ = child.wait()=>{},_ = stop_rx=>{let _=child.kill().await;let _=child.wait().await;}}
        });
        Ok(client)
    }
    pub async fn send(&self, value: Value) -> Result<()> {
        if !self.alive.load(Ordering::SeqCst) {
            bail!("Agent disconnected");
        }
        let mut bytes = serde_json::to_vec(&value)?;
        bytes.push(b'\n');
        let written = tokio::time::timeout(std::time::Duration::from_secs(self.timeout), async {
            let mut input = self.stdin.lock().await;
            input.write_all(&bytes).await?;
            input.flush().await
        })
        .await;
        match written {
            Ok(Ok(())) => Ok(()),
            _ => {
                self.close();
                Err(anyhow!("Agent input timed out or disconnected"))
            }
        }
    }
    pub async fn call(&self, method: &str, params: Value) -> Result<Value> {
        let id = self.next.fetch_add(1, Ordering::SeqCst);
        let (tx, rx) = oneshot::channel();
        self.pending.lock().unwrap().insert(id, tx);
        let _pending = PendingCall { client: self, id };
        let result = tokio::time::timeout(std::time::Duration::from_secs(self.timeout), async {
            self.send(json!({"jsonrpc":"2.0","id":id,"method":method,"params":params}))
                .await?;
            rx.await
                .map_err(|_| anyhow!("Agent disconnected"))?
                .map_err(anyhow::Error::msg)
        })
        .await;
        match result {
            Ok(result) => result,
            Err(_) => {
                self.close();
                Err(anyhow!("Agent request timed out"))
            }
        }
    }

    pub fn close(&self) {
        if let Some(tx) = self.stop.lock().unwrap().take() {
            let _ = tx.send(());
        }
    }
}

pub struct RpcCloseGuard {
    client: Option<Arc<RpcClient>>,
}
impl RpcCloseGuard {
    pub fn new(client: Arc<RpcClient>) -> Self {
        Self {
            client: Some(client),
        }
    }
    pub fn disarm(&mut self) {
        self.client = None;
    }
}
impl Drop for RpcCloseGuard {
    fn drop(&mut self) {
        if let Some(client) = self.client.take() {
            client.close();
        }
    }
}

struct PendingCall<'a> {
    client: &'a RpcClient,
    id: u64,
}
impl Drop for PendingCall<'_> {
    fn drop(&mut self) {
        self.client.pending.lock().unwrap().remove(&self.id);
    }
}
