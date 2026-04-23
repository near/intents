use std::future::Ready;

use tower::retry::Policy;

#[derive(Clone)]
pub struct Attempts(pub usize);

impl<Req: Clone, Res, E> Policy<Req, Res, E> for Attempts {
    type Future = Ready<()>;

    fn retry(&mut self, _: &mut Req, result: &mut Result<Res, E>) -> Option<Self::Future> {
        result.is_err().then_some(())?;
        self.0 = self.0.checked_sub(1)?;
        Some(std::future::ready(()))
    }

    fn clone_request(&mut self, req: &Req) -> Option<Req> {
        Some(req.clone())
    }
}
