# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

> **Shared agent instructions live in [`AGENTS.md`](./AGENTS.md).** Read it for project conventions, risk tiers, workflow rules, and anti-patterns. This file adds Claude Code-specific context.

## Project Overview

ZeroClaw is a Rust-first autonomous AI assistant runtime. This is a fork ([Wty2003328/zeroclaw](https://github.com/Wty2003328/zeroclaw_forked)) adding WeCom channel support, channel-based tool approval, multi-step task planner, auto memory recall, and progressive tool loading. See `FORK.md` for details on fork-specific features.

## Commands

```bash
# Format, lint, test (the CI gate)
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test --locked

# Full CI validation
./dev/ci.sh all

# Build
cargo build --release --locked

# Run in development
cargo run -- <COMMAND>

# Just command runner (alternative)
just ci          # fmt-check + lint + test
just test-lib    # unit tests only (faster)
just build       # release build
just dev <ARGS>  # cargo run wrapper
```

TOML formatting: `taplo format --check` (via `just fmt-toml-check`).

## Architecture

Trait-driven modular design. Extend by implementing a trait and registering in the corresponding factory module.

**Core extension traits** (see AGENTS.md for full list):
- `src/providers/traits.rs` — LLM providers (17+: OpenAI, Anthropic, Ollama, etc.)
- `src/channels/traits.rs` — Messaging channels (40+: Telegram, Discord, Slack, WeCom, etc.)
- `src/tools/traits.rs` — Action tools (100+: shell, file ops, browser, memory, etc.)
- `src/memory/traits.rs` — Memory backends (SQLite, PostgreSQL, Markdown, Lucid)
- `src/peripherals/traits.rs` — Hardware boards (STM32, RPi GPIO)

**Key modules** (`src/`):
- `main.rs` / `lib.rs` — CLI entrypoint, command routing (`Commands` enum: `agent`, `daemon`, `gateway`, `channel`, `doctor`, `cron`, `hardware`, `skill`, etc.)
- `agent/` — Orchestration loop (`loop_.rs`), system prompt construction (`prompt.rs`), extended thinking (`thinking.rs`), context compression, loop detection
- `config/` — TOML schema (`schema.rs`), workspace management
- `gateway/` — Axum HTTP/WebSocket server, webhook receiver, embedded web dashboard
- `security/` — Access control policies, DM pairing, ChaCha20-Poly1305 secret store
- `memory/` — Multi-backend memory with embeddings, vector search, knowledge graphs, consolidation, decay
- `tools/` — Tool execution surface. Progressive loading: core tools loaded eagerly, others deferred via `DeferredToolRegistry`
- `approval/` — Channel-based tool approval workflow (fork feature)
- `sop/` — Standard Operating Procedures engine

**Frontend**: `web/` — TypeScript/Vite dashboard, built during `cargo build` via `build.rs`.

**Tests**: `tests/` — Four suites: `component/`, `integration/`, `system/`, `live/` (live requires credentials). Entrypoints: `test_component.rs`, `test_integration.rs`, `test_system.rs`, `test_live.rs`.

**Feature flags** (Cargo.toml): Default features include `observability-prometheus`, `channel-nostr`, `channel-lark`, `skill-creation`. Use `--features ci-all` for comprehensive CI builds. Hardware features (`hardware`, `peripheral-rpi`, `probe`) require system C libraries.

## Fork-Specific Notes

- WeCom channel: `src/channels/wecom.rs` — AES-256-CBC encryption, SHA1 signature verification, XML message parsing, OAuth token management
- Channel approval: `src/approval/` — opt-in via `enable_channel_approval = true` in `[autonomy]` config
- Task planner: `plan` and `plan_update` tools for structured multi-step plans
- Progressive tool loading: opt-out with `runtime.deferred_builtin_tools = false`
- Auto memory recall: `auto_recall` field in `[memory]` config
