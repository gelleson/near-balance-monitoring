use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "near-balance", about = "NEAR Protocol balance detector")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Query and display current balance
    Balance {
        /// NEAR account ID
        account_id: String,
    },
    /// Monitor balance for changes over time
    Monitor {
        /// NEAR account ID
        account_id: String,
        /// Polling interval in seconds
        #[arg(long, default_value_t = 10)]
        interval: u64,
    },
    /// Start Telegram bot
    Bot,
    /// Fetch and display recent transactions
    Txs {
        /// NEAR account ID
        account_id: String,
    },
}
