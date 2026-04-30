mod error;
mod host;
mod outcome;
mod runtime;

pub use error::ExecutionError;
pub use outcome::ExecutionOutcome;
pub use runtime::{Context, DEFAULT_FUEL_LIMIT, DEFAULT_MEMORY_LIMIT, VmRuntime, VmRuntimeBuilder};
pub use wasmtime::component::Component;
pub use wasmtime_wasi::p2::pipe::{MemoryInputPipe, MemoryOutputPipe};
