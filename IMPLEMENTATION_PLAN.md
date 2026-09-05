# Implementation Plan: `cdpx` (v0.1)

`cdpx` is a single-binary, token-compressed CDP browser controller and stdio MCP server written in Rust. It eliminates Node.js runtime overhead, bypasses common Playwright driver automation detection vectors, and reduces page state tokens by 90%+ via Semantic Action-State Projection (SASP).

## User Review Required

> [!IMPORTANT]
> - Target Directory: `C:\Users\saves\Desktop\cdpx`
> - Tech Stack: Rust (2024 edition), `tokio`, `tokio-tungstenite`, `serde_json`, `clap`.
> - License: MIT (Copyright 2026 Akash Priyadarshi).
> - Target Chromium: Auto-detected local Chrome / Edge, or ungoogled-chromium via `--remote-debugging-port`.

## Documentation Suite (To be created in repo root)

- [x] [`README.md`](file:///C:/Users/saves/Desktop/cdpx/README.md) - House standard README with quickstart, CLI usage, and MCP setup.
- [x] [`ARCHITECTURE.md`](file:///C:/Users/saves/Desktop/cdpx/ARCHITECTURE.md) - Protocol flow, SASP tree-walker design, and two-phase action engine.
- [x] [`SPEC.md`](file:///C:/Users/saves/Desktop/cdpx/SPEC.md) - Formal v0.1 PRD, scope boundary, and JSON-RPC MCP schema.
- [x] [`BENCHMARK.md`](file:///C:/Users/saves/Desktop/cdpx/BENCHMARK.md) - Memory footprint, latency, and token-cost comparisons against Playwright.
- [x] [`CONTRIBUTING.md`](file:///C:/Users/saves/Desktop/cdpx/CONTRIBUTING.md) - Build, test, and code style guidelines.
- [x] [`LICENSE`](file:///C:/Users/saves/Desktop/cdpx/LICENSE) - MIT License.

## Proposed Code Structure (v0.1 Implementation)

### [Core Rust Binary]

#### [NEW] [`Cargo.toml`](file:///C:/Users/saves/Desktop/cdpx/Cargo.toml)
Defines binary metadata, dependencies (`tokio`, `tokio-tungstenite`, `clap`, `serde`, `serde_json`, `which`), and release profile optimization (`opt-level = "z"`, `lto = true`, `panic = "abort"`).

#### [NEW] [`src/main.rs`](file:///C:/Users/saves/Desktop/cdpx/src/main.rs)
CLI entrypoint and mode router (`open`, `act`, `fill`, `dump`, `--mcp`).

#### [NEW] [`src/browser.rs`](file:///C:/Users/saves/Desktop/cdpx/src/browser.rs)
Finds installed Chromium/Edge on Windows, Linux, macOS. Launches with stealth flags (`--disable-blink-features=AutomationControlled`, clean User-Agent) and connects to WebSocket debug URL.

#### [NEW] [`src/cdp.rs`](file:///C:/Users/saves/Desktop/cdpx/src/cdp.rs)
Lean, typed CDP WebSocket transport over `tokio-tungstenite`. Implements `Page`, `Target` (with `autoAttach: true, flatten: true`), `Input`, and lazy `Runtime.evaluate`.

#### [NEW] [`src/dom_walker.min.js`](file:///C:/Users/saves/Desktop/cdpx/src/dom_walker.min.js)
Embedded JavaScript tree walker. Filters out div-soup, performs viewport culling and occlusion checks (`elementFromPoint`), caches element handles in `window.__CDPX_CACHE`, and outputs SASP handles (`@e1..@eN`).

#### [NEW] [`src/action.rs`](file:///C:/Users/saves/Desktop/cdpx/src/action.rs)
Two-phase action engine:
1. `scrollIntoView` + fresh coordinate calculation inside page context.
2. Trusted CDP `Input.dispatchMouseEvent` sequence (`mouseMoved` -> `mousePressed` -> `mouseReleased`).
3. Mutation settle check to prevent dead clicks on hydrating SPAs.

#### [NEW] [`src/mcp.rs`](file:///C:/Users/saves/Desktop/cdpx/src/mcp.rs)
Lightweight stdio JSON-RPC 2.0 server exposing `browser_navigate`, `browser_act`, `browser_fill_form`, and `browser_extract`.

## Verification Plan

### Automated Tests
- Unit tests for CLI parsing and SASP serialization: `cargo test`
- Integration test against a local headless Chromium instance navigating to a static fixture and testing click dispatch.

### Manual Verification
1. Run `cdpx open https://news.ycombinator.com` and verify token-compact element map (`@e1`, `@e2`).
2. Run `cdpx click @e1` and verify navigation side-effect occurs.
3. Verify MCP stdio handshake via JSON-RPC test script.

* [ ] **Milestone 5: SEO, Marketing Artifacts & Landing Page**
  * **Repository Metadata**: Curated GitHub topics (`cdp`, `browser-automation`, `mcp-server`, `rust`, `playwright-alternative`, `ai-agent`, `token-compression`, `stealth-browser`), one-line tagline, and social preview badges.
  * **README & Docs SEO Optimization**: Rich search-intent sections, LLM benchmark comparisons, and copy-paste install commands.
  * **Marketing Landing Page (`site/index.html`)**: Bespoke editorial brutalist landing page with live interactive token-comparison widget, zero external runtime dependencies, full metadata OpenGraph tags, and official ecosystem footer.
