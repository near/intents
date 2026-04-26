use std::cell::RefCell;

use defuse_outlayer_crypto::signer::InMemorySigner;
use defuse_outlayer_host::crypto::Host;
use defuse_outlayer_primitives::{AccountIdRef, AppId};

thread_local! {
    pub(crate) static HOST: RefCell<Host> =
        RefCell::new(Host::new(
            AppId::Near(AccountIdRef::new_or_panic("test.near").into()),
            InMemorySigner::from_seed(&[]), // TODO
        ));
}

pub fn set(host: Host) {
    HOST.set(host);
}

pub fn with<R>(f: impl FnOnce(&mut Host) -> R) -> R {
    HOST.with_borrow_mut(f)
}
