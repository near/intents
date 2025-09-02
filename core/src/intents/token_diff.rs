use super::{ExecutableIntent, IntentEvent};
use crate::{
    DefuseError, Result,
    accounts::AccountEvent,
    amounts::Amounts,
    engine::{Engine, Inspector, State, StateView},
    events::DefuseEvent,
    fees::Pips,
    token_id::{TokenId, TokenIdType},
};
use defuse_num_utils::CheckedMulDiv;
use impl_tools::autoimpl;
use near_sdk::{AccountId, AccountIdRef, CryptoHash, near};
use serde_with::{DisplayFromStr, serde_as};
use std::{borrow::Cow, collections::BTreeMap};

pub type TokenDeltas = Amounts<BTreeMap<TokenId, i128>>;

#[cfg_attr(
    all(feature = "abi", not(target_arch = "wasm32")),
    serde_as(schemars = true)
)]
#[cfg_attr(
    not(all(feature = "abi", not(target_arch = "wasm32"))),
    serde_as(schemars = false)
)]
#[near(serializers = [borsh, json])]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[autoimpl(Deref using self.diff)]
#[autoimpl(DerefMut using self.diff)]
/// The user declares the will to have a set of changes done to set of tokens. For example,
/// a simple trade of 100 of token A for 200 of token B, can be represented by `TokenDiff`
/// of {"A": -100, "B": 200} (this format is just for demonstration purposes).
/// In general, the user can submit multiple changes with many tokens,
/// not just token A for token B.
pub struct TokenDiff {
    #[serde_as(as = "Amounts<BTreeMap<_, DisplayFromStr>>")]
    pub diff: TokenDeltas,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub memo: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub referral: Option<AccountId>,
}

impl ExecutableIntent for TokenDiff {
    fn execute_intent<S, I>(
        self,
        signer_id: &AccountIdRef,
        engine: &mut Engine<S, I>,
        intent_hash: CryptoHash,
    ) -> Result<()>
    where
        S: State,
        I: Inspector,
    {
        if self.diff.is_empty() {
            return Err(DefuseError::InvalidIntent);
        }

        let protocol_fee = engine.state.fee();
        let mut fees_collected: Amounts = Amounts::default();

        for (token_id, delta) in &self.diff {
            if *delta == 0 {
                return Err(DefuseError::InvalidIntent);
            }

            // add delta to signer's account
            engine
                .state
                .internal_apply_deltas(signer_id, [(token_id.clone(), *delta)])?;

            // take fees only from negative deltas (i.e. token_in)
            if *delta < 0 {
                let amount = delta.unsigned_abs();
                let fee = Self::token_fee(token_id, amount, protocol_fee).fee_ceil(amount);

                // collect fee
                fees_collected
                    .add(token_id.clone(), fee)
                    .ok_or(DefuseError::BalanceOverflow)?;
            }
        }

        engine.inspector.on_event(DefuseEvent::TokenDiff(
            [IntentEvent::new(
                AccountEvent::new(
                    signer_id,
                    TokenDiffEvent {
                        diff: Cow::Borrowed(&self),
                        fees_collected: fees_collected.clone(),
                    },
                ),
                intent_hash,
            )]
            .as_slice()
            .into(),
        ));

        // deposit fees to collector
        if !fees_collected.is_empty() {
            engine
                .state
                .internal_add_balance(engine.state.fee_collector().into_owned(), fees_collected)?;
        }

        Ok(())
    }
}

#[cfg_attr(
    all(feature = "abi", not(target_arch = "wasm32")),
    serde_as(schemars = true)
)]
#[cfg_attr(
    not(all(feature = "abi", not(target_arch = "wasm32"))),
    serde_as(schemars = false)
)]
#[near(serializers = [json])]
#[derive(Debug, Clone)]
/// An event emitted when a `TokenDiff` intent is executed.
pub struct TokenDiffEvent<'a> {
    #[serde(flatten)]
    pub diff: Cow<'a, TokenDiff>,

    #[serde_as(as = "Amounts<BTreeMap<_, DisplayFromStr>>")]
    #[serde(skip_serializing_if = "Amounts::is_empty")]
    pub fees_collected: Amounts,
}

