# cdpx Usage Manual

`cdpx` is a single-binary, token-compressed Chrome DevTools Protocol controller and stdio Model Context Protocol (MCP) server designed for AI coding agents and automated developer workflows.

---

## 1. CLI Commands

### Basic Navigation (`open`)
Navigates to a URL, awaits page hydration and mutation settling, and outputs the token-compressed Semantic Action-State Projection (SASP) element map:

```bash
cdpx open https://news.ycombinator.com
```

Headed mode (opens visible browser window for debugging):
```bash
cdpx open https://news.ycombinator.com --headed
```

Example Output:
```text
[Page: Hacker News] (url: https://news.ycombinator.com/)
------------------------------------------------------------
@e1: link "Hacker News" (x: 135, y: 20)
@e2: link "new" (x: 194, y: 20)
@e3: link "past" (x: 232, y: 20)
@e4: link "comments" (x: 288, y: 20)
@e5: link "ask" (x: 343, y: 20)
@e6: link "show" (x: 382, y: 20)
@e7: link "jobs" (x: 423, y: 20)
@e8: link "submit" (x: 470, y: 20)
@e9: link "login" (x: 676, y: 20)
```

---

### Interactive Clicking (`click`)
Executes a two-phase trusted click: scrolls target into view, re-computes live geometry, and dispatches native CDP mouse events (`mouseMoved` -> `mousePressed` -> `mouseReleased`):

```bash
# Pass element handle and target URL
cdpx click --url https://news.ycombinator.com @e9

# Or using bare integer ID (recommended on Windows PowerShell to avoid @-splatting)
cdpx click --url https://news.ycombinator.com 9
```

> **PowerShell Note**: In Windows PowerShell, `@` is the array-splatting operator. When passing element references like `@e1`, quote them (`'@e1'`) or use the bare integer (`1`).

---

### Interactive Typing (`type`)
Focuses target element, selects any existing value, and dispatches trusted key events through CDP:

```bash
# Type search query into DuckDuckGo search box
cdpx type --url https://duckduckgo.com 5 "rust programming"
```

The output returns the updated, settled page state reflecting any dynamic autocomplete dropdowns or input value changes:
```text
@e5: combobox "Search with DuckDuckGo" (x: 568, y: 59) [value: "rust programming"]
@e6: button "Clear search input" (x: 815, y: 59)
```

---

## 2. Model Context Protocol (MCP) Integration

`cdpx` runs as a zero-configuration stdio MCP server for agentic IDEs and command-line tools.

### Claude Code (`claude.json` / CLI)
Run or configure in project `claude.json`:
```json
{
  "mcpServers": {
    "cdpx": {
      "command": "cdpx",
      "args": ["--mcp"]
    }
  }
}
```

### Antigravity IDE / Gemini
Add to settings or global MCP configuration:
```json
{
  "mcpServers": {
    "cdpx": {
      "command": "C:\\Users\\saves\\Desktop\\cdpx\\target\\release\\cdpx.exe",
      "args": ["--mcp"]
    }
  }
}
```

### Claude Desktop (`claude_desktop_config.json`)
```json
{
  "mcpServers": {
    "cdpx": {
      "command": "cdpx",
      "args": ["--mcp"]
    }
  }
}
```

---

## 3. MCP Tools Schema

When connected via MCP, `cdpx` exposes 3 core primitives:

### `browser_navigate`
Navigates the browser to a target URL, waits for network idle and client-side hydration, and returns the SASP element snapshot:
* **Argument**: `url` (string, required): e.g. `"https://github.com/login"`

### `browser_act`
Performs an action on a target element reference (`@e1`, `@e2`, ...):
* **`ref`** (string, required): Element handle.
* **`action`** (string, required): `"click"` or `"type"`.
* **`value`** (string, optional): Text string to type when action is `"type"`.

### `browser_snapshot`
Re-evaluates the active DOM and returns a fresh token-budgeted interactive element map without performing any action.

---

## 4. Environment Variables

| Variable | Description | Default |
|---|---|---|
| `CHROME_BIN` | Explicit path to Chrome, Chromium, or Edge binary | Auto-detected system install |
| `BROWSER_PATH` | Fallback browser executable path | None |

---

## 5. Troubleshooting & FAQ

#### Browser fails to start
Ensure Google Chrome, Chromium, or Microsoft Edge is installed in standard platform locations (`Program Files` on Windows, `/Applications` on macOS, `/usr/bin` on Linux). Or export `CHROME_BIN=/path/to/chrome`.

#### CAPTCHA or WAF Challenge on specific sites
Sites like Amazon, Cloudflare-protected portals, or Reddit may present bot verification screens to headless browsers. Pass `--headed` to run in headed mode and manually satisfy challenges when debugging locally.
