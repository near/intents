use near_account_id::AccountId;
use near_kit::{AccessKeyPermissionView, BlockReference, RpcError};
#[cfg(feature = "tracing")]
use tracing::instrument;

use crate::{
    access_keys::{AccessKeyAuthorization, PublicKey},
    resolver::{ResolveErrorKind, RpcResolver},
};

impl RpcResolver {
    // TODO: add it to docs
    // TODO: full access key takes precedence over resolved by w_resolve_auth?
    // TODO: tracing
    // TODO: return optional?
    #[cfg_attr(feature = "tracing", instrument(level = "DEBUG", skip_all))]
    pub(super) async fn resolve_access_key(
        &self,
        account_id: &AccountId,
        // TODO: slice?
        path: &[AccountId],
        auth: AccessKeyAuthorization,
        block: BlockReference,
    ) -> Result<String, ResolveErrorKind> {
        // check chain_id
        if auth.msg.chain_id != self.chain_id {
            return Err(AccessKeyError::InvalidChainId.into());
        }

        // check signer_id
        if auth.msg.signer_id != *account_id {
            return Err(AccessKeyError::InvalidSignerId(auth.msg.signer_id).into());
        }

        // check reversed path
        if !auth.msg.path.iter().eq(path.iter().rev()) {
            return Err(AccessKeyError::InvalidPath.into());
        }

        // TODO: check timestamp of a block

        // verify signature
        if !auth.verify() {
            return Err(AccessKeyError::InvalidSignature.into());
        }

        // check access key
        let is_full_access = match self
            .client
            .view_access_key(account_id, &auth.public_key.clone().into(), block)
            .await
        {
            // Access key exists -> allow only if it has FullAccess permission.
            Ok(access_key) => matches!(
                access_key.permission,
                AccessKeyPermissionView::FullAccess
                    | AccessKeyPermissionView::GasKeyFullAccess { .. }
            ),

            // Account exists but it doesn't have this public key added as an access key -> reject.
            // Even if this is an implicit account ID derived from this public key, it could have
            // been deleted by the owner.
            //
            // TODO: A Universal Implicit AccountId can be first "created" by incoming transfer,
            // and only later initialized via StateInit. So, we need to fetch account's metadata
            // and fallback to `Err(AccountNotFound(_))` branch below if "initialized" flag is
            // not set.
            Err(RpcError::AccessKeyNotFound { .. }) => false,

            // Account doesn't exist on-chain yet -> allow only if it's an implicit account derived
            // from this public key, since it can be initialized any time in the future.
            Err(RpcError::AccountNotFound(_)) => {
                // TODO: or check if we have `self.state_inits.get(&account_id)` with this public
                // key added, since it can be just one of them
                auth.public_key.to_implicit_account_id() == *account_id
            }

            // Other RPC error -> reject.
            Err(err) => return Err(err.into()),
        };

        if !is_full_access {
            return Err(AccessKeyError::NoFullAccess(auth.public_key).into());
        }

        // authorize signed payload
        Ok(auth.msg.payload)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum AccessKeyError {
    #[error("invalid chain_id")]
    InvalidChainId,

    #[error("invalid path")]
    InvalidPath,

    #[error("invalid signer_id: {0}")]
    InvalidSignerId(AccountId),

    #[error("invalid signature")]
    InvalidSignature,

    #[error("access key without FullAccess permission: {0}")]
    NoFullAccess(PublicKey),
}
