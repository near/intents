//! [EIP-712](https://eips.ethereum.org/EIPS/eip-712) Typed structured data
//! hashing and signing.
//!
//! Ethereum wallets (e.g. `MetaMask`, `WalletConnect`) expose this standard as
//! `eth_signTypedData_v4`: instead of signing an opaque byte string (as with
//! [ERC-191](https://eips.ethereum.org/EIPS/eip-191)), the user signs a
//! *typed* structure bound to a *domain*, which the wallet displays in a
//! structured form and which cannot be replayed in another domain.
//!
//! This crate provides the hashing primitives only: callers define their own
//! type strings and encode struct members into 32-byte words themselves.

use defuse_crypto::{Curve, RecoverableCurve, secp256k1::Secp256k1};
use defuse_digest::{Digest, sha3::Keccak256};

/// `keccak256` output, i.e. a 32-byte hash
pub type Hash = [u8; 32];

/// [EIP-712](https://eips.ethereum.org/EIPS/eip-712) Typed structured data
/// hashing and signing
pub struct Eip712;

impl Eip712 {
    /// `typeHash = keccak256(encodeType(typeOf(s)))`, where `encodeType` is
    /// the canonical type string, e.g.
    /// `"WalletMessage(bytes32 hash)"`.
    #[inline]
    pub fn type_hash(encode_type: impl AsRef<[u8]>) -> Hash {
        Keccak256::digest(encode_type).into()
    }

    /// `hashStruct(s) = keccak256(typeHash ‖ encodeData(s))`, where
    /// `encodeData(s)` is the concatenation of struct members, each already
    /// [encoded](Self::encode_bytes) into a single 32-byte word.
    #[inline]
    pub fn hash_struct(type_hash: &Hash, encode_data: impl IntoIterator<Item = Hash>) -> Hash {
        encode_data
            .into_iter()
            .fold(Keccak256::new_with_prefix(type_hash), Digest::chain_update)
            .finalize()
            .into()
    }

    /// `encodeData` for dynamic members (i.e. `string` and `bytes`), which
    /// are encoded as `keccak256` of their contents.
    ///
    /// Atomic members of exactly 32 bytes (e.g. `bytes32`) are encoded
    /// directly as-is.
    #[inline]
    pub fn encode_bytes(value: impl AsRef<[u8]>) -> Hash {
        Keccak256::digest(value).into()
    }

    /// `encodeData` for unsigned integer members (i.e. `uintN`), which are
    /// encoded as their big-endian representation, left-padded with zeroes
    /// to 32 bytes.
    #[inline]
    pub fn encode_uint(value: impl Into<u128>) -> Hash {
        let mut buf = [0u8; 32];
        buf[16..].copy_from_slice(&value.into().to_be_bytes());
        buf
    }

    /// `encodeData` for `bool` members, which are encoded as `uint256`
    /// `0`/`1`.
    #[inline]
    pub fn encode_bool(value: bool) -> Hash {
        Self::encode_uint(u8::from(value))
    }

    /// `encodeData` for array members (e.g. `string[]`), which are encoded
    /// as `keccak256` of the concatenation of their contents, each already
    /// [encoded](Self::encode_bytes) into a single 32-byte word.
    #[inline]
    pub fn encode_array(items: impl IntoIterator<Item = Hash>) -> Hash {
        items
            .into_iter()
            .fold(Keccak256::new(), Digest::chain_update)
            .finalize()
            .into()
    }

    /// Derive prehash for signing according to following schema:
    ///
    /// ```text
    /// keccak256(0x19 ‖ 0x01 ‖ domainSeparator ‖ hashStruct(message))
    /// ```
    #[inline]
    pub fn prehash(domain_separator: &Hash, struct_hash: &Hash) -> Hash {
        Keccak256::new_with_prefix(b"\x19\x01")
            .chain_update(domain_separator)
            .chain_update(struct_hash)
            .finalize()
            .into()
    }

    /// Try to recover public key which signed given message according to
    /// [EIP-712](https://eips.ethereum.org/EIPS/eip-712) and produced given
    /// signature and recovery id.
    #[must_use = "check recovered public key"]
    #[inline]
    pub fn recover(
        domain_separator: &Hash,
        struct_hash: &Hash,
        signature: &<Secp256k1 as Curve>::Signature,
        recovery_id: <Secp256k1 as RecoverableCurve>::RecoveryId,
    ) -> Option<<Secp256k1 as Curve>::PublicKey> {
        Secp256k1::recover(
            &Self::prehash(domain_separator, struct_hash),
            signature,
            recovery_id,
        )
    }
}

