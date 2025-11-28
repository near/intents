
#[near(serializers = [json])]
#[derive(AccessControlRole, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Role {
    /// Can upgrade the contract
    Owner,
    /// Can call cancel on the proxy contract (forwarded to escrow)
    Canceller,
    /// Can rotate the relay public key
    KeyManager,
}
