use crate::{
    blockchain::StorageError,
    intercom::{self, TransactionMsg},
    rest::Context,
};
use chain_core::property::{Deserialize, Fragment as _};
use chain_crypto::{digest::Error as DigestError, hash::Error as HashError, PublicKeyFromStrError};
use chain_impl_mockchain::{
    fragment::{Fragment, FragmentId},
    value::ValueError,
};
use futures::{channel::mpsc::SendError, channel::mpsc::TrySendError, prelude::*};
use jormungandr_lib::interfaces::{FragmentLog, FragmentOrigin, FragmentStatus};
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
    Deserialize(std::io::Error),
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
}

pub async fn get_fragments_statuses(
    context: &Context,
    ids: Vec<String>,
) -> Result<HashMap<String, FragmentStatus>, Error> {
    let ids = ids
        .into_iter()
        .map(|s| FragmentId::from_str(&s))
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
    messages: Vec<String>,
) -> Result<Vec<String>, Error> {
    let fragments = messages
        .into_iter()
        .map(|message| {
            let message = hex::decode(message)?;
            Fragment::deserialize(message.as_slice()).map_err(Error::Deserialize)
        })
        .collect::<Result<Vec<_>, _>>()?;
    let fragment_ids = fragments
        .iter()
        .map(|fragment| fragment.id().to_string())
        .collect();
    let mut msgbox = context.try_full()?.transaction_task.clone();
    for fragment in fragments.into_iter() {
        let msg = TransactionMsg::SendTransaction(FragmentOrigin::Rest, vec![fragment]);
        msgbox.try_send(msg)?;
    }
    Ok(fragment_ids)
}

pub async fn get_fragments_logs(context: &Context) -> Result<Vec<FragmentLog>, Error> {
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
