use std::{
    borrow::Cow,
    cmp::Reverse,
    collections::{BTreeMap, HashMap},
    iter,
};

use defuse_crypto::PublicKey;
use defuse_map_utils::cleanup::DefaultMap;
use defuse_nep245::{MtEvent, MtTransferEvent};
use near_sdk::{json_types::U128, near, AccountId, AccountIdRef};
use serde_with::{serde_as, DisplayFromStr};

use crate::{
    fees::Pips,
    intents::{
        token_diff::TokenDeltas,
        tokens::{FtWithdraw, MtWithdraw, NativeWithdraw, NftWithdraw},
    },
    tokens::{Amounts, TokenId},
    DefuseError, Nonce, Result,
};

use super::{State, StateView};

pub struct Deltas<S> {
    state: S,
    deltas: TransferMatcher,
}

impl<S> Deltas<S> {
    #[inline]
    pub fn new(state: S) -> Self {
        Self {
            state,
            deltas: TransferMatcher::new(),
        }
    }

    #[inline]
    pub fn finalize(self) -> Result<Transfers, InvariantViolated> {
        self.deltas.finalize()
    }
}

impl<S> StateView for Deltas<S>
where
    S: StateView,
{
    #[inline]
    fn verifying_contract(&self) -> Cow<'_, AccountIdRef> {
        self.state.verifying_contract()
    }

    #[inline]
    fn wnear_id(&self) -> Cow<'_, AccountIdRef> {
        self.state.wnear_id()
    }

    #[inline]
    fn fee(&self) -> Pips {
        self.state.fee()
    }

    #[inline]
    fn fee_collector(&self) -> Cow<'_, AccountIdRef> {
        self.state.fee_collector()
    }

    #[inline]
    fn has_public_key(&self, account_id: &AccountIdRef, public_key: &PublicKey) -> bool {
        self.state.has_public_key(account_id, public_key)
    }

    #[inline]
    fn iter_public_keys(&self, account_id: &AccountIdRef) -> impl Iterator<Item = PublicKey> + '_ {
        self.state.iter_public_keys(account_id)
    }

    #[inline]
    fn is_nonce_used(&self, account_id: &AccountIdRef, nonce: Nonce) -> bool {
        self.state.is_nonce_used(account_id, nonce)
    }

    #[inline]
    fn balance_of(&self, account_id: &AccountIdRef, token_id: &TokenId) -> u128 {
        self.state.balance_of(account_id, token_id)
    }
}

impl<S> State for Deltas<S>
where
    S: State,
{
    #[must_use]
    #[inline]
    fn add_public_key(&mut self, account_id: AccountId, public_key: PublicKey) -> bool {
        self.state.add_public_key(account_id, public_key)
    }

    #[must_use]
    #[inline]
    fn remove_public_key(&mut self, account_id: AccountId, public_key: PublicKey) -> bool {
        self.state.remove_public_key(account_id, public_key)
    }

    #[must_use]
    #[inline]
    fn commit_nonce(&mut self, account_id: AccountId, nonce: Nonce) -> bool {
        self.state.commit_nonce(account_id, nonce)
    }

    fn internal_add_balance(
        &mut self,
        owner_id: AccountId,
        tokens: impl IntoIterator<Item = (TokenId, u128)>,
    ) -> Result<()> {
        for (token_id, amount) in tokens {
            self.state
                .internal_add_balance(owner_id.clone(), [(token_id.clone(), amount)])?;
            if !self.deltas.deposit(owner_id.clone(), token_id, amount) {
                return Err(DefuseError::BalanceOverflow);
            }
        }
        Ok(())
    }

    fn internal_sub_balance(
        &mut self,
        owner_id: &AccountIdRef,
        tokens: impl IntoIterator<Item = (TokenId, u128)>,
    ) -> Result<()> {
        for (token_id, amount) in tokens {
            self.state
                .internal_sub_balance(owner_id, [(token_id.clone(), amount)])?;
            if !self.deltas.withdraw(owner_id.to_owned(), token_id, amount) {
                return Err(DefuseError::BalanceOverflow);
            }
        }
        Ok(())
    }

    #[inline]
    fn ft_withdraw(&mut self, owner_id: &AccountIdRef, withdraw: FtWithdraw) -> Result<()> {
        self.state.ft_withdraw(owner_id, withdraw)
    }

    #[inline]
    fn nft_withdraw(&mut self, owner_id: &AccountIdRef, withdraw: NftWithdraw) -> Result<()> {
        self.state.nft_withdraw(owner_id, withdraw)
    }

    #[inline]
    fn mt_withdraw(&mut self, owner_id: &AccountIdRef, withdraw: MtWithdraw) -> Result<()> {
        self.state.mt_withdraw(owner_id, withdraw)
    }

    #[inline]
    fn native_withdraw(&mut self, owner_id: &AccountIdRef, withdraw: NativeWithdraw) -> Result<()> {
        self.state.native_withdraw(owner_id, withdraw)
    }
}

