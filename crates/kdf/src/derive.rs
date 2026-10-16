use defuse_crypto::{Curve, RecoverableCurve};

use crate::{
    DeriveSigner, RecoverableDeriveSigner,
    schema::{Schema, Value},
};

/// A helper extension trait for [`Schema`] and [`DeriveSigner`].
pub trait DeriveExt {
    /// Derive a public key for given path on curve `C`.
    ///
    /// This is a shorthand for:
    /// [`.schema()`](DeriveSigner::schema)[`.derive(path)`](Schema::derive).
    ///
    /// > ℹ️ Tips:
    /// > 1. Use [`.with_schema(inner)`](Self::with_schema) to compose derivable
    /// >    signer with `inner` sub-schema.
    /// > 2. Use [`.at_path(path)`](Self::at_path) to convert [`DeriveSigner`]
    /// >    into a [`Signer`](defuse_crypto::Signer) with fixed derivation `path`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use defuse_kdf::{DeriveExt, crypto::ed25519::Ed25519};
    /// # use defuse_kdf::{
    /// #     crypto::ed25519::ed25519_dalek,
    /// #     schema::{digest::Digest, ReduceScalar},
    /// # };
    /// # use defuse_digest::sha3::Sha3_256;
    /// # use hex_literal::hex;
    ///
    /// # let signer = ed25519_dalek::SigningKey::from_bytes(
    /// #     &hex!("5e20282e090a2be1487b29a38750d746ffcdb0b47ebff09441a2167b7523260f"),
    /// # )
    /// # .with_schema(ReduceScalar::<Ed25519>::new())
    /// # .with_schema(Digest::<Sha3_256>::default());
    /// let path = "derivation path";
    /// let public_key = signer.derive_public_key::<Ed25519, _>(path);
    /// # assert_eq!(
    /// #     *public_key.as_bytes(),
    /// #     hex!("6eb4ab63bc0c36517d7dd86b8790778b9ee7f17171325cd2446b9d4970a3a471"),
    /// #     "derived public key has changed",
    /// # );
    /// ```
    #[inline]
    fn derive_public_key<C: Curve, P>(&self, path: P) -> C::PublicKey
    where
        Self: DeriveSigner<C, P>,
    {
        self.schema().derive(path)
    }

    /// Sign given message with a secret key **internally** derived
    /// for given path according to [`schema`](DeriveSigner::schema).
    ///
    /// **NOTE**:
    /// * Implementations MAY require `msg` to be prehash (i.e. output
    ///   of cryptographic hash function) of a fixed length and fail
    ///   otherwise. Check corresponding docs before using.
    /// * The returned signatures MIGHT be non-deterministic, i.e.
    ///   implementations MAY return different signatures for the same
    ///   `path` and `msg`.
    ///
    /// This method forwards to [`DeriveSigner::derive_sign()`].
    ///
    /// > ℹ️ Tips:
    /// > 1. Use [`.with_schema(inner)`](Self::with_schema) to compose derivable
    /// >    signer with `inner` sub-schema.
    /// > 2. Use [`.at_path(path)`](Self::at_path) to convert [`DeriveSigner`]
    /// >    into a [`Signer`](defuse_crypto::Signer) with fixed derivation `path`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use defuse_kdf::{DeriveExt, crypto::{Curve, ed25519::Ed25519}};
    /// # use defuse_kdf::{
    /// #     crypto::ed25519::ed25519_dalek,
    /// #     schema::{digest::Digest, ReduceScalar},
    /// # };
    /// # use defuse_digest::sha3::Sha3_256;
    ///
    /// # #[tokio::main(flavor = "local")]
    /// # async fn main() -> Result<(), Box<dyn core::error::Error>> {
    /// # let signer = ed25519_dalek::SigningKey::from_bytes(&[0u8; 32])
    /// # .with_schema(ReduceScalar::<Ed25519>::new())
    /// # .with_schema(Digest::<Sha3_256>::default());
    /// let path = "derivation path";
    /// let msg = b"message";
    ///
    /// let signature = signer.derive_sign::<Ed25519, _>(path, msg).await?;
    ///
    /// assert!(Ed25519::verify(
    ///     &signer.derive_public_key::<Ed25519, _>(path),
    ///     msg,
    ///     &signature,
    /// ));
    /// # Ok(()) }
    /// ```
    #[inline]
    fn derive_sign<C: Curve, P>(
        &self,
        path: P,
        msg: &[u8],
    ) -> impl Future<Output = Result<C::Signature, Self::Error>> + Send
    where
        Self: DeriveSigner<C, P>,
        P: Send,
    {
        DeriveSigner::<C, P>::derive_sign(self, path, msg.as_ref())
    }

