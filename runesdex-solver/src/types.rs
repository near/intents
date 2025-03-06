use near_sdk::json_types::U128;
use near_sdk::{AccountId, Balance};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Swap intent received from NEAR Intents
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwapIntent {
    /// ID of the quote request
    pub quote_id: String,
    
    /// Asset identifier for the input token
    pub defuse_asset_identifier_in: String,
    
    /// Asset identifier for the output token
    pub defuse_asset_identifier_out: String,
    
    /// Exact amount of input token (if specified)
    pub exact_amount_in: Option<String>,
    
    /// Exact amount of output token (if specified)
    pub exact_amount_out: Option<String>,
    
    /// Minimum deadline in milliseconds
    pub min_deadline_ms: u64,
}

/// Swap quote response to be sent back to the solver bus
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwapQuote {
    /// ID of the quote
    pub quote_id: String,
    
    /// Output details of the quote
    pub quote_output: QuoteOutput,
    
    /// Signed data for the quote
    pub signed_data: SignedIntentData,
}

/// Output details for a quote
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuoteOutput {
    /// Amount of input token (if exact_amount_out was specified)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub amount_in: Option<String>,
    
    /// Amount of output token (if exact_amount_in was specified)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub amount_out: Option<String>,
}

/// Signed intent data for the solver
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignedIntentData {
    /// Signature standard (nep413, eip712, etc.)
    pub standard: String,
    
    /// Payload for the signature
    pub payload: SignaturePayload,
    
    /// Signature of the data
    pub signature: String,
    
    /// Public key used for signing
    pub public_key: String,
}

/// Payload for the signature
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignaturePayload {
    /// JSON message containing the intent
    pub message: String,
    
    /// Nonce to prevent replay attacks
    pub nonce: String,
    
    /// Recipient of the intent (usually the defuse contract)
    pub recipient: String,
    
    /// Optional callback URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub callback_url: Option<String>,
}

/// Token diff intent message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentMessage {
    /// ID of the signer account
    pub signer_id: String,
    
    /// Deadline for the intent
    pub deadline: IntentDeadline,
    
    /// Vector of intents to execute
    pub intents: Vec<Intent>,
}

/// Deadline for an intent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentDeadline {
    /// Unix timestamp in seconds
    pub timestamp: u64,
}

/// Intent data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Intent {
    /// Type of intent (usually "token_diff")
    pub intent: String,
    
    /// Token differences
    pub diff: HashMap<String, String>,
}

/// RunesDex API response for a swap
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunesDexSwapResponse {
    /// Request ID from RunesDex
    pub request_id: String,
    
    /// Pair ID in RunesDex
    pub pair_id: u64,
    
    /// Trading pair name
    pub pair: String,
    
    /// Base address for the swap
    pub base_address: String,
    
    /// Quote address for the swap
    pub quote_address: String,
    
    /// Base amount for the swap
    pub base_amount: String,
    
    /// Quote amount for the swap
    pub quote_amount: String,
    
    /// BTC transaction fee in satoshis
    pub btc_tx_fee: u64,
    
    /// Partially Signed Bitcoin Transaction
    pub psbt: String,
    
    /// Raw transaction data
    pub raw_tx: String,
    
    /// Indices of base inputs
    pub base_inputs: Vec<u64>,
    
    /// Indices of quote inputs
    pub quote_inputs: Vec<u64>,
} 