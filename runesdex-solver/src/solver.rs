use crate::{
    runesdex::RunesDexClient,
    types::{
        Intent, IntentDeadline, IntentMessage, QuoteOutput, SignaturePayload, SignedIntentData,
        SwapIntent, SwapQuote,
    },
    Solver,
};
use async_trait::async_trait;
use near_crypto::{InMemorySigner, KeyType, Signer};
use near_sdk::{AccountId, CryptoHash};
use rand::Rng;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::error::Error;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::net::TcpStream;
use tokio::sync::Mutex;
use tokio_tungstenite::{
    connect_async,
    tungstenite::{self, Message},
    MaybeTlsStream, WebSocketStream,
};

// NEAR Intents contract address
const INTENTS_CONTRACT: &str = "intents.near";

/// Message received from the solver bus websocket
#[derive(Debug, Deserialize)]
struct SolverBusMessage {
    jsonrpc: String,
    method: String,
    params: SolverBusParams,
}

/// Parameters for a solver bus message
#[derive(Debug, Deserialize)]
struct SolverBusParams {
    subscription: String,
    #[serde(flatten)]
    intent: Option<SwapIntent>,
}

/// JSON-RPC request for the solver bus
#[derive(Debug, Serialize)]
struct JsonRpcRequest {
    jsonrpc: String,
    id: u64,
    method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    params: Option<Vec<String>>,
}

/// JSON-RPC response from the solver bus
#[derive(Debug, Deserialize)]
struct JsonRpcResponse {
    jsonrpc: String,
    id: u64,
    result: String,
}

/// Implementation of a NEAR Intents solver that connects to the solver bus
pub struct NearIntentsSolver {
    /// Account ID used for signing transactions
    account_id: AccountId,
    
    /// Signer for NEAR transactions
    signer: InMemorySigner,
    
    /// URL of the solver bus
    solver_bus_url: String,
    
    /// Client for RunesDex API
    runesdex_client: RunesDexClient,
    
    /// Active WebSocket connection to the solver bus
    ws_stream: Arc<Mutex<Option<WebSocketStream<MaybeTlsStream<TcpStream>>>>>,
}

impl NearIntentsSolver {
    /// Create a new NEAR Intents solver
    pub fn new(
        account_id: AccountId,
        private_key: String,
        solver_bus_url: String,
        runesdex_client: RunesDexClient,
    ) -> Self {
        let signer = if private_key.starts_with("ed25519:") {
            let key = private_key.trim_start_matches("ed25519:");
            InMemorySigner::from_seed(account_id.clone(), KeyType::ED25519, key)
        } else {
            InMemorySigner::from_secret_key(account_id.clone(), private_key.parse().unwrap())
        };
        
        Self {
            account_id,
            signer,
            solver_bus_url,
            runesdex_client,
            ws_stream: Arc::new(Mutex::new(None)),
        }
    }
    
    /// Start the solver and connect to the solver bus
    pub async fn start(&self) -> Result<(), Box<dyn Error>> {
        log::info!("Connecting to solver bus at: {}", self.solver_bus_url);
        
        // Connect to the solver bus
        let (ws_stream, _) = connect_async(&self.solver_bus_url).await?;
        log::info!("Connected to solver bus");
        
        // Store the WebSocket stream
        {
            let mut stream_guard = self.ws_stream.lock().await;
            *stream_guard = Some(ws_stream);
        }
        
        // Subscribe to quote events
        self.subscribe_to_quotes().await?;
        
        // Main event loop
        self.event_loop().await?;
        
        Ok(())
    }
    
    /// Subscribe to quote events on the solver bus
    async fn subscribe_to_quotes(&self) -> Result<String, Box<dyn Error>> {
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: 1,
            method: "subscribe".to_string(),
            params: Some(vec!["quote".to_string()]),
        };
        
        let request_json = serde_json::to_string(&request)?;
        log::debug!("Sending subscription request: {}", request_json);
        
        // Send subscription request
        {
            let mut stream_guard = self.ws_stream.lock().await;
            if let Some(stream) = &mut *stream_guard {
                stream.send(Message::Text(request_json)).await?;
                
                // Wait for response
                let msg = stream.next_message().await?;
                let response: JsonRpcResponse = serde_json::from_str(&msg.to_string()?)?;
                
                log::info!("Subscribed to quotes, subscription ID: {}", response.result);
                return Ok(response.result);
            }
        }
        
