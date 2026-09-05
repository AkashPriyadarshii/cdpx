use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::time::Duration;
use tempfile::TempDir;
use which::which;

#[derive(Debug)]
pub enum BrowserError {
    NotFound,
    SpawnFailed(String),
    PortDiscoveryFailed(String),
    ConnectionFailed(String),
}

impl std::fmt::Display for BrowserError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BrowserError::NotFound => {
                write!(f, "Chromium or Chrome executable not found on system")
            }
            BrowserError::SpawnFailed(e) => write!(f, "Failed to spawn browser process: {}", e),
            BrowserError::PortDiscoveryFailed(e) => {
                write!(f, "Failed to discover remote debugging port: {}", e)
            }
            BrowserError::ConnectionFailed(e) => {
                write!(f, "Failed to connect to browser CDP endpoint: {}", e)
            }
        }
    }
}

impl std::error::Error for BrowserError {}

#[allow(dead_code)]
pub struct BrowserInstance {
    pub child: Child,
    pub user_data_dir: TempDir,
    pub ws_url: String,
    pub port: u16,
}

impl Drop for BrowserInstance {
    fn drop(&mut self) {
        #[cfg(target_os = "windows")]
        {
            let pid = self.child.id();
            let _ = Command::new("taskkill")
                .args(["/F", "/T", "/PID", &pid.to_string()])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status();
        }
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// Discovers an installed Chrome, Chromium, or Edge binary on the system.
pub fn find_chrome_binary() -> Result<PathBuf, BrowserError> {
    if let Ok(env_path) = std::env::var("CHROME_BIN").or_else(|_| std::env::var("BROWSER_PATH")) {
        let p = PathBuf::from(env_path);
        if p.exists() {
            return Ok(p);
        }
    }

    #[cfg(target_os = "windows")]
    {
        let candidates = [
            r"C:\Program Files\Google\Chrome\Application\chrome.exe",
            r"C:\Program Files (x86)\Google\Chrome\Application\chrome.exe",
            r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe",
            r"C:\Program Files\Microsoft\Edge\Application\msedge.exe",
        ];

        for c in candidates {
            let p = PathBuf::from(c);
            if p.exists() {
                return Ok(p);
            }
        }

        if let Ok(local_app_data) = std::env::var("LOCALAPPDATA") {
            let chrome_user =
                PathBuf::from(&local_app_data).join(r"Google\Chrome\Application\chrome.exe");
            if chrome_user.exists() {
                return Ok(chrome_user);
            }
            let edge_user =
                PathBuf::from(&local_app_data).join(r"Microsoft\Edge\Application\msedge.exe");
            if edge_user.exists() {
                return Ok(edge_user);
            }
        }
    }

    #[cfg(target_os = "macos")]
    {
        let candidates = [
            "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
            "/Applications/Chromium.app/Contents/MacOS/Chromium",
            "/Applications/Microsoft Edge.app/Contents/MacOS/Microsoft Edge",
        ];
        for c in candidates {
            let p = PathBuf::from(c);
            if p.exists() {
                return Ok(p);
            }
        }
    }

    #[cfg(target_os = "linux")]
    {
        let names = [
            "google-chrome",
            "google-chrome-stable",
            "chromium",
            "chromium-browser",
            "microsoft-edge",
        ];
        for name in names {
            if let Ok(path) = which(name) {
                return Ok(path);
            }
        }
    }

    for name in &["chrome", "google-chrome", "chromium", "msedge"] {
        if let Ok(path) = which(name) {
            return Ok(path);
        }
    }

    Err(BrowserError::NotFound)
}

/// Launches Chromium with clean anti-detection flags and returns a connected instance.
pub async fn launch_browser(headless: bool) -> Result<BrowserInstance, BrowserError> {
    let chrome_path = find_chrome_binary()?;
    let user_data_dir = tempfile::Builder::new()
        .prefix("cdpx_profile_")
        .tempdir()
        .map_err(|e| BrowserError::SpawnFailed(format!("Failed to create temp profile: {}", e)))?;

    let mut cmd = Command::new(&chrome_path);
    cmd.arg("--remote-debugging-port=0")
        .arg("--remote-debugging-address=127.0.0.1")
        .arg("--remote-allow-origins=*")
        .arg("--window-size=1280,800")
        .arg(format!("--user-data-dir={}", user_data_dir.path().display()))
        .arg("--disable-blink-features=AutomationControlled")
        .arg("--exclude-switches=enable-automation")
        .arg("--disable-infobars")
        .arg("--disable-background-networking")
        .arg("--disable-default-apps")
        .arg("--disable-sync")
        .arg("--no-first-run")
        .arg("--no-default-browser-check")
        .arg("--hide-scrollbars")
        .arg("--disable-features=Translate,OptimizationHints,MediaRouter")
        .arg("--user-agent=Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/133.0.0.0 Safari/537.36")
        .arg("about:blank")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    if headless {
        cmd.arg("--headless=new");
    }

    let child = cmd
        .spawn()
        .map_err(|e| BrowserError::SpawnFailed(e.to_string()))?;

    let port_file = user_data_dir.path().join("DevToolsActivePort");
    let mut port: Option<u16> = None;

    for _ in 0..60 {
        if port_file.exists()
            && let Ok(content) = std::fs::read_to_string(&port_file)
            && let Some(first_line) = content.lines().next()
            && let Ok(p) = first_line.trim().parse::<u16>()
        {
            port = Some(p);
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    let port = port.ok_or_else(|| {
        BrowserError::PortDiscoveryFailed("Timed out waiting for DevToolsActivePort".to_string())
    })?;

    let version_url = format!("http://127.0.0.1:{}/json/version", port);
    let mut ws_url: Option<String> = None;

    for _ in 0..30 {
        if let Ok(resp) = reqwest::get(&version_url).await
            && let Ok(json) = resp.json::<serde_json::Value>().await
            && let Some(url_str) = json.get("webSocketDebuggerUrl").and_then(|v| v.as_str())
        {
            ws_url = Some(url_str.to_string());
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    let ws_url = ws_url.ok_or_else(|| {
        BrowserError::ConnectionFailed(format!(
            "Failed to retrieve webSocketDebuggerUrl from {}",
            version_url
        ))
    })?;

    Ok(BrowserInstance {
        child,
        user_data_dir,
        ws_url,
        port,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_chrome_binary() {
        let binary = find_chrome_binary();
        assert!(
            binary.is_ok(),
            "Should discover Chrome or Edge on host: {:?}",
            binary.err()
        );
        let path = binary.unwrap();
        assert!(
            path.exists(),
            "Discovered binary path should exist: {:?}",
            path
        );
    }
}
