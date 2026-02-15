# NEAR Balance Monitor

A lightweight Rust application to monitor NEAR Protocol account balances. It provides a CLI for one-time checks or local monitoring and a Telegram Bot for remote alerts.

## Features

- **CLI Mode**: Query balances directly from your terminal.
- **Monitor Mode**: Watch a specific account for changes with a configurable interval.
- **Telegram Bot**: 
  - Multi-user support.
  - Monitor multiple accounts simultaneously.
  - Persistent background monitoring (60s intervals).
  - Real-time alerts when a balance changes.

## Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) (2024 edition)
- [Just](https://github.com/casey/just) (optional, for automation)

## Installation

### From Source

```bash
cargo build --release
```

### Using Just (System-wide)

```bash
just install
```

## Usage

### CLI Commands

**Check a single balance:**
```bash
cargo run -- balance <account_id>.near
```

**Monitor an account in the terminal:**
```bash
cargo run -- monitor <account_id>.near --interval 30
```

### Telegram Bot

To run the bot, you need a `TELEGRAM_BOT_TOKEN` from [@BotFather](https://t.me/botfather).

**Run locally:**
```bash
export TELEGRAM_BOT_TOKEN="your-token"
export RUST_LOG=info
cargo run -- bot
```

**Bot Commands:**
- `/help` - Show available commands.
- `/add <account_id>` - Add a NEAR account to your watchlist.
- `/remove <account_id>` - Stop monitoring an account.
- `/list` - List all accounts you are currently monitoring.

## Deployment

### Systemd Service

You can deploy the monitor as a background service using the provided `justfile`:

```bash
just setup-service <your-telegram-token>
```

This will:
1. Build the binary in release mode.
2. Install it to `/usr/local/bin/near-monitor`.
3. Create and enable a systemd service named `near-monitor.service`.

### Manual Service Management

```bash
sudo systemctl start near-monitor
sudo systemctl status near-monitor
sudo journalctl -u near-monitor -f
```

## Architecture

- **`src/near.rs`**: Handles RPC communication with the NEAR Protocol.
- **`src/bot.rs`**: Telegram bot implementation using `teloxide`.
- **`src/cli.rs`**: Command-line interface definitions using `clap`.
- **`src/commands.rs`**: Shared execution logic for all modes.

## License

This project is licensed under the MIT License.