        Err("WebSocket connection not established".into())
    }
    
    /// Main event loop for processing messages from the solver bus
    async fn event_loop(&self) -> Result<(), Box<dyn Error>> {
        log::info!("Starting event loop");
        
        loop {
            // Wait for messages from the solver bus
            let msg = {
                let mut stream_guard = self.ws_stream.lock().await;
                if let Some(stream) = &mut *stream_guard {
                    match stream.next_message().await {
                        Ok(msg) => msg,
                        Err(e) => {
                            log::error!("Error receiving message: {}", e);
                            // Attempt to reconnect
                            drop(stream_guard);
                            self.reconnect().await?;
                            continue;
                        }
                    }
                } else {
                    return Err("WebSocket connection not established".into());
                }
            };
            
            // Process the message
            match msg {
                Message::Text(text) => {
                    log::debug!("Received message: {}", text);
                    
                    // Parse the message as a solver bus message
                    match serde_json::from_str::<SolverBusMessage>(&text) {
                        Ok(solver_msg) => {
                            if solver_msg.method == "subscribe" {
                                if let Some(intent) = solver_msg.params.intent {
                                    log::info!("Received swap intent: {:?}", intent);
                                    
                                    // Process the intent and send a quote
                                    match self.process_intent(&intent).await {
                                        Ok(quote) => {
                                            log::info!("Generated quote: {:?}", quote);
                                            self.send_quote_response(&quote).await?;
                                        }
                                        Err(e) => {
                                            log::error!("Error processing intent: {}", e);
                                        }
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            log::error!("Error parsing message: {}", e);
                        }
                    }
                }
                Message::Close(_) => {
                    log::info!("WebSocket connection closed");
                    // Attempt to reconnect
                    self.reconnect().await?;
                }
                _ => {
                    // Ignore other message types
                }
            }
        }
    }
    
    /// Reconnect to the solver bus
    async fn reconnect(&self) -> Result<(), Box<dyn Error>> {
        log::info!("Reconnecting to solver bus...");
        
        // Connect to the solver bus
        let (ws_stream, _) = connect_async(&self.solver_bus_url).await?;
        log::info!("Reconnected to solver bus");
        
        // Store the WebSocket stream
        {
            let mut stream_guard = self.ws_stream.lock().await;
            *stream_guard = Some(ws_stream);
        }
        
        // Resubscribe to quote events
        self.subscribe_to_quotes().await?;
        
        Ok(())
    }
    
    /// Send a quote response to the solver bus
    async fn send_quote_response(&self, quote: &SwapQuote) -> Result<(), Box<dyn Error>> {
        let request = json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "quote_response",
            "params": [quote]
        });
        
        let request_json = serde_json::to_string(&request)?;
        log::debug!("Sending quote response: {}", request_json);
        
        // Send quote response
        {
            let mut stream_guard = self.ws_stream.lock().await;
            if let Some(stream) = &mut *stream_guard {
                stream.send(Message::Text(request_json)).await?;
                
                // Wait for response
                let msg = stream.next_message().await?;
                log::debug!("Quote response reply: {}", msg);
                
                return Ok(());
            }
        }
        
        Err("WebSocket connection not established".into())
    }
    
    /// Generate a random nonce for intent signing
    fn generate_nonce(&self) -> String {
        let mut rng = rand::thread_rng();
        let nonce: [u8; 32] = rng.gen();
        base64::encode(nonce)
    }
    
    /// Sign the intent hash using the NEAR signer
    fn sign_intent_hash(&self, intent_hash: &[u8]) -> (String, String) {
        let signature = self.signer.sign(intent_hash);
        let signature_base58 = format!("ed25519:{}", signature.to_string());
        let public_key_base58 = format!("ed25519:{}", self.signer.public_key.to_string());
        
        (signature_base58, public_key_base58)
    }
}

