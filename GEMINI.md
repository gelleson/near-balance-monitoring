# GEMINIContext

## Project Overview
**NEAR Balance Monitor** is a Rust-based application designed to track and notify users about balance changes on the NEAR Protocol. It supports three primary modes of operation:
- **CLI**: One-time balance queries.
- **Monitor**: Continuous terminal-based polling for a single account.
- **Telegram Bot**: A multi-user bot that allows users to monitor multiple NEAR accounts and receive real-time alerts upon balance changes.

### Key Technologies
- **Language**: Rust (2024 edition)
- **Runtime**: `tokio` (Asynchronous I/O)
- **RPC Communication**: `reqwest` & `serde_json` (interfacing with NEAR RPC)
- **Telegram Bot Framework**: `teloxide`
- **CLI Parsing**: `clap`
- **Logging**: `pretty_env_logger` & `log`

## Architecture
The project is modularly structured:
- `src/main.rs`: Entry point that initializes logging and delegates to the command runner.
- `src/cli.rs`: Defines the command-line interface and subcommands (`balance`, `monitor`, `bot`).
- `src/near.rs`: Contains the `NearClient` responsible for JSON-RPC requests to the NEAR network.
- `src/bot.rs`: Implements the Telegram bot logic, including a background task for polling balances and a command REPL for user interaction.
- `src/commands.rs`: Shared logic for executing CLI commands.
- `src/utils.rs`: Utility functions for formatting NEAR denominations and timestamps.

## Building and Running

### Common Commands (via `just`)
The project uses a `justfile` for automation:
- **Build**: `just build` (compiles in release mode)
- **Install**: `just install` (installs the binary to `/usr/local/bin/near-monitor`)
- **Deploy as Service**: `just setup-service <TELEGRAM_TOKEN>` (configures and enables a systemd service)
- **Service Management**: `just start`, `just stop`, `just restart`, `just status`.

### Manual Execution
- **Check Balance**: `cargo run -- balance <account_id>.near`
- **Monitor Account**: `cargo run -- monitor <account_id>.near --interval 30`
- **Run Bot**: 
  ```bash
  export TELOXIDE_TOKEN="your-token"
  cargo run -- bot
  ```

**Bot Commands**:
- `/help` - Show available commands.
- `/add <account_id>` - Add a NEAR account to your watchlist.
- `/remove <account_id>` - Stop monitoring an account.
- `/list` - List all accounts you are currently monitoring.
- `/trxs <account_id>` - List the last 10 transactions for an account.
- `/balance <account_id>` - Check the balance of an account.

## Development Conventions
- **Asynchronous Code**: Uses `tokio` extensively. Ensure any new I/O operations are non-blocking.
- **Error Handling**: Prefers `Result<T, String>` for high-level command logic and propagates errors using the `?` operator.
- **State Management**: The Telegram bot uses `Arc<Mutex<Vec<MonitoredAccount>>>` for shared state across the REPL and the monitoring loop.
- **Deployment**: Aim for compatibility with the provided systemd service template in the `justfile`.
- **Environment Variables**: `TELOXIDE_TOKEN` is required for the bot, and `RUST_LOG` (e.g., `info`, `debug`) controls logging verbosity.
