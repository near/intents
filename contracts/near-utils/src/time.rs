use chrono::{DateTime, Utc};
use near_sdk::env;

pub fn now() -> DateTime<Utc> {
    DateTime::from_timestamp_nanos(
        env::block_timestamp()
            .try_into()
            .unwrap_or_else(|_| unreachable!()),
    )
}
