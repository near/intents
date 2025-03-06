use async_trait::async_trait;
use near_sdk::AccountId;
use std::error::Error;

pub mod config;
pub mod runesdex;
pub mod solver;
pub mod types;

/// Main entry point for the RunesDex NEAR Intents integration
pub struct RunesDexSolver {
    pub config: config::Config,
    pub runesdex_client: runesdex::RunesDexClient,
}

impl RunesDexSolver {
    /// Create a new RunesDex solver instance
    pub fn new(config: config::Config) -> Self {
        let runesdex_client = runesdex::RunesDexClient::new(&config.runesdex_api_key);
        Self {
            config,
            runesdex_client,
        }
    }

    /// Initialize the solver with default configuration
    pub async fn init_default() -> Result<Self, Box<dyn Error>> {
        let config = config::Config::from_env()?;
        Ok(Self::new(config))
    }

    /// Start the solver and connect to the NEAR Intents protocol
    pub async fn start(&self) -> Result<(), Box<dyn Error>> {
        log::info!("Starting RunesDex solver for NEAR Intents");
        
        // Connect to the solver bus and start processing intents
        let solver = solver::NearIntentsSolver::new(
            self.config.near_account_id.clone(),
            self.config.near_private_key.clone(),
            self.config.solver_bus_url.clone(),
            self.runesdex_client.clone(),
        );
        
        solver.start().await?;
        
        Ok(())
    }
}

#[async_trait]
pub trait Solver {
    async fn process_intent(&self, intent: &types::SwapIntent) -> Result<types::SwapQuote, Box<dyn Error>>;
    async fn execute_swap(&self, quote: &types::SwapQuote) -> Result<String, Box<dyn Error>>;
} 