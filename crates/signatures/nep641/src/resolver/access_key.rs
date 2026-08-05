use futures::join;
use near_account_id::AccountId;
use near_kit::{AccessKeyPermissionView, BlockReference, RpcError};
#[cfg(feature = "tracing")]
use tracing::instrument;

use crate::{
    AuthorizationResolution,
    access_keys::{AccessKeyAuthorization, PublicKey},
    resolver::{ResolveErrorKind, Resolved, RpcResolver},
};

impl RpcResolver {
    /// Try to resolve a single authorization via `FullAccessKey`
    #[cfg_attr(feature = "tracing", instrument(level = "DEBUG", skip_all))]
    pub(super) async fn resolve_access_key(
        &self,
        account_id: &AccountId,
        path: &[AccountId],
        auth: &str,
        block: BlockReference,
    ) -> Result<Resolved, ResolveErrorKind> {
        let auth: AccessKeyAuthorization = serde_json::from_str(auth)?;

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

        // verify signature
        if !auth.verify() {
            return Err(AccessKeyError::InvalidSignature.into());
        }

        let (block, access_key) = {
            let rpc_pk = auth.public_key.clone().into();
            if let BlockReference::Hash(block_hash) = block {
                // fetch the block concurrently with access key only if block_hash is already known
                join!(
                    self.client.block(block_hash.into()),
                    self.client
                        .view_access_key(account_id, &rpc_pk, block_hash.into())
                )
            } else {
                // otherwise, fetch the block first
                let block = self.client.block(block.clone()).await?;
                // and then the access key against fetched block hash
                let access_key = self
                    .client
                    .view_access_key(account_id, &rpc_pk, block.header.hash.into())
                    .await;
                (Ok(block), access_key)
            }
        };
        let block = block?;

        // check timestamp
        if auth.msg.timestamp.as_nanos() > block.header.timestamp.into() {
            return Err(AccessKeyError::FromTheFuture.into());
        }

        // check access key
        let is_full_access = match access_key {
            // Access key exists -> allow only if it has FullAccess permission.
            Ok(ref access_key) => matches!(
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

        // authorize signed payload, without any pending sub-authorizations
        Ok(Resolved {
            res: AuthorizationResolution::new(auth.msg.payload),
            block_hash: block.header.hash,
            block_height: block.header.height,
        })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum AccessKeyError {
    #[error("message is from the future")]
    FromTheFuture,

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
