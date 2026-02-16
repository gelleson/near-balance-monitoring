//! Telegram bot implementation.
//!
//! This module implements a Telegram bot for monitoring NEAR account balances.
//! The bot supports multiple users simultaneously, each with their own watchlist
//! of accounts. A background task polls accounts every 60 seconds and sends alerts
//! when balances change.
//!
//! # Architecture
//!
//! - **Persistent State**: `Arc<Mutex<AccountPersistenceManager>>` holds all monitored accounts
//!   and persists them to `monitored_accounts.json` for durability across restarts
//! - **Background Task**: Runs in a separate tokio task, polling every 60 seconds
//! - **Multi-User**: Each user (chat ID) has their own list of monitored accounts
//! - **Data Persistence**: All CRUD operations automatically save to disk using atomic writes
//!
//! # Bot Commands
//!
//! - `/start` - Welcome message
//! - `/help` - Show available commands
//! - `/balance <account>` - Query current balance
//! - `/add <account>` - Add account to watchlist
//! - `/remove <account>` - Remove account from watchlist
//! - `/list` - List monitored accounts
//! - `/trxs <account>` - Show recent transactions

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::HashSet;
use std::fs;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use teloxide::prelude::*;
use teloxide::utils::command::BotCommands;
use tokio::sync::Mutex;
use tokio::time;

use crate::near::NearClient;
use crate::persistence::AccountPersistenceManager;
use crate::utils;

/// Telegram bot commands.
///
/// These commands are automatically parsed by teloxide's `BotCommands` derive macro.
/// Command descriptions appear in the bot's help menu.
#[derive(BotCommands, Clone, Debug)]
#[command(
    rename_rule = "lowercase",
    description = "These commands are supported:"
)]
enum Command {
    #[command(description = "display this text.")]
    Help,
    #[command(description = "start the bot.")]
    Start,
    #[command(description = "fetch balance of an account. Usage: /balance <account_id>")]
    Balance(String),
    #[command(description = "add an account to monitor.")]
    Add(String),
    #[command(description = "remove an account from monitoring.")]
    Remove(String),
    #[command(description = "remove an account from monitoring.")]
    Delete(String),
    #[command(description = "edit an account ID. Usage: /edit <old_id> <new_id>")]
    Edit(String),
    #[command(description = "list monitored accounts.")]
    List,
    #[command(description = "list last 10 transactions. Usage: /trxs <account_id>")]
    Trxs(String),
}

/// Manages the persistence of user IDs to enable broadcasting and startup notifications.
///
/// This manager maintains a set of unique Telegram chat IDs and ensures they are
/// saved to a JSON file for persistence across bot restarts.
struct UserManager {
    /// Set of unique Telegram chat IDs.
    users: HashSet<i64>,
    /// Path to the JSON file where user IDs are stored.
    file_path: String,
}

impl UserManager {
    /// Loads known users from the specified file path.
    ///
    /// If the file does not exist, an empty `UserManager` is returned.
    fn load(file_path: &str) -> Self {
        log::info!("Loading user manager file={}", file_path);
        let users = if Path::new(file_path).exists() {
            let data = fs::read_to_string(file_path).unwrap_or_default();
            serde_json::from_str(&data).unwrap_or_default()
        } else {
            HashSet::new()
        };
        log::info!(
            "User manager loaded user_count={} file={}",
            users.len(),
            file_path
        );
        Self {
            users,
            file_path: file_path.to_string(),
        }
    }

    /// Adds a new user to the system.
    ///
    /// Returns `true` if the user was newly added, `false` if they were already known.
    /// Automatically saves the updated user list to disk.
    fn add_user(&mut self, chat_id: i64) -> bool {
        if self.users.insert(chat_id) {
            log::info!("User added chat_id={}", chat_id);
            self.save();
            true
        } else {
            log::debug!("User already exists chat_id={}", chat_id);
            false
        }
    }

    /// Saves the current list of users to the configured file path.
    fn save(&self) {
        if let Ok(data) = serde_json::to_string(&self.users) {
            match fs::write(&self.file_path, data) {
                Ok(_) => log::debug!(
                    "User list saved user_count={} file={}",
                    self.users.len(),
                    self.file_path
                ),
                Err(e) => log::error!("Failed to save user list file={}: {}", self.file_path, e),
            }
        } else {
            log::warn!("Failed to serialize user data");
        }
    }

    /// Returns a list of all unique user IDs currently tracked by the bot.
    fn get_all_users(&self) -> Vec<i64> {
        self.users.iter().cloned().collect()
    }
}

