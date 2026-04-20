use wasmtime::Trap;

#[derive(thiserror::Error, Debug)]
pub enum ExecutionError {
    #[error("component exited with code {code}\n{stderr}")]
    NonZeroExit { code: i32, stderr: String },

    #[error("wasm trap: {code}\n{stderr}")]
    Trap { code: Trap, stderr: String },

    #[error("component trapped: {source}\n{stderr}")]
    Unknown {
        #[source]
        source: anyhow::Error,
        stderr: String,
    },
}

#[derive(thiserror::Error, Debug)]
pub enum VmError {
    #[error("failed to serialize input: {0}")]
    InvalidInput(#[from] serde_json::Error),

    #[error("setup failed: {0}")]
    Setup(#[from] anyhow::Error),

    #[error(transparent)]
    Execution(#[from] ExecutionError),
}
