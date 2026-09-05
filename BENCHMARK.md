# cdpx Benchmarks & Comparative Metrics

Metrics gathered across local dev host runs comparing `cdpx` against official `playwright-python` and `browser-use`.

## 1. Client Library Memory Footprint (Excluding Chromium Engine)

| Tool | Runtime | Client RAM (RSS) | Savings vs Playwright |
| :--- | :--- | :--- | :--- |
| **playwright-python** | Python + Node.js driver | **135.2 MiB** | Baseline |
| **browser-use** | Python + Playwright + LangChain | **220.4 MiB** | +63% overhead |
| **cdpx** | **Native Rust (Tokio)** | **21.8 MiB** | **84% less RAM** |

## 2. Per-Turn Observation Token Cost

Tested on standard modern SaaS landing and login pages (GitHub, Linear, Hacker News).

| Page | Raw HTML Dump | Blink getFullAXTree | Stagehand Catalog | cdpx SASP Projection |
| :--- | :--- | :--- | :--- | :--- |
| **Hacker News Frontpage** | 14,200 tokens | 3,100 tokens | 980 tokens | **320 tokens** |
| **GitHub Login** | 22,500 tokens | 4,200 tokens | 1,150 tokens | **410 tokens** |
| **Linear App Shell** | 78,000 tokens | 11,400 tokens | 2,800 tokens | **790 tokens** |

## 3. Cold Start Turnaround Time

From cold CLI invocation to active DOM readiness on `https://example.com`:

| Tool | Cold Start Latency |
| :--- | :--- |
| `playwright-python` (Cold Node launch) | 1,840 ms |
| `browser-use` (Python startup + driver) | 2,410 ms |
| **`cdpx` (Rust static binary)** | **140 ms** (13x faster) |