#[async_trait]
impl Solver for NearIntentsSolver {
    /// Process a swap intent and generate a quote
    async fn process_intent(&self, intent: &SwapIntent) -> Result<SwapQuote, Box<dyn Error>> {
        log::info!("Processing swap intent: {:?}", intent);
        
        // Calculate swap amounts using RunesDex API
        let (base_amount, quote_amount) = self.runesdex_client.calculate_swap(intent).await?;
        
        // Determine which amount was requested and which one to return in the quote
        let quote_output = if intent.exact_amount_in.is_some() {
            QuoteOutput {
                amount_in: None,
                amount_out: Some(quote_amount.clone()),
            }
        } else {
            QuoteOutput {
                amount_in: Some(base_amount.clone()),
                amount_out: None,
            }
        };
        
        // Create the intent message with token differences
        let in_token = &intent.defuse_asset_identifier_in;
        let out_token = &intent.defuse_asset_identifier_out;
        
        // Create a map of token diffs
        let mut diff = HashMap::new();
        if intent.exact_amount_in.is_some() {
            // If the user specifies exact input, they're sending tokens and receiving something back
            diff.insert(in_token.clone(), intent.exact_amount_in.clone().unwrap());
            diff.insert(out_token.clone(), format!("-{}", quote_amount));
        } else {
            // If the user specifies exact output, they're receiving exact tokens and sending something
            diff.insert(in_token.clone(), base_amount);
            diff.insert(out_token.clone(), format!("-{}", intent.exact_amount_out.clone().unwrap()));
        }
        
        // Create the intent array with a token_diff intent
        let intents = vec![Intent {
            intent: "token_diff".to_string(),
            diff,
        }];
        
        // Set a deadline 10 minutes in the future
        let deadline = IntentDeadline {
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs() + 600, // 10 minutes
        };
        
        // Create the intent message
        let intent_message = IntentMessage {
            signer_id: self.account_id.to_string(),
            deadline,
            intents,
        };
        
        // Serialize the intent message to JSON
        let message_json = serde_json::to_string(&intent_message)?;
        
        // Generate a nonce for this intent
        let nonce = self.generate_nonce();
        
        // Create the signature payload
        let payload = SignaturePayload {
            message: message_json.clone(),
            nonce: nonce.clone(),
            recipient: INTENTS_CONTRACT.to_string(),
            callback_url: None,
        };
        
        // TODO: Properly serialize and hash the intent for signing
        // For now, we'll use a simplified approach
        let intent_hash = format!("{}{}{}", message_json, nonce, INTENTS_CONTRACT).as_bytes().to_vec();
        
        // Sign the intent hash
        let (signature, public_key) = self.sign_intent_hash(&intent_hash);
        
        // Create the signed intent data
        let signed_data = SignedIntentData {
            standard: "nep413".to_string(),
            payload,
            signature,
            public_key,
        };
        
        // Create the complete swap quote
        let swap_quote = SwapQuote {
            quote_id: intent.quote_id.clone(),
            quote_output,
            signed_data,
        };
        
        Ok(swap_quote)
    }
    
    /// Execute a swap based on the given quote
    async fn execute_swap(&self, quote: &SwapQuote) -> Result<String, Box<dyn Error>> {
        log::info!("Executing swap for quote: {:?}", quote);
        
        // TODO: Implement the actual swap execution
        // This would involve:
        // 1. Decoding the intent from the quote
        // 2. Calling the RunesDex API to execute the swap
        // 3. Submitting the transaction to the blockchain
        // 4. Returning the transaction ID
        
        // For now, we'll just return a placeholder
        Ok("swap_executed".to_string())
    }
}

trait WebSocketStreamExt {
    async fn next_message(&mut self) -> tungstenite::Result<Message>;
}

impl WebSocketStreamExt for WebSocketStream<MaybeTlsStream<TcpStream>> {
    async fn next_message(&mut self) -> tungstenite::Result<Message> {
        tokio::select! {
            msg = self.next() => {
                match msg {
                    Some(msg) => msg,
                    None => Err(tungstenite::Error::ConnectionClosed),
                }
            }
            _ = tokio::time::sleep(Duration::from_secs(30)) => {
                // Send a ping to keep the connection alive
                self.send(Message::Ping(vec![])).await?;
                // Wait for the next message after sending the ping
                match self.next().await {
                    Some(msg) => msg,
                    None => Err(tungstenite::Error::ConnectionClosed),
                }
            }
        }
    }
} 