/// Internal state for an account being monitored by a specific user/chat.
///
/// Each instance represents one account being watched by one user.
/// The same account can be monitored by multiple users (multiple instances with different chat IDs).
#[derive(Clone, Serialize, Deserialize)]
pub struct MonitoredAccount {
    /// NEAR account ID being monitored (e.g., "example.near")
    pub account_id: String,
    /// Last known balance in yoctoNEAR. Used to detect changes.
    /// `None` means the initial balance hasn't been fetched yet.
    pub last_balance: Option<u128>,
    /// The Telegram chat ID to send notifications to when balance changes.
    #[serde(
        serialize_with = "serialize_chat_id",
        deserialize_with = "deserialize_chat_id"
    )]
    pub chat_id: ChatId,
}

/// Serializes a ChatId as an i64.
fn serialize_chat_id<S>(chat_id: &ChatId, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_i64(chat_id.0)
}

/// Deserializes a ChatId from an i64.
fn deserialize_chat_id<'de, D>(deserializer: D) -> Result<ChatId, D::Error>
where
    D: Deserializer<'de>,
{
    let id = i64::deserialize(deserializer)?;
    Ok(ChatId(id))
}

/// Starts the Telegram bot and the background monitoring loop.
///
/// This function initializes the bot, spawns a background task for monitoring
/// account balances, and starts the command handler loop.
///
/// # Environment Variables
///
/// Requires `TELOXIDE_TOKEN` to be set with a valid Telegram bot token.
///
/// # Returns
///
/// Returns `Ok(())` when the bot stops gracefully, or an error message.
///
/// # Errors
///
/// Returns `Err(String)` if the bot token is invalid or missing.
///
/// # Architecture
///
/// The function spawns two concurrent tasks:
/// 1. **Command Handler**: Processes user commands via `Command::repl`
/// 2. **Background Monitor**: Polls accounts every 60 seconds and sends alerts
///
/// # Examples
///
/// ```no_run
/// # use near_balance_monitor::bot;
/// # #[tokio::main]
/// # async fn main() -> Result<(), String> {
/// // Set TELOXIDE_TOKEN environment variable first
/// bot::run().await?;
/// # Ok(())
/// # }
/// ```
pub async fn run() -> Result<(), String> {
    log::info!("Starting bot...");

    let bot = Bot::from_env();
    log::info!("Bot initialized successfully");

    // Shared state: List of monitored accounts and known users
    let monitored_accounts: Arc<Mutex<AccountPersistenceManager>> = Arc::new(Mutex::new(
        AccountPersistenceManager::load("monitored_accounts.json"),
    ));
    let user_manager: Arc<Mutex<UserManager>> =
        Arc::new(Mutex::new(UserManager::load("users.json")));

    let monitored_accounts_for_loop = monitored_accounts.clone();
    let bot_for_loop = bot.clone();

    // Notify users about new deployment/restart
    {
        let users = user_manager.lock().await.get_all_users();
        log::info!("Loaded user manager user_count={}", users.len());
        log::info!(
            "Broadcasting deployment notification user_count={}",
            users.len()
        );
        let mut success_count = 0;
        let mut fail_count = 0;
        for user_id in users {
            match bot
                .send_message(
                    ChatId(user_id),
                    "🚀 New version deployed and bot restarted!",
                )
                .await
            {
                Ok(_) => success_count += 1,
                Err(_) => fail_count += 1,
            }
        }
        log::info!(
            "Deployment notifications sent successful={} failed={}",
            success_count,
            fail_count
        );
    }

    // Spawn monitoring loop
    log::info!("Background monitoring task started interval=60s");
    tokio::spawn(async move {
        let near_client = NearClient::new();
        let mut interval = time::interval(Duration::from_secs(60)); // Check every minute
        let mut cycle_count: u64 = 0;
        let task_start = std::time::Instant::now();

        loop {
            interval.tick().await;
            cycle_count += 1;

            let accounts_to_check: Vec<MonitoredAccount> = {
                let guard = monitored_accounts_for_loop.lock().await;
                guard.get_all_accounts()
            };

            let account_count = accounts_to_check.len();
            log::debug!(
                "Background poll cycle account_count={} cycle={}",
                account_count,
                cycle_count
            );

            for account in &accounts_to_check {
                log::debug!(
                    "Polling account={} chat_id={}",
                    account.account_id,
                    account.chat_id
                );
                match near_client.fetch_balance(&account.account_id).await {
                    Ok(current_balance) => {
                        let changed = account.last_balance != Some(current_balance);
                        if changed {
                            log::info!(
                                "Balance change detected account={} chat_id={} old={:?} new={}",
                                account.account_id,
                                account.chat_id,
                                account.last_balance,
                                current_balance
                            );
                            let message = format!(
                                "🚨 Balance Update for {}!\n\nOld: {}\nNew: {}",
                                account.account_id,
                                account
                                    .last_balance
                                    .map_or("Unknown".to_string(), utils::format_near),
                                utils::format_near(current_balance)
                            );

                            if let Err(e) =
                                bot_for_loop.send_message(account.chat_id, message).await
                            {
                                log::error!("Failed to send alert to {}: {}", account.chat_id, e);
                            }

                            // Persist updated balance
                            let mut guard = monitored_accounts_for_loop.lock().await;
                            guard.update_balance(
                                &account.account_id,
                                account.chat_id,
                                current_balance,
                            );
                            log::debug!(
                                "Updated account state account={} chat_id={} balance={}",
                                account.account_id,
                                account.chat_id,
                                current_balance
                            );
                        }
                    }
                    Err(e) => {
                        log::error!("Error fetching balance for {}: {}", account.account_id, e);
                    }
                }
            }

            if cycle_count % 10 == 0 {
                log::info!(
                    "Background monitor heartbeat cycle={} uptime_mins={} active_accounts={}",
                    cycle_count,
                    task_start.elapsed().as_secs() / 60,
                    account_count
                );
            }
        }
    });

    log::info!("Command handler started, bot ready");
    Command::repl(bot, move |bot, msg, cmd| {
        let monitored_accounts = monitored_accounts.clone();
        let user_manager = user_manager.clone();
        async move { answer(bot, msg, cmd, monitored_accounts, user_manager).await }
    })
    .await;

    Ok(())
}

