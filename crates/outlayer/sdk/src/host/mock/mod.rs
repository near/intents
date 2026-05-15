use std::cell::RefCell;

use defuse_outlayer_host::{Context, Host};
use defuse_outlayer_primitives::AppId;
use defuse_outlayer_signer::InMemorySigner;

thread_local! {
    pub(crate) static HOST: RefCell<Host<'static>> =
        RefCell::new(Host::new(
            Context {
                app_id: AppId::EXAMPLE,
            },
            // TODO
            InMemorySigner::from_seed(TEST_SEED),
        ));
}

const TEST_SEED: &[u8] = b"test";

// TODO: functions to set/modify mock
