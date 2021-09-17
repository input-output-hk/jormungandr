use super::{
    error::ApiError,
    scalars::{PayloadType, VotePlanId},
    BlockDate, Proposal,
};
use crate::db::{self, chain_storable::VotePlanMeta, schema::Txn};
use async_graphql::{FieldResult, Object, Union};
use std::sync::Arc;
use tokio::sync::Mutex;

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

#[Object]
impl VotePlanCertificate {
    /// the vote start validity
    pub async fn vote_start(&self) -> FieldResult<BlockDate> {
        Err(ApiError::Unimplemented.into())
    }

    /// the duration within which it is possible to vote for one of the proposals
    /// of this voting plan.
    pub async fn vote_end(&self) -> FieldResult<BlockDate> {
        Err(ApiError::Unimplemented.into())
    }

    /// the committee duration is the time allocated to the committee to open
    /// the ballots and publish the results on chain
    pub async fn committee_end(&self) -> FieldResult<BlockDate> {
        Err(ApiError::Unimplemented.into())
    }

    pub async fn payload_type(&self) -> FieldResult<PayloadType> {
        Err(ApiError::Unimplemented.into())
    }

    /// the proposals to vote for
    pub async fn proposals(&self) -> FieldResult<Vec<Proposal>> {
        Err(ApiError::Unimplemented.into())
    }
}

#[Object]
impl PublicVoteCastCertificate {
    pub async fn vote_plan(&self) -> FieldResult<VotePlanId> {
        Err(ApiError::Unimplemented.into())
    }

    pub async fn proposal_index(&self) -> FieldResult<u8> {
        Err(ApiError::Unimplemented.into())
    }

    pub async fn choice(&self) -> FieldResult<u8> {
        Err(ApiError::Unimplemented.into())
    }
}

#[Object]
impl PrivateVoteCastCertificate {
    pub async fn vote_plan(&self) -> FieldResult<VotePlanId> {
        Err(ApiError::Unimplemented.into())
    }

    pub async fn proposal_index(&self) -> FieldResult<u8> {
        Err(ApiError::Unimplemented.into())
    }
}
