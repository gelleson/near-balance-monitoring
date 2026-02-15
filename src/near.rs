use serde::{Deserialize, Serialize};

const NEAR_RPC_URL: &str = "https://h36uashbwvxlllkjfzzaxgfu-near-rpc.defuse.org";
pub const YOCTO_NEAR: f64 = 1e24;

#[derive(Serialize)]
struct RpcRequest {
    jsonrpc: &'static str,
    id: &'static str,
    method: &'static str,
    params: serde_json::Value,
}

#[derive(Deserialize)]
struct RpcResponse {
    result: Option<AccountView>,
    error: Option<serde_json::Value>,
}

#[derive(Deserialize)]
struct AccountView {
    amount: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Transaction {
    pub hash: String,
    pub signer_id: String,
    pub receiver_id: String,
    pub block_timestamp: String,
}

#[derive(Deserialize)]
struct FastNearResponse {
    transactions: Vec<Transaction>,
}

/// Client for interacting with the NEAR Protocol RPC.
pub struct NearClient {
    client: reqwest::Client,
}

impl NearClient {
    /// Creates a new `NearClient` with a default `reqwest` client.
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }

    /// Fetches the last 10 transactions for a NEAR account.
    pub async fn fetch_transactions(&self, account_id: &str) -> Result<Vec<Transaction>, String> {
        let url = format!("https://api.fastnear.com/v1/account/{}/txns?limit=10", account_id);
        
        let response = self.client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("HTTP request failed: {e}"))?;

        let fast_near_response: FastNearResponse = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {e}"))?;

        Ok(fast_near_response.transactions)
    }

    /// Fetches the current balance of a NEAR account in yoctoNEAR.
    /// 
    /// # Arguments
    /// * `account_id` - The NEAR account ID (e.g., "example.near").
    pub async fn fetch_balance(&self, account_id: &str) -> Result<u128, String> {
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

        let response = self.client
            .post(NEAR_RPC_URL)
            .json(&request)
            .send()
            .await
            .map_err(|e| format!("HTTP request failed: {e}"))?;

        let rpc_response: RpcResponse = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {e}"))?;

        if let Some(error) = rpc_response.error {
            return Err(format!("RPC error: {error}"));
        }

        let result = rpc_response.result.ok_or("No result in response")?;
        result
            .amount
            .parse::<u128>()
            .map_err(|e| format!("Failed to parse amount: {e}"))
    }
}
