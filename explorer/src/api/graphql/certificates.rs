use super::error::Error;
use super::scalars::{PayloadType, PoolId, PublicKey, TimeOffsetSeconds, VotePlanId};
use super::{Address, BlockDate, Context, ExplorerAddress, Pool, Proposal, TaxType};
use chain_impl_mockchain::certificate;
use juniper::graphql_union;
use juniper::FieldResult;
use std::convert::TryFrom;

// interface for grouping certificates as a graphl union
pub enum Certificate {
    StakeDelegation(StakeDelegation),
    OwnerStakeDelegation(OwnerStakeDelegation),
    PoolRegistration(Box<PoolRegistration>),
    PoolRetirement(PoolRetirement),
    PoolUpdate(Box<PoolUpdate>),
    VotePlan(VotePlan),
    VoteCast(VoteCast),
    VoteTally(VoteTally),
}

pub struct StakeDelegation(certificate::StakeDelegation);

pub struct PoolRegistration(certificate::PoolRegistration);

pub struct OwnerStakeDelegation(certificate::OwnerStakeDelegation);

/// Retirement info for a pool
pub struct PoolRetirement(certificate::PoolRetirement);

pub struct PoolUpdate(certificate::PoolUpdate);

pub struct VotePlan(certificate::VotePlan);

pub struct VoteCast(certificate::VoteCast);

pub struct VoteTally(certificate::VoteTally);

graphql_union!(Certificate: Context |&self| {
    // the left hand side of the `instance_resolvers` match-like pub structure is the one
    // that's used to match in the graphql query with the `__typename` field
    instance_resolvers: |_| {
        &StakeDelegation => match *self { Certificate::StakeDelegation(ref c) => Some(c), _ => None },
        &OwnerStakeDelegation => match *self { Certificate::OwnerStakeDelegation(ref c) => Some(c), _ => None },
        &PoolRegistration => match *self { Certificate::PoolRegistration(ref c) => Some(&**c), _ => None },
        &PoolUpdate => match *self { Certificate::PoolUpdate(ref c) => Some(&**c), _ => None},
        &PoolRetirement => match *self { Certificate::PoolRetirement(ref c) => Some(c), _ => None},
        &VotePlan => match *self { Certificate::VotePlan(ref c) => Some(c), _ => None},
        &VoteCast => match *self { Certificate::VoteCast(ref c) => Some(c), _ => None},
        &VoteTally => match *self { Certificate::VoteTally(ref c) => Some(c), _ => None},
    }
});

#[juniper::object(
    Context = Context,
)]
impl StakeDelegation {
    // FIXME: Maybe a new Account type would be better?
    pub fn account(&self, context: &Context) -> FieldResult<Address> {
        let discrimination = context.db.blockchain_config.discrimination;
        self.0
            .account_id
            .to_single_account()
            .ok_or_else(||
                // TODO: Multisig address?
                Error::Unimplemented("account stake delegation".to_owned()))
            .map(|single| {
                chain_addr::Address(discrimination, chain_addr::Kind::Account(single.into()))
            })
            .map(|addr| Address::from(&ExplorerAddress::New(addr)))
            .map_err(|e| e.into())
    }

    pub fn pools(&self, context: &Context) -> Vec<Pool> {
        use chain_impl_mockchain::account::DelegationType;
        use std::iter::FromIterator as _;

        match self.0.get_delegation_type() {
            DelegationType::NonDelegated => vec![],
            DelegationType::Full(id) => vec![Pool::from_valid_id(id.clone())],
            DelegationType::Ratio(delegation_ratio) => Vec::from_iter(
                delegation_ratio
                    .pools()
                    .iter()
                    .cloned()
                    .map(|(p, _)| Pool::from_valid_id(p)),
            ),
        }
    }
}

#[juniper::object(
    Context = Context,
)]
impl PoolRegistration {
    pub fn pool(&self, context: &Context) -> Pool {
        Pool::from_valid_id(self.0.to_id())
    }

    /// Beginning of validity for this pool, this is used
    /// to keep track of the period of the expected key and the expiry
    pub fn start_validity(&self) -> TimeOffsetSeconds {
        self.0.start_validity.into()
    }

    /// Management threshold for owners, this need to be <= #owners and > 0
    pub fn management_threshold(&self) -> i32 {
        // XXX: u8 fits in i32, but maybe some kind of custom scalar is better?
        self.0.management_threshold().into()
    }

    /// Owners of this pool
    pub fn owners(&self) -> Vec<PublicKey> {
        self.0.owners.iter().map(PublicKey::from).collect()
    }

    pub fn operators(&self) -> Vec<PublicKey> {
        self.0.operators.iter().map(PublicKey::from).collect()
    }

    pub fn rewards(&self) -> TaxType {
        TaxType(self.0.rewards)
    }

    /// Reward account
    pub fn reward_account(&self, context: &Context) -> Option<Address> {
        use chain_impl_mockchain::transaction::AccountIdentifier;
        let discrimination = context.db.blockchain_config.discrimination;

        // FIXME: Move this transformation to a point earlier

        self.0
            .reward_account
            .clone()
            .map(|acc_id| match acc_id {
                AccountIdentifier::Single(d) => ExplorerAddress::New(chain_addr::Address(
                    discrimination,
                    chain_addr::Kind::Account(d.into()),
                )),
                AccountIdentifier::Multi(d) => {
                    let mut bytes = [0u8; 32];
                    bytes.copy_from_slice(&d.as_ref()[0..32]);
                    ExplorerAddress::New(chain_addr::Address(
                        discrimination,
                        chain_addr::Kind::Multisig(bytes),
                    ))
                }
            })
            .map(|explorer_address| Address {
                id: explorer_address,
            })
    }

