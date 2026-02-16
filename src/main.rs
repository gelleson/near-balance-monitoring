//! NEAR Balance Monitor
//!
//! A lightweight Rust application for monitoring NEAR Protocol account balances.
//!
//! # Features
//!
//! - **CLI Mode**: Query balances directly from your terminal
//! - **Monitor Mode**: Watch a specific account for changes with a configurable interval
//! - **Telegram Bot**: Multi-user support with real-time alerts
//!
//! # Usage
//!
//! ```bash
//! # Check a single balance
//! near-monitor balance example.near
//!
//! # Monitor an account
//! near-monitor monitor example.near --interval 30
//!
//! # Run Telegram bot
//! export TELOXIDE_TOKEN="your-token"
//! near-monitor bot
//! ```

mod bot;
mod cli;
mod commands;
mod near;
mod persistence;
mod utils;

use clap::Parser;
use cli::Cli;

#[tokio::main]
async fn main() {
    pretty_env_logger::init();
    log::info!("Application started version={}", env!("CARGO_PKG_VERSION"));
    let cli = Cli::parse();

    if let Err(e) = commands::run(cli).await {
        log::error!("Application error: {}", e);
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}
