use crate::types::{RunesDexSwapResponse, SwapIntent};
use reqwest::{Client, header};
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::time::{SystemTime, UNIX_EPOCH};

const RUNESDEX_API_BASE_URL: &str = "https://app.runesdex.com/v1";

#[derive(Debug, Clone)]
pub struct RunesDexClient {
    client: Client,
    api_key: String,
}

#[derive(Debug, Serialize)]
struct RunesDexSwapRequest {
    bid_asset: String,
    bid_amount: String,
    bid_address: String,
    bid_address_pubkey: Option<String>,
    ask_address: String,
    ask_amount: String,
    fee_address: String,
    fee_address_pubkey: Option<String>,
    rate: String,
    slippage: f64,
}

impl RunesDexClient {
    /// Create a new RunesDex client
    pub fn new(api_key: &str) -> Self {
        let mut headers = header::HeaderMap::new();
        headers.insert(
            "x-api-key",
            header::HeaderValue::from_str(api_key).unwrap(),
        );
        
        let client = Client::builder()
            .default_headers(headers)
            .build()
            .unwrap();
            
        Self {
            client,
            api_key: api_key.to_string(),
        }
    }
    
    /// Get a trading pair by base and quote assets
    pub async fn get_trading_pair(&self, base: &str, quote: &str) -> Result<serde_json::Value, Box<dyn Error>> {
        let url = format!("{}/pairs/{}-{}", RUNESDEX_API_BASE_URL, base, quote);
        
        let response = self.client.get(&url)
            .send()
            .await?
            .json::<serde_json::Value>()
            .await?;
            
        Ok(response)
    }
    
    /// Calculate a swap based on the given intent
    pub async fn calculate_swap(&self, intent: &SwapIntent) -> Result<(String, String), Box<dyn Error>> {
        // Extract base and quote from the defuse asset identifiers
        let base = extract_token_name(&intent.defuse_asset_identifier_in)?;
        let quote = extract_token_name(&intent.defuse_asset_identifier_out)?;
        
        // Get the current trading pair info
        let pair = self.get_trading_pair(base, quote).await?;
        
        // Calculate swap amounts based on the intent
        let url = format!("{}/pairs/{}-{}/calculate", RUNESDEX_API_BASE_URL, base, quote);
        
        // Add X-Trade-Session-Id header for this request to maintain session
        let mut headers = header::HeaderMap::new();
        headers.insert(
            "X-Trade-Session-Id",
            header::HeaderValue::from_str(&generate_session_id())?,
        );
        
        let response = self.client.get(&url)
            .headers(headers)
            .send()
            .await?
            .json::<serde_json::Value>()
            .await?;
            
        let base_amount = response["base"].as_str()
            .ok_or("Missing base amount in response")?
            .to_string();
            
        let quote_amount = response["quote"].as_str()
            .ok_or("Missing quote amount in response")?
            .to_string();
            
        Ok((base_amount, quote_amount))
    }
    
    /// Execute a swap through the RunesDex API
    pub async fn execute_swap(
        &self,
        base: &str,
        quote: &str,
        bid_asset: &str,
        bid_amount: &str,
        bid_address: &str,
        ask_address: &str,
        ask_amount: &str,
        session_id: &str,
    ) -> Result<RunesDexSwapResponse, Box<dyn Error>> {
        let url = format!("{}/pairs/{}-{}/swap", RUNESDEX_API_BASE_URL, base, quote);
        
        // Use the same fee address as the bid address for simplicity
        let fee_address = bid_address;
        
        // Use a small slippage tolerance (0.5%)
        let slippage = 0.5;
        
        // Calculate rate based on ask and bid amounts
        let bid_amount_f = bid_amount.parse::<f64>()?;
        let ask_amount_f = ask_amount.parse::<f64>()?;
        let rate = if bid_amount_f > 0.0 {
            (ask_amount_f / bid_amount_f).to_string()
        } else {
            return Err("Bid amount must be greater than zero".into());
        };
        
        let swap_request = RunesDexSwapRequest {
            bid_asset: bid_asset.to_string(),
            bid_amount: bid_amount.to_string(),
            bid_address: bid_address.to_string(),
            bid_address_pubkey: None, // Optional in the API
            ask_address: ask_address.to_string(),
            ask_amount: ask_amount.to_string(),
            fee_address: fee_address.to_string(),
            fee_address_pubkey: None, // Optional in the API
            rate,
            slippage,
        };
        
        // Add X-Trade-Session-Id header to maintain the same session
        let mut headers = header::HeaderMap::new();
        headers.insert(
            "X-Trade-Session-Id",
            header::HeaderValue::from_str(session_id)?,
        );
        
        let response = self.client.post(&url)
            .headers(headers)
            .json(&swap_request)
            .send()
            .await?
            .json::<RunesDexSwapResponse>()
            .await?;
            
        Ok(response)
    }
    
    /// Submit a signed transaction to the RunesDex API
    pub async fn submit_transaction(&self, psbt: &str, request_id: &str) -> Result<String, Box<dyn Error>> {
        let url = format!("{}/publish-tx", RUNESDEX_API_BASE_URL);
        
        #[derive(Serialize)]
        struct SubmitTxRequest {
            psbt: String,
            request_id: String,
        }
        
        let submit_request = SubmitTxRequest {
            psbt: psbt.to_string(),
            request_id: request_id.to_string(),
        };
        
        let response = self.client.post(&url)
            .json(&submit_request)
            .send()
            .await?
            .json::<serde_json::Value>()
            .await?;
            
        let tx_id = response["tx_id"].as_str()
            .ok_or("Missing tx_id in response")?
            .to_string();
            
        Ok(tx_id)
    }
}

/// Extract token name from a defuse asset identifier
fn extract_token_name(defuse_asset_id: &str) -> Result<&str, Box<dyn Error>> {
    // Format is typically "nep141:token.near" or similar
    let parts: Vec<&str> = defuse_asset_id.split(':').collect();
    if parts.len() != 2 {
        return Err(format!("Invalid defuse asset identifier: {}", defuse_asset_id).into());
    }
    
    Ok(parts[1])
}

/// Generate a unique session ID for RunesDex API
fn generate_session_id() -> String {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();
        
    format!("intent-session-{}", timestamp)
} 