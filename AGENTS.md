# AGENTS.md - cdpx Developer & Agent Guidelines

## Repository Overview
`cdpx` is a single-binary, token-compressed Chrome DevTools Protocol (CDP) browser controller and stdio MCP server written in Rust.
Target runtime: 64-bit AMD64 Windows / Linux / macOS.
Budget: Rs 0 infrastructure, zero external Node.js/Python driver processes.

## Precedence & Persona
1. **Global Rules**: Internalize `C:\Users\saves\.gemini\GEMINI.md` (Akash Priyadarshi persona, Rs 0 budget, ADHD concise rules, security standards).
2. **Project Rules**: This file governs all work in the `cdpx` repository.

## Architecture at a Glance
- **Language**: Rust (2024 edition).
- **Core Async Runtime**: `tokio` (configured with minimal worker threads to enforce <25MB RSS).
- **CDP Transport**: Direct WebSocket connection over `tokio-tungstenite` to Chromium's `--remote-debugging-port`.
- **Page State Engine**: Embedded `dom_walker.min.js` injected into the page to produce Semantic Action-State Projections (SASP) under 800 tokens.
- **Action Engine**: Two-phase execution (in-page scroll + center rect measurement, followed by trusted CDP `Input.dispatchMouseEvent` sequence).
- **Agent Interface**: Dual-mode binary: CLI subcommands (`open`, `act`, `fill`, `dump`) + stdio JSON-RPC 2.0 MCP server (`--mcp`).

## Key Files & Layout
| Path | Purpose |
| :--- | :--- |
| `src/main.rs` | CLI entrypoint, argument routing via `clap`, and `--mcp` flag trigger. |
| `src/browser.rs` | Chromium/Edge discovery on Windows/Linux/macOS, stealth process spawn. |
| `src/cdp.rs` | Raw Tokio WebSocket client for CDP JSON-RPC, `Target.setAutoAttach` listener. |
| `src/dom_walker.min.js` | Embedded JS tree-walker with viewport culling and occlusion hit-testing. |
| `src/sasp.rs` | SASP serialization and monotonic `@e1..@eN` handle management. |
| `src/action.rs` | Two-phase click/type engine with DOM mutation quiet period settlement. |
| `src/mcp.rs` | Lightweight stdio JSON-RPC 2.0 MCP server loop. |
| `Cargo.toml` | Release binary tuning (`opt-level = "z"`, `lto = true`, `panic = "abort"`). |

## Build & Test Verification Commands
```bash
# Debug build
cargo build

# Run all unit and integration tests
cargo test

# Release build (stripped binary)
cargo build --release

# Lints and formatting
cargo fmt --check
cargo clippy -- -D warnings
```

## Mandatory Coding Standards
- **No AI Slop in Code or Docs**: Never use buzzwords like "comprehensive", "streamline", "robust", "leverage", "delve", or "unleash".
- **No Em Dashes**: Never use em dashes in commit messages or code docs; use standard hyphens (`-`) or colons.
- **Immutability First**: Create new objects and return fresh structs rather than mutating shared state.
- **TDD Requirement**: Write unit tests before implementing new modules; maintain 80%+ test coverage.
- **Resource Constraints**: Driver idle RSS must remain strictly under 25MB. Never spawn a secondary Node.js or Python subprocess.
