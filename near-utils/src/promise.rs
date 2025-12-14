use near_sdk::Promise;

pub trait PromiseExt: Sized {
    fn and_maybe(self, p: Option<Promise>) -> Promise;
}

impl PromiseExt for Promise {
    #[inline]
    fn and_maybe(self, p: Option<Promise>) -> Promise {
        if let Some(p) = p { self.and(p) } else { self }
    }
}
