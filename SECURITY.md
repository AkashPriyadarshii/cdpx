# Security Policy

`cdpx` takes security, isolation, and user privacy seriously. As a tool designed for AI agent execution, maintaining strict operational boundaries is paramount.

---

## Supported Versions

| Version | Supported | Notes |
|---|:---:|---|
| `0.1.x` | Yes | Active maintenance |
| `< 0.1.0` | No | Pre-release prototypes |

---

## Threat Model & Security Boundaries

1. **Loopback-Only Binding**:
   - `cdpx` only connects to Chromium remote debugging interfaces explicitly bound to `127.0.0.1`.
   - External network requests for debugging protocols are blocked by default.

2. **Ephemeral Profile Isolation**:
   - Every ad-hoc session uses an isolated, temporary user profile directory created via `tempfile::Builder`.
   - Profiles are removed upon process termination, preventing persistent session leakage or cookie persistence unless explicitly requested.

3. **DOM Clobbering Defense**:
   - Internal element registries are attached to non-enumerable, un-clobberable properties (`window.__CDPX_REGISTRY__`), preventing malicious scripts from spoofing element handles.

4. **Process Tree Lifecycle**:
   - On process termination, `cdpx` cleans up the full browser process tree (including GPU and utility helper processes) to prevent orphan headless background listeners.

5. **Input Validation**:
   - Element reference handles (`@e1`, `@e2`) are strictly validated against numeric indices before execution.

---

## Reporting a Vulnerability

If you discover a potential security vulnerability in `cdpx`, please report it responsibly:

* **GitHub Security Advisory**: Open a private advisory on the [GitHub repository](https://github.com/AkashPriyadarshii/cdpx/security/advisories).
* **Direct Contact**: Contact Akash Priyadarshi via personal contact channels listed on [akashpriyadarshi.vercel.app](https://akashpriyadarshi.vercel.app).

Please do not disclose security issues in public issue trackers until a fix has been published.
