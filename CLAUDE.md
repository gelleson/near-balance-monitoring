# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

NEAR Balance Monitor is a Rust CLI application for monitoring NEAR Protocol account balances. It operates in three modes:
- **CLI Mode**: One-time balance checks and transaction queries
- **Monitor Mode**: Continuous polling of an account with configurable intervals
- **Telegram Bot**: Multi-user remote monitoring with real-time alerts (60s polling interval)

Tech stack: Rust 2024 edition, tokio (async runtime), reqwest (HTTP client), clap (CLI framework), teloxide (Telegram bot framework), pretty_env_logger + log (logging).

## Build and Run Commands

```bash
# Build
cargo build --release
just build  # equivalent

# Run CLI commands
cargo run -- balance <account_id>.near
cargo run -- monitor <account_id>.near --interval 30
cargo run -- txs <account_id>.near

# Run Telegram Bot (requires TELOXIDE_TOKEN env var)
export TELOXIDE_TOKEN="your-bot-token"
export RUST_LOG=info
cargo run -- bot

# Lint and format
cargo clippy
cargo fmt --check
cargo fmt

# Test (no tests currently exist)
cargo test
cargo test <test_name>
```

## Deployment

```bash
# Install binary to /usr/local/bin/near-monitor
just install

# Setup systemd service (builds, installs, creates service)
just setup-service <telegram-bot-token>

# Service management
sudo systemctl start near-monitor
sudo systemctl status near-monitor
sudo journalctl -u near-monitor -f

# Deploy (install + restart service)
just deploy
```

## Architecture

- **src/main.rs**: Entry point, initializes logger and parses CLI
- **src/cli.rs**: CLI command definitions using clap derive macros
- **src/near.rs**: NEAR RPC client for balance queries and transaction fetching
- **src/bot.rs**: Telegram bot implementation with background monitoring task
- **src/commands.rs**: Shared command execution logic for all modes
- **src/utils.rs**: Formatting utilities (balance, timestamps, etc.)

### Key Design Patterns

**NEAR RPC Client** (`NearClient` in src/near.rs):
- Uses NEAR RPC endpoint: `https://h36uashbwvxlllkjfzzaxgfu-near-rpc.defuse.org`
- Transactions fetched from NearBlocks API: `https://api.nearblocks.io/v1/account/{account}/txns`
- Balance stored as `u128` in yoctoNEAR (1 NEAR = 10^24 yoctoNEAR)
- All RPC methods return `Result<T, String>` with user-friendly error messages

**Async State Management** (Telegram bot):
- Shared state uses `Arc<Mutex<HashMap<ChatId, Vec<MonitoredAccount>>>>`
- Clone `Arc` when spawning background tasks
- Background monitoring task runs in `tokio::spawn` with 60s intervals

**Error Handling**:
- Use `Result<T, String>` for command logic
- Convert errors with `map_err(|e| format!("..."))` for user-friendly messages
- Use `?` operator for error propagation
- Log errors with `log::error!` before returning

**CLI Framework** (clap):
- Use derive macros on structs/enums
- Doc comments become command descriptions
- `#[arg(...)]` for argument configuration

**Telegram Bot** (teloxide):
- Use `BotCommands` derive trait for command definitions
- Use `Command::repl` for simple message handling
- Handler functions return `ResponseResult<()>`

## Environment Variables

| Variable | Required | Description |
|----------|----------|-------------|
| `TELOXIDE_TOKEN` | Bot mode | Telegram bot token from @BotFather |
| `RUST_LOG` | Optional | Log level: `info`, `debug`, `error` (default: none) |

## Code Style

**Import Order** (with blank lines between groups):
1. `std`, `core`
2. External crates (`tokio`, `serde`, `clap`, etc.)
3. Local modules (`crate::`, `super::`)

**Type Usage**:
- `u128` for yoctoNEAR amounts
- `String` over `&str` for owned data in return types
- Explicit types in function signatures

**Logging**:
- Initialize with `pretty_env_logger::init()` in main
- Use `log::info!`, `log::error!`, `log::debug!` macros
- Set level via `RUST_LOG` environment variable

## Common Development Tasks

**Add new CLI command**:
1. Add variant to `Commands` enum in `src/cli.rs`
2. Add match arm in `commands::run()` in `src/commands.rs`

**Add new bot command**:
1. Add variant to `Command` enum in `src/bot.rs`
2. Implement handler logic in `answer()` function

**Add new NEAR RPC method**:
1. Add method to `NearClient` impl in `src/near.rs`
2. Define request/response structs using serde
3. Follow existing error handling patterns
