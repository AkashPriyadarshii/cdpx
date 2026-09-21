# cdpx - Driverless, Token-Compressed Browser Engine for AI Agents

[![crates.io](https://img.shields.io/crates/v/cdpx?style=flat-square)](https://crates.io/crates/cdpx) [![downloads](https://img.shields.io/crates/d/cdpx?style=flat-square)](https://crates.io/crates/cdpx) [![release](https://img.shields.io/github/v/release/AkashPriyadarshii/cdpx?style=flat-square&label=release)](https://github.com/AkashPriyadarshii/cdpx/releases)

[![Crates.io](https://img.shields.io/crates/v/cdpx?style=flat-square&color=orange)](https://crates.io/crates/cdpx)
[![License](https://img.shields.io/github/license/AkashPriyadarshii/cdpx?style=flat-square)](LICENSE)

`cdpx` is a driverless Chrome DevTools Protocol (CDP) browser controller and stdio MCP server compiled into a single static Rust binary. It connects directly to Chromium via WebSockets, compresses dynamic web state by 90%+ into interactive element handles (`@e1`, `@e2`), and executes trusted two-phase actions with zero Node.js or Python runtime dependencies.
.
```bash
# Launch and get a clean, token-compressed element map (<800 tokens)
cdpx open "https://github.com/login"

# Act deterministically with trusted CDP inputs
cdpx type @e2 "octocat"
cdpx type @e3 "secretpassword"
cdpx click @e4

# Run as an MCP server for Claude Code, Antigravity, or Codex
cdpx --mcp
```

## Why cdpx?

* **No Node.js Driver Tax**: Official Playwright in Python or Go spawns a 150MB+ Node.js subprocess. `cdpx` is written in pure Rust on Tokio, idling under 25MB RSS.
* **Semantic Action-State Projection (SASP)**: Dumps div-soup and offscreen nodes. Emits only visible, interactive elements with concise `@eN` references, keeping per-turn context under 800 tokens.
* **Hydration-Safe Actions**: Eliminates Next.js/Remix dead clicks by waiting for DOM mutation and React fiber quiet periods before firing trusted CDP inputs.
* **Driverless Stealth**: No `__playwright__binding__` globals, no driver bootstrap, and no default `Runtime.enable` leaks that trigger Cloudflare Turnstile or DataDome.

## Install

```bash
# Cargo (crates.io)
cargo install cdpx
```

## Quick Start

```bash
# 1. Open any page and inspect interactive elements
cdpx open "https://news.ycombinator.com"

# Output:
# [Page: Hacker News] (url: https://news.ycombinator.com)
# ------------------------------------------------------------
# @e1: link "Hacker News"
# @e2: link "new"
# @e3: link "past"
# @e4: link "comments"
# @e5: link "ask"
# @e6: link "show"
# @e7: link "jobs"
# @e8: link "submit"
# @e9: link "login"

# 2. Click any reference
cdpx click @e9

# 3. Run as an MCP server for Claude Code, Antigravity, or Codex
cdpx --mcp
```

## Model Context Protocol (MCP) Setup

`cdpx` includes a built-in stdio JSON-RPC 2.0 MCP server. Add it to your agent config:

### Claude Code / Antigravity / Gemini
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

### Tools Exposed to Agents
* `browser_navigate(url)` - Navigates, settles hydration, returns compressed SASP map.
* `browser_act(ref, action, value?)` - Performs `click` / `type` on target `@eN`.
* `browser_snapshot()` - Returns the token-compressed SASP element map of the current page.

## Development

```bash
# Build
cargo build --release

# Run tests
cargo test

# Check formatting & lints
cargo fmt --check
cargo clippy -- -D warnings
```

## Ecosystem

Built by Akash Priyadarshi in Patna, Bihar, India as part of the Rs 0 high-performance systems ecosystem:
* [rustygrep](https://github.com/AkashPriyadarshii/rustygrep) - Token-efficient grep for LLM coding agents.
* [zcat](https://github.com/AkashPriyadarshii/zcat) - 179KB Zig cat replacement with JSON output for AI agents.
* [repomap](https://github.com/AkashPriyadarshii/repomap) - Token-budgeted repository maps in Go.

## License

[MIT](LICENSE) (c) 2026 Akash Priyadarshi
