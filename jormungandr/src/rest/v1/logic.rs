use crate::{
    blockchain::StorageError,
    intercom::{self, TransactionMsg},
    rest::Context,
};
use chain_crypto::{
    digest::Error as DigestError, hash::Error as HashError, PublicKey, PublicKeyFromStrError,
};
use chain_impl_mockchain::{
    account::{AccountAlg, Identifier},
    fragment::FragmentId,
    value::ValueError,
};
use futures::{channel::mpsc::SendError, channel::mpsc::TrySendError, prelude::*};
use jormungandr_lib::interfaces::{
    FragmentLog, FragmentOrigin, FragmentStatus, FragmentsBatch, FragmentsProcessingSummary,
    VotePlanId,
};
use std::{collections::HashMap, convert::TryInto, str::FromStr};
use tracing::{span, Level};
use tracing_futures::Instrument;

#[allow(clippy::large_enum_variant)]
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Context(#[from] crate::rest::context::Error),
    #[error(transparent)]
    PublicKey(#[from] PublicKeyFromStrError),
    #[error(transparent)]
    Intercom(#[from] intercom::Error),
    #[error(transparent)]
    TxMsgSend(#[from] TrySendError<TransactionMsg>),
    #[error(transparent)]
    MsgSend(#[from] SendError),
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

fn parse_account_id(id_hex: &str) -> Result<Identifier, Error> {
    PublicKey::<AccountAlg>::from_str(id_hex)
        .map(Into::into)
        .map_err(Into::into)
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
                Error::MsgSend(e)
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
    if reply.is_error() {
        Err(Error::Fragments(reply))
    } else {
        Ok(reply)
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
                Error::MsgSend(e)
            })?;
        reply_future.await.map_err(Into::into)
    }
    .instrument(span)
    .await
}

pub async fn get_account_votes(
    context: &Context,
    vote_plan_id: VotePlanId,
    account_id_hex: String,
) -> Result<Option<Vec<u8>>, Error> {
    let span = span!(parent: context.span()?, Level::TRACE, "get_account_votes", request = "get_account_votes");

    let identifier =
        chain_impl_mockchain::transaction::UnspecifiedAccountIdentifier::from_single_account(
            parse_account_id(&account_id_hex)?,
        );

    let vote_plan_id: chain_crypto::digest::DigestOf<_, _> = vote_plan_id.into_digest().into();

    async move {
        let maybe_vote_plan = context
            .blockchain_tip()?
            .get_ref()
            .await
            .ledger()
            .active_vote_plans()
            .into_iter()
            .find(|x| x.id == vote_plan_id);
        let vote_plan = match maybe_vote_plan {
            Some(vote_plan) => vote_plan,
            None => return Ok(None),
        };
        let result = vote_plan
            .proposals
            .into_iter()
            .enumerate()
            .filter(|(_, x)| x.votes.contains_key(&identifier))
            .map(|(i, _)| i.try_into().unwrap())
            .collect();
        Ok(Some(result))
    }
    .instrument(span)
    .await
}