    /// Recoveryably [sign](DeriveSigner::derive_sign) given message with a
    /// secret key **internally** derived for given `path` according to
    /// [`schema`](DeriveSigner::schema) and return [signature](Curve::Signature)
    /// along with [recovery id](RecoverableCurve::RecoveryId).
    ///
    /// **NOTE**:
    /// * Implementations MAY require `msg` to be prehash (i.e. output
    ///   of cryptographic hash function) of a fixed length and fail
    ///   otherwise. Check corresponding docs before using.
    /// * The returned signatures MIGHT be non-deterministic, i.e.
    ///   implementations MAY return different signatures for the same
    ///   `path` and `msg`.
    ///
    /// This method forwards to [`RecoverableDeriveSigner::derive_sign_recoverable()`].
    ///
    /// > ℹ️ Tips:
    /// > 1. Use [`.with_schema(inner)`](Self::with_schema) to compose derivable
    /// >    signer with `inner` sub-schema.
    /// > 2. Use [`.at_path(path)`](Self::at_path) to convert [`DeriveSigner`]
    /// >    into a [`Signer`](defuse_crypto::Signer) with fixed derivation `path`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use defuse_kdf::{DeriveExt, crypto::{RecoverableCurve, secp256k1::Secp256k1}};
    /// # use defuse_kdf::{
    /// #     crypto::secp256k1::k256,
    /// #     schema::{digest::Digest, ReduceScalar},
    /// # };
    /// # use defuse_digest::sha3::Sha3_256;
    /// # use hex_literal::hex;
    ///
    /// # #[tokio::main(flavor = "local")]
    /// # async fn main() -> Result<(), Box<dyn core::error::Error>> {
    /// # let signer = k256::ecdsa::SigningKey::from_bytes(
    /// #     &hex!("f9151587531d664b5e266a1943c56530d8c6cebeedf631b15736176f1f9d8a90").into(),
    /// # )
    /// # .unwrap()
    /// # .with_schema(ReduceScalar::<Secp256k1>::new())
    /// # .with_schema(Digest::<Sha3_256>::default());
    /// let path = "derivation path";
    /// // secp256k1 signs over 32-byte prehash
    /// let prehash = hex!("f2ca1bb6c7e907d06dafe4687e579fce76b37e4e93b7605022da52e6ccc26fd2");
    ///
    /// let (sig, recovery_id) = signer.derive_sign_recoverable::<Secp256k1, _>(path, &prehash).await?;
    ///
    /// assert_eq!(
    ///     Secp256k1::recover(&prehash, &sig, recovery_id),
    ///     Some(signer.derive_public_key::<Secp256k1, _>(path)),
    /// );
    /// # Ok(()) }
    #[inline]
    fn derive_sign_recoverable<C: RecoverableCurve, P>(
        &self,
        path: P,
        msg: &[u8],
    ) -> impl Future<Output = Result<(C::Signature, C::RecoveryId), Self::Error>> + Send
    where
        Self: RecoverableDeriveSigner<C, P>,
        P: Send,
    {
        RecoverableDeriveSigner::<C, P>::derive_sign_recoverable(self, path, msg)
    }

    /// Compose [`Schema`] or [`DeriveSigner`] with given inner sub-schema.
    ///
    /// # Composing schemas
    ///
    /// Schemas can be composed with each other in any arbitrary way. When a nested
    /// schema `A.with_schema(B)` is being evaluated on a given `path`, the inner
    /// sub-schema `B` is evaluated first and its output is passed to the outer
    /// schema `A`. Finally, this results into `A(B(path))`:
    ///
    /// ```rust
    /// use defuse_kdf::{DeriveExt, schema::{Schema, SchemaFn}};
    ///
    /// let a = SchemaFn::new(|p| format!("A({p})"));
    /// let b = SchemaFn::new(|p| format!("B({p})"));
    ///
    /// let ab = a.with_schema(b);
    /// assert_eq!(ab.derive("path"), "A(B(path))");
    /// ```
    ///
    /// # Nested signers
    ///
    /// Each [`DeriveSigner`] has its [own](DeriveSigner::schema) schema, which is
    /// used to derive both public keys and internal secret keys. Such approach
    /// makes it possible to compose derivable signers with sub-schemas, so that
    /// schema-level paths are mapped into signer's ones before derivation happens:
    ///
    /// > ℹ️ Tip: use [`.at_path(path)`](Self::at_path) to convert [`DeriveSigner`]
    /// > into a [`Signer`](defuse_crypto::Signer) with fixed derivation `path`.
    ///
    /// ```rust
    /// use defuse_kdf::{
    ///     DeriveExt,
    ///     crypto::{Curve, ed25519::Ed25519},
    /// #   crypto::ed25519::ed25519_dalek,
    ///     schema::SchemaFn,
    /// #   schema::{digest::Digest, ReduceScalar},
    /// };
    /// # use defuse_digest::sha3::Sha3_256;
    ///
    /// # #[tokio::main(flavor = "local")]
    /// # async fn main() -> Result<(), Box<dyn core::error::Error>> {
    /// # let signer = ed25519_dalek::SigningKey::from_bytes(&[0u8; 32])
    /// # .with_schema(ReduceScalar::<Ed25519>::new())
    /// # .with_schema(Digest::<Sha3_256>::default());
    /// // define sub-schema: "alice" -> "users/alice"
    /// let schema = SchemaFn::new(|user| format!("users/{user}"));
    /// // compose signer with sub-schema
    /// let nested = signer.with_schema(schema);
    /// # // re-create
    /// # let signer = ed25519_dalek::SigningKey::from_bytes(&[0u8; 32])
    /// # .with_schema(ReduceScalar::<Ed25519>::new())
    /// # .with_schema(Digest::<Sha3_256>::default());
    ///
    /// // derive public key for nested path
    /// let path = "alice";
    /// let derived_pk = nested.derive_public_key::<Ed25519, _>(path);
    /// assert_eq!(
    ///     derived_pk,
    ///     signer.derive_public_key::<Ed25519, _>("users/alice"),
    /// );
    /// // sign with key derived for nested path
    /// let msg = b"message";
    /// let signature = nested.derive_sign::<Ed25519, _>(path, msg).await?;
    /// assert!(Ed25519::verify(&derived_pk, msg, &signature));
    /// # Ok(()) }
    /// ```
    #[inline]
    fn with_schema<D>(self, inner: D) -> WithSchema<Self, D>
    where
        Self: Sized,
    {
        WithSchema::new(self, inner)
    }

