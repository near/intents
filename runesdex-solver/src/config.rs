use near_sdk::AccountId;
use std::env;
use std::error::Error;
use thiserror::Error;

#[derive(Debug, Clone)]
pub struct Config {
    /// API key for RunesDex
    pub runesdex_api_key: String,
    
    /// NEAR account ID used for signing transactions
    pub near_account_id: AccountId,
    
    /// NEAR private key used for signing transactions
    pub near_private_key: String,
    
    /// URL of the solver bus
    pub solver_bus_url: String,
}

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Missing environment variable: {0}")]
    MissingEnv(String),
    
    #[error("Invalid NEAR account ID: {0}")]
    InvalidNearAccountId(String),
    
    #[error("Environment variable error: {0}")]
    EnvError(#[from] env::VarError),
}

impl Config {
    /// Create a new configuration from environment variables
    pub fn from_env() -> Result<Self, Box<dyn Error>> {
        let runesdex_api_key = env::var("RUNESDEX_API_KEY")
            .map_err(|_| ConfigError::MissingEnv("RUNESDEX_API_KEY".to_string()))?;
            
        let near_account_id_str = env::var("NEAR_ACCOUNT_ID")
            .map_err(|_| ConfigError::MissingEnv("NEAR_ACCOUNT_ID".to_string()))?;
            
        let near_account_id: AccountId = near_account_id_str
            .parse()
            .map_err(|_| ConfigError::InvalidNearAccountId(near_account_id_str))?;
            
        let near_private_key = env::var("NEAR_PRIVATE_KEY")
            .map_err(|_| ConfigError::MissingEnv("NEAR_PRIVATE_KEY".to_string()))?;
            
        let solver_bus_url = env::var("SOLVER_BUS_URL")
            .unwrap_or_else(|_| "wss://solver-relay-v2.chaindefuser.com/ws".to_string());
            
        Ok(Self {
            runesdex_api_key,
            near_account_id,
            near_private_key,
            solver_bus_url,
        })
    }
} 