# AGENTS.md - Developer Guide

Guidelines for agentic coding agents working on the NEAR Balance Monitor project.

## Project Overview

- **NEAR Balance Monitor**: Rust CLI app for monitoring NEAR Protocol account balances
- **Modes**: CLI (one-time), Monitor (polling), Telegram Bot (multi-user alerts)
- **Tech Stack**: Rust 2024, tokio, reqwest, clap, teloxide, log + pretty_env_logger

## Architecture
- `src/main.rs`: Entry point
- `src/cli.rs`: CLI definitions (clap derive)
- `src/near.rs`: NEAR RPC client
- `src/bot.rs`: Telegram bot implementation
- `src/commands.rs`: Shared execution logic
- `src/utils.rs`: Formatting utilities

---

## Build, Lint, and Test Commands

```bash
# Build
cargo build --release   # or: just build

# Run
cargo run -- balance <account_id>.near
cargo run -- monitor <account_id>.near --interval 30
cargo run -- txs <account_id>.near

# Bot (requires TELOXIDE_TOKEN env var)
export TELOXIDE_TOKEN="your-token"
cargo run -- bot

# Lint & Format
cargo clippy
cargo fmt --check
cargo fmt

# Test
cargo test
cargo test <test_name>  # No tests currently exist
```

---

## Code Style Guidelines

### Imports (order with blank lines)
1. `std`, `core`
2. External crates (`tokio`, `serde`, `clap`, etc.)
3. Local modules (`crate::`, `super::`)

### Naming
- Functions/variables: `snake_case`
- Types/Enums: `PascalCase`
- Constants: `SCREAMING_SNAKE_CASE`

### Types
- Explicit types in function signatures
- `String` over `&str` for owned data
- `u128` for yoctoNEAR amounts
- `Option<T>` for nullable values

### Error Handling
- Use `Result<T, String>` for command logic
- Convert errors with `map_err` to user-friendly messages
- Use `?` operator for propagation
- Log errors at `error` level before returning

```rust
pub async fn fetch_balance(&self, account_id: &str) -> Result<u128, String> {
    let response = self.client.get(&url).send().await
        .map_err(|e| format!("HTTP request failed: {e}"))?;
    // ...
}
```

### Async Patterns
- Use `#[tokio::main]` for main entry point
- Use `tokio::time` for intervals/timeouts
- Use `Arc<Mutex<T>>` for shared state across async tasks
- Clone `Arc` when spawning new tasks

```rust
let state: Arc<Mutex<Vec<MonitoredAccount>>> = Arc::new(Mutex::new(Vec::new()));
let state_clone = state.clone();
tokio::spawn(async move { /* use state_clone */ });
```

### Logging
- Use `log::info!`, `log::error!`, `log::debug!`
- Initialize with `pretty_env_logger::init()`
- Set level via `RUST_LOG` env var

### Telegram Bot (teloxide)
- Use `BotCommands` derive for commands
- Use `Command::repl` for simple handling
- Use `ResponseResult<()>` for handlers

```rust
#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase")]
enum Command {
    #[command(description = "...")]
    Help,
}
```

### CLI (clap)
- Use derive macros on structs/enums
- Doc comments for command descriptions
- `#[arg(...)]` for arguments

---

## Environment Variables

| Variable | Required | Description |
|----------|----------|-------------|
| `TELOXIDE_TOKEN` | Bot mode | Telegram bot token |
| `RUST_LOG` | Optional | Log level (`info`, `debug`, `error`) |

---

## Common Tasks

### New CLI command: Add variant to `Commands` in `src/cli.rs`, then add arm in `commands.rs::run()`

### New bot command: Add variant to `Command` in `src/bot.rs`, implement in `answer()`

### New RPC method: Add to `NearClient` in `src/near.rs`, define request/response structs

---

## Deployment Notes
- Binary: `near-monitor` when installed
- Service: `near-monitor.service`
- Bot interval: 60s (hardcoded in `src/bot.rs`)
- RPC: defined in `src/near.rs`
- Tx API: `api.nearblocks.io`