/// Accumulates internal deposits and withdrawals on different tokens
/// to match transfers using `.finalize()`
#[derive(Debug, Default)]
pub struct TransferMatcher(HashMap<TokenId, TokenTransferMatcher>);

impl TransferMatcher {
    #[inline]
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    #[inline]
    pub fn deposit(&mut self, owner_id: AccountId, token_id: TokenId, amount: u128) -> bool {
        self.0.entry_or_default(token_id).deposit(owner_id, amount)
    }

    #[inline]
    pub fn withdraw(&mut self, owner_id: AccountId, token_id: TokenId, amount: u128) -> bool {
        self.0.entry_or_default(token_id).withdraw(owner_id, amount)
    }

    #[inline]
    pub fn add_delta(&mut self, owner_id: AccountId, token_id: TokenId, delta: i128) -> bool {
        self.0.entry_or_default(token_id).add_delta(owner_id, delta)
    }

    // Finalizes all transfers, or returns unmatched deltas.
    // If unmatched deltas overflow, then Err(None) is returned.
    pub fn finalize(self) -> Result<Transfers, InvariantViolated> {
        let mut transfers = Transfers::default();
        let mut deltas = TokenDeltas::default();
        for (token_id, transfer_matcher) in self.0 {
            if let Err(unmatched) = transfer_matcher.finalize_into(&token_id, &mut transfers) {
                if unmatched == 0 || deltas.apply_delta(token_id, unmatched).is_none() {
                    return Err(InvariantViolated::Overflow);
                }
            }
        }
        if !deltas.is_empty() {
            return Err(InvariantViolated::UnmatchedDeltas {
                unmatched_deltas: deltas,
            });
        }
        Ok(transfers)
    }
}

type AccountAmounts = Amounts<HashMap<AccountId, u128>>;

// Accumulates internal deposits and withdrawals on a single token
#[derive(Debug, Default, PartialEq, Eq)]
pub struct TokenTransferMatcher {
    deposits: AccountAmounts,
    withdrawals: AccountAmounts,
}

impl TokenTransferMatcher {
    #[inline]
    pub fn deposit(&mut self, owner_id: AccountId, amount: u128) -> bool {
        Self::sub_add(&mut self.withdrawals, &mut self.deposits, owner_id, amount)
    }

    #[inline]
    pub fn withdraw(&mut self, owner_id: AccountId, amount: u128) -> bool {
        Self::sub_add(&mut self.deposits, &mut self.withdrawals, owner_id, amount)
    }

    #[inline]
    pub fn add_delta(&mut self, owner_id: AccountId, delta: i128) -> bool {
        let amount = delta.unsigned_abs();
        if delta.is_negative() {
            self.withdraw(owner_id, amount)
        } else {
            self.deposit(owner_id, amount)
        }
    }

    fn sub_add(
        sub: &mut AccountAmounts,
        add: &mut AccountAmounts,
        owner_id: AccountId,
        mut amount: u128,
    ) -> bool {
        let s = sub.amount_for(&owner_id);
        if s > 0 {
            let a = s.min(amount);
            sub.sub(owner_id.clone(), a)
                .unwrap_or_else(|| unreachable!());
            amount = amount.saturating_sub(a);
            if amount == 0 {
                return true;
            }
        }
        add.add(owner_id, amount).is_some()
    }

