//! Persistence layer for monitored account data.
//!
//! This module provides the `AccountPersistenceManager` which handles
//! loading and saving monitored accounts to a JSON file. This ensures
//! that monitored accounts survive bot restarts and redeployments.
//!
//! The persistence mechanism uses atomic file writes (write to temp file,
//! then rename) to prevent data corruption during saves.

use std::fs;
use std::path::Path;
use teloxide::types::ChatId;

use crate::bot::MonitoredAccount;

/// Manages persistence of monitored accounts to a JSON file.
///
/// This manager maintains a list of monitored accounts and ensures they are
/// saved to a JSON file for persistence across bot restarts. All mutation
/// operations automatically trigger a save to disk.
///
/// # File Format
///
/// The accounts are stored as a JSON array of `MonitoredAccount` objects:
/// ```json
/// [
///   {
///     "account_id": "example.near",
///     "last_balance": 1500000000000000000000000,
///     "chat_id": 123456789
///   }
/// ]
/// ```
///
/// # Error Handling
///
/// - Load failures result in an empty state (bot continues operating)
/// - Save failures are logged but don't crash the bot
/// - Corrupted JSON files are handled gracefully with error logging
pub struct AccountPersistenceManager {
    /// List of all monitored accounts across all users.
    accounts: Vec<MonitoredAccount>,
    /// Path to the JSON file where accounts are persisted.
    file_path: String,
}

impl AccountPersistenceManager {
    /// Loads monitored accounts from the specified file path.
    ///
    /// If the file does not exist or contains invalid JSON, an empty
    /// `AccountPersistenceManager` is returned with a warning logged.
    ///
    /// # Arguments
    ///
    /// * `file_path` - Path to the JSON file for persistence
    ///
    /// # Returns
    ///
    /// Returns a new `AccountPersistenceManager` with accounts loaded from disk,
    /// or an empty manager if loading fails.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use near_balance_monitor::persistence::AccountPersistenceManager;
    ///
    /// let manager = AccountPersistenceManager::load("monitored_accounts.json");
    /// ```
    pub fn load(file_path: &str) -> Self {
        log::info!("Loading monitored accounts file={}", file_path);

        let accounts = if Path::new(file_path).exists() {
            match fs::read_to_string(file_path) {
                Ok(data) => match serde_json::from_str(&data) {
                    Ok(accounts) => accounts,
                    Err(e) => {
                        log::error!(
                            "Failed to parse monitored accounts JSON file={}: {}",
                            file_path,
                            e
                        );
                        Vec::new()
                    }
                },
                Err(e) => {
                    log::error!(
                        "Failed to read monitored accounts file={}: {}",
                        file_path,
                        e
                    );
                    Vec::new()
                }
            }
        } else {
            log::info!(
                "Monitored accounts file does not exist, starting with empty state file={}",
                file_path
            );
            Vec::new()
        };

        log::info!(
            "Loaded {} monitored accounts from file={}",
            accounts.len(),
            file_path
        );

        Self {
            accounts,
            file_path: file_path.to_string(),
        }
    }

    /// Adds a new monitored account to the system.
    ///
    /// Returns `true` if the account was newly added, `false` if it was already
    /// being monitored by this user (duplicate check based on account_id + chat_id).
    /// Automatically saves the updated account list to disk.
    ///
    /// # Arguments
    ///
    /// * `account` - The `MonitoredAccount` to add
    ///
    /// # Returns
    ///
    /// Returns `true` if the account was newly added, `false` if it already exists.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use near_balance_monitor::persistence::AccountPersistenceManager;
    /// # use near_balance_monitor::bot::MonitoredAccount;
    /// # use teloxide::types::ChatId;
    /// let mut manager = AccountPersistenceManager::load("monitored_accounts.json");
    /// // let account = MonitoredAccount { ... };
    /// // let added = manager.add_account(account);
    /// ```
    pub fn add_account(&mut self, account: MonitoredAccount) -> bool {
        // Check for duplicates
        if self
            .accounts
            .iter()
            .any(|a| a.account_id == account.account_id && a.chat_id == account.chat_id)
        {
            log::debug!(
                "Account already exists chat_id={} account={}",
                account.chat_id,
                account.account_id
            );
            return false;
        }

        log::info!(
            "Account added chat_id={} account={}",
            account.chat_id,
            account.account_id
        );
        self.accounts.push(account);
        self.save();
        true
    }

    /// Removes a monitored account from the system.
    ///
    /// The account is identified by both account_id and chat_id to ensure
    /// we only remove the specific user's monitoring entry.
    /// Automatically saves the updated account list to disk.
    ///
    /// # Arguments
    ///
    /// * `account_id` - The NEAR account ID to remove
    /// * `chat_id` - The Telegram chat ID of the user
    ///
    /// # Returns
    ///
    /// Returns `true` if an account was removed, `false` if not found.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use near_balance_monitor::persistence::AccountPersistenceManager;
    /// # use teloxide::types::ChatId;
    /// let mut manager = AccountPersistenceManager::load("monitored_accounts.json");
    /// let removed = manager.remove_account("example.near", ChatId(123456789));
    /// ```
    pub fn remove_account(&mut self, account_id: &str, chat_id: ChatId) -> bool {
        let len_before = self.accounts.len();
        self.accounts
            .retain(|a| !(a.account_id == account_id && a.chat_id == chat_id));

        let removed = self.accounts.len() < len_before;
        if removed {
            log::info!("Account removed chat_id={} account={}", chat_id, account_id);
            self.save();
        } else {
            log::debug!(
                "Account not found for removal chat_id={} account={}",
                chat_id,
                account_id
            );
        }

        removed
    }

