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
pub struct ActionsAgg {
    pub deposit: f64,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Transaction {
    #[serde(rename = "transaction_hash")]
    pub hash: String,
    #[serde(rename = "predecessor_account_id")]
    pub signer_id: String,
    #[serde(rename = "receiver_account_id")]
    pub receiver_id: String,
    pub actions_agg: ActionsAgg,
}

#[derive(Deserialize)]
struct NearBlocksResponse {
    txns: Vec<Transaction>,
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
        let url = format!("https://api.nearblocks.io/v1/account/{}/txns?limit=25", account_id);
        
        let response = self.client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("HTTP request failed: {e}"))?;

        let near_blocks_response: NearBlocksResponse = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {e}"))?;

        let mut txs = Vec::new();
        let mut seen_hashes = std::collections::HashSet::new();

        for tx in near_blocks_response.txns {
            if seen_hashes.insert(tx.hash.clone()) {
                txs.push(tx);
                if txs.len() >= 10 {
                    break;
                }
            }
        }

        Ok(txs)
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