    // Genesis Praos keys
    // pub keys: GenesisPraosLeader,
}

#[juniper::object(
    Context = Context,
)]
impl OwnerStakeDelegation {
    fn pools(&self) -> Vec<Pool> {
        use chain_impl_mockchain::account::DelegationType;
        use std::iter::FromIterator as _;

        match self.0.get_delegation_type() {
            DelegationType::NonDelegated => vec![],
            DelegationType::Full(id) => vec![Pool::from_valid_id(id.clone())],
            DelegationType::Ratio(delegation_ratio) => Vec::from_iter(
                delegation_ratio
                    .pools()
                    .iter()
                    .cloned()
                    .map(|(p, _)| Pool::from_valid_id(p)),
            ),
        }
    }
}

#[juniper::object(
    Context = Context,
)]
impl PoolRetirement {
    pub fn pool_id(&self) -> PoolId {
        PoolId(format!("{}", self.0.pool_id))
    }

    pub fn retirement_time(&self) -> TimeOffsetSeconds {
        self.0.retirement_time.into()
    }
}

#[juniper::object(
    Context = Context,
)]
impl PoolUpdate {
    pub fn pool_id(&self) -> PoolId {
        PoolId(format!("{}", self.0.pool_id))
    }

    pub fn start_validity(&self) -> TimeOffsetSeconds {
        self.0.new_pool_reg.start_validity.into()
    }

    // TODO: Previous keys?
    // TODO: Updated keys?
}

#[juniper::object(
    Context = Context,
)]
impl VotePlan {
    /// the vote start validity
    pub fn vote_start(&self) -> BlockDate {
        self.0.vote_start().into()
    }

    /// the duration within which it is possible to vote for one of the proposals
    /// of this voting plan.
    pub fn vote_end(&self) -> BlockDate {
        self.0.vote_end().into()
    }

    /// the committee duration is the time allocated to the committee to open
    /// the ballots and publish the results on chain
    pub fn committee_end(&self) -> BlockDate {
        self.0.committee_end().into()
    }

    pub fn payload_type(&self) -> PayloadType {
        self.0.payload_type().into()
    }

    /// the proposals to vote for
    pub fn proposals(&self) -> Vec<Proposal> {
        self.0.proposals().iter().cloned().map(Proposal).collect()
    }
}

#[juniper::object(
    Context = Context,
)]
impl VoteCast {
    pub fn vote_plan(&self) -> VotePlanId {
        self.0.vote_plan().clone().into()
    }

    pub fn proposal_index(&self) -> i32 {
        self.0.proposal_index() as i32
    }
}

#[juniper::object(
    Context = Context,
)]
impl VoteTally {
    pub fn vote_plan(&self) -> VotePlanId {
        self.0.id().clone().into()
    }
}

/*------------------------------*/
/*------- Conversions ---------*/
/*----------------------------*/

impl TryFrom<chain_impl_mockchain::certificate::Certificate> for Certificate {
    type Error = super::error::Error;
    fn try_from(
        original: chain_impl_mockchain::certificate::Certificate,
    ) -> Result<Certificate, Self::Error> {
        match original {
            certificate::Certificate::StakeDelegation(c) => {
                Ok(Certificate::StakeDelegation(StakeDelegation(c)))
            }
            certificate::Certificate::OwnerStakeDelegation(c) => {
                Ok(Certificate::OwnerStakeDelegation(OwnerStakeDelegation(c)))
            }
            certificate::Certificate::PoolRegistration(c) => {
                Ok(Certificate::PoolRegistration(Box::new(PoolRegistration(c))))
            }
            certificate::Certificate::PoolRetirement(c) => {
                Ok(Certificate::PoolRetirement(PoolRetirement(c)))
            }
            certificate::Certificate::PoolUpdate(c) => {
                Ok(Certificate::PoolUpdate(Box::new(PoolUpdate(c))))
            }
            certificate::Certificate::VotePlan(c) => Ok(Certificate::VotePlan(VotePlan(c))),
            certificate::Certificate::VoteCast(c) => Ok(Certificate::VoteCast(VoteCast(c))),
            certificate::Certificate::VoteTally(c) => Ok(Certificate::VoteTally(VoteTally(c))),
        }
    }
}

impl From<certificate::StakeDelegation> for StakeDelegation {
    fn from(delegation: certificate::StakeDelegation) -> StakeDelegation {
        StakeDelegation(delegation)
    }
}

impl From<certificate::OwnerStakeDelegation> for OwnerStakeDelegation {
    fn from(owner_stake_delegation: certificate::OwnerStakeDelegation) -> OwnerStakeDelegation {
        OwnerStakeDelegation(owner_stake_delegation)
    }
}

impl From<certificate::PoolRegistration> for PoolRegistration {
    fn from(registration: certificate::PoolRegistration) -> PoolRegistration {
        PoolRegistration(registration)
    }
}

impl From<certificate::PoolRetirement> for PoolRetirement {
    fn from(pool_retirement: certificate::PoolRetirement) -> PoolRetirement {
        PoolRetirement(pool_retirement)
    }
}

impl From<certificate::PoolUpdate> for PoolUpdate {
    fn from(pool_update: certificate::PoolUpdate) -> PoolUpdate {
        PoolUpdate(pool_update)
    }
}

impl From<certificate::VotePlan> for VotePlan {
    fn from(vote_plan: certificate::VotePlan) -> VotePlan {
        VotePlan(vote_plan)
    }
}

impl From<certificate::VoteCast> for VoteCast {
    fn from(vote_cast: certificate::VoteCast) -> VoteCast {
        VoteCast(vote_cast)
    }
}
