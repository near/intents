pub mod cache;
pub mod retry;

use tower::BoxError;

use crate::error::ExecutionStackError;

// tower::TimeoutLayer always boxes both the inner service error and tower::timeout::error::Elapsed
// into BoxError, forcing a runtime downcast at the consumer side. The bound `E: Into<ExecutionStackError>`
// provides a compile-time check that the expected error type is wired into the stack correctly.
pub(crate) fn timeout_err<E>(e: BoxError) -> ExecutionStackError
where
    E: std::error::Error + Send + Sync + 'static + Into<ExecutionStackError>,
{
    if e.is::<tower::timeout::error::Elapsed>() {
        tracing::warn!("request timed out");
        ExecutionStackError::Timeout
    } else {
        match e.downcast::<E>() {
            Ok(e) => (*e).into(),
            Err(e) => ExecutionStackError::Internal(e),
        }
    }
}
