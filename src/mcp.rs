use crate::action::{click_element, type_element, wait_for_settle};
use crate::browser::{BrowserInstance, launch_browser};
use crate::cdp::CdpClient;
use crate::jev;
use crate::sasp::SaspSnapshot;
use serde_json::{Value, json};
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
                            "description": "Perform an action (click, type) on an element reference (@e1, @e2). Includes Jev safety validation.",
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
                        },
                        {
                            "name": "browser_goal",
                            "description": "Set a natural-language goal. Jev selects the best element and action to execute.",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "goal": { "type": "string", "description": "Natural language goal for the browser" }
                                },
                                "required": ["goal"]
                            }
                        },
                        {
                            "name": "browser_suggest",
                            "description": "Rank all visible elements by relevance to a natural-language goal using Jev.",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "goal": { "type": "string", "description": "Natural language goal to rank elements against" }
                                },
                                "required": ["goal"]
                            }
                        }
                    ]
                }
            }),

            "tools/call" => {
                let params = req.get("params");
                let tool_name = params
                    .and_then(|p| p.get("name"))
                    .and_then(|n| n.as_str())
                    .unwrap_or("");
                let arguments = params
                    .and_then(|p| p.get("arguments"))
                    .cloned()
                    .unwrap_or(json!({}));

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
            let url = args
                .get("url")
                .and_then(|u| u.as_str())
                .unwrap_or("https://example.com");
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

            let snapshot_val = client.evaluate(DOM_WALKER_JS).await;
            let safety = match snapshot_val {
                Ok(val) => {
                    if let Ok(snap) = serde_json::from_value::<SaspSnapshot>(val) {
                        jev_safety_check(client, &snap, ref_str, action).await
                    } else {
                        true
                    }
                }
                Err(_) => true,
            };

            if !safety {
                return format!(
                    "Jev safety gate blocked action {} on {} (unsafe)",
                    action, ref_str
                );
            }

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

        "browser_goal" => {
            let goal = args.get("goal").and_then(|g| g.as_str()).unwrap_or("");
            handle_goal(client, goal).await
        }

        "browser_suggest" => {
            let goal = args.get("goal").and_then(|g| g.as_str()).unwrap_or("");
            handle_suggest(client, goal).await
        }

        _ => format!("Unknown tool: {}", name),
    }
}

pub(super) async fn jev_safety_check(
    _client: &CdpClient,
    snap: &SaspSnapshot,
    ref_str: &str,
    action: &str,
) -> bool {
    let state = json!({
        "page": {
            "url": snap.url,
            "title": snap.title,
            "text": snap.to_compact_string()
        },
        "elements": snap.elements.iter().map(|e| json!({
            "id": e.id,
            "role": e.role,
            "name": e.name,
            "x": e.x,
            "y": e.y
        })).collect::<Vec<_>>()
    });

    let questions = json!({
        "action_safe": {
            "type": "noul",
            "instructions": format!("Is it safe to {} on element {} '{}'?", action, ref_str, snap.elements.iter().find(|e| e.id.to_string() == ref_str.trim_start_matches('@').trim_start_matches('e')).map(|e| e.name.as_str()).unwrap_or(ref_str)),
            "criteria": {
                "true": "Action is safe and expected",
                "false": "Action is dangerous, blocked, or unexpected"
            }
        }
    });

    match jev::noul(state, questions).await {
        Ok(result) => result.noul >= 0.7,
        Err(_) => true,
    }
}

pub(super) async fn handle_goal(client: &CdpClient, goal: &str) -> String {
    let snapshot_val = client.evaluate(DOM_WALKER_JS).await;
    let snap = match snapshot_val {
        Ok(val) => match serde_json::from_value::<SaspSnapshot>(val) {
            Ok(s) => s,
            Err(_) => return "Failed to parse snapshot".to_string(),
        },
        Err(_) => return "Failed to get snapshot".to_string(),
    };

    let state = json!({
        "page": {
            "url": snap.url,
            "title": snap.title,
            "text": snap.to_compact_string()
        },
        "elements": snap.elements.iter().map(|e| json!({
            "id": e.id,
            "role": e.role,
            "name": e.name,
            "value": e.value,
            "x": e.x,
            "y": e.y
        })).collect::<Vec<_>>()
    });

    let operations = ["CLICK", "TYPE_TEXT", "DONE"];
    let _criteria: std::collections::HashMap<String, String> = snap
        .elements
        .iter()
        .map(|e| (format!("@e{}", e.id), format!("{} '{}'", e.role, e.name)))
        .collect();

    let questions = json!({
        "operation": {
            "type": "choice",
            "criteria": operations.iter().map(|op| (op.to_string(), format!("Perform {} operation", op))).collect::<std::collections::HashMap<String, String>>(),
            "instructions": {"goal": goal, "rules": "Choose the operation that best advances the goal"}
        }
    });

    match jev::choice(state, questions).await {
        Ok(choice) => {
            let element_id = choice.choice.clone();
            let target = snap
                .elements
                .iter()
                .find(|e| format!("@e{}", e.id) == element_id);
            match target {
                Some(el) => {
                    if el.role == "textbox" || el.role == "combobox" || el.role == "searchbox" {
                        format!(
                            "Goal: {}. Recommended action: TYPE_TEXT on @{} '{}'",
                            goal, el.id, el.name
                        )
                    } else {
                        format!(
                            "Goal: {}. Recommended action: CLICK on @{} '{}'",
                            goal, el.id, el.name
                        )
                    }
                }
                None => format!(
                    "Goal: {}. Jev selected {} but no matching element found",
                    goal, element_id
                ),
            }
        }
        Err(e) => format!("Goal: {}. Jev decision failed: {}", goal, e),
    }
}

pub(super) async fn handle_suggest(client: &CdpClient, goal: &str) -> String {
    let snapshot_val = client.evaluate(DOM_WALKER_JS).await;
    let snap = match snapshot_val {
        Ok(val) => match serde_json::from_value::<SaspSnapshot>(val) {
            Ok(s) => s,
            Err(_) => return "Failed to parse snapshot".to_string(),
        },
        Err(_) => return "Failed to get snapshot".to_string(),
    };

    let state = json!({
        "page": {
            "url": snap.url,
            "title": snap.title,
            "text": snap.to_compact_string()
        },
        "elements": snap.elements.iter().map(|e| json!({
            "id": e.id,
            "role": e.role,
            "name": e.name,
            "value": e.value,
            "x": e.x,
            "y": e.y
        })).collect::<Vec<_>>()
    });

    let _criteria: std::collections::HashMap<String, String> = snap
        .elements
        .iter()
        .map(|e| {
            (
                format!("@e{}", e.id),
                format!("{} '{}' at ({}, {})", e.role, e.name, e.x, e.y),
            )
        })
        .collect();

    let questions = json!({
        "ranking": {
            "type": "score",
            "instructions": {"goal": goal, "rules": "Score each element by how relevant it is to the goal"},
            "criteria": _criteria
        }
    });

    match jev::score(state, questions).await {
        Ok(score) => {
            format!(
                "Goal: {}. Elements ranked by Jev (score={}): {}",
                goal,
                score.score,
                snap.to_compact_string()
            )
        }
        Err(e) => format!("Goal: {}. Suggestion failed: {}", goal, e),
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
