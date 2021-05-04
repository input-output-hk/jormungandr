use crate::{
    blockchain::StorageError,
    intercom::{self, TransactionMsg},
    rest::Context,
};
use chain_crypto::{digest::Error as DigestError, hash::Error as HashError, PublicKeyFromStrError};
use chain_impl_mockchain::{fragment::FragmentId, value::ValueError};
use futures::{channel::mpsc::SendError, channel::mpsc::TrySendError, prelude::*};
use jormungandr_lib::interfaces::{
    FragmentLog, FragmentOrigin, FragmentStatus, FragmentsBatch, FragmentsProcessingSummary,
};
use std::{collections::HashMap, str::FromStr};
use tracing::{span, Level};
use tracing_futures::Instrument;

#[allow(clippy::large_enum_variant)]
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    ContextError(#[from] crate::rest::context::Error),
    #[error(transparent)]
    PublicKey(#[from] PublicKeyFromStrError),
    #[error(transparent)]
    IntercomError(#[from] intercom::Error),
    #[error(transparent)]
    TxMsgSendError(#[from] TrySendError<TransactionMsg>),
    #[error(transparent)]
    MsgSendError(#[from] SendError),
    #[error("Block value calculation error")]
    Value(#[from] ValueError),
    #[error(transparent)]
    Hash(#[from] HashError),
    #[error(transparent)]
    Digest(#[from] DigestError),
    #[error(transparent)]
    Storage(#[from] StorageError),
    #[error(transparent)]
    Hex(#[from] hex::FromHexError),
    #[error("Could not process all fragments")]
    Fragments(FragmentsProcessingSummary),
}

pub async fn get_fragment_statuses<'a>(
    context: &Context,
    ids: impl IntoIterator<Item = &'a str>,
) -> Result<HashMap<String, FragmentStatus>, Error> {
    let ids = ids
        .into_iter()
        .map(|s| FragmentId::from_str(s))
        .collect::<Result<Vec<_>, _>>()?;
    let span = span!(parent: context.span()?, Level::TRACE, "fragment_statuses", request = "message_statuses");
    async move {
        let (reply_handle, reply_future) = intercom::unary_reply();
        let mut mbox = context.try_full()?.transaction_task.clone();
        mbox.send(TransactionMsg::GetStatuses(ids, reply_handle))
            .await
            .map_err(|e| {
                tracing::debug!(reason = %e, "error getting message statuses");
                Error::MsgSendError(e)
            })?;
        reply_future
            .await
            .map_err(Into::into)
            .map(|result_intermediate| {
                let mut result = HashMap::new();
                result_intermediate.into_iter().for_each(|(k, v)| {
                    result.insert(k.to_string(), v);
                });
                result
            })
    }
    .instrument(span)
    .await
}

pub async fn post_fragments(
    context: &Context,
    batch: FragmentsBatch,
) -> Result<FragmentsProcessingSummary, Error> {
    let mut msgbox = context.try_full()?.transaction_task.clone();
    let (reply_handle, reply_future) = intercom::unary_reply();
    let msg = TransactionMsg::SendTransactions {
        origin: FragmentOrigin::Rest,
        fragments: batch.fragments,
        fail_fast: batch.fail_fast,
        reply_handle,
    };
    msgbox.try_send(msg)?;
    let reply = reply_future.await?;
    if reply.rejected.is_empty() {
        Ok(reply)
    } else {
        Err(Error::Fragments(reply))
    }
}

pub async fn get_fragment_logs(context: &Context) -> Result<Vec<FragmentLog>, Error> {
    let span =
        span!(parent: context.span()?, Level::TRACE, "fragment_logs", request = "fragment_logs");
    async move {
        let (reply_handle, reply_future) = intercom::unary_reply();
        let mut mbox = context.try_full()?.transaction_task.clone();
        mbox.send(TransactionMsg::GetLogs(reply_handle))
            .await
            .map_err(|e| {
                tracing::debug!(reason = %e, "error getting fragment logs");
                Error::MsgSendError(e)
            })?;
        reply_future.await.map_err(Into::into)
    }
    .instrument(span)
    .await
}
