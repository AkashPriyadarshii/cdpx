# Contributing to cdpx

Thank you for helping build `cdpx`.

## Guiding Principles
1. **Ponytail Ladder**: Stop at the first rung that holds. No overengineering, no single-use abstractions, stdlib and direct platform features first.
2. **Zero-Cost First**: Everything must run on free-tier infrastructure.
3. **No AI Slop**: Commit messages and docs must be plain, terse, and devoid of marketing buzzwords.

## Build Setup
Ensure you have Rust 1.85+ installed.

```bash
# Clone
git clone https://github.com/AkashPriyadarshii/cdpx.git
cd cdpx

# Build debug
cargo build

# Run unit and integration tests
cargo test

# Build optimized binary
cargo build --release
```

## Commit Standards
Follow conventional commits (`feat:`, `fix:`, `docs:`, `chore:`, `refactor:`).
- Max 72 characters per line.
- Plain descriptions. Never use em dashes or buzzwords.
