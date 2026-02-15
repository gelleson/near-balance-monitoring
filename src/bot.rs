use std::sync::Arc;
use teloxide::prelude::*;
use teloxide::utils::command::BotCommands;
use tokio::sync::Mutex;
use tokio::time;
use std::time::Duration;

use crate::near::NearClient;
use crate::utils;

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase", description = "These commands are supported:")]
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

/// Internal state for an account being monitored by a specific user/chat.
#[derive(Clone)]
struct MonitoredAccount {
    account_id: String,
    /// Last known balance in yoctoNEAR. Used to detect changes.
    last_balance: Option<u128>,
    /// The Telegram chat ID to send notifications to.
    chat_id: ChatId,
}

/// Starts the Telegram bot and the background monitoring loop.
/// 
/// This function blocks until the bot is stopped.
pub async fn run() -> Result<(), String> {
    log::info!("Starting bot...");

    let bot = Bot::from_env();

    // Shared state: List of monitored accounts
    // Using simple Arc<Mutex<Vec<...>>> for MVP.
    let monitored_accounts: Arc<Mutex<Vec<MonitoredAccount>>> = Arc::new(Mutex::new(Vec::new()));

    let monitored_accounts_for_loop = monitored_accounts.clone();
    let bot_for_loop = bot.clone();

    // Spawn monitoring loop
    tokio::spawn(async move {
        let near_client = NearClient::new();
        let mut interval = time::interval(Duration::from_secs(60)); // Check every minute

        loop {
            interval.tick().await;

            // We iterate and update in place. Since we hold the lock for the duration of the check
            // (or we could clone, check, then re-lock to update), holding the lock might be simpler for MVP
            // but blocking the bot from adding new accounts is bad.
            // Better: Clone the list, check balances, then re-lock to update if changed.
            
            let accounts_to_check: Vec<MonitoredAccount> = {
                let guard = monitored_accounts_for_loop.lock().await;
                guard.clone()
            };

            for account in accounts_to_check {
                match near_client.fetch_balance(&account.account_id).await {
                    Ok(current_balance) => {
                        let changed = account.last_balance.map_or(true, |prev| prev != current_balance);
                        if changed {
                            let message = format!(
                                "🚨 Balance Update for {}!\n\nOld: {}\nNew: {}",
                                account.account_id,
                                account.last_balance.map_or("Unknown".to_string(), utils::format_near),
                                utils::format_near(current_balance)
                            );
                            
                            if let Err(e) = bot_for_loop.send_message(account.chat_id, message).await {
                                log::error!("Failed to send alert to {}: {}", account.chat_id, e);
                            }

                            // Update state
                            let mut guard = monitored_accounts_for_loop.lock().await;
                            if let Some(acc) = guard.iter_mut().find(|a| a.account_id == account.account_id && a.chat_id == account.chat_id) {
                                acc.last_balance = Some(current_balance);
                            }
                        }
                    }
                    Err(e) => {
                        log::error!("Error fetching balance for {}: {}", account.account_id, e);
                    }
                }
            }
        }
    });

    Command::repl(bot, move |bot, msg, cmd| {
        let monitored_accounts = monitored_accounts.clone();
        async move {
            answer(bot, msg, cmd, monitored_accounts).await
        }
    })
    .await;

    Ok(())
}

