use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, oneshot, Mutex};
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CdpRequest {
    pub id: u64,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
    #[serde(rename = "sessionId", skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CdpResponse {
    pub id: Option<u64>,
    pub result: Option<Value>,
    pub error: Option<Value>,
    pub method: Option<String>,
    pub params: Option<Value>,
    #[serde(rename = "sessionId")]
    pub session_id: Option<String>,
}

#[derive(Clone)]
pub struct CdpClient {
    next_id: Arc<AtomicU64>,
    outgoing_tx: mpsc::Sender<Message>,
    pending_requests: Arc<Mutex<HashMap<u64, oneshot::Sender<Result<Value, String>>>>>,
    pub session_id: Option<String>,
}

impl CdpClient {
    /// Connects to a browser WebSocket URL and starts the duplex event loop.
    pub async fn connect(ws_url: &str) -> Result<Self, String> {
        let (ws_stream, _) = connect_async(ws_url)
            .await
            .map_err(|e| format!("WebSocket connection error to {}: {}", ws_url, e))?;

        let (mut write, mut read) = ws_stream.split();
        let (outgoing_tx, mut outgoing_rx) = mpsc::channel::<Message>(128);
        let pending_requests: Arc<Mutex<HashMap<u64, oneshot::Sender<Result<Value, String>>>>> =
            Arc::new(Mutex::new(HashMap::new()));

        let pending_for_read = Arc::clone(&pending_requests);

        // Outgoing writer
        tokio::spawn(async move {
            while let Some(msg) = outgoing_rx.recv().await {
                if let Err(e) = write.send(msg).await {
                    eprintln!("cdpx: failed to send WS message: {}", e);
                    break;
                }
            }
        });

        // Incoming reader
        tokio::spawn(async move {
            while let Some(msg_result) = read.next().await {
                match msg_result {
                    Ok(Message::Text(text)) => {
                        if let Ok(resp) = serde_json::from_str::<CdpResponse>(&text) {
                            if let Some(id) = resp.id {
                                let mut pending = pending_for_read.lock().await;
                                if let Some(sender) = pending.remove(&id) {
                                    if let Some(res) = resp.result {
                                        let _ = sender.send(Ok(res));
                                    } else if let Some(err) = resp.error {
                                        let _ = sender.send(Err(err.to_string()));
                                    } else {
                                        let _ = sender.send(Ok(Value::Null));
                                    }
                                }
                            }
                        }
                    }
                    Ok(Message::Close(_)) => break,
                    Err(_) => break,
                    _ => {}
                }
            }

            // Drain any pending requests to prevent hanging callers on connection drop
            let mut pending = pending_for_read.lock().await;
            for (_, tx) in pending.drain() {
                let _ = tx.send(Err("CDP connection closed".to_string()));
            }
        });

        let browser_client = CdpClient {
            next_id: Arc::new(AtomicU64::new(1)),
            outgoing_tx,
            pending_requests,
            session_id: None,
        };

        // Create or attach to target page
        browser_client.attach_to_page().await
    }

    /// Attaches to an existing page or creates a new target, returning a session-scoped client.
    pub async fn attach_to_page(&self) -> Result<Self, String> {
        let targets = self.call("Target.getTargets", None).await?;
        let mut page_target_id: Option<String> = None;

        if let Some(target_infos) = targets.get("targetInfos").and_then(|v| v.as_array()) {
            for info in target_infos {
                if info.get("type").and_then(|t| t.as_str()) == Some("page") {
                    if let Some(tid) = info.get("targetId").and_then(|i| i.as_str()) {
                        page_target_id = Some(tid.to_string());
                        break;
                    }
                }
            }
        }

        let target_id = match page_target_id {
            Some(tid) => tid,
            None => {
                let new_target = self
                    .call("Target.createTarget", Some(json!({ "url": "about:blank" })))
                    .await?;
                new_target
                    .get("targetId")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| "Failed to get targetId from Target.createTarget".to_string())?
                    .to_string()
            }
        };

        let attach_res = self
            .call(
                "Target.attachToTarget",
                Some(json!({ "targetId": target_id, "flatten": true })),
            )
            .await?;

        let session_id = attach_res
            .get("sessionId")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "Failed to get sessionId from Target.attachToTarget".to_string())?
            .to_string();

        Ok(CdpClient {
            next_id: Arc::clone(&self.next_id),
            outgoing_tx: self.outgoing_tx.clone(),
            pending_requests: Arc::clone(&self.pending_requests),
            session_id: Some(session_id),
        })
    }

    /// Sends a raw CDP command and awaits the JSON result.
    pub async fn call(&self, method: &str, params: Option<Value>) -> Result<Value, String> {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let req = CdpRequest {
            id,
            method: method.to_string(),
            params,
            session_id: self.session_id.clone(),
        };

        let json_str = serde_json::to_string(&req).map_err(|e| e.to_string())?;
        let (tx, rx) = oneshot::channel();

        {
            let mut pending = self.pending_requests.lock().await;
            pending.insert(id, tx);
        }

        self.outgoing_tx
            .send(Message::Text(json_str.into()))
            .await
            .map_err(|e| format!("Failed to queue CDP command: {}", e))?;

        rx.await
            .map_err(|_| "CDP response channel dropped prematurely".to_string())?
    }

    /// Navigates current target to URL and waits for document readiness.
    pub async fn navigate(&self, url: &str) -> Result<(), String> {
        let _ = self.call("Page.enable", None).await;
        self.call("Page.navigate", Some(json!({ "url": url }))).await?;

        // Await document.readyState transition to interactive or complete
        let start = std::time::Instant::now();
        while start.elapsed() < Duration::from_secs(8) {
            if let Ok(val) = self.evaluate("document.readyState").await {
                if let Some(s) = val.as_str() {
                    if s == "complete" || s == "interactive" {
                        break;
                    }
                }
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        Ok(())
    }

    /// Evaluates JavaScript expression and returns result.
    pub async fn evaluate(&self, expression: &str) -> Result<Value, String> {
        let res = self
            .call(
                "Runtime.evaluate",
                Some(json!({
                    "expression": expression,
                    "returnByValue": true,
                    "awaitPromise": true
                })),
            )
            .await?;

        if let Some(result_obj) = res.get("result") {
            if let Some(val) = result_obj.get("value") {
                return Ok(val.clone());
            }
        }
        Ok(res)
    }
}