/// [EIP-712](https://eips.ethereum.org/EIPS/eip-712) domain with `name` and
/// `version` members only.
///
/// `chainId` and `verifyingContract` are intentionally omitted: NEAR contracts
/// have no EVM chain id and their account ids do not fit into `address`. The
/// signed message itself is expected to bind the network and the contract
/// instance instead.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Eip712Domain<'a> {
    /// `EIP712Domain.name`
    pub name: &'a str,
    /// `EIP712Domain.version`
    pub version: &'a str,
}

impl Eip712Domain<'_> {
    /// `encodeType(EIP712Domain)`
    pub const ENCODE_TYPE: &'static str = "EIP712Domain(string name,string version)";

    /// `domainSeparator = hashStruct(eip712Domain)`
    #[inline]
    pub fn separator(&self) -> Hash {
        Eip712::hash_struct(
            &Eip712::type_hash(Self::ENCODE_TYPE),
            [
                Eip712::encode_bytes(self.name),
                Eip712::encode_bytes(self.version),
            ],
        )
    }
}

#[cfg(test)]
mod tests {
    use defuse_crypto::RecoverableSigner;
    use hex_literal::hex;
    use k256::{ecdsa::SigningKey, elliptic_curve::Generate};
    use rstest::rstest;

    use super::*;

    const DOMAIN: Eip712Domain<'static> = Eip712Domain {
        name: "NEAR Wallet Contract",
        version: "1",
    };

    const MAIL_TYPE: &str = "Mail(string contents)";

    #[rstest]
    fn domain_separator() {
        // known-answer vector: `ethers.TypedDataEncoder.hashDomain({
        //   name: "NEAR Wallet Contract", version: "1" })`
        assert_eq!(
            DOMAIN.separator(),
            hex!("9ed0384a9369f42aa89e172b67359e73238fce303b45b92842e471874e2f6d22"),
        );
    }

    #[rstest]
    #[case(0u32, hex!("0000000000000000000000000000000000000000000000000000000000000000"))]
    #[case(
        42u32,
        hex!("000000000000000000000000000000000000000000000000000000000000002a")
    )]
    #[case(
        u32::MAX,
        hex!("00000000000000000000000000000000000000000000000000000000ffffffff")
    )]
    fn encode_uint(#[case] value: u32, #[case] encoded: Hash) {
        assert_eq!(Eip712::encode_uint(value), encoded);
    }

    #[rstest]
    #[case(false, hex!("0000000000000000000000000000000000000000000000000000000000000000"))]
    #[case(true, hex!("0000000000000000000000000000000000000000000000000000000000000001"))]
    fn encode_bool(#[case] value: bool, #[case] encoded: Hash) {
        assert_eq!(Eip712::encode_bool(value), encoded);
    }

    #[rstest]
    fn encode_array() {
        // empty array is encoded as keccak256 of empty input
        assert_eq!(
            Eip712::encode_array([]),
            hex!("c5d2460186f7233c927e7db2dcc703c0e500b653ca82273b7bfad8045d85a470"),
        );

        // known-answer vector (cross-checked against ethers):
        // keccak256(keccak256("wallet.near") ‖ keccak256("v1.signer"))
        assert_eq!(
            Eip712::encode_array(["wallet.near", "v1.signer"].map(Eip712::encode_bytes)),
            hex!("b6d2f824e6d61dbb48dede311263a3b2ae0915082ca066a234428bcd1ce7674b"),
        );
    }

    #[rstest]
    fn type_hash() {
        // keccak256("Mail(string contents)")
        assert_eq!(
            Eip712::type_hash(MAIL_TYPE),
            hex!("391581b66dfce93075def2f759ecf34a96f4b25b7efdd0b492e207d2ed9fbc76"),
        );
    }

    #[rstest]
    #[tokio::test]
    async fn sign_recover() {
        let signer = SigningKey::generate();

        let struct_hash = Eip712::hash_struct(
            &Eip712::type_hash(MAIL_TYPE),
            [Eip712::encode_bytes("Hello world!")],
        );
        let domain_separator = DOMAIN.separator();

        let (signature, recovery_id) = RecoverableSigner::<Secp256k1>::sign_recoverable(
            &signer,
            &Eip712::prehash(&domain_separator, &struct_hash),
        )
        .await
        .unwrap();

        assert_eq!(
            Eip712::recover(&domain_separator, &struct_hash, &signature, recovery_id).as_ref(),
            Some(signer.verifying_key()),
            "can't recover signer's public key",
        );
    }
}