    /// Updates an existing account's ID.
    ///
    /// Finds the account by old ID and chat ID, then updates the account_id field.
    /// The last_balance is reset to `None` to trigger a fresh balance check.
    /// Automatically saves the updated account list to disk.
    ///
    /// # Arguments
    ///
    /// * `old_id` - The current account ID to find
    /// * `chat_id` - The Telegram chat ID of the user
    /// * `new_id` - The new account ID to set
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if the account was found and updated, or an error message.
    ///
    /// # Errors
    ///
    /// Returns `Err(String)` if the account is not found.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use near_balance_monitor::persistence::AccountPersistenceManager;
    /// # use teloxide::types::ChatId;
    /// let mut manager = AccountPersistenceManager::load("monitored_accounts.json");
    /// manager.update_account("old.near", ChatId(123456789), "new.near".to_string())?;
    /// # Ok::<(), String>(())
    /// ```
    pub fn update_account(
        &mut self,
        old_id: &str,
        chat_id: ChatId,
        new_id: String,
    ) -> Result<(), String> {
        if let Some(account) = self
            .accounts
            .iter_mut()
            .find(|a| a.account_id == old_id && a.chat_id == chat_id)
        {
            log::info!(
                "Account updated chat_id={} old={} new={}",
                chat_id,
                old_id,
                new_id
            );
            account.account_id = new_id;
            account.last_balance = None; // Reset to trigger new check
            self.save();
            Ok(())
        } else {
            log::debug!(
                "Account not found for update chat_id={} account={}",
                chat_id,
                old_id
            );
            Err(format!("Account {} not found", old_id))
        }
    }

    /// Updates the last known balance for a monitored account.
    ///
    /// This is called by the background monitoring loop when a balance change
    /// is detected. The update is only performed if the balance has actually changed.
    /// Automatically saves the updated account list to disk.
    ///
    /// # Arguments
    ///
    /// * `account_id` - The NEAR account ID to update
    /// * `chat_id` - The Telegram chat ID of the user
    /// * `balance` - The new balance in yoctoNEAR
    ///
    /// # Returns
    ///
    /// Returns `true` if the account was found and updated, `false` otherwise.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use near_balance_monitor::persistence::AccountPersistenceManager;
    /// # use teloxide::types::ChatId;
    /// let mut manager = AccountPersistenceManager::load("monitored_accounts.json");
    /// let updated = manager.update_balance("example.near", ChatId(123456789), 1500000000000000000000000);
    /// ```
    pub fn update_balance(&mut self, account_id: &str, chat_id: ChatId, balance: u128) -> bool {
        if let Some(account) = self
            .accounts
            .iter_mut()
            .find(|a| a.account_id == account_id && a.chat_id == chat_id)
        {
            // Only save if balance actually changed
            if account.last_balance != Some(balance) {
                log::debug!(
                    "Balance updated account={} chat_id={} balance={}",
                    account_id,
                    chat_id,
                    balance
                );
                account.last_balance = Some(balance);
                self.save();
            }
            true
        } else {
            log::warn!(
                "Account not found for balance update chat_id={} account={}",
                chat_id,
                account_id
            );
            false
        }
    }

    /// Returns all accounts being monitored by a specific user/chat.
    ///
    /// # Arguments
    ///
    /// * `chat_id` - The Telegram chat ID to filter by
    ///
    /// # Returns
    ///
    /// Returns a vector of references to `MonitoredAccount` objects for the user.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use near_balance_monitor::persistence::AccountPersistenceManager;
    /// # use teloxide::types::ChatId;
    /// let manager = AccountPersistenceManager::load("monitored_accounts.json");
    /// let accounts = manager.get_accounts_for_chat(ChatId(123456789));
    /// ```
    pub fn get_accounts_for_chat(&self, chat_id: ChatId) -> Vec<&MonitoredAccount> {
        self.accounts
            .iter()
            .filter(|a| a.chat_id == chat_id)
            .collect()
    }

    /// Returns a clone of all monitored accounts across all users.
    ///
    /// This is used by the background monitoring loop to get a snapshot
    /// of all accounts to check without holding the mutex lock.
    ///
    /// # Returns
    ///
    /// Returns a cloned vector of all `MonitoredAccount` objects.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use near_balance_monitor::persistence::AccountPersistenceManager;
    /// let manager = AccountPersistenceManager::load("monitored_accounts.json");
    /// let all_accounts = manager.get_all_accounts();
    /// ```
    pub fn get_all_accounts(&self) -> Vec<MonitoredAccount> {
        self.accounts.clone()
    }

    /// Saves the current list of accounts to the configured file path.
    ///
    /// Uses an atomic write pattern (write to temp file, then rename) to
    /// prevent data corruption during writes. Failures are logged but do
    /// not panic, allowing the bot to continue operating.
    ///
    /// # Panics
    ///
    /// This function does not panic. All errors are logged and handled gracefully.
    fn save(&self) {
        match serde_json::to_string_pretty(&self.accounts) {
            Ok(data) => {
                let temp_path = format!("{}.tmp", self.file_path);

                // Write to temp file first
                if let Err(e) = fs::write(&temp_path, data) {
                    log::error!("Failed to write temp file file={}: {}", temp_path, e);
                    return;
                }

                // Atomic rename on POSIX systems
                if let Err(e) = fs::rename(&temp_path, &self.file_path) {
                    log::error!("Failed to rename temp file to {} : {}", self.file_path, e);
                    // Try to clean up temp file
                    let _ = fs::remove_file(&temp_path);
                    return;
                }

                log::debug!(
                    "Saved {} monitored accounts to file={}",
                    self.accounts.len(),
                    self.file_path
                );
            }
            Err(e) => {
                log::error!("Failed to serialize monitored accounts: {}", e);
            }
        }
    }
}
