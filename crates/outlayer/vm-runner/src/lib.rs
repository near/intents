mod context;
mod error;
mod outcome;
mod runtime;

pub use defuse_outlayer_host as host;

pub use wasmtime;
pub use wasmtime_wasi;

pub use error::ExecutionError;
pub use outcome::{ExecutionDetails, ExecutionOutcome};
pub use runtime::{Context, VmRuntime};
pub use wasmtime::component::Component;
pub use wasmtime_wasi::p2::pipe::{MemoryInputPipe, MemoryOutputPipe};
