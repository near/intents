use near_sdk::{near};

#[must_use = "make sure to `.emit()` this event"]
#[near(event_json(standard = "transfer_auth"))]
#[derive(Debug, Clone)]
pub enum Event {
    #[event_version("1.0.0")]
    Authorized,
    #[event_version("1.0.0")]
    Timeout,
}