impl TokenDiff {
    /// Returns [`TokenDiff`] closure to successfully execute `self`
    /// assuming given `fee`
    #[inline]
    pub fn closure(self, fee: Pips) -> Option<TokenDeltas> {
        Self::closure_deltas(self.diff.into_inner(), fee)
    }

    /// Returns [`TokenDiff`] closure to successfully execute given set
    /// of distinct [`TokenDiff`] assuming given `fee`
    #[inline]
    pub fn closure_many(diffs: impl IntoIterator<Item = Self>, fee: Pips) -> Option<TokenDeltas> {
        Self::closure_deltas(diffs.into_iter().flat_map(|d| d.diff.into_inner()), fee)
    }

    /// Returns closure for deltas that should be given in a single
    /// [`TokenDiff`] to successfully execute given set of distinct `deltas`
    /// assuming given `fee`
    #[inline]
    pub fn closure_deltas(
        deltas: impl IntoIterator<Item = (TokenId, i128)>,
        fee: Pips,
    ) -> Option<TokenDeltas> {
        deltas
            .into_iter()
            // collect total supply deltas
            .try_fold(TokenDeltas::default(), |deltas, (token_id, delta)| {
                let supply_delta = Self::supply_delta(&token_id, delta, fee)?;
                deltas.with_apply_delta(token_id, supply_delta)
            })?
            .into_inner()
            .into_iter()
            // calculate closures from total supply deltas
            .try_fold(TokenDeltas::default(), |deltas, (token_id, delta)| {
                let closure = Self::closure_supply_delta(&token_id, delta, fee)?;
                deltas.with_apply_delta(token_id, closure)
            })
    }

    /// Returns closure for delta that should be given in a single
    /// [`TokenDiff`] to successfully execute [`TokenDiff`] with given
    /// `delta` on the same token assuming given `fee`.
    #[inline]
    pub fn closure_delta(token_id: &TokenId, delta: i128, fee: Pips) -> Option<i128> {
        Self::closure_supply_delta(token_id, Self::supply_delta(token_id, delta, fee)?, fee)
    }

    /// Returns total supply delta from token delta
    #[inline]
    fn supply_delta(token_id: &TokenId, delta: i128, fee: Pips) -> Option<i128> {
        if delta < 0 {
            // fee is taken only on negative deltas (i.e. token_in)
            delta.checked_mul_div_ceil(
                Self::token_fee(token_id, delta.unsigned_abs(), fee)
                    .invert()
                    .as_pips()
                    .into(),
                Pips::MAX.as_pips().into(),
            )
        } else {
            // token_out
            Some(delta)
        }
    }

    /// Returns closure for total supply delta that should be given in
    /// a single [`TokenDiff`] to successfully execute [`TokenDiff`] with
    /// given `delta` on the same token assuming given `fee`.
    #[inline]
    pub fn closure_supply_delta(token_id: &TokenId, delta: i128, fee: Pips) -> Option<i128> {
        let closure = delta.checked_neg()?;
        if closure < 0 {
            // fee is taken only on negative deltas (i.e. token_in)
            closure.checked_mul_div_euclid(
                Pips::MAX.as_pips().into(),
                Self::token_fee(token_id, delta.unsigned_abs(), fee)
                    .invert()
                    .as_pips()
                    .into(),
            )
        } else {
            // token_out
            Some(closure)
        }
    }

    #[inline]
    pub fn token_fee(token_id: impl Into<TokenIdType>, amount: u128, fee: Pips) -> Pips {
        let token_id = token_id.into();
        match token_id {
            TokenIdType::Nep141 => {}
            TokenIdType::Nep245 if amount > 1 => {}

            // do not take fees on NFTs and MTs with |delta| <= 1
            TokenIdType::Nep171 | TokenIdType::Nep245 => return Pips::ZERO,
        }
        fee
    }
}

#[cfg(test)]
mod tests {
    use itertools::Itertools;
    use rstest::rstest;

    use crate::token_id::{nep141::Nep141TokenId, nep171::Nep171TokenId, nep245::Nep245TokenId};

    use super::*;

