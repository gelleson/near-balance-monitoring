//! Command execution logic.
//!
//! This module contains the core execution logic for all CLI commands and modes.
//! It handles:
//! - One-time balance queries
//! - Continuous monitoring with polling
//! - Transaction history display
//! - Telegram bot initialization

use crate::cli::{Cli, Commands};
use crate::near::NearClient;
use crate::utils;
use crate::bot;
use std::time::Duration;
use tokio::time;

/// Executes the CLI command specified in the parsed arguments.
///
/// This is the main entry point for command execution. It routes to the
/// appropriate handler based on the command type.
///
/// # Arguments
///
/// * `cli` - Parsed CLI arguments containing the command to execute
///
/// # Returns
///
/// Returns `Ok(())` on successful execution, or an error message describing the failure.
///
/// # Errors
///
/// Returns `Err(String)` if:
/// - Network requests fail
/// - NEAR RPC returns an error
/// - Account doesn't exist
/// - Telegram bot token is invalid (for bot mode)
///
/// # Examples
///
/// ```no_run
/// # use near_balance_monitor::cli::Cli;
/// # use near_balance_monitor::commands;
/// # use clap::Parser;
/// # #[tokio::main]
/// # async fn main() -> Result<(), String> {
/// let cli = Cli::parse();
/// commands::run(cli).await?;
/// # Ok(())
/// # }
/// ```
pub async fn run(cli: Cli) -> Result<(), String> {
    let command_name = match &cli.command {
        Commands::Balance { .. } => "balance",
        Commands::Monitor { .. } => "monitor",
        Commands::Bot => "bot",
        Commands::Txs { .. } => "txs",
    };
    log::info!("Executing command={}", command_name);

    let near_client = NearClient::new();

    match cli.command {
        Commands::Balance { account_id } => {
            log::info!("Fetching balance account={}", account_id);
            let balance = near_client.fetch_balance(&account_id).await?;
            print_balance(&account_id, balance);
        }
        Commands::Monitor {
            account_id,
            interval,
        } => {
            log::info!("Monitor started account={} interval={}s", account_id, interval);
            println!("Monitoring {account_id} every {interval}s...");
            let mut ticker = time::interval(Duration::from_secs(interval));
            let mut previous_balance: Option<u128> = None;
            let mut poll_count: u64 = 0;
            let mut success_count: u64 = 0;
            let mut error_count: u64 = 0;
            let start_time = std::time::Instant::now();

            loop {
                ticker.tick().await;
                poll_count += 1;
                log::debug!("Monitor poll account={} poll_count={}", account_id, poll_count);

                match near_client.fetch_balance(&account_id).await {
                    Ok(balance) => {
                        success_count += 1;
                        let changed = previous_balance != Some(balance);
                        if changed {
                            log::info!("Balance changed account={} old={:?} new={}", account_id, previous_balance, balance);
                            print_balance(&account_id, balance);
                            previous_balance = Some(balance);
                        }
                    }
                    Err(e) => {
                        error_count += 1;
                        log::error!("Monitor fetch failed account={}: {}", account_id, e);
                        eprintln!("[{}] Error: {e}", utils::now_timestamp());
                    }
                }

                if poll_count % 10 == 0 {
                    log::info!("Monitor heartbeat account={} uptime_secs={} polls={} success={} errors={}",
                               account_id, start_time.elapsed().as_secs(), poll_count, success_count, error_count);
                }
            }
        }
        Commands::Bot => {
            log::info!("Starting Telegram bot mode");
            bot::run().await?;
        }
        Commands::Txs { account_id } => {
            log::info!("Fetching transactions account={}", account_id);
            let txs = near_client.fetch_transactions(&account_id).await?;
            if txs.is_empty() {
                log::warn!("No transactions found account={}", account_id);
                println!("No transactions found for {account_id}");
            } else {
                log::info!("Displaying transactions account={} count={}", account_id, txs.len());
                println!("Last transactions for {account_id}:");
                for tx in txs {
                    println!("- Time:   {}\n  Hash:   {}\n  From:   {}\n  To:     {}\n  Amount: {}\n",
                        utils::format_timestamp(tx.block_timestamp),
                        tx.hash,
                        tx.signer_id,
                        tx.receiver_id,
                        utils::format_near(tx.actions_agg.deposit as u128)
                    );
                }
            }
        }
    }
    log::info!("Command completed successfully");
    Ok(())
}

/// Prints a formatted balance message with timestamp.
///
/// Outputs the balance in a human-readable format with the current timestamp
/// and account ID.
///
/// # Arguments
///
/// * `account_id` - The NEAR account ID
/// * `balance` - The balance in yoctoNEAR
///
/// # Examples
///
/// ```no_run
/// # fn main() {
/// # let account_id = "example.near";
/// # let balance = 1000000000000000000000000u128;
/// // Output: [2026-02-15 10:30:45 PST] example.near — 1.0000 NEAR
/// # }
/// ```
fn print_balance(account_id: &str, balance: u128) {
    println!(
        "[{}] {} — {}",
        utils::now_timestamp(),
        account_id,
        utils::format_near(balance)
    );
}