    /// Convert [`DeriveSigner`] into a [`Signer`](defuse_crypto::Signer) with a
    /// fixed derivation path.
    ///
    /// Methods of this trait require to pass derivation path on every callsite,
    /// which might be inconvenient when always using the same one. This helper
    /// method allows to fix the derivation path once, so that further code can
    /// use following methods instead, which don't require to pass one anymore:
    ///
    /// * [`.derive_public_key(path)`](Self::derive_public_key) ->
    ///   [`.public_key()`](defuse_crypto::SignerExt::public_key)
    /// * [`.derive_sign(path, msg)`](Self::derive_sign) ->
    ///   [`.sign(msg)`](defuse_crypto::SignerExt::sign)
    /// * [`.derive_sign_recoverable(path, msg)`](Self::derive_sign_recoverable) ->
    ///   [`.sign_recoverable(msg)`](defuse_crypto::SignerExt::sign_recoverable)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use defuse_kdf::{DeriveExt, crypto::{Curve, SignerExt, ed25519::Ed25519}};
    /// # use defuse_kdf::{
    /// #     crypto::ed25519::ed25519_dalek,
    /// #     schema::{digest::Digest, ReduceScalar},
    /// # };
    /// # use defuse_digest::sha3::Sha3_256;
    ///
    /// # #[tokio::main(flavor = "local")]
    /// # async fn main() -> Result<(), Box<dyn core::error::Error>> {
    /// # let derive_signer = ed25519_dalek::SigningKey::from_bytes(&[0u8; 32])
    /// # .with_schema(ReduceScalar::<Ed25519>::new())
    /// # .with_schema(Digest::<Sha3_256>::default());
    /// // fix the derivation path on a signer
    /// let path = "derivation path";
    /// let signer = derive_signer.at_path(path);
    /// # // re-create
    /// # let derive_signer = ed25519_dalek::SigningKey::from_bytes(&[0u8; 32])
    /// # .with_schema(ReduceScalar::<Ed25519>::new())
    /// # .with_schema(Digest::<Sha3_256>::default());
    ///
    /// // get public key of derived signer
    /// let public_key = signer.public_key::<Ed25519>();
    /// assert_eq!(
    ///     public_key,
    ///     derive_signer.derive_public_key::<Ed25519, _>(path),
    /// );
    ///
    /// // signer will use this path for signing every time
    /// let msg = b"message";
    /// let signature = signer.sign::<Ed25519>(msg).await?;
    /// assert!(Ed25519::verify(&public_key, msg, &signature));
    /// # Ok(()) }
    /// ```
    #[inline]
    fn at_path<P>(self, path: P) -> WithSchema<Self, Value<P>>
    where
        Self: Sized,
    {
        self.with_schema(Value::new(path))
    }
}
impl<T> DeriveExt for T {}

/// Derive adaptor for [`Schema`] and [`DeriveSigner`].
///
/// See [`.with_schema()`](DeriveExt::with_schema).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct WithSchema<S, D> {
    pub(crate) outer: S,
    pub(crate) inner: D,
}

impl<S, D> WithSchema<S, D> {
    #[must_use]
    #[inline]
    pub const fn new(outer: S, inner: D) -> Self {
        Self { outer, inner }
    }
}
