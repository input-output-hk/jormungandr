use crate::db::{self, chain_storable::VotePlanMeta, schema::Txn};

use async_graphql::{Context, FieldResult, Object, Union};
use std::{convert::TryFrom, sync::Arc};
use tokio::sync::Mutex;

use super::scalars::{PayloadType, PoolId, PublicKey, TimeOffsetSeconds, VotePlanId};
use super::{Address, BlockDate, Pool, Proposal, TaxType};

// interface for grouping certificates as a graphl union
#[derive(Union)]
pub enum Certificate {
    VotePlan(VotePlanCertificate),
    PublicVoteCast(PublicVoteCastCertificate),
    PrivateVoteCast(PrivateVoteCastCertificate),
}

pub struct VotePlanCertificate {
    pub data: db::chain_storable::StorableHash,
    pub txn: Arc<Txn>,
    pub meta: Mutex<Option<VotePlanMeta>>,
}

pub struct PublicVoteCastCertificate {
    pub data: db::chain_storable::PublicVoteCast,
}

pub struct PrivateVoteCastCertificate {
    pub data: db::chain_storable::PrivateVoteCast,
}

impl VotePlanCertificate {
    pub async fn get_meta(&self) -> FieldResult<VotePlanMeta> {
        let mut guard = self.meta.lock().await;

        if let Some(meta) = &*guard {
            return Ok(meta.clone());
        }

        let data = self.data.clone();

        let txn = Arc::clone(&self.txn);
        let meta = tokio::task::spawn_blocking(move || {
            txn.get_vote_plan_meta(&data).map(|option| option.cloned())
        })
        .await
        .unwrap()?
        .unwrap();

        *guard = Some(meta.clone());

        Ok(meta)
    }
}

#[Object]
impl VotePlanCertificate {
    /// the vote start validity
    pub async fn vote_start(&self) -> FieldResult<BlockDate> {
        Ok(self.get_meta().await?.vote_start.into())
    }

    /// the duration within which it is possible to vote for one of the proposals
    /// of this voting plan.
    pub async fn vote_end(&self) -> FieldResult<BlockDate> {
        Ok(self.get_meta().await?.vote_end.into())
    }

    /// the committee duration is the time allocated to the committee to open
    /// the ballots and publish the results on chain
    pub async fn committee_end(&self) -> FieldResult<BlockDate> {
        Ok(self.get_meta().await?.committee_end.into())
    }

    pub async fn payload_type(&self) -> FieldResult<PayloadType> {
        match self.get_meta().await?.payload_type {
            db::chain_storable::PayloadType::Public => Ok(PayloadType::Public),
            db::chain_storable::PayloadType::Private => Ok(PayloadType::Private),
        }
    }

    /// the proposals to vote for
    pub async fn proposals(&self) -> FieldResult<Vec<Proposal>> {
        // TODO: add pagination
        Err(ApiError::Unimplemented.into())
    }
}

#[Object]
impl PublicVoteCastCertificate {
    pub async fn vote_plan(&self) -> VotePlanId {
        self.data.vote_plan_id.clone().into()
    }

    pub async fn proposal_index(&self) -> u8 {
        self.data.proposal_index
    }

    pub async fn choice(&self) -> u8 {
        self.data.choice
    }
}

#[Object]
impl PrivateVoteCastCertificate {
    pub async fn vote_plan(&self) -> VotePlanId {
        self.data.vote_plan_id.clone().into()
    }

    pub async fn proposal_index(&self) -> u8 {
        self.data.proposal_index
    }
}