async fn answer(
    bot: Bot,
    msg: Message,
    cmd: Command,
    monitored_accounts: Arc<Mutex<Vec<MonitoredAccount>>>,
) -> ResponseResult<()> {
    match cmd {
        Command::Help => {
            bot.send_message(msg.chat.id, Command::descriptions().to_string()).await?;
        }
        Command::Start => {
            bot.send_message(msg.chat.id, "Welcome to the NEAR Balance Monitor Bot! Use /help to see available commands.").await?;
        }
        Command::Balance(account_id) => {
            if account_id.is_empty() {
                bot.send_message(msg.chat.id, "Please provide an account ID. Usage: /balance <account_id>").await?;
                return Ok(());
            }

            let near_client = NearClient::new();
            match near_client.fetch_balance(&account_id).await {
                Ok(balance) => {
                    bot.send_message(msg.chat.id, format!("Balance for {}: {}", account_id, utils::format_near(balance))).await?;
                }
                Err(e) => {
                    bot.send_message(msg.chat.id, format!("Error fetching balance: {}", e)).await?;
                }
            }
        }
        Command::Add(account_id) => {
            if account_id.is_empty() {
                bot.send_message(msg.chat.id, "Please provide an account ID.").await?;
                return Ok(());
            }

            let mut guard = monitored_accounts.lock().await;
            // Check if already monitored
            if guard.iter().any(|acc| acc.account_id == account_id && acc.chat_id == msg.chat.id) {
                 bot.send_message(msg.chat.id, format!("{} is already being monitored.", account_id)).await?;
            } else {
                guard.push(MonitoredAccount {
                    account_id: account_id.clone(),
                    last_balance: None,
                    chat_id: msg.chat.id,
                });
                bot.send_message(msg.chat.id, format!("Added {} to monitoring list.", account_id)).await?;
            }
        }
        Command::Remove(account_id) | Command::Delete(account_id) => {
            let mut guard = monitored_accounts.lock().await;
            let len_before = guard.len();
            guard.retain(|acc| !(acc.account_id == account_id && acc.chat_id == msg.chat.id));
            
            if guard.len() < len_before {
                bot.send_message(msg.chat.id, format!("Removed {} from monitoring list.", account_id)).await?;
            } else {
                bot.send_message(msg.chat.id, format!("Account {} was not found.", account_id)).await?;
            }
        }
        Command::Edit(args) => {
            let parts: Vec<&str> = args.split_whitespace().collect();
            if parts.len() != 2 {
                bot.send_message(msg.chat.id, "Usage: /edit <old_id> <new_id>").await?;
                return Ok(());
            }
            let old_id = parts[0];
            let new_id = parts[1];

            let mut guard = monitored_accounts.lock().await;
            if let Some(acc) = guard.iter_mut().find(|a| a.account_id == old_id && a.chat_id == msg.chat.id) {
                acc.account_id = new_id.to_string();
                acc.last_balance = None; // Reset to trigger a new check
                bot.send_message(msg.chat.id, format!("Updated {} to {}.", old_id, new_id)).await?;
            } else {
                bot.send_message(msg.chat.id, format!("Account {} was not found.", old_id)).await?;
            }
        }
        Command::List => {
            let guard = monitored_accounts.lock().await;
            let accounts: Vec<String> = guard.iter()
                .filter(|acc| acc.chat_id == msg.chat.id)
                .map(|acc| acc.account_id.clone())
                .collect();

            if accounts.is_empty() {
                bot.send_message(msg.chat.id, "You are not monitoring any accounts.").await?;
            } else {
                let list = accounts.join("\n");
                bot.send_message(msg.chat.id, format!("Monitoring:\n{}", list)).await?;
            }
        }
        Command::Trxs(account_id) => {
            if account_id.is_empty() {
                bot.send_message(msg.chat.id, "Please provide an account ID. Usage: /trxs <account_id>").await?;
                return Ok(());
            }

            let near_client = NearClient::new();
            match near_client.fetch_transactions(&account_id).await {
                Ok(txs) => {
                    if txs.is_empty() {
                        bot.send_message(msg.chat.id, format!("No transactions found for {}.", account_id)).await?;
                    } else {
                        let mut response = format!("Last 10 transactions for {}:\n", account_id);
                        for tx in txs {
                            response.push_str(&format!(
                                "\nHash: {}...\nFrom: {}\nTo: {}\nAmount: {}\n",
                                &tx.hash[..10],
                                tx.signer_id,
                                tx.receiver_id,
                                utils::format_near(tx.actions_agg.deposit as u128)
                            ));
                        }
                        bot.send_message(msg.chat.id, response).await?;
                    }
                }
                Err(e) => {
                    bot.send_message(msg.chat.id, format!("Error fetching transactions: {}", e)).await?;
                }
            }
        }
    };
    Ok(())
}
