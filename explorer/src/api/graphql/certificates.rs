use crate::db;

use super::{error::ApiError, extract_context};
use async_graphql::{Context, FieldResult, Object, Union};
use chain_impl_mockchain::certificate;
use std::convert::TryFrom;

use super::scalars::{PayloadType, PoolId, PublicKey, TimeOffsetSeconds, VotePlanId};
use super::{Address, BlockDate, Pool, Proposal, TaxType};

// interface for grouping certificates as a graphl union
#[derive(Union)]
pub enum Certificate {
    VotePlan(VotePlanCertificate),
    PublicVoteCast(VoteCastCertificate),
}

pub struct VotePlanCertificate {
    id: db::chain_storable::VotePlanId,
}

pub struct VoteCastCertificate {
    data: db::chain_storable::PublicVoteCast,
}

#[Object]
impl VotePlanCertificate {
    /// the vote start validity
    pub async fn vote_start(&self) -> BlockDate {
        todo!()
    }

    /// the duration within which it is possible to vote for one of the proposals
    /// of this voting plan.
    pub async fn vote_end(&self) -> BlockDate {
        todo!()
    }

    /// the committee duration is the time allocated to the committee to open
    /// the ballots and publish the results on chain
    pub async fn committee_end(&self) -> BlockDate {
        todo!()
    }

    pub async fn payload_type(&self) -> PayloadType {
        todo!()
    }

    // /// the proposals to vote for
    // pub async fn proposals(&self) -> Vec<Proposal> {
    //     todo!()
    // }
}

#[Object]
impl VoteCastCertificate {
    pub async fn vote_plan(&self) -> VotePlanId {
        todo!()
    }

    pub async fn proposal_index(&self) -> i32 {
        todo!()
    }
}
