use std::future::Ready;

use tower::retry::Policy;

#[derive(Clone)]
pub struct Attempts(pub usize);

impl<Req: Clone, Res, E> Policy<Req, Res, E> for Attempts {
    type Future = Ready<()>;

    fn retry(&mut self, _: &mut Req, result: &mut Result<Res, E>) -> Option<Self::Future> {
        result.is_err().then_some(())?;
        let remaining = self.0.checked_sub(1)?;
        tracing::warn!(attempts_remaining = remaining, "request failed, retrying");
        self.0 = remaining;
        Some(std::future::ready(()))
    }

    fn clone_request(&mut self, req: &Req) -> Option<Req> {
        Some(req.clone())
    }
}