    #[rstest]
    #[test]
    fn closure_delta(
        #[values(
            (Nep141TokenId::new("ft.near".parse().unwrap()).into(), 1_000_000),
            (Nep141TokenId::new("ft.near".parse().unwrap()).into(), -1_000_000),
            (Nep171TokenId::new("nft.near".parse().unwrap(), "1".to_string()).unwrap().into(), 1),
            (Nep171TokenId::new("nft.near".parse().unwrap(), "1".to_string()).unwrap().into(), -1),
            (Nep245TokenId::new("mt.near".parse().unwrap(), "ft1".to_string()).unwrap().into(), 1_000_000),
            (Nep245TokenId::new("mt.near".parse().unwrap(), "ft1".to_string()).unwrap().into(), -1_000_000),
            (Nep245TokenId::new("mt.near".parse().unwrap(), "nft1".to_string()).unwrap().into(), 1),
            (Nep245TokenId::new("mt.near".parse().unwrap(), "nft1".to_string()).unwrap().into(), -1),
        )]
        token_delta: (TokenId, i128),
        #[values(
            Pips::ZERO,
            Pips::ONE_PIP,
            Pips::ONE_BIP,
            Pips::ONE_PERCENT,
            Pips::ONE_PERCENT * 50,
        )]
        fee: Pips,
    ) {
        let (token_id, delta) = token_delta;
        let closure = TokenDiff::closure_delta(&token_id, delta, fee).unwrap();

        assert_eq!(
            TokenDiff::supply_delta(&token_id, delta, fee).unwrap()
                + TokenDiff::supply_delta(&token_id, closure, fee).unwrap(),
            0,
            "invariant violated for {token_id}: delta: {delta}, closure: {closure}, fee: {fee}",
        );
    }

    #[test]
    fn closure_deltas_empty() {
        assert!(
            TokenDiff::closure_deltas(None, Pips::ONE_BIP)
                .unwrap()
                .is_empty()
        );
    }

    #[rstest]
    #[test]
    fn closure_deltas_nonoverlapping(
        #[values(
            Pips::ZERO,
            Pips::ONE_PIP,
            Pips::ONE_BIP,
            Pips::ONE_BIP * 12,
            Pips::ONE_PERCENT,
            Pips::ONE_PERCENT * 50,
        )]
        fee: Pips,
    ) {
        let [t1, t2, t3] =
            ["ft1", "ft2", "ft3"].map(|t| TokenId::from(Nep141TokenId::new(t.parse().unwrap())));

        for (d1, d2, d3) in [0, 1, -1, 50, -50, 100, -100, 300, -300, 10_000, -10_000]
            .into_iter()
            .tuple_combinations()
        {
            assert_eq!(
                TokenDiff::closure_deltas(
                    [
                        TokenDeltas::default()
                            .with_apply_deltas([(t1.clone(), d1), (t2.clone(), d2)])
                            .unwrap(),
                        TokenDeltas::default()
                            .with_apply_deltas([(t3.clone(), d3)])
                            .unwrap(),
                    ]
                    .into_iter()
                    .flatten(),
                    fee
                )
                .unwrap(),
                TokenDeltas::default()
                    .with_apply_deltas([
                        (t1.clone(), TokenDiff::closure_delta(&t1, d1, fee).unwrap()),
                        (t2.clone(), TokenDiff::closure_delta(&t2, d2, fee).unwrap()),
                        (t3.clone(), TokenDiff::closure_delta(&t3, d3, fee).unwrap()),
                    ])
                    .unwrap(),
                "d1: {d1}, d2: {d2}, d3: {d3}"
            );
        }
    }

    #[rstest]
    #[test]
    fn arbitrage_means_somebody_looses(#[values(Pips::ZERO, Pips::ONE_BIP)] fee: Pips) {
        let [t1, t2, t3] =
            ["ft1", "ft2", "ft3"].map(|t| TokenId::from(Nep141TokenId::new(t.parse().unwrap())));

        let closure = TokenDiff::closure_deltas(
            [
                TokenDeltas::default()
                    .with_apply_deltas([(t1.clone(), -100), (t2.clone(), 200)])
                    .unwrap(),
                TokenDeltas::default()
                    .with_apply_deltas([(t2, -200), (t3.clone(), 300)])
                    .unwrap(),
                TokenDeltas::default()
                    .with_apply_deltas([(t3, -300), (t1, 101)])
                    .unwrap(),
            ]
            .into_iter()
            .flatten(),
            fee,
        )
        .unwrap();
        assert!(!closure.is_empty());
        assert!(closure.into_inner().into_values().all(i128::is_negative));
    }
}
