use defuse_outlayer_worker_host::WorkerHost;

pub mod crypto;

pub struct HostState {
    worker: WorkerHost,
}

impl HostState {
    pub const fn new(worker: WorkerHost) -> Self {
        Self { worker }
    }
}
