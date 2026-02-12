use core::marker::PhantomData;
use std::borrow::Cow;

use near_sdk::near;

use crate::{RequestMessage, signature::SigningStandard};

pub struct WalletDomainPrefix<S: ?Sized>(PhantomData<S>);

impl<S: ?Sized> WalletDomainPrefix<S> {
    const DOMAIN_PREFIX: &str = "NEAR_WALLET_CONTRACT";
}

impl<'a, S> SigningStandard<&'a RequestMessage> for WalletDomainPrefix<S>
where
    S: SigningStandard<SignatureDomain<'static, WalletDomain<'a>>>,
{
    type PublicKey = S::PublicKey;

    fn verify(msg: &'a RequestMessage, public_key: &Self::PublicKey, signature: &str) -> bool {
        S::verify(
            SignatureDomain::new(Self::DOMAIN_PREFIX, WalletDomain::V1(Cow::Borrowed(msg))),
            public_key,
            signature,
        )
    }
}

#[near(serializers = [borsh, json])]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignatureDomain<'a, T> {
    pub domain: Cow<'a, str>,
    #[serde(flatten)]
    pub data: T,
}

impl<'a, T> SignatureDomain<'a, T> {
    pub fn new(domain: impl Into<Cow<'a, str>>, data: T) -> Self {
        Self {
            domain: domain.into(),
            data,
        }
    }
}

#[near(serializers = [borsh(use_discriminant = true), json])]
#[serde(tag = "version", content = "message", rename_all = "snake_case")]
#[derive(Debug, Clone, PartialEq, Eq)]
#[repr(u8)]
pub enum WalletDomain<'a> {
    V1(Cow<'a, RequestMessage>) = 0,
}
