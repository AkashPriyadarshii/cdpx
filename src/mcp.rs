use crate::action::{click_element, type_element, wait_for_settle};
use crate::browser::{launch_browser, BrowserInstance};
use crate::cdp::CdpClient;
use crate::sasp::SaspSnapshot;
use serde_json::{json, Value};
use std::io::{self, BufRead, Write};
use std::sync::Arc;
use tokio::sync::Mutex;

const DOM_WALKER_JS: &str = include_str!("dom_walker.min.js");

pub struct McpState {
    pub browser: Option<BrowserInstance>,
    pub client: Option<CdpClient>,
}

pub async fn run_mcp_server() -> Result<(), Box<dyn std::error::Error>> {
    let state = Arc::new(Mutex::new(McpState {
        browser: None,
        client: None,
    }));

    let stdin = io::stdin();
    let mut stdout = io::stdout();

    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => break,
        };

        if line.trim().is_empty() {
            continue;
        }

        let req: Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(_) => continue,
        };

        let id = req.get("id").cloned();
        let method = req.get("method").and_then(|m| m.as_str()).unwrap_or("");

        let response = match method {
            "initialize" => json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {
                    "protocolVersion": "2024-11-05",
                    "capabilities": { "tools": {} },
                    "serverInfo": {
                        "name": "cdpx",
                        "version": env!("CARGO_PKG_VERSION")
                    }
                }
            }),

            "tools/list" => json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {
                    "tools": [
                        {
                            "name": "browser_navigate",
                            "description": "Navigate to a URL and return the token-compressed interactive element map.",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "url": { "type": "string", "description": "Target HTTP/HTTPS URL" }
                                },
                                "required": ["url"]
                            }
                        },
                        {
                            "name": "browser_act",
                            "description": "Perform an action (click, type) on an element reference (@e1, @e2).",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "ref": { "type": "string", "description": "Element handle (@e1, @e2)" },
                                    "action": { "type": "string", "enum": ["click", "type"] },
                                    "value": { "type": "string", "description": "Text value when typing" }
                                },
                                "required": ["ref", "action"]
                            }
                        },
                        {
                            "name": "browser_snapshot",
                            "description": "Returns a refreshed token-budgeted interactive element snapshot of the current page.",
                            "inputSchema": {
                                "type": "object"
                            }
                        }
                    ]
                }
            }),

            "tools/call" => {
                let params = req.get("params");
                let tool_name = params.and_then(|p| p.get("name")).and_then(|n| n.as_str()).unwrap_or("");
                let arguments = params.and_then(|p| p.get("arguments")).cloned().unwrap_or(json!({}));

                let res_text = handle_tool_call(Arc::clone(&state), tool_name, arguments).await;

                json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": {
                        "content": [
                            {
                                "type": "text",
                                "text": res_text
                            }
                        ]
                    }
                })
            }

            _ => json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {}
            }),
        };

        let out_str = serde_json::to_string(&response)?;
        writeln!(stdout, "{}", out_str)?;
        stdout.flush()?;
    }

    Ok(())
}

async fn handle_tool_call(state: Arc<Mutex<McpState>>, name: &str, args: Value) -> String {
    let mut s = state.lock().await;

    // Ensure browser is running
    if s.browser.is_none() || s.client.is_none() {
        match launch_browser(true).await {
            Ok(inst) => match CdpClient::connect(&inst.ws_url).await {
                Ok(c) => {
                    s.browser = Some(inst);
                    s.client = Some(c);
                }
                Err(e) => return format!("Failed to connect to browser CDP: {}", e),
            },
            Err(e) => return format!("Failed to launch browser: {}", e),
        }
    }

    let client = s.client.as_ref().unwrap();

    match name {
        "browser_navigate" => {
            let url = args.get("url").and_then(|u| u.as_str()).unwrap_or("https://example.com");
            if let Err(e) = client.navigate(url).await {
                return format!("Navigation failed: {}", e);
            }
            let _ = wait_for_settle(client, 1500).await;
            extract_snapshot(client).await
        }

        "browser_act" => {
            let ref_str = args.get("ref").and_then(|r| r.as_str()).unwrap_or("");
            let action = args.get("action").and_then(|a| a.as_str()).unwrap_or("");
            let value = args.get("value").and_then(|v| v.as_str()).unwrap_or("");

            match action {
                "click" => {
                    if let Err(e) = click_element(client, ref_str).await {
                        return format!("Click failed on {}: {}", ref_str, e);
                    }
                }
                "type" => {
                    if let Err(e) = type_element(client, ref_str, value).await {
                        return format!("Type failed on {}: {}", ref_str, e);
                    }
                }
                _ => return format!("Unsupported action: {}", action),
            }

            let _ = wait_for_settle(client, 800).await;
            extract_snapshot(client).await
        }

        "browser_snapshot" => extract_snapshot(client).await,

        _ => format!("Unknown tool: {}", name),
    }
}

pub async fn extract_snapshot(client: &CdpClient) -> String {
    match client.evaluate(DOM_WALKER_JS).await {
        Ok(val) => {
            if let Ok(snap) = serde_json::from_value::<SaspSnapshot>(val) {
                snap.to_compact_string()
            } else {
                "Failed to parse DOM snapshot into SASP format".to_string()
            }
        }
        Err(e) => format!("Failed to evaluate tree walker: {}", e),
    }
}
