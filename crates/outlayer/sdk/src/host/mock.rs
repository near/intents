use std::cell::RefCell;

use defuse_outlayer_crypto::signer::InMemorySigner;
use defuse_outlayer_host::context::HostContext;
use defuse_outlayer_primitives::{AccountIdRef, AppId};

thread_local! {
    pub(crate) static HOST: RefCell<HostContext> =
        RefCell::new(HostContext::new(
            AppId::Near(AccountIdRef::new_or_panic("test.near").into()),
            InMemorySigner::from_seed(&[]), // TODO
        ));
}

pub fn set(host: HostContext) {
    HOST.set(host);
}

pub fn with<R>(f: impl FnOnce(&mut HostContext) -> R) -> R {
    HOST.with_borrow_mut(f)
}
