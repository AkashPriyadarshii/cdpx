# Contributing to cdpx

Thank you for your interest in contributing to `cdpx`.

---

## Guiding Engineering Principles

1. **Ponytail Ladder**: Stop at the first rung that holds. Never overengineer. Check if stdlib or direct native platform features solve the problem before pulling in external dependencies.
2. **Zero-Cost First**: All development, testing, and CI must operate on free-tier infrastructure.
3. **No AI Slop**: Write like a practical senior systems engineer. Zero marketing fluff. No em dashes in documentation or commit messages.
4. **Token Efficiency**: Every feature must respect LLM context budgets. If an output can be conveyed in 50 tokens instead of 500, compress it.

---

## Development Setup

Requirements:
* Rust 1.85+ (2024 edition)
* System Google Chrome, Chromium, or Microsoft Edge installed

```bash
# Clone the repository
git clone https://github.com/AkashPriyadarshii/cdpx.git
cd cdpx

# Build debug binary
cargo build

# Run unit tests
cargo test

# Check formatting and clippy lints
cargo fmt -- --check
cargo clippy -- -D warnings

# Build optimized release binary (<1.2MB)
cargo build --release
```

---

## Pull Request & Commit Standards

- **Conventional Commits**: Use lowercase prefixes (`feat:`, `fix:`, `docs:`, `chore:`, `refactor:`, `perf:`, `style:`).
- **Direct & Terse**: Subject lines must be 50-72 characters max, describing the concrete change plainly.
- **Pre-Push Checks**: All PRs must pass `cargo test`, `cargo fmt -- --check`, and `cargo clippy -- -D warnings`.
