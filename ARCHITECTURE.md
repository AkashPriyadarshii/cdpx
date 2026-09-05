# cdpx Architecture

This document details the internal design and protocol flow of `cdpx`.

## 1. System Overview

```
+-------------------------------------------------------------+
|                     User / AI Agent                         |
+------------------------------+------------------------------+
                               |
               +---------------+---------------+
               |                               |
       [ CLI Subcommands ]             [ stdio MCP Server ]
       `cdpx open / click`             JSON-RPC 2.0 (`--mcp`)
               |                               |
               +---------------+---------------+
                               |
                   +-----------v-----------+
                   |     Core Controller   |
                   +-----------+-----------+
                               |
         +---------------------+---------------------+
         |                                           |
+--------v--------+                         +--------v--------+
|  Stealth Launch |                         |  CDP Session    |
|  - Headless/Dir |                         |  - Tokio WS     |
|  - Args Hygiene |                         |  - Raw JSON-RPC |
+-----------------+                         +--------+--------+
                                                     |
                                            +--------v--------+
                                            |  SASP Engine    |
                                            | - Injected JS   |
                                            | - Ref IDs (@eN) |
                                            +-----------------+
```

## 2. Browser Discovery and Stealth Launch

`cdpx` avoids bundling heavy Chromium binaries. It discovers existing local installations:
- **Windows**: Checks `C:\Program Files\Google\Chrome\Application\chrome.exe`, `Edge\Application\msedge.exe`, and `%LOCALAPPDATA%`.
- **Linux**: Scans `PATH` for `google-chrome`, `chromium`, `chromium-browser`.
- **macOS**: Scans `/Applications/Google Chrome.app/Contents/MacOS/Google Chrome`.

### Stealth Launch Parameters
```bash
chromium \
  --remote-debugging-port=0 \
  --disable-blink-features=AutomationControlled \
  --exclude-switches=enable-automation \
  --disable-infobars \
  --no-first-run \
  --no-default-browser-check \
  --user-data-dir=<TEMP_EPHEMERAL_DIR>
```

Upon spawn, `cdpx` reads the ephemeral port from `DevToolsActivePort` or standard error, then queries `http://127.0.0.1:<PORT>/json/version` for `webSocketDebuggerUrl`.

## 3. CDP Transport Layer (`src/cdp.rs`)

Built directly on `tokio-tungstenite` with zero intermediate driver layers:
- Single WebSocket connection handling JSON-RPC request-response matching via monotonic request IDs.
- Subscribes to `Target.setAutoAttach` with `flatten: true` to seamlessly receive frames and Out-of-Process Iframes (OOPIF).
- Zero `Runtime.enable` on default navigation paths to prevent CDP fingerprint leaks.

## 4. Semantic Action-State Projection (SASP)

Rather than sending 20,000 raw DOM nodes or 5,000 unpruned Blink AXTree nodes to the LLM, `cdpx` injects an embedded in-page tree walker:

1. **Interactivity Heuristic**:
   - Matches: `a[href]`, `button`, `input`, `select`, `textarea`, `summary`.
   - Functional ARIA roles: `[role="button"]`, `[role="link"]`, `[role="checkbox"]`, `[role="menuitem"]`.
   - Dynamic indicators: `cursor: pointer` + `pointer-events !== 'none'`.
2. **Viewport Culling**:
   - Computes `getBoundingClientRect()`.
   - Drops elements with width/height <= 0 or lying entirely outside `(0, 0, window.innerWidth, window.innerHeight)`.
3. **Occlusion Hit-Testing**:
   - Runs `document.elementFromPoint(centerX, centerY)`.
   - If the element is covered by a backdrop or modal, it is culled.
4. **Handle Assignment**:
   - Stores raw DOM nodes in `window.__CDPX_CACHE = new Map()`.
   - Returns compact JSON: `{ id: 1, role: "button", name: "Sign In", x: 400, y: 250 }`.

## 5. Two-Phase Action Engine (`src/action.rs`)

To resolve hydration dead clicks on Next.js/Remix apps:

- **Phase 1: In-Page Settle & Measurement**:
  - Validates that DOM mutations have stabilized for >= 150ms.
  - Calls `target.scrollIntoView({ block: 'center', inline: 'center', behavior: 'instant' })`.
  - Re-reads `getBoundingClientRect()` to account for any layout shift.
- **Phase 2: Trusted CDP Pointer Sequence**:
  - Emits `Input.dispatchMouseEvent` with `type: mouseMoved`.
  - Emits `Input.dispatchMouseEvent` with `type: mousePressed, button: left, clickCount: 1`.
  - Emits `Input.dispatchMouseEvent` with `type: mouseReleased, button: left, clickCount: 1`.
- **Phase 3: Post-Action Verification**:
  - Awaits URL change, navigation, or DOM mutation.
