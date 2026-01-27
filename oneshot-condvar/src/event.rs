use near_sdk::near;

#[must_use = "make sure to `.emit()` this event"]
#[near(event_json(standard = "oneshot_condvar"))]
#[derive(Debug, Clone)]
pub enum Event {
    /// Emitted when the notifier successfully authorizes the contract.
    #[event_version("1.0.0")]
    Authorized,
    /// Emitted when the yield promise times out.
    #[event_version("1.0.0")]
    Timeout,
    /// Emitted when the contract is about to delete itself.
    #[event_version("1.0.0")]
    Cleanup,
}
