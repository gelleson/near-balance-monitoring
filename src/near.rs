//! NEAR Protocol RPC client.
//!
//! This module provides a client for interacting with the NEAR Protocol RPC API
//! and NearBlocks API. It handles balance queries and transaction fetching.
//!
//! # Examples
//!
//! ```no_run
//! use near_balance_monitor::near::NearClient;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), String> {
//!     let client = NearClient::new();
//!     let balance = client.fetch_balance("example.near").await?;
//!     println!("Balance: {} yoctoNEAR", balance);
//!     Ok(())
//! }
//! ```

use std::time::Instant;
use serde::{Deserialize, Serialize};

/// NEAR RPC endpoint URL.
const NEAR_RPC_URL: &str = "https://h36uashbwvxlllkjfzzaxgfu-near-rpc.defuse.org";

/// Conversion factor from yoctoNEAR to NEAR.
/// 1 NEAR = 10^24 yoctoNEAR.
pub const YOCTO_NEAR: f64 = 1e24;

/// JSON-RPC request structure for NEAR RPC calls.
#[derive(Serialize)]
struct RpcRequest {
    jsonrpc: &'static str,
    id: &'static str,
    method: &'static str,
    params: serde_json::Value,
}

/// JSON-RPC response structure from NEAR RPC.
#[derive(Deserialize)]
struct RpcResponse {
    result: Option<AccountView>,
    error: Option<serde_json::Value>,
}

/// Account view returned by the NEAR RPC `view_account` method.
#[derive(Deserialize)]
struct AccountView {
    /// Account balance in yoctoNEAR as a string.
    amount: String,
}

/// Aggregated transaction actions data.
///
/// Contains summarized information about transaction actions,
/// particularly the total deposit amount.
#[derive(Deserialize, Debug, Clone)]
pub struct ActionsAgg {
    /// Total deposit amount in the transaction.
    pub deposit: f64,
}

/// NEAR blockchain transaction information.
///
/// Represents a transaction fetched from the NearBlocks API.
#[derive(Deserialize, Debug, Clone)]
pub struct Transaction {
    /// Transaction hash (unique identifier).
    #[serde(rename = "transaction_hash")]
    pub hash: String,
    /// Account that signed/initiated the transaction.
    #[serde(rename = "predecessor_account_id")]
    pub signer_id: String,
    /// Account that received the transaction.
    #[serde(rename = "receiver_account_id")]
    pub receiver_id: String,
    /// Block timestamp in nanoseconds (as a string).
    pub block_timestamp: String,
    /// Aggregated actions data (deposits, etc.).
    pub actions_agg: ActionsAgg,
}

/// Response structure from NearBlocks API transaction endpoint.
#[derive(Deserialize)]
struct NearBlocksResponse {
    /// List of transactions.
    txns: Vec<Transaction>,
}

/// Client for interacting with the NEAR Protocol RPC and NearBlocks API.
///
/// This client provides methods to:
/// - Fetch account balances from NEAR RPC
/// - Fetch transaction history from NearBlocks API
///
/// # Examples
///
/// ```no_run
/// # use near_balance_monitor::near::NearClient;
/// # #[tokio::main]
/// # async fn main() -> Result<(), String> {
/// let client = NearClient::new();
/// let balance = client.fetch_balance("example.near").await?;
/// let transactions = client.fetch_transactions("example.near").await?;
/// # Ok(())
/// # }
/// ```
pub struct NearClient {
    /// Internal HTTP client for making requests.
    client: reqwest::Client,
}