/// Handles incoming bot commands and executes the appropriate action.
///
/// This function is called by the teloxide framework for each user command.
/// It processes the command, interacts with NEAR RPC, and sends responses.
///
/// # Arguments
///
/// * `bot` - The Telegram bot instance
/// * `msg` - The incoming message containing the command
/// * `cmd` - The parsed command enum
/// * `monitored_accounts` - Shared state of monitored accounts
/// * `user_manager` - Shared state of known users
///
/// # Returns
///
/// Returns `Ok(())` if the command was handled successfully, or a teloxide error.
///
/// # Error Handling
///
/// Errors are caught and sent back to the user as error messages rather than
/// propagated up, so the bot continues running even if individual commands fail.
async fn answer(
    bot: Bot,
    msg: Message,
    cmd: Command,
    monitored_accounts: Arc<Mutex<AccountPersistenceManager>>,
    user_manager: Arc<Mutex<UserManager>>,
) -> ResponseResult<()> {
    log::debug!(
        "Received message chat_id={} command={:?}",
        msg.chat.id.0,
        cmd
    );

    // Record user
    {
        let mut guard = user_manager.lock().await;
        if guard.add_user(msg.chat.id.0) {
            log::info!("New user registered chat_id={}", msg.chat.id.0);
        }
    }

    match cmd {
        Command::Help => {
            log::info!("Help command chat_id={}", msg.chat.id.0);
            if let Err(e) = bot
                .send_message(msg.chat.id, Command::descriptions().to_string())
                .await
            {
                log::error!(
                    "Failed to send Help response chat_id={}: {}",
                    msg.chat.id.0,
                    e
                );
                return Err(e);
            }
        }
        Command::Start => {
            log::info!("Start command chat_id={}", msg.chat.id.0);
            if let Err(e) = bot
                .send_message(
                    msg.chat.id,
                    "Welcome to the NEAR Balance Monitor Bot! Use /help to see available commands.",
                )
                .await
            {
                log::error!(
                    "Failed to send Start response chat_id={}: {}",
                    msg.chat.id.0,
                    e
                );
                return Err(e);
            }
        }
        Command::Balance(account_id) => {
            log::info!(
                "Balance command chat_id={} account={}",
                msg.chat.id.0,
                account_id
            );
            if account_id.is_empty() {
                if let Err(e) = bot
                    .send_message(
                        msg.chat.id,
                        "Please provide an account ID. Usage: /balance <account_id>",
                    )
                    .await
                {
                    log::error!(
                        "Failed to send Balance validation error chat_id={}: {}",
                        msg.chat.id.0,
                        e
                    );
                    return Err(e);
                }
                return Ok(());
            }

            let near_client = NearClient::new();
            match near_client.fetch_balance(&account_id).await {
                Ok(balance) => {
                    log::info!(
                        "Balance command completed chat_id={} account={} balance={}",
                        msg.chat.id.0,
                        account_id,
                        balance
                    );
                    if let Err(e) = bot
                        .send_message(
                            msg.chat.id,
                            format!(
                                "Balance for {}: {}",
                                account_id,
                                utils::format_near(balance)
                            ),
                        )
                        .await
                    {
                        log::error!(
                            "Failed to send Balance success response chat_id={}: {}",
                            msg.chat.id.0,
                            e
                        );
                        return Err(e);
                    }
                }
                Err(e) => {
                    log::error!(
                        "Balance command failed chat_id={} account={}: {}",
                        msg.chat.id.0,
                        account_id,
                        e
                    );
                    if let Err(send_err) = bot
                        .send_message(msg.chat.id, format!("Error fetching balance: {}", e))
                        .await
                    {
                        log::error!(
                            "Failed to send Balance error response chat_id={}: {}",
                            msg.chat.id.0,
                            send_err
                        );
                        return Err(send_err);
                    }
                }
            }
        }
        Command::Add(account_id) => {
            log::info!(
                "Add command chat_id={} account={}",
                msg.chat.id.0,
                account_id
            );
            if account_id.is_empty() {
                if let Err(e) = bot
                    .send_message(msg.chat.id, "Please provide an account ID.")
                    .await
                {
                    log::error!(
                        "Failed to send Add validation error chat_id={}: {}",
                        msg.chat.id.0,
                        e
                    );
                    return Err(e);
                }
                return Ok(());
            }

            let mut guard = monitored_accounts.lock().await;
            let account = MonitoredAccount {
                account_id: account_id.clone(),
                last_balance: None,
                chat_id: msg.chat.id,
            };

            if guard.add_account(account) {
                log::info!(
                    "Account added to monitoring chat_id={} account={}",
                    msg.chat.id.0,
                    account_id
                );
                if let Err(e) = bot
                    .send_message(
                        msg.chat.id,
                        format!("Added {} to monitoring list.", account_id),
                    )
                    .await
                {
                    log::error!(
                        "Failed to send Add success response chat_id={}: {}",
                        msg.chat.id.0,
                        e
                    );
                    return Err(e);
                }
            } else {
                log::warn!(
                    "Add command: already monitored chat_id={} account={}",
                    msg.chat.id.0,
                    account_id
                );
                if let Err(e) = bot
                    .send_message(
                        msg.chat.id,
                        format!("{} is already being monitored.", account_id),
                    )
                    .await
                {
                    log::error!(
                        "Failed to send Add duplicate response chat_id={}: {}",
                        msg.chat.id.0,
                        e
                    );
                    return Err(e);
                }
            }
        }
        Command::Remove(account_id) | Command::Delete(account_id) => {
            log::info!(
                "Remove command chat_id={} account={}",
                msg.chat.id.0,
                account_id
            );
            let mut guard = monitored_accounts.lock().await;

            if guard.remove_account(&account_id, msg.chat.id) {
                log::info!(
                    "Account removed chat_id={} account={}",
                    msg.chat.id.0,
                    account_id
                );
                if let Err(e) = bot
                    .send_message(
                        msg.chat.id,
                        format!("Removed {} from monitoring list.", account_id),
                    )
                    .await
                {
                    log::error!(
                        "Failed to send Remove success response chat_id={}: {}",
                        msg.chat.id.0,
                        e
                    );
                    return Err(e);
                }
            } else {
                log::warn!(
                    "Remove command: not found chat_id={} account={}",
                    msg.chat.id.0,
                    account_id
                );
                if let Err(e) = bot
                    .send_message(
                        msg.chat.id,
                        format!("Account {} was not found.", account_id),
                    )
                    .await
                {
                    log::error!(
                        "Failed to send Remove not found response chat_id={}: {}",
                        msg.chat.id.0,
                        e
                    );
                    return Err(e);
                }
            }
        }
        Command::Edit(args) => {
            log::info!("Edit command chat_id={} args={}", msg.chat.id.0, args);
            let parts: Vec<&str> = args.split_whitespace().collect();
            if parts.len() != 2 {
                if let Err(e) = bot
                    .send_message(msg.chat.id, "Usage: /edit <old_id> <new_id>")
                    .await
                {
                    log::error!(
                        "Failed to send Edit validation error chat_id={}: {}",
                        msg.chat.id.0,
                        e
                    );
                    return Err(e);
                }
                return Ok(());
            }
            let old_id = parts[0];
            let new_id = parts[1];

            let mut guard = monitored_accounts.lock().await;
            match guard.update_account(old_id, msg.chat.id, new_id.to_string()) {
                Ok(_) => {
                    log::info!(
                        "Account updated chat_id={} old={} new={}",
                        msg.chat.id.0,
                        old_id,
                        new_id
                    );
                    if let Err(e) = bot
                        .send_message(msg.chat.id, format!("Updated {} to {}.", old_id, new_id))
                        .await
                    {
                        log::error!(
                            "Failed to send Edit success response chat_id={}: {}",
                            msg.chat.id.0,
                            e
                        );
                        return Err(e);
                    }
                }
                Err(_) => {
                    log::warn!(
                        "Edit command: not found chat_id={} old={}",
                        msg.chat.id.0,
                        old_id
                    );
                    if let Err(e) = bot
                        .send_message(msg.chat.id, format!("Account {} was not found.", old_id))
                        .await
                    {
                        log::error!(
                            "Failed to send Edit not found response chat_id={}: {}",
                            msg.chat.id.0,
                            e
                        );
                        return Err(e);
                    }
                }
            }
        }
        Command::List => {
            let guard = monitored_accounts.lock().await;
            let accounts: Vec<String> = guard
                .get_accounts_for_chat(msg.chat.id)
                .iter()
                .map(|acc| acc.account_id.clone())
                .collect();
            log::info!(
                "List command chat_id={} account_count={}",
                msg.chat.id.0,
                accounts.len()
            );
            drop(guard); // Explicitly drop mutex guard before sending message

            if accounts.is_empty() {
                if let Err(e) = bot
                    .send_message(msg.chat.id, "You are not monitoring any accounts.")
                    .await
                {
                    log::error!(
                        "Failed to send List empty response chat_id={}: {}",
                        msg.chat.id.0,
                        e
                    );
                    return Err(e);
                }
            } else {
                let list = accounts.join("\n");
                if let Err(e) = bot
                    .send_message(msg.chat.id, format!("Monitoring:\n{}", list))
                    .await
                {
                    log::error!(
                        "Failed to send List success response chat_id={}: {}",
                        msg.chat.id.0,
                        e
                    );
                    return Err(e);
                }
            }
        }
        Command::Trxs(account_id) => {
            if account_id.is_empty() {
                if let Err(e) = bot
                    .send_message(
                        msg.chat.id,
                        "Please provide an account ID. Usage: /trxs <account_id>",
                    )
                    .await
                {
                    log::error!(
                        "Failed to send Trxs validation error chat_id={}: {}",
                        msg.chat.id.0,
                        e
                    );
                    return Err(e);
                }
                return Ok(());
            }

            let near_client = NearClient::new();
            match near_client.fetch_transactions(&account_id).await {
                Ok(txs) => {
                    if txs.is_empty() {
                        if let Err(e) = bot
                            .send_message(
                                msg.chat.id,
                                format!("No transactions found for {}.", account_id),
                            )
                            .await
                        {
                            log::error!(
                                "Failed to send Trxs empty response chat_id={}: {}",
                                msg.chat.id.0,
                                e
                            );
                            return Err(e);
                        }
                    } else {
                        let mut response = format!("Last 10 transactions for {}:\n", account_id);
                        for tx in txs {
                            response.push_str(&format!(
                                "\nTime: {}\nHash: {}...\nFrom: {}\nTo: {}\nAmount: {}\n",
                                utils::format_timestamp(tx.block_timestamp),
                                &tx.hash[..10],
                                tx.signer_id,
                                tx.receiver_id,
                                utils::format_near(tx.actions_agg.deposit as u128)
                            ));
                        }
                        if let Err(e) = bot.send_message(msg.chat.id, response).await {
                            log::error!(
                                "Failed to send Trxs success response chat_id={}: {}",
                                msg.chat.id.0,
                                e
                            );
                            return Err(e);
                        }
                    }
                }
                Err(e) => {
                    if let Err(send_err) = bot
                        .send_message(msg.chat.id, format!("Error fetching transactions: {}", e))
                        .await
                    {
                        log::error!(
                            "Failed to send Trxs error response chat_id={}: {}",
                            msg.chat.id.0,
                            send_err
                        );
                        return Err(send_err);
                    }
                }
            }
        }
    };
    Ok(())
}
