//! Command-line interface definitions.
//!
//! This module defines the CLI structure using `clap` derive macros.
//! All CLI commands are defined here and parsed automatically by clap.

use clap::{Parser, Subcommand};

/// Main CLI structure for the NEAR Balance Monitor application.
#[derive(Parser)]
#[command(name = "near-balance", about = "NEAR Protocol balance detector")]
pub struct Cli {
    /// The subcommand to execute
    #[command(subcommand)]
    pub command: Commands,
}

/// Available CLI commands.
///
/// Each variant represents a different mode of operation:
/// - `Balance`: One-time balance query
/// - `Monitor`: Continuous balance monitoring
/// - `Bot`: Telegram bot mode
/// - `Txs`: Transaction history lookup
#[derive(Subcommand)]
pub enum Commands {
    /// Query and display current balance
    Balance {
        /// NEAR account ID (e.g., "example.near")
        account_id: String,
    },
    /// Monitor balance for changes over time
    Monitor {
        /// NEAR account ID (e.g., "example.near")
        account_id: String,
        /// Polling interval in seconds (default: 10s)
        #[arg(long, default_value_t = 10)]
        interval: u64,
    },
    /// Start Telegram bot for remote monitoring
    Bot,
    /// Fetch and display recent transactions
    Txs {
        /// NEAR account ID (e.g., "example.near")
        account_id: String,
    },
}
