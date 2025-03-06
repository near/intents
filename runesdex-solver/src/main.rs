use runesdex_solver::RunesDexSolver;
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Initialize logger
    env_logger::init_from_env(env_logger::Env::default().default_filter_or("info"));
    
    // Initialize the RunesDex solver with configuration from environment variables
    let solver = RunesDexSolver::init_default().await?;
    
    // Start the solver
    solver.start().await?;
    
    Ok(())
} 