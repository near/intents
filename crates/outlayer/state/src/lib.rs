pub use defuse_outlayer_worker_host::WorkerHost;

pub mod crypto;

#[derive(Debug, Default)]
pub struct HostState {
    worker: WorkerHost,
}