    // Finalizes transfer of this token, or returns unmatched delta.
    // If returned delta is zero, then overflow happened
    pub fn finalize_into(self, token_id: &TokenId, transfers: &mut Transfers) -> Result<(), i128> {
        // sort deposits and withdrawals in descending order
        let [mut deposits, mut withdrawals] = [self.deposits, self.withdrawals].map(|amounts| {
            let mut amounts: Vec<_> = amounts.into_iter().collect();
            amounts.sort_unstable_by_key(|(_, amount)| Reverse(*amount));
            amounts.into_iter()
        });

        // take first sender and receiver
        let (mut deposit, mut withdraw) = (deposits.next(), withdrawals.next());

        // as long as there is both: sender and receiver
        while let (Some((sender, send)), Some((receiver, receive))) =
            (withdraw.as_mut(), deposit.as_mut())
        {
            // get min amount and transfer
            let transfer = (*send).min(*receive);
            transfers
                .transfer(sender.clone(), receiver.clone(), token_id.clone(), transfer)
                // no error can happen since we add only one transfer for each
                // combination of (sender, receiver, token_id)
                .unwrap_or_else(|| unreachable!());

            // subtract amount from sender and receiver
            *send = send.saturating_sub(transfer);
            *receive = receive.saturating_sub(transfer);

            if *send == 0 {
                // select next sender
                withdraw = withdrawals.next();
            }
            if *receive == 0 {
                // select next receiver
                deposit = deposits.next();
            }
        }

        // only sender(s) left
        if let Some((_, send)) = withdraw {
            return Err(withdrawals
                .try_fold(send, |total, (_, s)| total.checked_add(s))
                .and_then(|total| i128::try_from(total).ok())
                .and_then(i128::checked_neg)
                .unwrap_or_default());
        }
        // only receiver(s) left
        if let Some((_, receive)) = deposit {
            return Err(deposits
                .try_fold(receive, |total, (_, r)| total.checked_add(r))
                .and_then(|total| i128::try_from(total).ok())
                .unwrap_or_default());
        }

        Ok(())
    }
}

/// Raw transfers between accounts
#[must_use]
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Transfers(
    /// `sender_id` -> `receiver_id` -> `token_id` -> `amount`
    HashMap<AccountId, HashMap<AccountId, Amounts<HashMap<TokenId, u128>>>>,
);

impl Transfers {
    #[must_use]
    pub fn transfer(
        &mut self,
        sender_id: AccountId,
        receiver_id: AccountId,
        token_id: TokenId,
        amount: u128,
    ) -> Option<u128> {
        let mut sender = self.0.entry_or_default(sender_id);
        let mut receiver = sender.entry_or_default(receiver_id);
        receiver.add(token_id, amount)
    }

    pub fn with_transfer(
        mut self,
        sender_id: AccountId,
        receiver_id: AccountId,
        token_id: TokenId,
        amount: u128,
    ) -> Option<Self> {
        self.transfer(sender_id, receiver_id, token_id, amount)?;
        Some(self)
    }

    pub fn as_mt_event(&self) -> Option<MtEvent<'_>> {
        if self.0.is_empty() {
            return None;
        }
        Some(MtEvent::MtTransfer(
            self.0
                .iter()
                .flat_map(|(sender_id, transfers)| iter::repeat(sender_id).zip(transfers))
                .map(|(sender_id, (receiver_id, transfers))| {
                    let (token_ids, amounts) = transfers
                        .iter()
                        .map(|(token_id, amount)| (token_id.to_string(), U128(*amount)))
                        .unzip();
                    MtTransferEvent {
                        authorized_id: None,
                        old_owner_id: Cow::Borrowed(sender_id),
                        new_owner_id: Cow::Borrowed(receiver_id),
                        token_ids: Cow::Owned(token_ids),
                        amounts: Cow::Owned(amounts),
                        memo: None,
                    }
                })
                .collect::<Vec<_>>()
                .into(),
        ))
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
#[serde(tag = "error", rename_all = "snake_case")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InvariantViolated {
    UnmatchedDeltas {
        #[serde_as(as = "Amounts<BTreeMap<_, DisplayFromStr>>")]
        unmatched_deltas: TokenDeltas,
    },
    Overflow,
}

