use crate::cdp::CdpClient;
use serde_json::json;
use std::time::Duration;

#[derive(Debug)]
pub enum ActionError {
    ElementNotFound(String),
    DispatchFailed(String),
    EvaluationFailed(String),
}

impl std::fmt::Display for ActionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ActionError::ElementNotFound(r) => write!(f, "Element reference not found: {}", r),
            ActionError::DispatchFailed(e) => write!(f, "Input dispatch failed: {}", e),
            ActionError::EvaluationFailed(e) => write!(f, "Script evaluation failed: {}", e),
        }
    }
}

impl std::error::Error for ActionError {}

/// Parses an element reference string like "@e4" or "4" into a numeric ID.
pub fn parse_ref_id(ref_str: &str) -> Option<u32> {
    let clean = ref_str.trim().trim_start_matches('@').trim_start_matches('e');
    clean.parse::<u32>().ok()
}

/// Settles dynamic DOM mutations before an action is executed.
pub async fn wait_for_settle(client: &CdpClient, max_wait_ms: u64) -> Result<(), ActionError> {
    let check_script = r#"
        new Promise((resolve) => {
            let timeout;
            const observer = new MutationObserver(() => {
                clearTimeout(timeout);
                timeout = setTimeout(() => {
                    observer.disconnect();
                    resolve(true);
                }, 150);
            });
            observer.observe(document.body || document.documentElement, {
                childList: true,
                subtree: true,
                attributes: true
            });
            timeout = setTimeout(() => {
                observer.disconnect();
                resolve(true);
            }, 150);
        })
    "#;

    let _ = tokio::time::timeout(
        Duration::from_millis(max_wait_ms),
        client.evaluate(check_script),
    )
    .await;

    Ok(())
}

/// Two-phase click: in-page scroll & fresh coordinate measurement -> trusted CDP event sequence.
pub async fn click_element(client: &CdpClient, ref_str: &str) -> Result<(), ActionError> {
    let id = parse_ref_id(ref_str)
        .ok_or_else(|| ActionError::ElementNotFound(ref_str.to_string()))?;

    // Phase 1: In-page scroll into view and re-read live bounding box
    let prep_script = format!(
        r#"(() => {{
            if (!window.__CDPX_CACHE || !window.__CDPX_CACHE.has({id})) return null;
            const el = window.__CDPX_CACHE.get({id});
            el.scrollIntoView({{ block: 'center', inline: 'center', behavior: 'instant' }});
            const rect = el.getBoundingClientRect();
            return {{
                x: Math.round(rect.left + rect.width / 2),
                y: Math.round(rect.top + rect.height / 2),
                w: Math.round(rect.width),
                h: Math.round(rect.height)
            }};
        }})()"#
    );

    let coords_val = client
        .evaluate(&prep_script)
        .await
        .map_err(ActionError::EvaluationFailed)?;

    if coords_val.is_null() {
        return Err(ActionError::ElementNotFound(ref_str.to_string()));
    }

    let cx = coords_val.get("x").and_then(|v| v.as_i64()).unwrap_or(0);
    let cy = coords_val.get("y").and_then(|v| v.as_i64()).unwrap_or(0);

    // Phase 2: Trusted CDP Mouse Event Sequence
    client
        .call(
            "Input.dispatchMouseEvent",
            Some(json!({
                "type": "mouseMoved",
                "x": cx,
                "y": cy
            })),
        )
        .await
        .map_err(ActionError::DispatchFailed)?;

    tokio::time::sleep(Duration::from_millis(15)).await;

    client
        .call(
            "Input.dispatchMouseEvent",
            Some(json!({
                "type": "mousePressed",
                "button": "left",
                "clickCount": 1,
                "x": cx,
                "y": cy
            })),
        )
        .await
        .map_err(ActionError::DispatchFailed)?;

    tokio::time::sleep(Duration::from_millis(30)).await;

    client
        .call(
            "Input.dispatchMouseEvent",
            Some(json!({
                "type": "mouseReleased",
                "button": "left",
                "clickCount": 1,
                "x": cx,
                "y": cy
            })),
        )
        .await
        .map_err(ActionError::DispatchFailed)?;

    // Phase 3: Wait for post-action settlement
    let _ = wait_for_settle(client, 400).await;

    Ok(())
}

/// Types text into an element reference by focusing and emitting raw CDP key events.
pub async fn type_element(client: &CdpClient, ref_str: &str, text: &str) -> Result<(), ActionError> {
    let id = parse_ref_id(ref_str)
        .ok_or_else(|| ActionError::ElementNotFound(ref_str.to_string()))?;

    // Focus element
    let focus_script = format!(
        r#"(() => {{
            if (!window.__CDPX_CACHE || !window.__CDPX_CACHE.has({id})) return false;
            const el = window.__CDPX_CACHE.get({id});
            el.focus();
            if (el.value !== undefined) el.value = '';
            return true;
        }})()"#
    );

    let focused = client
        .evaluate(&focus_script)
        .await
        .map_err(ActionError::EvaluationFailed)?;

    if !focused.as_bool().unwrap_or(false) {
        return Err(ActionError::ElementNotFound(ref_str.to_string()));
    }

    // Insert text directly via Input.insertText for speed and zero IME drift
    client
        .call("Input.insertText", Some(json!({ "text": text })))
        .await
        .map_err(ActionError::DispatchFailed)?;

    // Trigger synthetic input and change events in page
    let trigger_script = format!(
        r#"(() => {{
            const el = window.__CDPX_CACHE.get({id});
            if (el) {{
                el.dispatchEvent(new Event('input', {{ bubbles: true }}));
                el.dispatchEvent(new Event('change', {{ bubbles: true }}));
            }}
        }})()"#
    );
    let _ = client.evaluate(&trigger_script).await;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_ref_id() {
        assert_eq!(parse_ref_id("@e1"), Some(1));
        assert_eq!(parse_ref_id("@e42"), Some(42));
        assert_eq!(parse_ref_id("e9"), Some(9));
        assert_eq!(parse_ref_id("7"), Some(7));
        assert_eq!(parse_ref_id("invalid"), None);
    }
}
