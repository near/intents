use core::time::Duration;

use defuse_deadline::Deadline;
use near_sdk::{env, near};

use crate::{Actor, Request, hook::Hook};

#[near(serializers = [borsh])]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Keepalive {
    last_updated_at: Deadline,
    timeout: Duration,
}

impl Keepalive {
    const ERR_EXPIRED: &str = "expired";

    #[inline]
    fn now() -> Deadline {
        // We need to truncate the current timestamp down to seconds, since
        // `self.last_updated_at` is serialized as `TimestampSeconds<u32>`.
        // As a result, `now()` might be (less than 1 second) behind the actual
        // block timestamp, which is acceptable: we're just assuming the receipt
        // arrived a bit faster.
        Deadline::now().trunc_subsecs()
    }
}

impl Hook for Keepalive {
    fn on_request(&mut self, _request: &Request, _actor: Actor<'_>) {
        let now = Self::now();
        if self.last_updated_at < now - self.timeout {
            env::panic_str(Self::ERR_EXPIRED);
        }
        self.last_updated_at = now;
    }
}
