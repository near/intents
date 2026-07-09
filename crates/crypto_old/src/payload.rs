//! Traits for hashing and verifying message payloads used within Intents.
//!
//! These traits provide a small abstraction layer over various signing
//! standards supported by Intents. Each standard (e.g. BIP-322, TIP-191,
//! ERC-191) defines its own structure that implements [`Payload`] and
//! [`SignedPayload`]. The implementations expose a uniform API so the
//! Intents engine can compute message hashes and verify signatures without
//! knowing the concrete standard.

/// 32-byte cryptographic hash output.
pub type CryptoHash = [u8; 32];


