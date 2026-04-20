use defuse_outlayer_worker_host::WorkerHost;

thread_local! {
    pub(super) static HOST: std::cell::RefCell<WorkerHost>
        = std::cell::RefCell::new(WorkerHost);
}

// TODO
// pub fn set(host: WorkerHost) {
//     HOST.set(host);
// }