impl NearClient {
    /// Creates a new `NearClient` instance.
    ///
    /// Initializes a default `reqwest` HTTP client for making RPC requests.
    ///
    /// # Examples
    ///
    /// ```
    /// use near_balance_monitor::near::NearClient;
    ///
    /// let client = NearClient::new();
    /// ```
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }

    /// Fetches the last 10 unique transactions for a NEAR account.
    ///
    /// Queries the NearBlocks API for transaction history, deduplicates by hash,
    /// sorts by timestamp (descending), and returns up to 10 transactions.
    ///
    /// # Arguments
    ///
    /// * `account_id` - The NEAR account ID (e.g., "example.near")
    ///
    /// # Returns
    ///
    /// Returns `Ok(Vec<Transaction>)` with up to 10 transactions, or an error message.
    ///
    /// # Errors
    ///
    /// Returns `Err(String)` if:
    /// - The HTTP request fails
    /// - The response cannot be parsed
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use near_balance_monitor::near::NearClient;
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), String> {
    /// let client = NearClient::new();
    /// let transactions = client.fetch_transactions("example.near").await?;
    /// for tx in transactions {
    ///     println!("Transaction: {}", tx.hash);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn fetch_transactions(&self, account_id: &str) -> Result<Vec<Transaction>, String> {
        log::debug!("Fetching transactions account={} limit=25", account_id);
        let url = format!("https://api.nearblocks.io/v1/account/{}/txns?limit=25", account_id);

        let start = Instant::now();
        let response = self.client
            .get(&url)
            .send()
            .await
            .map_err(|e| {
                log::error!("NearBlocks API request failed account={}: {}", account_id, e);
                format!("HTTP request failed: {e}")
            })?;

        log::debug!("NearBlocks API responded account={} duration_ms={} status={:?}", account_id, start.elapsed().as_millis(), response.status());

        let near_blocks_response: NearBlocksResponse = response
            .json()
            .await
            .map_err(|e| {
                log::error!("Failed to parse NearBlocks response account={}: {}", account_id, e);
                format!("Failed to parse response: {e}")
            })?;

        let mut txs = Vec::new();
        let mut seen_hashes = std::collections::HashSet::new();

        for tx in near_blocks_response.txns {
            if seen_hashes.insert(tx.hash.clone()) {
                txs.push(tx);
            }
        }

        log::debug!("Deduplicated transactions account={} unique_count={}", account_id, txs.len());

        // Sort by timestamp descending
        txs.sort_by(|a, b| b.block_timestamp.cmp(&a.block_timestamp));
        txs.truncate(10);

        log::info!("Successfully fetched transactions account={} count={}", account_id, txs.len());

        Ok(txs)
    }

    /// Fetches the current balance of a NEAR account in yoctoNEAR.
    ///
    /// Queries the NEAR RPC `view_account` method with finality set to "final"
    /// to get the most recent confirmed balance.
    ///
    /// # Arguments
    ///
    /// * `account_id` - The NEAR account ID (e.g., "example.near")
    ///
    /// # Returns
    ///
    /// Returns `Ok(u128)` with the balance in yoctoNEAR (1 NEAR = 10^24 yoctoNEAR),
    /// or an error message.
    ///
    /// # Errors
    ///
    /// Returns `Err(String)` if:
    /// - The HTTP request fails
    /// - The RPC returns an error (e.g., account not found)
    /// - The response cannot be parsed
    /// - The balance amount cannot be parsed as u128
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use near_balance_monitor::near::NearClient;
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), String> {
    /// let client = NearClient::new();
    /// let balance = client.fetch_balance("example.near").await?;
    /// println!("Balance: {} yoctoNEAR", balance);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn fetch_balance(&self, account_id: &str) -> Result<u128, String> {
        log::debug!("Fetching balance account={} endpoint={}", account_id, NEAR_RPC_URL);

        let request = RpcRequest {
            jsonrpc: "2.0",
            id: "1",
            method: "query",
            params: serde_json::json!({
                "request_type": "view_account",
                "finality": "final",
                "account_id": account_id,
            }),
        };

        let start = Instant::now();
        let response = self.client
            .post(NEAR_RPC_URL)
            .json(&request)
            .send()
            .await
            .map_err(|e| format!("HTTP request failed: {e}"))?;

        log::debug!("RPC request completed account={} duration_ms={} status={:?}", account_id, start.elapsed().as_millis(), response.status());

        let rpc_response: RpcResponse = response
            .json()
            .await
            .map_err(|e| {
                log::error!("Failed to parse RPC response account={}: {}", account_id, e);
                format!("Failed to parse response: {e}")
            })?;

        if let Some(error) = rpc_response.error {
            log::error!("RPC error account={}: {:?}", account_id, error);
            return Err(format!("RPC error: {error}"));
        }

        let result = rpc_response.result.ok_or_else(|| {
            log::error!("No result in RPC response account={}", account_id);
            "No result in response"
        })?;

        let balance = result
            .amount
            .parse::<u128>()
            .map_err(|e| {
                log::error!("Failed to parse balance amount account={}: {}", account_id, e);
                format!("Failed to parse amount: {e}")
            })?;

        log::debug!("Successfully fetched balance account={} balance_yocto={}", account_id, balance);

        Ok(balance)
    }
}