impl InvariantViolated {
    #[inline]
    pub const fn as_unmatched_deltas(&self) -> Option<&TokenDeltas> {
        match self {
            Self::UnmatchedDeltas {
                unmatched_deltas: deltas,
            } => Some(deltas),
            Self::Overflow => None,
        }
    }

    #[inline]
    pub fn into_unmatched_deltas(self) -> Option<TokenDeltas> {
        match self {
            Self::UnmatchedDeltas {
                unmatched_deltas: deltas,
            } => Some(deltas),
            Self::Overflow => None,
        }
    }
}

#[cfg(test)]
#[allow(clippy::many_single_char_names)]
mod tests {
    use super::*;

    #[test]
    fn test_transfers() {
        let mut transfers = TransferMatcher::default();
        let [a, b, c, d, e, f, g]: [AccountId; 7] =
            ["a", "b", "c", "d", "e", "f", "g"].map(|s| format!("{s}.near").parse().unwrap());
        let [ft1, ft2] =
            ["ft1", "ft2"].map(|a| TokenId::Nep141(format!("{a}.near").parse().unwrap()));

        let deltas: HashMap<AccountId, TokenDeltas> = [
            (&a, [(&ft1, -5), (&ft2, 1)].as_slice()),
            (&b, [(&ft1, 4), (&ft2, -1)].as_slice()),
            (&c, [(&ft1, 3)].as_slice()),
            (&d, [(&ft1, -10)].as_slice()),
            (&e, [(&ft1, -1)].as_slice()),
            (&f, [(&ft1, 10)].as_slice()),
            (&g, [(&ft1, -1)].as_slice()),
        ]
        .into_iter()
        .map(|(owner_id, deltas)| {
            (
                owner_id.clone(),
                TokenDeltas::default()
                    .with_apply_deltas(
                        deltas
                            .iter()
                            .map(|(token_id, delta)| ((*token_id).clone(), *delta)),
                    )
                    .unwrap(),
            )
        })
        .collect();

        for (owner, (token_id, delta)) in deltas
            .iter()
            .flat_map(|(owner_id, deltas)| iter::repeat(owner_id).zip(deltas))
        {
            assert!(transfers.add_delta(owner.clone(), token_id.clone(), *delta));
        }

        let transfers = transfers.finalize().unwrap();
        let mut new_deltas: HashMap<AccountId, TokenDeltas> = HashMap::new();

        for (sender_id, transfers) in transfers.0 {
            for (receiver_id, amounts) in transfers {
                for (token_id, amount) in amounts {
                    new_deltas
                        .entry_or_default(sender_id.clone())
                        .sub(token_id.clone(), amount)
                        .unwrap();

                    new_deltas
                        .entry_or_default(receiver_id.clone())
                        .add(token_id, amount)
                        .unwrap();
                }
            }
        }

        assert_eq!(new_deltas, deltas);
    }

    #[test]
    fn test_unmatched() {
        let mut deltas = TransferMatcher::default();
        let [a, b, _c, d, e, f, g]: [AccountId; 7] =
            ["a", "b", "c", "d", "e", "f", "g"].map(|s| format!("{s}.near").parse().unwrap());
        let [ft1, ft2] =
            ["ft1", "ft2"].map(|a| TokenId::Nep141(format!("{a}.near").parse().unwrap()));

        for (owner, token_id, delta) in [
            (&a, &ft1, -5),
            (&b, &ft1, 4),
            (&d, &ft1, -10),
            (&e, &ft1, -1),
            (&f, &ft1, 10),
            (&g, &ft1, -1),
            (&a, &ft2, -1),
        ] {
            assert!(deltas.add_delta(owner.clone(), token_id.clone(), delta));
        }

        assert_eq!(
            deltas.finalize().unwrap_err(),
            InvariantViolated::UnmatchedDeltas {
                unmatched_deltas: TokenDeltas::default()
                    .with_apply_delta(ft1.clone(), -3)
                    .unwrap()
                    .with_apply_delta(ft2.clone(), -1)
                    .unwrap()
            }
        );
    }
}
