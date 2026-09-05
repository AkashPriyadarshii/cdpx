# cdpx v0.1 Specification & PRD

## 1. Problem Statement
Existing browser automation tools designed for human regression testing (Playwright, Puppeteer) suffer from major impedance mismatches when used by autonomous AI agents:
1. **Memory bloat**: 150MB - 350MB client driver RAM per session.
2. **Context window explosion**: 15,000 - 80,000 tokens for raw HTML or unpruned AXTree dumps.
3. **Flaky SPA hydration**: Synthetic clicks fired into unhydrated React/Vue root containers result in silent failures.
4. **Driver fingerprinting**: Easily detected by Cloudflare Turnstile and DataDome.

## 2. Target Audience
- Autonomous coding agents (Antigravity, Claude Code, Codex, OpenClaw).
- Systems engineers running batch web extraction on zero-cost free-tier VPS / GitHub Actions runners.

## 3. Functional Requirements

### CLI Commands
- `cdpx open <URL>`: Launches/connects, navigates, settles, outputs SASP element map.
- `cdpx click <ref>`: Executes two-phase trusted click on element reference.
- `cdpx type <ref> <TEXT>`: Focuses and types text with synthetic key event cascades.
- `cdpx fill --data <JSON> [--submit <ref>]`: Atomically populates multiple inputs and submits.
- `cdpx dump [--format markdown|json]`: Serializes current page content in token-compressed format.
- `cdpx --mcp`: Starts stdio JSON-RPC 2.0 MCP server.

### MCP Tools Schema
```json
[
  {
    "name": "browser_navigate",
    "description": "Navigate to a URL and return a token-compressed interactive element map.",
    "parameters": {
      "type": "object",
      "properties": { "url": { "type": "string" } },
      "required": ["url"]
    }
  },
  {
    "name": "browser_act",
    "description": "Execute a trusted action on an element reference (@e1, @e2).",
    "parameters": {
      "type": "object",
      "properties": {
        "ref": { "type": "string" },
        "action": { "type": "string", "enum": ["click", "type", "hover", "press"] },
        "value": { "type": "string" }
      },
      "required": ["ref", "action"]
    }
  },
  {
    "name": "browser_fill_form",
    "description": "Fill multiple form fields in a single atomic transaction.",
    "parameters": {
      "type": "object",
      "properties": {
        "fields": { "type": "object", "additionalProperties": { "type": "string" } },
        "submit_ref": { "type": "string" }
      },
      "required": ["fields"]
    }
  },
  {
    "name": "browser_extract",
    "description": "Extract token-compressed Markdown or structured content.",
    "parameters": {
      "type": "object",
      "properties": {
        "format": { "type": "string", "enum": ["markdown", "text", "raw"] }
      }
    }
  }
]
```

## 4. Non-Functional Requirements
- **Binary Footprint**: < 8MB stripped release binary.
- **Client RAM**: < 25MB RSS (excluding Chromium process).
- **Startup Time**: < 100ms from CLI execution to WebSocket handshake.
- **Token Budget**: Standard landing page output <= 800 tokens.
