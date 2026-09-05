# Security Policy

## Reporting Security Issues
If you discover a security vulnerability in `cdpx`, please contact Akash Priyadarshi or open a confidential security advisory on GitHub.

## Threat Model & Isolation
- `cdpx` communicates strictly over local loopback (`127.0.0.1`) WebSockets with Chromium.
- Ephemeral user data directories are cleaned up upon process termination.
- No user tokens, credentials, or session cookies are stored or transmitted externally.
